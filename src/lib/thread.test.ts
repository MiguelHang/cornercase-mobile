import { describe, expect, it } from 'vitest'
import type { AgentRow_Serialize as AgentRow, LastMessage_Serialize as LastMessage } from './bindings'
import type { Reading } from './chat.svelte'
import { AT_END_MARGIN, STALL_MS, agentLine, atEnd, compactHeader, isAnswer, sendNews, threadOf } from './thread'

const NOW = Date.parse('2026-11-09T21:30:00.000Z')

const MINUTE = 60_000

const agentIn = (status: string | null, since: string | null = null): AgentRow => ({
  boot: 'b7',
  project: 8,
  workspace: 9,
  tab: 10,
  pane: 11,
  project_name: 'terminal',
  workspace_name: 'thread',
  tab_name: 'line',
  workspace_count: 3,
  agent: 'claude',
  title: 'Fix the chat',
  status,
  since,
  parent: null,
  depth: 0,
  role: null,
  rank_status: status,
})

const ago = (millis: number) => new Date(NOW - millis).toISOString()

const lineOf = (status: string | null, since: string | null = null, lastNews = NOW) =>
  agentLine(agentIn(status, since), NOW, lastNews)

describe('agentLine', () => {
  it.each([
    ['working', 'working', 'claude is working…'],
    ['shell', 'working', 'claude is waiting for a background command…'],
    ['waiting', 'waiting', 'claude is waiting for your answer'],
    ['failed', 'failed', 'claude stopped with an error'],
  ])('says what a %s agent is doing, in its tone', (status, tone, phrase) => {
    expect(lineOf(status)).toEqual({ tone, phrase, time: '', stalled: false })
  })

  it.each(['done', 'pending', 'idle', 'something new', null])('draws no line for %o', (status) => {
    expect(lineOf(status)).toBeNull()
  })

  it.each([
    ['working', 'now'],
    ['shell', '7 min'],
  ])('gives a %s agent the time since its status changed, as the board does', (status, time) => {
    const since = status === 'working' ? ago(40_000) : ago(7 * MINUTE + 5_000)
    expect(lineOf(status, since)?.time).toBe(time)
  })

  it.each(['waiting', 'failed'])('gives a %s agent no time', (status) => {
    expect(lineOf(status, ago(5 * MINUTE))?.time).toBe('')
  })

  it('names the agent the row says', () => {
    const row = { ...agentIn('working'), agent: 'codex' }
    expect(agentLine(row, NOW, NOW)?.phrase).toBe('codex is working…')
  })
})

describe('agentLine, without news', () => {
  it('keeps the working line one millisecond before the threshold', () => {
    expect(lineOf('working', ago(30 * MINUTE), NOW - STALL_MS + 1)).toEqual({
      tone: 'working',
      phrase: 'claude is working…',
      time: '30 min',
      stalled: false,
    })
  })

  it('says there is no news, muted, once the threshold is reached', () => {
    expect(lineOf('working', ago(30 * MINUTE), NOW - STALL_MS)).toEqual({
      tone: 'muted',
      phrase: 'claude is working… · no news',
      time: 'for 2 min · Ctrl-C interrupts',
      stalled: true,
    })
  })

  it('counts the minutes since the last news, not since the status changed', () => {
    expect(lineOf('working', ago(30 * MINUTE), NOW - 9 * MINUTE - 59_000)?.time).toBe('for 9 min · Ctrl-C interrupts')
  })

  it('waits two minutes', () => {
    expect(STALL_MS).toBe(2 * MINUTE)
  })

  it.each(['shell', 'waiting', 'failed'])('never calls a %s agent stalled', (status) => {
    expect(lineOf(status, null, NOW - 10 * STALL_MS)?.stalled).toBe(false)
  })

  it('keeps the phrase while the time runs', () => {
    const since = ago(MINUTE)
    const phrases = [0, MINUTE, 5 * MINUTE].map((later) => agentLine(agentIn('working', since), NOW + later, NOW + later))
    expect(phrases.map((line) => line?.phrase)).toEqual(['claude is working…', 'claude is working…', 'claude is working…'])
    expect(phrases.map((line) => line?.time)).toEqual(['1 min', '2 min', '6 min'])
  })

  it('keeps the stalled phrase while the minutes without news grow', () => {
    const lines = [STALL_MS, 4 * MINUTE].map((quiet) => lineOf('working', null, NOW - quiet))
    expect(lines.map((line) => line?.phrase)).toEqual(['claude is working… · no news', 'claude is working… · no news'])
    expect(lines.map((line) => line?.time)).toEqual(['for 2 min · Ctrl-C interrupts', 'for 4 min · Ctrl-C interrupts'])
  })
})

