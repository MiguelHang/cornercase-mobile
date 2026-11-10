import type { HistoryEntry_Serialize as Entry, LastMessage_Serialize as LastMessage, SendState } from './bindings'
import type { Reading } from './chat.svelte'

type Heard = Reading | { kind: 'lost'; reason: string }

type Recorded = { entries: Entry[]; id: number }

type AgentEntry = Extract<Entry, { kind: 'agent' }>

export const nextId = (entries: Entry[]) => entries.reduce((next, entry) => Math.max(next, entry.id + 1), 0)

const gap = (entries: Entry[], seen: number): Entry => ({ kind: 'gap', id: nextId(entries), seen })

const opened = (entries: Entry[], seen: number, continuous: boolean) => {
  const last = entries.at(-1)
  if (last === undefined || (!continuous && last.kind !== 'gap')) {
    return [...entries, gap(entries, seen)]
  }
  return entries
}

const tidy = (entries: Entry[]) => {
  const kept = entries.filter((entry, at) => entry.kind !== 'gap' || entries[at - 1]?.kind !== 'gap')
  const end = kept.findLastIndex((entry) => entry.kind !== 'gap')
  return kept.slice(0, end + 1)
}

export const sent = (entries: Entry[], text: string, chips: string[], seen: number): Recorded => {
  const before = opened(entries, seen, true)
  const id = nextId(before)
  return { entries: [...before, { kind: 'you', id, text, chips, state: 'sending', reason: null, seen }], id }
}

export const restated = (entries: Entry[], id: number, state: SendState, reason: string | null) =>
  entries.map((entry) => (entry.kind === 'you' && entry.id === id ? { ...entry, state, reason } : entry))

export const edited = (entries: Entry[], id: number) =>
  tidy(entries.filter((entry) => entry.kind !== 'you' || entry.id !== id))

const unchanged = (last: AgentEntry, message: LastMessage) =>
  last.text === message.text && last.written === message.written && last.turn_over === message.turn_over

const grows = (last: AgentEntry, message: LastMessage) =>
  (last.text === message.text && last.written === message.written) ||
  (last.written !== null && last.written === message.written) ||
  (message.text.length > last.text.length && message.text.startsWith(last.text))

const grown = (entries: Entry[], last: AgentEntry, message: LastMessage) =>
  entries.map((entry) =>
    entry === last ? { ...last, text: message.text, written: message.written, turn_over: message.turn_over } : entry,
  )

const lastAgent = (entries: Entry[]) => entries.findLast((entry): entry is AgentEntry => entry.kind === 'agent')

export const heard = (entries: Entry[], reading: Heard, seen: number, continuous: boolean) => {
  if (reading.kind !== 'message') {
    return entries
  }
  const { message } = reading
  const last = lastAgent(entries)
  if (last !== undefined && unchanged(last, message)) {
    return entries
  }
  if (last !== undefined && grows(last, message)) {
    return grown(entries, last, message)
  }
  const before = opened(entries, seen, continuous)
  const { text, written, turn_over } = message
  return [...before, { kind: 'agent' as const, id: nextId(before), text, written, turn_over, seen }]
}
