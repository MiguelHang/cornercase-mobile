import { vi } from 'vitest'
import type { AttachImage, Discard, Images, PasteAttached } from './attach.svelte'
import type {
  AgentRow_Serialize as AgentRow,
  CommandError,
  HistoryEntry_Serialize as HistoryEntry,
  LastMessage_Serialize as LastMessage,
  QuestionChanged_Serialize as QuestionChanged,
  Style_Serialize as Style,
} from './bindings'
import {
  Chat,
  type Clock,
  type MarkReviewed,
  type Page,
  type PressKey,
  type ReadLast,
  type SendText,
} from './chat.svelte'
import type { HapticsApi } from './haptics'
import type { History } from './history'
import type { Asker } from './question.svelte'

export const MACHINE = 1
export const PANE = 12
export const TAB = 6

export const row = (pane: number, status: string | null): AgentRow => ({
  boot: 'b1',
  project: 4,
  workspace: 5,
  tab: TAB,
  pane,
  project_name: 'mobile',
  workspace_name: 'chat',
  tab_name: 'agent',
  workspace_count: 1,
  agent: 'codex',
  title: null,
  status,
  since: null,
  parent: null,
  depth: 0,
  role: null,
  rank_status: status,
})

export const message = (text: string): LastMessage => ({ pane: PANE, agent: 'claude', text, written: null, turn_over: true })

export const ok = <T>(data: T) => ({ status: 'ok' as const, data })

export const refused = (message: string, code: CommandError['code'] = 'failed') => ({
  status: 'error' as const,
  error: { code, message },
})

export const gate = <T>() => {
  let open: (value: T) => void = () => {}
  const promise = new Promise<T>((resolve) => (open = resolve))
  return { promise, open }
}

export const fakeHistory = (stored: HistoryEntry[] = [], loaded: Promise<void> = Promise.resolve()) => {
  const kept: HistoryEntry[][] = []
  const history: History = {
    open: vi.fn<History['open']>(() => loaded.then(() => kept.at(-1) ?? stored)),
    keep: vi.fn<History['keep']>((entries) => {
      kept.push(entries)
    }),
  }
  return { history, kept }
}

export const noRead: ReadLast = () => Promise.resolve(ok(message('')))

export const sentOk: SendText = () => Promise.resolve(ok(null))

export const pressedOk: PressKey = () => Promise.resolve(ok(null))

export const markedOk: MarkReviewed = () => Promise.resolve(ok(null))

export const fakeMarker = (answer: () => ReturnType<MarkReviewed> = () => markedOk(PANE)) => {
  const marked: number[] = []
  const markReviewed: MarkReviewed = (pane) => {
    marked.push(pane)
    return answer()
  }
  return { markReviewed, marked }
}

export const PASTED_PATH = '/Users/miguelhang/.local/state/cornercase/pasted/2026-11-09T19-40-12Z/IMG_0042.jpg'

export const THUMBNAIL = 'data:image/jpeg;base64,/9j/2w=='

export const image = (id: number) => ({ id, name: 'IMG_0042.jpg', bytes: 48213, thumbnail: THUMBNAIL })

export const attachedOk: AttachImage = () => Promise.resolve(ok(image(1)))

export const pastedOk: PasteAttached = (pane) => Promise.resolve(ok({ pane, path: PASTED_PATH, bytes: 48213 }))

export const discardedOk: Discard = () => Promise.resolve()

export const still = (): HapticsApi => ({ impactFeedback: vi.fn(() => Promise.resolve(ok(null))) })

export const fakeClock = () => {
  let now = 0
  const waiting: { at: number; then: () => void; live: boolean }[] = []
  const clock: Clock = {
    after: (ms, then) => {
      const timer = { at: now + ms, then, live: true }
      waiting.push(timer)
      return () => (timer.live = false)
    },
    now: () => now,
  }
  const tick = (ms: number) => {
    now += ms
    waiting.filter((timer) => timer.live && timer.at <= now).forEach((timer) => ((timer.live = false), timer.then()))
  }
  const pending = () => waiting.filter((timer) => timer.live).length
  return { clock, tick, pending }
}

