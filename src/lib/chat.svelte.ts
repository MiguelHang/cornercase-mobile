import type {
  AgentRow_Serialize as AgentRow,
  commands,
  LastMessage_Serialize as LastMessage,
  PaneStatusChanged_Serialize as PaneStatusChanged,
  Toast,
} from './bindings'
import { Attachments, type Chip, type Images } from './attach.svelte'
import { tap, type HapticsApi } from './haptics'
import { digitOf, type KeyName } from './keys'
import { Questions, type Asker } from './question.svelte'
import { deliver, messageOf } from './toasts'

export type ReadLast = typeof commands.lastMessage

export type SendText = typeof commands.send

export type PressKey = typeof commands.keys

export type Clock = { after: (ms: number, then: () => void) => () => void; now: () => number }

export const INTERRUPT_WINDOW_MS = 3000

export const FOLLOW_MS = 10_000

export const staysOnTheMac = (agent: string) => `The image stays in ${agent}'s input on the Mac`

export const timers: Clock = {
  after: (ms, then) => {
    const timer = setTimeout(then, ms)
    return () => clearTimeout(timer)
  },
  now: () => Date.now(),
}

export type Reading =
  | { kind: 'loading' }
  | { kind: 'message'; message: LastMessage }
  | { kind: 'unreadable'; reason: string }

export type Outgoing = {
  text: string
  chips: Chip[]
  state: 'sending' | 'sent' | 'failed'
  reason: string | null
  answered: LastMessage | null
}

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

const news = (shown: Reading, next: Reading) =>
  next.kind === 'message' && (shown.kind !== 'message' || shown.message.text !== next.message.text)

export const touches = (changed: PaneStatusChanged, machine: number | null, pane: number) =>
  changed.machine === machine && (changed.pane === null || changed.pane === pane)

export type Page = Pick<EventTarget, 'addEventListener' | 'removeEventListener'> & {
  visibilityState: DocumentVisibilityState
}

export const whileShown = (page: Page, start: () => () => void) => {
  let stop: (() => void) | null = null
  const look = () => {
    const shown = page.visibilityState === 'visible'
    if (shown && stop === null) {
      stop = start()
    } else if (!shown && stop !== null) {
      stop()
      stop = null
    }
  }
  look()
  page.addEventListener('visibilitychange', look)
  return () => {
    page.removeEventListener('visibilitychange', look)
    stop?.()
  }
}

export class Chat {
  readonly machine: number | null
  readonly pane: number
  draft = $state('')
  outgoing = $state<Outgoing | null>(null)
  lastNews = $state(0)
  reading = $state<Reading>({ kind: 'loading' })
  row = $state<AgentRow | null>(null)
  gone = $state(false)
  interrupting = $state(false)
  keysOpen = $state(false)
  readonly attachments: Attachments
  readonly #questions: Questions
  #readLast: ReadLast
  #sendText: SendText
  #pressKey: PressKey
  #haptics: HapticsApi
  #clock: Clock
  #cancelExpiry: () => void = () => {}
  #cancelFollow: (() => void) | null = null
  #following = false
  #questioning = false
  #asked = 0
  #busy = false
  #again = false
  #starting = false
  #heard: string | null | undefined = undefined
  #leftOnTheMac = false
  #keysByWaiting = false

  constructor(
    machine: number | null,
    pane: number,
    readLast: ReadLast,
    sendText: SendText,
    pressKey: PressKey,
    images: Images,
    haptics: HapticsApi,
    clock: Clock,
    asker: Asker,
  ) {
    this.machine = machine
    this.pane = pane
    this.#readLast = readLast
    this.#sendText = sendText
    this.#pressKey = pressKey
    this.attachments = new Attachments(pane, images)
    this.#questions = new Questions(machine, pane, asker)
    this.#haptics = haptics
    this.#clock = clock
    this.lastNews = clock.now()
  }

  now() {
    return this.#clock.now()
  }

  get sending() {
    return this.outgoing?.state === 'sending'
  }

  get asking() {
    return this.row?.status === 'waiting'
  }

  get question() {
    return this.#questions.question
  }

  get canSend() {
    const something = this.draft.trim() !== '' || this.attachments.chips.length > 0
    return something && this.#free()
  }

  get canRetry() {
    return this.outgoing?.state === 'failed' && this.#free()
  }