const said = (text: string, written: string | null = '2026-11-09T21:20:00.000Z'): LastMessage => ({
  pane: 11,
  agent: 'claude',
  text,
  written,
  turn_over: true,
})

const shown = (message: LastMessage): Reading => ({ kind: 'message', message })

const BEFORE = said('Which file should I fix?')

const LOADING: Reading = { kind: 'loading' }

const UNREADABLE: Reading = { kind: 'unreadable', reason: 'no transcript' }

describe('sendNews', () => {
  it.each([
    ['sending', null, 'Sending'],
    ['sent', null, 'Sent'],
    ['failed', 'the agent is gone', 'Not sent: the agent is gone'],
    ['failed', null, 'Not sent'],
  ] as const)('says a message %s with reason %o as %o', (state, reason, news) => {
    expect(sendNews({ state, reason })).toBe(news)
  })

  it('says nothing before anything is sent', () => {
    expect(sendNews(null)).toBe('')
  })
})

describe('isAnswer', () => {
  it('takes the message seen when sending for no answer', () => {
    expect(isAnswer(BEFORE, shown({ ...BEFORE }))).toBe(false)
  })

  it('takes a message with other text for the answer', () => {
    expect(isAnswer(BEFORE, shown({ ...BEFORE, text: 'Fixed thread.ts.' }))).toBe(true)
  })

  it('takes the same text written again for the answer', () => {
    expect(isAnswer(BEFORE, shown({ ...BEFORE, written: '2026-11-09T21:25:00.000Z' }))).toBe(true)
  })

  it('takes a message that gains its time for the answer', () => {
    expect(isAnswer(said('Done.', null), shown(said('Done.')))).toBe(true)
  })

  it('looks at no clock but the message', () => {
    expect(isAnswer(BEFORE, shown({ ...BEFORE, turn_over: false, pane: 12, agent: 'codex' }))).toBe(false)
  })

  it('takes any message for the answer when none was seen when sending', () => {
    expect(isAnswer(null, shown(BEFORE))).toBe(true)
  })

  it.each([
    ['loading', LOADING],
    ['unreadable', UNREADABLE],
  ])('takes no answer from a %s reading', (_, reading) => {
    expect(isAnswer(BEFORE, reading)).toBe(false)
    expect(isAnswer(null, reading)).toBe(false)
  })
})

describe('threadOf', () => {
  it('shows only the agent before anything is sent', () => {
    expect(threadOf(shown(BEFORE), null)).toEqual(['agent'])
  })

  it('puts what you sent under the message it answers', () => {
    expect(threadOf(shown(BEFORE), { answered: BEFORE })).toEqual(['agent', 'you'])
  })

  it('puts the answer under what you sent', () => {
    expect(threadOf(shown(said('Fixed.')), { answered: BEFORE })).toEqual(['you', 'agent'])
  })

  it('keeps what you sent under the reading while the message is not read yet', () => {
    expect(threadOf(LOADING, { answered: null })).toEqual(['agent', 'you'])
  })

  it('puts the first message read after sending under it', () => {
    expect(threadOf(shown(BEFORE), { answered: null })).toEqual(['you', 'agent'])
  })
})

describe('atEnd', () => {
  it('margins 48 px', () => {
    expect(AT_END_MARGIN).toBe(48)
  })

  it.each([
    [1952, true],
    [1951, false],
    [2000, true],
    [0, false],
  ])('at scroll %i of a 2,800 px page in an 800 px window is %s', (scrollY, end) => {
    expect(atEnd(scrollY, 800, 2800)).toBe(end)
  })

  it('takes a page shorter than the window as at the end', () => {
    expect(atEnd(0, 800, 500)).toBe(true)
  })

  it('takes another margin', () => {
    expect(atEnd(1990, 800, 2800, 10)).toBe(true)
    expect(atEnd(1989, 800, 2800, 10)).toBe(false)
  })
})

describe('compactHeader', () => {
  it.each([
    [0, false],
    [180, false],
    [181, true],
    [900, true],
  ])('at scroll %i past a header ending at 180 px is %s', (scrollY, compact) => {
    expect(compactHeader(scrollY, 180)).toBe(compact)
  })

  it('stays large while the header is not measured', () => {
    expect(compactHeader(400, 0)).toBe(false)
  })
})
