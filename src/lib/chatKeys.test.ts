import { describe, expect, it, vi } from 'vitest'
import type { Chat } from './chat.svelte'
import { MACHINE, PANE, row, pressedOk, fakeClock, fresh, seen } from './chatFakes'

const turned = (chat: Chat, ...statuses: (string | null)[]) => {
  const open: boolean[] = []
  for (const status of statuses) {
    chat.see(MACHINE, [row(3, 'working'), row(PANE, status)])
    open.push(chat.keysOpen)
  }
  return open
}

describe('Chat, its keys', () => {
  it('start closed, a tap opens them and another closes them', () => {
    const chat = seen()
    const open = [chat.keysOpen]
    chat.toggleKeys()
    open.push(chat.keysOpen)
    chat.toggleKeys()
    expect([...open, chat.keysOpen]).toEqual([false, true, false])
  })

  it('start closed before the agent is seen', () => {
    expect(fresh().keysOpen).toBe(false)
  })

  it('open on their own when the agent waits and close once it stops', () => {
    expect(turned(fresh(), 'working', 'waiting', 'done')).toEqual([false, true, false])
  })

  it('open on their own when the chat starts on a waiting agent', () => {
    expect(turned(fresh(), 'waiting')).toEqual([true])
  })

  it('stay open while the agent keeps waiting', () => {
    expect(turned(fresh(), 'working', 'waiting', 'waiting')).toEqual([false, true, true])
  })

  it('stay open after the wait when they were opened by hand before it', () => {
    const chat = seen({}, 'working')
    chat.toggleKeys()
    expect(turned(chat, 'waiting', 'done')).toEqual([true, true])
  })

  it('stay closed for the rest of the wait once closed by hand, and open again on the next one', () => {
    const chat = seen({}, 'working')
    turned(chat, 'waiting')
    chat.toggleKeys()
    expect(turned(chat, 'waiting', 'working', 'waiting')).toEqual([false, false, true])
  })

  it('stay open after the wait when they were closed and opened again by hand during it', () => {
    const chat = seen({}, 'working')
    turned(chat, 'waiting')
    chat.toggleKeys()
    chat.toggleKeys()
    expect(turned(chat, 'done')).toEqual([true])
  })

  it('close on their own when the agent stops waiting with no status', () => {
    expect(turned(fresh(), 'waiting', null)).toEqual([true, false])
  })
})

describe('Chat, closing its keys', () => {
  const armed = async (status = 'working') => {
    const pressKey = vi.fn(pressedOk)
    const chat = seen({ pressKey, clock: fakeClock().clock }, status)
    if (!chat.keysOpen) {
      chat.toggleKeys()
    }
    await chat.press('ctrl+c')
    return { chat, pressKey }
  }

  it('by hand disarms an armed Ctrl-C, so the next tap arms again', async () => {
    const { chat, pressKey } = await armed()
    chat.toggleKeys()
    const disarmed = chat.interrupting
    chat.toggleKeys()
    await chat.press('ctrl+c')
    expect([disarmed, chat.interrupting, pressKey.mock.calls]).toEqual([false, true, []])
  })

  it('on their own disarms an armed Ctrl-C', async () => {
    const { chat } = await armed('waiting')
    turned(chat, 'working')
    expect([chat.keysOpen, chat.interrupting]).toEqual([false, false])
  })

  it('leaves Ctrl-C armed while they stay open', async () => {
    const { chat } = await armed()
    turned(chat, 'waiting', 'done')
    expect([chat.keysOpen, chat.interrupting]).toEqual([true, true])
  })
})
