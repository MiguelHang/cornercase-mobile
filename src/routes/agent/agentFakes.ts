import type { AgentRow_Serialize as AgentRow } from '../../lib/bindings'
import type { Chip, Images } from '../../lib/attach.svelte'
import {
  Chat,
  timers,
  type Clock,
  type MarkReviewed,
  type PressKey,
  type ReadLast,
  type SendText,
} from '../../lib/chat.svelte'
import { fakeAsker, fakeClock } from '../../lib/chatFakes'

export const ROW: AgentRow = {
  boot: 'b1',
  agent: 'claude',
  title: 'Fix the login flow',
  status: 'waiting',
  since: '2026-11-09T11:40:00.000Z',
  parent: null,
  depth: 0,
  role: null,
  rank_status: 'waiting',
  pane: 12,
  tab: 4,
  tab_name: 'agent',
  workspace: 9,
  workspace_name: 'login',
  workspace_count: 2,
  project: 5,
  project_name: 'web',
}

export const unread: ReadLast = () => new Promise(() => {})

export const unsent: SendText = () => new Promise(() => {})

export const unpressed: PressKey = () => new Promise(() => {})

export const unmarked: MarkReviewed = () => new Promise(() => {})

export const unattached: Images = {
  attach: () => new Promise(() => {}),
  paste: () => new Promise(() => {}),
  discard: () => Promise.resolve(),
}

export const unasked = fakeAsker().asker

export const CHIPS: Chip[] = [
  { id: 1, name: 'IMG_0042.jpg', size: '47 KiB', thumbnail: null, pasted: false },
  { id: 2, name: 'pasted.png', size: '900 B', thumbnail: null, pasted: false },
]

export const still = { impactFeedback: () => Promise.resolve({ status: 'ok' as const, data: null }) }

export const flat = (text: string) => text.replace(/\s+/g, ' ')

export const chatWith = (row: AgentRow | null, draft = '') => {
  const chat = new Chat(1, ROW.pane, unread, unsent, unpressed, unmarked, unattached, still, timers, unasked)
  if (row !== null) {
    chat.see(1, [row])
  }
  chat.draft = draft
  return chat
}

export const SINCE = Date.parse(ROW.since ?? '')

export const LATER = SINCE + 3 * 60_000

export const clockAt = (millis: number) => {
  const { clock, tick } = fakeClock()
  tick(millis)
  return { clock, tick }
}

export const lineChat = (status: string | null, since: string | null = ROW.since, clock: Clock = clockAt(LATER).clock) => {
  const chat = new Chat(1, ROW.pane, unread, unsent, unpressed, unmarked, unattached, still, clock, unasked)
  chat.see(1, [{ ...ROW, status, since }])
  return chat
}