const PLAIN: Style = {
  bold: false,
  dim: false,
  italic: false,
  underline: false,
  overline: false,
  blink: false,
  inverse: false,
  invisible: false,
  strikethrough: false,
  fg: null,
  bg: null,
  underline_style: null,
  underline_colour: null,
}

export const asked = (lines: string[], changes: Partial<QuestionChanged> = {}): QuestionChanged => ({
  machine: MACHINE,
  tab: TAB,
  pane: PANE,
  colours: { fg: null, bg: null, cursor: null, palette: {} },
  rows: lines.map((text) => [{ text, width: text.length, style: PLAIN }]),
  options: lines.flatMap((text) => {
    const numbered = /^(\d)\. (.+)$/.exec(text)
    return numbered === null ? [] : [{ number: Number(numbered[1]), label: numbered[2] }]
  }),
  closed: false,
  ...changes,
})

export const fakeAsker = () => {
  const listeners = new Set<(changed: QuestionChanged) => void>()
  const asker = {
    open: vi.fn<Asker['open']>(() => Promise.resolve(ok(null))),
    close: vi.fn<Asker['close']>(() => Promise.resolve()),
    listen: vi.fn<Asker['listen']>((heard) => {
      listeners.add(heard)
      return Promise.resolve(() => listeners.delete(heard))
    }),
  }
  const tell = (changed: QuestionChanged) => listeners.forEach((heard) => heard(changed))
  return { asker, tell, listening: () => listeners.size }
}

export type Calls = {
  readLast?: ReadLast
  sendText?: SendText
  pressKey?: PressKey
  markReviewed?: MarkReviewed
  attachImage?: AttachImage
  pasteAttached?: PasteAttached
  discard?: Discard
  haptics?: HapticsApi
  clock?: Clock
  asker?: Asker
}

const imagesOf = ({ attachImage = attachedOk, pasteAttached = pastedOk, discard = discardedOk }: Calls): Images => ({
  attach: attachImage,
  paste: pasteAttached,
  discard,
})

export const fresh = (calls: Calls = {}) => {
  const {
    readLast = noRead,
    sendText = sentOk,
    pressKey = pressedOk,
    markReviewed = markedOk,
    haptics = still(),
    clock = fakeClock().clock,
    asker = fakeAsker().asker,
  } = calls
  return new Chat(MACHINE, PANE, readLast, sendText, pressKey, markReviewed, imagesOf(calls), haptics, clock, asker)
}

export const seen = (calls: Calls = {}, status: string | null = 'done') => {
  const chat = fresh(calls)
  chat.see(MACHINE, [row(3, 'working'), row(PANE, status)])
  return chat
}

export const ready = (sendText: SendText = sentOk, status: string | null = 'done', readLast: ReadLast = noRead) =>
  seen({ sendText, readLast }, status)

export const settle = () => new Promise((resolve) => setTimeout(resolve))

export const PASTED = ok(image(1))

export const attached = async (sendText: SendText = sentOk, status: string | null = 'done') => {
  const chat = ready(sendText, status)
  await chat.attach()
  return chat
}

export class FakePage implements Page {
  visibilityState: DocumentVisibilityState
  #listeners = new Set<() => void>()

  constructor(visibilityState: DocumentVisibilityState) {
    this.visibilityState = visibilityState
  }

  addEventListener(name: string, listener: () => void) {
    if (name === 'visibilitychange') this.#listeners.add(listener)
  }

  removeEventListener(name: string, listener: () => void) {
    if (name === 'visibilitychange') this.#listeners.delete(listener)
  }

  turn(visibilityState: DocumentVisibilityState) {
    this.visibilityState = visibilityState
    for (const listener of this.#listeners) listener()
  }

  get listening() {
    return this.#listeners.size
  }
}
