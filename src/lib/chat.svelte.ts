import type {
  AgentRow_Serialize as AgentRow,
  commands,
  LastMessage_Serialize as LastMessage,
  PaneStatusChanged_Serialize as PaneStatusChanged,
} from './bindings'
import { tap, type HapticsApi } from './haptics'
import type { KeyName } from './keys'
import { messageOf } from './toasts'

export type ReadLast = typeof commands.lastMessage

export type SendText = typeof commands.send

export type PressKey = typeof commands.keys

export type Clock = { after: (ms: number, then: () => void) => () => void }

export const INTERRUPT_WINDOW_MS = 3000

export const timers: Clock = {
  after: (ms, then) => {
    const timer = setTimeout(then, ms)
    return () => clearTimeout(timer)
  },
}

export type Reading =
  | { kind: 'loading' }
  | { kind: 'message'; message: LastMessage }
  | { kind: 'unreadable'; reason: string }

type Answer = Reading | { kind: 'lost'; reason: string }

export const read = async (readLast: ReadLast, pane: number): Promise<Answer> => {
  try {
    const answer = await readLast(pane)
    if (answer.status === 'ok') {
      return { kind: 'message', message: answer.data }
    }
    const kind = answer.error.code === 'failed' ? 'unreadable' : 'lost'
    return { kind, reason: answer.error.message }
  } catch (error) {
    return { kind: 'lost', reason: messageOf(error) }
  }
}

const kept = (shown: Reading, answer: Answer): Reading => {
  if (answer.kind !== 'lost') {
    return answer
  }
  return shown.kind === 'message' ? shown : { kind: 'unreadable', reason: answer.reason }
}

export const deliver = async (call: () => ReturnType<SendText>): Promise<string | null> => {
  try {
    const sent = await call()
    return sent.status === 'ok' ? null : sent.error.message
  } catch (error) {
    return messageOf(error)
  }
}

export const touches = (changed: PaneStatusChanged, machine: number | null, pane: number) =>
  changed.machine === machine && (changed.pane === null || changed.pane === pane)

export class Chat {
  readonly machine: number | null
  readonly pane: number
  draft = $state('')
  sending = $state(false)
  reading = $state<Reading>({ kind: 'loading' })
  row = $state<AgentRow | null>(null)
  gone = $state(false)
  interrupting = $state(false)
  #readLast: ReadLast
  #sendText: SendText
  #pressKey: PressKey
  #haptics: HapticsApi
  #clock: Clock
  #cancelExpiry: () => void = () => {}
  #asked = 0
  #busy = false
  #again = false
  #starting = false

  constructor(
    machine: number | null,
    pane: number,
    readLast: ReadLast,
    sendText: SendText,
    pressKey: PressKey,
    haptics: HapticsApi,
    clock: Clock,
  ) {
    this.machine = machine
    this.pane = pane
    this.#readLast = readLast
    this.#sendText = sendText
    this.#pressKey = pressKey
    this.#haptics = haptics
    this.#clock = clock
  }

  get asking() {
    return this.row?.status === 'waiting'
  }

  get canSend() {
    return this.draft.trim() !== '' && !this.sending && this.row !== null && !this.asking
  }

  get canPress() {
    return this.row !== null
  }

  see(machine: number | null, agents: AgentRow[] | null, starting = false) {
    if (this.gone) {
      return
    }
    this.#starting ||= starting
    const elsewhere = machine !== this.machine
    const row = agents === null || elsewhere ? null : (agents.find((agent) => agent.pane === this.pane) ?? null)
    if (row === null && (elsewhere || this.#lost(agents))) {
      this.#leave()
      return
    }
    this.row = row
  }

  async refresh() {
    if (this.gone || this.row === null) {
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
      const answer = await read(this.#readLast, this.pane)
      if (number === this.#asked) {
        this.reading = kept(this.reading, answer)
      }
      this.#busy = this.#again
    }
  }

  async send(): Promise<string | null> {
    if (!this.canSend) {
      return null
    }
    this.sending = true
    const failed = await deliver(() => this.#sendText(this.pane, this.draft))
    this.sending = false
    if (failed === null) {
      this.draft = ''
    }
    return failed
  }

  #lost(agents: AgentRow[] | null) {
    return (agents !== null && !this.#starting) || this.row !== null
  }

  async press(key: KeyName): Promise<string | null> {
    if (!this.canPress) {
      return null
    }
    void tap(this.#haptics)
    if (key === 'ctrl+c' && !this.interrupting) {
      this.#arm()
      return null
    }
    this.#disarm()
    return deliver(() => this.#pressKey(this.pane, key))
  }

  #arm() {
    this.interrupting = true
    this.#cancelExpiry = this.#clock.after(INTERRUPT_WINDOW_MS, () => (this.interrupting = false))
  }

  #disarm() {
    this.#cancelExpiry()
    this.interrupting = false
  }

  #leave() {
    this.gone = true
    this.row = null
    this.draft = ''
    this.#asked += 1
    this.#again = false
  }
}
