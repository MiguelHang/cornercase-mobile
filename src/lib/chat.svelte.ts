import type {
  AgentRow_Serialize as AgentRow,
  commands,
  LastMessage_Serialize as LastMessage,
  PaneStatusChanged_Serialize as PaneStatusChanged,
} from './bindings'
import { messageOf } from './toasts'

export type ReadLast = typeof commands.lastMessage

export type SendText = typeof commands.send

export type Reading =
  | { kind: 'loading' }
  | { kind: 'message'; message: LastMessage }
  | { kind: 'unreadable'; reason: string }

export const read = async (readLast: ReadLast, pane: number): Promise<Reading> => {
  try {
    const answer = await readLast(pane)
    return answer.status === 'ok'
      ? { kind: 'message', message: answer.data }
      : { kind: 'unreadable', reason: answer.error.message }
  } catch (error) {
    return { kind: 'unreadable', reason: messageOf(error) }
  }
}

export const deliver = async (sendText: SendText, pane: number, text: string): Promise<string | null> => {
  try {
    const sent = await sendText(pane, text)
    return sent.status === 'ok' ? null : sent.error.message
  } catch (error) {
    return messageOf(error)
  }
}

export const touches = (changed: PaneStatusChanged, machine: number | null, pane: number) =>
  changed.machine === machine && (changed.pane === null || changed.pane === pane)

export class Chat {
  readonly pane: number
  draft = $state('')
  sending = $state(false)
  error = $state<string | null>(null)
  reading = $state<Reading>({ kind: 'loading' })
  row = $state<AgentRow | null>(null)
  gone = $state(false)
  #readLast: ReadLast
  #sendText: SendText
  #asked = 0
  #busy = false
  #again = false

  constructor(pane: number, readLast: ReadLast, sendText: SendText) {
    this.pane = pane
    this.#readLast = readLast
    this.#sendText = sendText
  }

  get asking() {
    return this.row?.status === 'waiting'
  }

  get canSend() {
    return this.draft.trim() !== '' && !this.sending && this.row !== null && !this.asking
  }

  see(agents: AgentRow[] | null) {
    if (this.gone) {
      return
    }
    const row = agents === null ? null : (agents.find((agent) => agent.pane === this.pane) ?? null)
    if (row === null && (agents !== null || this.row !== null)) {
      this.#leave()
      return
    }
    this.row = row
  }

  async refresh() {
    if (this.gone) {
      return
    }
    if (this.#busy) {
      this.#again = true
      return
    }
    this.#busy = true
    while (this.#busy) {
      this.#again = false
      const number = ++this.#asked
      const reading = await read(this.#readLast, this.pane)
      if (number === this.#asked) {
        this.reading = reading
      }
      this.#busy = this.#again
    }
  }

  async send() {
    if (!this.canSend) {
      return null
    }
    this.sending = true
    this.error = null
    const failed = await deliver(this.#sendText, this.pane, this.draft)
    this.sending = false
    if (failed === null) {
      this.draft = ''
    } else {
      this.error = failed
    }
    return failed
  }

  #leave() {
    this.gone = true
    this.row = null
    this.draft = ''
    this.#asked += 1
    this.#again = false
  }
}
