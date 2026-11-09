import { vi } from 'vitest'
import type { AttachImage } from './attach.svelte'
import type { AgentRow_Serialize as AgentRow, CommandError, LastMessage_Serialize as LastMessage } from './bindings'
import { Chat, type Clock, type PressKey, type ReadLast, type SendText } from './chat.svelte'
import type { HapticsApi } from './haptics'

export const MACHINE = 1
export const PANE = 12

export const row = (pane: number, status: string | null): AgentRow => ({
  boot: 'b1',
  project: 4,
  workspace: 5,
  tab: 6,
  pane,
  project_name: 'mobile',
  workspace_name: 'chat',
  tab_name: 'agent',
  workspace_count: 1,
  agent: 'codex',
  title: null,
  status,
  since: null,
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

export const noRead: ReadLast = () => Promise.resolve(ok(message('')))

export const sentOk: SendText = () => Promise.resolve(ok(null))

export const pressedOk: PressKey = () => Promise.resolve(ok(null))

export const PASTED_PATH = '/Users/miguelhang/.local/state/cornercase/pasted/2026-11-09T19-40-12Z/IMG_0042.jpg'

export const attachedOk: AttachImage = (pane) => Promise.resolve(ok({ pane, path: PASTED_PATH, bytes: 48213 }))

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
  return { clock, tick }
}

export type Calls = {
  readLast?: ReadLast
  sendText?: SendText
  pressKey?: PressKey
  attachImage?: AttachImage
  haptics?: HapticsApi
  clock?: Clock
}

export const fresh = ({
  readLast = noRead,
  sendText = sentOk,
  pressKey = pressedOk,
  attachImage = attachedOk,
  haptics = still(),
  clock = fakeClock().clock,
}: Calls = {}) => new Chat(MACHINE, PANE, readLast, sendText, pressKey, attachImage, haptics, clock)

export const seen = (calls: Calls = {}, status: string | null = 'done') => {
  const chat = fresh(calls)
  chat.see(MACHINE, [row(3, 'working'), row(PANE, status)])
  return chat
}

export const ready = (sendText: SendText = sentOk, status: string | null = 'done', readLast: ReadLast = noRead) =>
  seen({ sendText, readLast }, status)

export const settle = () => new Promise((resolve) => setTimeout(resolve))

export const PASTED = ok({ pane: PANE, path: PASTED_PATH, bytes: 48213 })

export const attached = async (sendText: SendText = sentOk, status: string | null = 'done') => {
  const chat = ready(sendText, status)
  await chat.attach()
  return chat
}
