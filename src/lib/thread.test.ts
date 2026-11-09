import { describe, expect, it } from 'vitest'
import type { AgentRow_Serialize as AgentRow } from './bindings'
import { STALL_MS, agentLine } from './thread'

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
