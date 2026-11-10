import { describe, expect, it, vi } from 'vitest'
import type { HistoryEntry_Serialize as Entry, LastMessage_Serialize as LastMessage } from './bindings'
import { gate, settle } from './chatFakes'
import { edited, heard, type HistoryCalls, LocalHistory, nextId, restated, sent } from './history'

const SEEN = 1_000

const message = (text: string, written: string | null = null, turn_over: boolean | null = true): LastMessage => ({
  pane: 7,
  agent: 'claude',
  text,
  written,
  turn_over,
})

const reading = (text: string, written: string | null = null, turn_over: boolean | null = true) => ({
  kind: 'message' as const,
  message: message(text, written, turn_over),
})

const gap = (id: number, seen = SEEN): Entry => ({ kind: 'gap', id, seen })

const agent = (id: number, text: string, written: string | null = null, seen = SEEN): Entry => ({
  kind: 'agent',
  id,
  text,
  written,
  turn_over: true,
  seen,
})

const you = (id: number, text: string, state: 'sending' | 'sent' | 'failed' = 'sent'): Entry => ({
  kind: 'you',
  id,
  text,
  chips: [],
  state,
  reason: null,
  seen: SEEN,
})

const kinds = (entries: Entry[]) => entries.map((entry) => entry.kind)

describe('nextId', () => {
  it('starts at zero', () => {
    expect(nextId([])).toBe(0)
  })

  it('goes past the highest id, wherever it is', () => {
    expect(nextId([gap(0), agent(5, 'Done.'), you(2, 'fix it')])).toBe(6)
  })
})

describe('sent', () => {
  it('starts a history with a gap, then the message as sending', () => {
    expect(sent([], 'fix the login', ['login.png'], SEEN)).toEqual({
      entries: [
        gap(0),
        { kind: 'you', id: 1, text: 'fix the login', chips: ['login.png'], state: 'sending', reason: null, seen: SEEN },
      ],
      id: 1,
    })
  })

  it('adds the message at the end, with no gap before it', () => {
    const { entries, id } = sent([gap(0), agent(1, 'Done.')], 'and the tests', [], SEEN + 1)
    expect(kinds(entries)).toEqual(['gap', 'agent', 'you'])
    expect(id).toBe(2)
  })
})

describe('restated', () => {
  const entries = [gap(0), you(1, 'fix the login', 'sending'), you(2, 'and the tests', 'sending')]

  it('changes the state of the message with that id, in place', () => {
    expect(restated(entries, 1, 'failed', 'the Mac is not reachable')).toEqual([
      gap(0),
      { ...you(1, 'fix the login', 'failed'), reason: 'the Mac is not reachable' },
      you(2, 'and the tests', 'sending'),
    ])
  })

  it('clears the reason once a retry is sent', () => {
    const failed = restated(entries, 1, 'failed', 'the Mac is not reachable')
    expect(restated(failed, 1, 'sent', null)[1]).toEqual(you(1, 'fix the login', 'sent'))
  })

  it('leaves an agent message with that id alone', () => {
    const answered = [gap(0), agent(1, 'Done.')]
    expect(restated(answered, 1, 'failed', 'gone')).toEqual(answered)
  })
})

describe('edited', () => {
  it('removes the message with that id', () => {
    expect(edited([gap(0), agent(1, 'Done.'), you(2, 'fix it', 'failed'), agent(3, 'Fixed.')], 2)).toEqual([
      gap(0),
      agent(1, 'Done.'),
      agent(3, 'Fixed.'),
    ])
  })

  it('never leaves two gaps in a row', () => {
    const entries = [gap(0), agent(1, 'Done.'), gap(2), you(3, 'fix it', 'failed'), gap(4), agent(5, 'Fixed.')]
    expect(edited(entries, 3)).toEqual([gap(0), agent(1, 'Done.'), gap(2), agent(5, 'Fixed.')])
  })

  it('never leaves a gap at the end', () => {
    expect(edited([gap(0), agent(1, 'Done.'), gap(2), you(3, 'fix it', 'failed')], 3)).toEqual([
      gap(0),
      agent(1, 'Done.'),
    ])
  })

  it('leaves nothing once the only message is gone', () => {
    expect(edited([gap(0), you(1, 'fix it', 'failed')], 1)).toEqual([])
  })

  it('leaves an agent message with that id alone', () => {
    const entries = [gap(0), agent(1, 'Done.')]
    expect(edited(entries, 1)).toEqual(entries)
  })
})

