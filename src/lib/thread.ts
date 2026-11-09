import type { AgentRow_Serialize as AgentRow, LastMessage_Serialize as LastMessage } from './bindings'
import { elapsed, span } from './board'
import type { Outgoing, Reading } from './chat.svelte'

export const STALL_MS = 120_000

export type LineTone = 'working' | 'waiting' | 'failed' | 'muted'

export type AgentLine = { tone: LineTone; phrase: string; time: string; stalled: boolean }

type Said = { tone: LineTone; doing: string; timed: boolean }

const SAID: Record<string, Said> = {
  working: { tone: 'working', doing: 'is working…', timed: true },
  shell: { tone: 'working', doing: 'is waiting for a background command…', timed: true },
  waiting: { tone: 'waiting', doing: 'is waiting for your answer', timed: false },
  failed: { tone: 'failed', doing: 'stopped with an error', timed: false },
}

const stalledLine = (row: AgentRow, quiet: number): AgentLine => ({
  tone: 'muted',
  phrase: `${row.agent} ${SAID.working.doing} · no news`,
  time: `for ${span(quiet)} · Ctrl-C interrupts`,
  stalled: true,
})

export const agentLine = (row: AgentRow, now: number, lastNews: number): AgentLine | null => {
  const said = SAID[row.status ?? '']
  if (said === undefined) {
    return null
  }
  if (row.status === 'working' && now - lastNews >= STALL_MS) {
    return stalledLine(row, now - lastNews)
  }
  const time = said.timed ? elapsed(row.since, now) : ''
  return { tone: said.tone, phrase: `${row.agent} ${said.doing}`, time, stalled: false }
}

export const sendNews = (outgoing: Pick<Outgoing, 'state' | 'reason'> | null) => {
  if (outgoing === null) {
    return ''
  }
  if (outgoing.state === 'failed') {
    return outgoing.reason === null ? 'Not sent' : `Not sent: ${outgoing.reason}`
  }
  return outgoing.state === 'sending' ? 'Sending' : 'Sent'
}

export type Part = 'agent' | 'you'

export const AT_END_MARGIN = 48

export const isAnswer = (answered: LastMessage | null, reading: Reading) =>
  reading.kind === 'message' &&
  (answered === null || reading.message.text !== answered.text || reading.message.written !== answered.written)

export const threadOf = (reading: Reading, outgoing: { answered: LastMessage | null } | null): Part[] => {
  if (outgoing === null) {
    return ['agent']
  }
  return isAnswer(outgoing.answered, reading) ? ['you', 'agent'] : ['agent', 'you']
}

export const atEnd = (scrollY: number, viewport: number, height: number, margin = AT_END_MARGIN) =>
  scrollY + viewport >= height - margin

export const compactHeader = (scrollY: number, headerBottom: number) => headerBottom > 0 && scrollY > headerBottom
