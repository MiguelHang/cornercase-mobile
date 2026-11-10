import { afterEach, describe, expect, it, vi } from 'vitest'
import type { MarkReviewed } from './chat.svelte'
import { MACHINE, PANE, fakeMarker, fresh, refused, row, settle } from './chatFakes'

const OTHER_MACHINE = 2

const NO_AGENT = 'there is no agent in pane 12'

const opened = (status: string | null, answer?: () => ReturnType<MarkReviewed>) => {
  const marker = fakeMarker(answer)
  const chat = fresh({ markReviewed: marker.markReviewed })
  chat.see(MACHINE, [row(3, 'done'), row(PANE, status)])
  return { chat, marked: marker.marked }
}

const watched = (status: string | null, answer?: () => ReturnType<MarkReviewed>) => {
  const open = opened(status, answer)
  const stop = open.chat.watchDone()
  return { ...open, stop }
}

const turned = (chat: ReturnType<typeof fresh>, ...statuses: string[]) => {
  statuses.forEach((status) => chat.see(MACHINE, [row(PANE, status)]))
}

const quiet = () => vi.spyOn(console, 'warn').mockImplementation(() => {})

afterEach(() => vi.restoreAllMocks())

describe('Chat, marking a done agent reviewed while it is in view', () => {
  it('marks the pane once when its screen opens on a done row', () => {
    const { marked } = watched('done')

    expect(marked).toEqual([PANE])
  })

  it('marks it once when the same done row is seen twice', () => {
    const { chat, marked } = watched('done')

    turned(chat, 'done', 'done')

    expect(marked).toEqual([PANE])
  })

  it('marks it as soon as the agent it watches turns done', () => {
    const { chat, marked } = watched('working')

    turned(chat, 'done')

    expect(marked).toEqual([PANE])
  })

  it('marks nothing until it watches, then marks a row already done', () => {
    const { chat, marked } = opened('working')
    turned(chat, 'done')

    expect(marked).toEqual([])

    chat.watchDone()

    expect(marked).toEqual([PANE])
  })

  it('marks every new arrival at done', () => {
    const { chat, marked } = watched('done')

    turned(chat, 'idle', 'done')

    expect(marked).toEqual([PANE, PANE])
  })

  it('marks again when it watches again after it stopped', () => {
    const { chat, stop, marked } = watched('done')
    stop()
    turned(chat, 'done')

    expect(marked).toEqual([PANE])

    chat.watchDone()

    expect(marked).toEqual([PANE, PANE])
  })

  it.each(['pending', 'waiting', 'failed', 'working', 'idle', null])('leaves a %s row alone', (status) => {
    const { marked } = watched(status)

    expect(marked).toEqual([])
  })

  it('leaves alone a pane it has no row for yet', () => {
    const marker = fakeMarker()
    const chat = fresh({ markReviewed: marker.markReviewed })
    chat.see(MACHINE, null)
    chat.watchDone()

    expect(chat.row).toBeNull()
    expect(marker.marked).toEqual([])
  })

  it('leaves alone a done pane of the same number on another machine', () => {
    const { chat, marked } = watched('working')

    chat.see(OTHER_MACHINE, [row(PANE, 'done')])

    expect(chat.gone).toBe(true)
    expect(marked).toEqual([])
  })

  it('marks nothing once its agent is gone', () => {
    const { chat, stop, marked } = watched('working')
    chat.see(MACHINE, [row(3, 'done')])
    stop()
    chat.watchDone()
    turned(chat, 'done')

    expect(chat.gone).toBe(true)
    expect(marked).toEqual([])
  })
})

describe('Chat, a mark that fails', () => {
  it.each<[string, () => ReturnType<MarkReviewed>]>([
    ['refused', () => Promise.resolve(refused(NO_AGENT))],
    ['thrown', () => Promise.reject(new Error(NO_AGENT))],
  ])('is %s into the log without touching the chat', async (_, answer) => {
    const warned = quiet()
    const { chat } = watched('done', answer)
    const before = { reading: chat.reading, outgoing: chat.outgoing, row: chat.row }
    await settle()

    expect(warned).toHaveBeenCalledWith(`mark --pane ${PANE} --reviewed failed: ${NO_AGENT}`)
    expect({ reading: chat.reading, outgoing: chat.outgoing, row: chat.row }).toEqual(before)
    expect(chat.gone).toBe(false)
  })

  it('is not tried again until the next arrival at done or the next watch', async () => {
    quiet()
    const { chat, stop, marked } = watched('done', () => Promise.resolve(refused(NO_AGENT)))
    await settle()
    turned(chat, 'done')

    expect(marked).toEqual([PANE])

    turned(chat, 'idle', 'done')
    stop()
    chat.watchDone()

    expect(marked).toEqual([PANE, PANE, PANE])
  })

  it('logs nothing when the mark goes through', async () => {
    const warned = quiet()
    watched('done')
    await settle()

    expect(warned).not.toHaveBeenCalled()
  })
})