  #free() {
    return !this.sending && !this.attachments.attaching && this.row !== null && !this.asking
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
      this.leave()
      return
    }
    this.#hear(row)
    this.row = row
    this.#pace()
    this.#ask()
  }

  #hear(row: AgentRow | null) {
    if (row !== null && row.status !== this.#heard) {
      this.#heard = row.status
      this.#turnKeys(row.status)
      this.lastNews = this.#clock.now()
    }
  }

  #turnKeys(status: string | null) {
    if (status === 'waiting' && !this.keysOpen) {
      this.keysOpen = true
      this.#keysByWaiting = true
    } else if (status !== 'waiting' && this.#keysByWaiting) {
      this.#closeKeys()
    }
  }

  toggleKeys() {
    if (this.keysOpen) {
      this.#closeKeys()
    } else {
      this.keysOpen = true
    }
  }

  #closeKeys() {
    this.keysOpen = false
    this.#keysByWaiting = false
    this.#disarm()
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
        this.#show(kept(this.reading, answer))
      }
      this.#busy = this.#again
    }
  }

  follow() {
    this.#following = true
    this.#pace()
    return () => {
      this.#following = false
      this.#pace()
    }
  }

  #pace() {
    if (!this.#following || this.row?.status !== 'working') {
      this.#cancelFollow?.()
      this.#cancelFollow = null
    } else if (this.#cancelFollow === null) {
      this.#cancelFollow = this.#clock.after(FOLLOW_MS, () => {
        this.#cancelFollow = null
        void this.refresh()
        this.#pace()
      })
    }
  }

  watchQuestion() {
    this.#questioning = true
    this.#ask()
    return () => {
      this.#questioning = false
      this.#ask()
    }
  }

  #ask() {
    const row = this.row
    this.#questions.want(this.#questioning && row?.status === 'waiting' ? row.tab : null)
  }

  #show(next: Reading) {
    if (news(this.reading, next)) {
      this.lastNews = this.#clock.now()
    }
    this.reading = next
  }

  async send() {
    if (!this.canSend) {
      return
    }
    const text = this.draft
    this.draft = ''
    await this.#deliver(text)
  }

  async retry() {
    if (this.canRetry && this.outgoing !== null) {
      await this.#deliver(this.outgoing.text)
    }
  }

  edit() {
    if (this.outgoing?.state !== 'failed') {
      return
    }
    this.draft = [this.outgoing.text, this.draft].filter((part) => part !== '').join('\n\n')
    this.outgoing = null
  }

  async #deliver(text: string) {
    const chips = this.attachments.take()
    const shown = this.reading.kind === 'message' ? { ...this.reading.message } : null
    const bubble: Outgoing = { text, chips, state: 'sending', reason: null, answered: shown }
    this.outgoing = bubble
    const failed = await this.#handOver(text, chips)
    if (this.gone) {
      void this.attachments.release(chips)
      return
    }
    if (failed === null) {
      this.#leftOnTheMac = false
      this.lastNews = this.#clock.now()
      this.outgoing = { ...bubble, state: 'sent' }
      return
    }
    this.attachments.giveBack(chips)
    this.outgoing = { ...bubble, state: 'failed', reason: failed }
  }

  async #handOver(text: string, chips: Chip[]): Promise<string | null> {
    for (const chip of chips.filter((chip) => !chip.pasted)) {
      const failed = await this.attachments.paste(chip)
      if (failed !== null || this.gone) {
        return failed
      }
    }
    const forced = chips.length > 0 || this.#leftOnTheMac
    return deliver(() => this.#sendText(this.pane, text, forced))
  }

  async attach(): Promise<string | null> {
    return this.canPress ? this.attachments.attach() : null
  }

  async remove(id: number): Promise<Toast | null> {
    const pasted = this.attachments.chips.some((chip) => chip.id === id && chip.pasted)
    const failed = await this.attachments.remove(id)
    if (failed !== null) {
      return { level: 'error', message: failed }
    }
    this.#leftOnTheMac ||= pasted
    return pasted ? { level: 'info', message: staysOnTheMac(this.row?.agent ?? 'the agent') } : null
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

  async pressOption(number: number): Promise<string | null> {
    const digit = digitOf(number)
    return digit === null ? null : this.press(digit)
  }

  #arm() {
    this.interrupting = true
    this.#cancelExpiry = this.#clock.after(INTERRUPT_WINDOW_MS, () => (this.interrupting = false))
  }

  #disarm() {
    this.#cancelExpiry()
    this.interrupting = false
  }

  leave() {
    this.gone = true
    this.row = null
    this.draft = ''
    this.outgoing = null
    void this.attachments.release()
    this.#asked += 1
    this.#again = false
    this.#pace()
    this.#ask()
  }
}