describe('heard', () => {
  const answered = [gap(0), agent(1, 'Working on it', '2026-11-10T10:00:00Z'), you(2, 'and the tests')]

  it.each([
    ['loading', { kind: 'loading' as const }],
    ['unreadable', { kind: 'unreadable' as const, reason: 'the transcript is gone' }],
    ['lost', { kind: 'lost' as const, reason: 'the Mac is not reachable' }],
  ])('keeps nothing from a %s reading', (_, nothing) => {
    expect(heard(answered, nothing, SEEN, false)).toBe(answered)
    expect(heard([], nothing, SEEN, false)).toEqual([])
  })

  it('starts a history with a gap, even when it followed the turn', () => {
    expect(heard([], reading('Done.', '2026-11-10T10:00:00Z'), SEEN, true)).toEqual([
      gap(0),
      agent(1, 'Done.', '2026-11-10T10:00:00Z'),
    ])
  })

  it('adds a new message with no gap when it followed the turn', () => {
    const added = heard(answered, reading('Fixed.'), SEEN + 5, true)
    expect(added).toEqual([...answered, agent(3, 'Fixed.', null, SEEN + 5)])
  })

  it('puts a gap before a new message it cannot vouch for', () => {
    expect(heard(answered, reading('Fixed.'), SEEN + 5, false)).toEqual([
      ...answered,
      gap(3, SEEN + 5),
      agent(4, 'Fixed.', null, SEEN + 5),
    ])
  })

  it('never puts a gap after another gap', () => {
    expect(heard([gap(0)], reading('Fixed.'), SEEN, false)).toEqual([gap(0), agent(1, 'Fixed.')])
  })

  it.each([
    ['without a gap', true],
    ['even without continuity', false],
  ])('keeps the same message once, %s', (_, continuous) => {
    expect(heard(answered, reading('Working on it', '2026-11-10T10:00:00Z'), SEEN + 5, continuous)).toBe(answered)
  })

  it.each([
    ['text that starts with the old one', 'Working on it, done', null],
    ['the same written time', 'Rewritten', '2026-11-10T10:00:00Z'],
  ])('replaces the last message in place when it grows: %s', (_, text, written) => {
    const grown = heard(answered, reading(text, written ?? '2026-11-10T10:05:00Z'), SEEN + 5, false)
    expect(grown).toEqual([gap(0), agent(1, text, written ?? '2026-11-10T10:05:00Z'), you(2, 'and the tests')])
  })

  it('replaces the last message in place when only the turn ends', () => {
    const working = [gap(0), { ...agent(1, 'Done.'), turn_over: false }]
    expect(heard(working, reading('Done.'), SEEN + 5, true)).toEqual([gap(0), agent(1, 'Done.')])
  })

  it('adds the same words written again as a new message', () => {
    const before = [gap(0), agent(1, 'Done.', '2026-11-10T10:00:00Z')]
    const again = heard(before, reading('Done.', '2026-11-10T10:09:00Z'), SEEN, true)
    expect(kinds(again)).toEqual(['gap', 'agent', 'agent'])
  })

  it('adds no gap for an answer followed on screen', () => {
    const opened = heard([], reading('What next?'), SEEN, false)
    const asked = sent(opened, 'fix the login', [], SEEN + 1)
    const delivered = restated(asked.entries, asked.id, 'sent', null)
    const answer = heard(delivered, reading('Fixed the login.'), SEEN + 2, true)
    expect(kinds(answer)).toEqual(['gap', 'agent', 'you', 'agent'])
  })
})

describe('LocalHistory', () => {
  const STUDIO = 3
  const BOOT = 'b1'
  const stored = [gap(0), agent(1, 'Done.')]

  const fakeCalls = (load: HistoryCalls['historyLoad'] = () => Promise.resolve(stored)) => ({
    historyLoad: vi.fn(load),
    historySave: vi.fn<HistoryCalls['historySave']>(() => Promise.resolve()),
  })

  const saved = (calls: ReturnType<typeof fakeCalls>) => calls.historySave.mock.calls

  it('loads the pane it opens', async () => {
    const calls = fakeCalls()
    expect(await new LocalHistory(calls).open(STUDIO, BOOT, 7)).toEqual(stored)
    expect(calls.historyLoad).toHaveBeenCalledWith(STUDIO, BOOT, 7)
  })

  it('opens an empty history when the load fails', async () => {
    const calls = fakeCalls(() => Promise.reject(new Error('the app is gone')))
    expect(await new LocalHistory(calls).open(STUDIO, BOOT, 7)).toEqual([])
  })

  it('keeps nothing before a pane is open', async () => {
    const calls = fakeCalls()
    new LocalHistory(calls).keep(stored)
    await settle()
    expect(saved(calls)).toEqual([])
  })

  it('keeps the entries in the pane it opened last', async () => {
    const calls = fakeCalls()
    const history = new LocalHistory(calls)
    await history.open(STUDIO, BOOT, 7)
    await history.open(STUDIO, 'b2', 8)
    history.keep(stored)
    await settle()
    expect(saved(calls)).toEqual([[STUDIO, 'b2', 8, stored]])
  })

  it('writes in order, even when the earlier write is slow', async () => {
    const calls = fakeCalls()
    const slow = gate<null>()
    calls.historySave.mockImplementationOnce(() => slow.promise.then(() => undefined))
    const history = new LocalHistory(calls)
    await history.open(STUDIO, BOOT, 7)
    const later = [...stored, you(2, 'and the tests')]
    history.keep(stored)
    history.keep(later)
    await settle()
    expect(saved(calls)).toEqual([[STUDIO, BOOT, 7, stored]])
    slow.open(null)
    await settle()
    expect(saved(calls)).toEqual([
      [STUDIO, BOOT, 7, stored],
      [STUDIO, BOOT, 7, later],
    ])
  })

  it('swallows a failed write and still makes the next one', async () => {
    const calls = fakeCalls()
    calls.historySave.mockImplementationOnce(() => Promise.reject(new Error('the app is gone')))
    const history = new LocalHistory(calls)
    await history.open(STUDIO, BOOT, 7)
    history.keep(stored)
    history.keep([gap(0)])
    await settle()
    expect(saved(calls).map(([, , , entries]) => entries)).toEqual([stored, [gap(0)]])
  })

  it('loads again only once the writes before it are done', async () => {
    const calls = fakeCalls()
    const slow = gate<null>()
    calls.historySave.mockImplementationOnce(() => slow.promise.then(() => undefined))
    const history = new LocalHistory(calls)
    await history.open(STUDIO, BOOT, 7)
    history.keep(stored)
    const reopened = history.open(STUDIO, BOOT, 7)
    await settle()
    expect(calls.historyLoad).toHaveBeenCalledTimes(1)
    slow.open(null)
    await reopened
    expect(calls.historyLoad).toHaveBeenCalledTimes(2)
  })
})
