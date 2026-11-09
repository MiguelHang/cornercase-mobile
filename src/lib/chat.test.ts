import { afterEach, describe, expect, it, vi } from 'vitest'
import type { AttachImage } from './attach.svelte'
import type { AgentRow_Serialize as AgentRow, LastMessage_Serialize as LastMessage } from './bindings'
import {
  Chat,
  deliver,
  FOLLOW_MS,
  INTERRUPT_WINDOW_MS,
  read,
  timers,
  touches,
  whileShown,
  type Page,
  type PressKey,
  type ReadLast,
  type SendText,
} from './chat.svelte'
import {
  MACHINE,
  PANE,
  row,
  message,
  ok,
  refused,
  gate,
  noRead,
  sentOk,
  pressedOk,
  attachedOk,
  still,
  fakeClock,
  type Calls,
  fresh,
  seen,
  ready,
  settle,
  PASTED,
  attached,
} from './chatFakes'
import type { KeyName } from './keys'

describe('read', () => {
  it('hands the message over', async () => {
    expect(await read(() => Promise.resolve(ok(message('Done.'))), PANE)).toEqual({
      kind: 'message',
      message: message('Done.'),
    })
  })

  it("tells cornercase's refusal as unreadable, with its reason", async () => {
    expect(await read(() => Promise.resolve(refused('no message yet')), PANE)).toEqual({
      kind: 'unreadable',
      reason: 'no message yet',
    })
  })

  it.each<[string, ReadLast, string]>([
    ['a dropped connection', () => Promise.resolve(refused('the connection dropped', 'dropped')), 'the connection dropped'],
    ['no machine', () => Promise.resolve(refused('no machine is connected', 'not_connected')), 'no machine is connected'],
    ['a rejected call', () => Promise.reject(new Error('IPC is gone')), 'IPC is gone'],
  ])('tells %s as lost on the way, with its reason', async (_, readLast, reason) => {
    expect(await read(readLast, PANE)).toEqual({ kind: 'lost', reason })
  })

  it('asks for the pane given', async () => {
    const readLast = vi.fn(noRead)
    await read(readLast, PANE)
    expect(readLast.mock.calls).toEqual([[PANE]])
  })
})

describe('deliver', () => {
  it.each<[string, SendText, string | null]>([
    ['nothing once sent', sentOk, null],
    ['the reason of a refusal', () => Promise.resolve(refused('the agent is asking a question')), 'the agent is asking a question'],
    ['the message of a rejected call', () => Promise.reject(new Error('no window')), 'no window'],
  ])('gives %s', async (_, sendText, failed) => {
    expect(await deliver(() => sendText(PANE, 'hi', false))).toBe(failed)
  })
})

describe('touches', () => {
  it.each<[string, number, number | null, boolean]>([
    ['the status of its pane', 7, PANE, true],
    ['a drop, which may hide any pane', 7, null, true],
    ['the status of another pane', 7, 30, false],
    ['the status of its pane on another machine', 8, PANE, false],
    ['a drop on another machine', 8, null, false],
  ])('tells %s', (_, machine, pane, touched) => {
    expect(touches({ machine, pane }, 7, PANE)).toBe(touched)
  })

  it('touches nothing without a machine', () => {
    expect(touches({ machine: 7, pane: PANE }, null, PANE)).toBe(false)
  })
})

describe('Chat, its field', () => {
  it('starts empty, loading, and unable to send', () => {
    const chat = ready()
    expect([chat.draft, chat.sending, chat.reading, chat.canSend]).toEqual(['', false, { kind: 'loading' }, false])
  })

  it.each([
    ['blank text', ' \n\t', 'done', false],
    ['text', 'go on', 'done', true],
    ['text to an agent asking on its screen', 'go on', 'waiting', false],
    ['text to an agent without a status', 'go on', null, true],
  ])('with %s can send or not', (_, draft, status, canSend) => {
    const chat = ready(sentOk, status)
    chat.draft = draft
    expect(chat.canSend).toBe(canSend)
  })

  it('cannot send before its row is seen', () => {
    const chat = fresh()
    chat.draft = 'go on'
    expect(chat.canSend).toBe(false)
  })

  it('tells when the agent is asking on its screen', () => {
    expect([ready(sentOk, 'waiting').asking, ready(sentOk, 'done').asking]).toEqual([true, false])
  })

  it('sends the text as typed to its pane, and empties once sent', async () => {
    const sendText = vi.fn(sentOk)
    const chat = ready(sendText)
    chat.draft = '- not a flag\n'
    await chat.send()
    expect(sendText.mock.calls).toEqual([[PANE, '- not a flag\n', false]])
    expect([chat.draft, chat.sending]).toEqual(['', false])
  })

  it('is sending, and cannot send again, until the answer', async () => {
    const answer = gate<{ status: 'ok'; data: null }>()
    const sendText = vi.fn(() => answer.promise)
    const chat = ready(sendText)
    chat.draft = 'go on'
    const sent = chat.send()
    expect([chat.sending, chat.canSend]).toEqual([true, false])
    await chat.send()
    answer.open(ok(null))
    await sent
    expect(sendText).toHaveBeenCalledTimes(1)
  })

  it('empties the field even when the send fails', async () => {
    const chat = ready(() => Promise.resolve(refused('nothing was sent')))
    chat.draft = 'go on'
    await chat.send()
    expect([chat.draft, chat.sending]).toEqual(['', false])
  })

  it.each([
    ['blank text', ' ', 'done'],
    ['an agent asking on its screen', 'go on', 'waiting'],
  ])('does not send %s, and keeps it', async (_, draft, status) => {
    const sendText = vi.fn(sentOk)
    const chat = ready(sendText, status)
    chat.draft = draft
    await chat.send()
    expect(sendText).not.toHaveBeenCalled()
    expect([chat.draft, chat.outgoing]).toEqual([draft, null])
  })
})

describe('Chat, its attachments', () => {
  it('attaches an image to its pane and shows its chip', async () => {
    const attachImage = vi.fn(attachedOk)
    const chat = seen({ attachImage })
    expect(await chat.attach()).toBeNull()
    expect(attachImage.mock.calls).toEqual([[PANE]])
    expect(chat.attachments.chips).toEqual([{ name: 'IMG_0042.jpg', size: '47 KiB' }])
  })

  it('gives the reason the image was not attached', async () => {
    const chat = seen({ attachImage: () => Promise.resolve(refused('no machine is connected', 'not_connected')) })
    expect(await chat.attach()).toBe('no machine is connected')
    expect(chat.attachments.chips).toEqual([])
  })

  it.each<[string, (chat: Chat) => void]>([
    ['before its row is seen', () => {}],
    ['once gone', (chat) => chat.see(MACHINE, [row(3, 'working')])],
  ])('attaches nothing %s', async (_, leave) => {
    const attachImage = vi.fn(attachedOk)
    const chat = fresh({ attachImage })
    leave(chat)
    expect(await chat.attach()).toBeNull()
    expect(attachImage).not.toHaveBeenCalled()
  })

  it('forces the text past the pasted image, and drops the chip once sent', async () => {
    const sendText = vi.fn(sentOk)
    const chat = await attached(sendText)
    chat.draft = 'what is wrong here?'
    await chat.send()
    expect(sendText.mock.calls).toEqual([[PANE, 'what is wrong here?', true]])
    expect([chat.draft, chat.attachments.chips]).toEqual(['', []])
  })

  it('can send the image alone with an empty field', async () => {
    const sendText = vi.fn(sentOk)
    const chat = await attached(sendText)
    expect(chat.canSend).toBe(true)
    await chat.send()
    expect(sendText.mock.calls).toEqual([[PANE, '', true]])
  })

  it('gives the chip back when the send fails', async () => {
    const chat = await attached(() => Promise.resolve(refused('nothing was sent', 'not_sent')))
    await chat.send()
    expect(chat.attachments.chips.length).toBe(1)
  })

  it('keeps a chip attached while the send was on its way', async () => {
    const answer = gate<{ status: 'ok'; data: null }>()
    const chat = await attached(() => answer.promise)
    const sent = chat.send()
    await chat.attach()
    answer.open(ok(null))
    await sent
    expect(chat.attachments.chips.length).toBe(1)
  })

  it('cannot send an image to an agent asking on its screen', async () => {
    expect((await attached(sentOk, 'waiting')).canSend).toBe(false)
  })

  it('drops its chips once gone', async () => {
    const chat = await attached()
    chat.see(MACHINE, null)
    expect(chat.attachments.chips).toEqual([])
  })
})

const pastedThenPressing = async (pressKey: PressKey, sendText: SendText = sentOk) => {
  const chat = seen({ pressKey, sendText })
  await chat.attach()
  return chat
}

describe('Chat, its attachments under the keys', () => {
  it('drops the chips once Enter submits them, and sends plain again', async () => {
    const sendText = vi.fn(sentOk)
    const chat = await pastedThenPressing(pressedOk, sendText)
    expect(await chat.press('enter')).toBeNull()
    expect(chat.attachments.chips).toEqual([])
    chat.draft = 'and now?'
    await chat.send()
    expect(sendText.mock.calls).toEqual([[PANE, 'and now?', false]])
  })

  it('keeps the chips when Enter fails', async () => {
    const chat = await pastedThenPressing(() => Promise.resolve(refused('nothing was sent', 'not_sent')))
    expect(await chat.press('enter')).toBe('nothing was sent')
    expect(chat.attachments.chips.length).toBe(1)
  })

  it.each<KeyName>(['esc', 'down', 'tab', '1'])('keeps the chips after %s', async (key) => {
    const chat = await pastedThenPressing(pressedOk)
    expect(await chat.press(key)).toBeNull()
    expect(chat.attachments.chips.length).toBe(1)
  })

  it('keeps an image attached while Enter was on its way', async () => {
    const pressed = gate<{ status: 'ok'; data: null }>()
    const chat = await pastedThenPressing(() => pressed.promise)
    const entered = chat.press('enter')
    await chat.attach()
    pressed.open(ok(null))
    expect(await entered).toBeNull()
    expect(chat.attachments.chips.length).toBe(1)
  })
})

const attachingAnother = async (calls: Calls = {}) => {
  const picked = gate<typeof PASTED>()
  const attachImage = vi.fn<AttachImage>().mockResolvedValueOnce(PASTED).mockReturnValueOnce(picked.promise)
  const chat = seen({ ...calls, attachImage })
  await chat.attach()
  const attaching = chat.attach()
  const attached = async () => {
    picked.open(PASTED)
    await attaching
  }
  return { chat, attached }
}

describe('Chat, while an image is being attached', () => {
  it('cannot send, sends nothing, and keeps the text and every chip', async () => {
    const sendText = vi.fn(sentOk)
    const { chat, attached } = await attachingAnother({ sendText })
    chat.draft = 'what is wrong here?'
    expect(chat.canSend).toBe(false)
    await chat.send()
    expect(sendText).not.toHaveBeenCalled()
    await attached()
    expect([chat.draft, chat.attachments.chips.length]).toEqual(['what is wrong here?', 2])
  })

  it('rests Enter, which sends nothing, taps nothing and keeps every chip', async () => {
    const pressKey = vi.fn(pressedOk)
    const haptics = still()
    const { chat, attached } = await attachingAnother({ pressKey, haptics })
    expect(chat.canPressKey('enter')).toBe(false)
    expect(await chat.press('enter')).toBeNull()
    expect(pressKey).not.toHaveBeenCalled()
    expect(haptics.impactFeedback).not.toHaveBeenCalled()
    await attached()
    expect(chat.attachments.chips.length).toBe(2)
  })

  it.each<KeyName>(['esc', 'down', '1'])('still sends %s', async (key) => {
    const pressKey = vi.fn(pressedOk)
    const { chat } = await attachingAnother({ pressKey })
    expect(chat.canPressKey(key)).toBe(true)
    await chat.press(key)
    expect(pressKey.mock.calls).toEqual([[PANE, key]])
  })

  it('sends again once the image is attached, with every chip', async () => {
    const sendText = vi.fn(sentOk)
    const { chat, attached } = await attachingAnother({ sendText })
    chat.draft = 'and these?'
    await attached()
    expect(chat.canSend).toBe(true)
    await chat.send()
    expect(sendText.mock.calls).toEqual([[PANE, 'and these?', true]])
    expect(chat.attachments.chips).toEqual([])
  })

  it('presses Enter again once the image is attached, dropping every chip', async () => {
    const pressKey = vi.fn(pressedOk)
    const { chat, attached } = await attachingAnother({ pressKey })
    await attached()
    expect(chat.canPressKey('enter')).toBe(true)
    expect(await chat.press('enter')).toBeNull()
    expect(pressKey.mock.calls).toEqual([[PANE, 'enter']])
    expect(chat.attachments.chips).toEqual([])
  })

  it('cannot press Enter without its row, attaching or not', () => {
    expect(fresh().canPressKey('enter')).toBe(false)
    expect(fresh().canPressKey('esc')).toBe(false)
  })
})

describe('Chat, its reading', () => {
  it('shows the message read, or the reason it cannot be read', async () => {
    const readLast = vi
      .fn<ReadLast>()
      .mockResolvedValueOnce(ok(message('All done.')))
      .mockResolvedValue(refused('this agent keeps no record'))
    const chat = ready(sentOk, 'done', readLast)
    await chat.refresh()
    expect(chat.reading).toEqual({ kind: 'message', message: message('All done.') })
    await chat.refresh()
    expect(chat.reading).toEqual({ kind: 'unreadable', reason: 'this agent keeps no record' })
  })

  it('keeps the message shown when a read fails on the way', async () => {
    const readLast = vi
      .fn<ReadLast>()
      .mockResolvedValueOnce(ok(message('All done.')))
      .mockResolvedValueOnce(refused('the connection dropped', 'dropped'))
      .mockRejectedValue(new Error('IPC is gone'))
    const chat = ready(sentOk, 'done', readLast)
    await chat.refresh()
    await chat.refresh()
    await chat.refresh()
    expect(chat.reading).toEqual({ kind: 'message', message: message('All done.') })
  })

  it('shows why a read failed on the way while no message was shown', async () => {
    const chat = ready(sentOk, 'done', () => Promise.resolve(refused('the connection dropped', 'dropped')))
    await chat.refresh()
    expect(chat.reading).toEqual({ kind: 'unreadable', reason: 'the connection dropped' })
  })

  it('reads nothing before its row is seen', async () => {
    const readLast = vi.fn(noRead)
    const chat = fresh({ readLast })
    await chat.refresh()
    expect([readLast.mock.calls.length, chat.reading]).toEqual([0, { kind: 'loading' }])
  })

  it('reads once more, not twice, for two events during a read', async () => {
    const first = gate<ReturnType<typeof ok<LastMessage>>>()
    const readLast = vi.fn<ReadLast>().mockReturnValueOnce(first.promise).mockResolvedValue(ok(message('second')))
    const chat = ready(sentOk, 'done', readLast)
    const reading = chat.refresh()
    await Promise.all([chat.refresh(), chat.refresh()])
    expect(readLast).toHaveBeenCalledTimes(1)
    first.open(ok(message('first')))
    await reading
    expect(readLast).toHaveBeenCalledTimes(2)
    expect(chat.reading).toEqual({ kind: 'message', message: message('second') })
  })

  it('shows the answer of a read before the one queued behind it', async () => {
    const first = gate<ReturnType<typeof ok<LastMessage>>>()
    const second = gate<ReturnType<typeof ok<LastMessage>>>()
    const readLast = vi.fn<ReadLast>().mockReturnValueOnce(first.promise).mockReturnValueOnce(second.promise)
    const chat = ready(sentOk, 'done', readLast)
    const reading = chat.refresh()
    void chat.refresh()
    first.open(ok(message('first')))
    await settle()
    expect(chat.reading).toEqual({ kind: 'message', message: message('first') })
    second.open(ok(message('second')))
    await reading
    expect(chat.reading).toEqual({ kind: 'message', message: message('second') })
  })

  it('reads again once a read has settled', async () => {
    const readLast = vi.fn(noRead)
    const chat = ready(sentOk, 'done', readLast)
    await chat.refresh()
    await chat.refresh()
    expect(readLast).toHaveBeenCalledTimes(2)
  })

  it('drops a late answer from before the agent went away, and reads no more', async () => {
    const first = gate<ReturnType<typeof ok<LastMessage>>>()
    const readLast = vi.fn<ReadLast>().mockReturnValueOnce(first.promise).mockResolvedValue(ok(message('other')))
    const chat = ready(sentOk, 'done', readLast)
    const reading = chat.refresh()
    void chat.refresh()
    chat.see(MACHINE, [row(3, 'working')])
    first.open(ok(message('old')))
    await reading
    await chat.refresh()
    expect(chat.reading).toEqual({ kind: 'loading' })
    expect(readLast).toHaveBeenCalledTimes(1)
  })
})

describe('Chat, its row', () => {
  it('follows its row as it changes', () => {
    const chat = ready()
    chat.see(MACHINE, [row(PANE, 'working')])
    expect([chat.row?.status, chat.gone]).toEqual(['working', false])
  })

  it('waits while the agents are not known, before its row is seen', () => {
    const chat = fresh()
    chat.see(MACHINE, null)
    expect([chat.row, chat.gone]).toEqual([null, false])
    chat.see(MACHINE, [row(PANE, 'done')])
    expect([chat.row?.status, chat.gone]).toEqual(['done', false])
  })

  it.each<[string, AgentRow[] | null]>([
    ['its row is missing', [row(3, 'working')]],
    ['the agents stop being known once its row was seen', null],
  ])('is gone when %s, and loses the draft', (_, agents) => {
    const chat = ready()
    chat.draft = 'go on'
    chat.see(MACHINE, agents)
    expect([chat.gone, chat.row, chat.draft, chat.canSend]).toEqual([true, null, '', false])
  })

  it('is gone when the agents are known without its row', () => {
    const chat = fresh()
    chat.see(MACHINE, [row(3, 'working')])
    expect(chat.gone).toBe(true)
  })

  it.each<[string, number | null, AgentRow[] | null]>([
    ['another machine lists a row with its pane', 2, [row(PANE, 'done')]],
    ['another machine has not sent its agents yet', 2, null],
    ['no machine is active', null, null],
  ])('is gone when %s, before or after its row was seen', (_, machine, agents) => {
    const unseen = fresh()
    const seen = ready()
    unseen.see(machine, agents)
    seen.see(machine, agents)
    expect([unseen.gone, unseen.row, seen.gone, seen.row]).toEqual([true, null, true, null])
  })

  it('waits for the row of an agent it started, even once it stops being asked to', () => {
    const chat = fresh()
    chat.see(MACHINE, [row(3, 'working')], true)
    chat.see(MACHINE, [row(3, 'working')])
    expect([chat.row, chat.gone]).toEqual([null, false])
    chat.see(MACHINE, [row(PANE, 'working')])
    expect([chat.row?.status, chat.gone]).toEqual(['working', false])
  })

  it.each<[string, number, AgentRow[] | null]>([
    ['its row goes once seen', MACHINE, [row(3, 'working')]],
    ['another machine becomes active', 2, [row(PANE, 'done')]],
  ])('is gone when it started the agent and %s', (_, machine, agents) => {
    const chat = fresh()
    chat.see(MACHINE, [row(PANE, 'working')], true)
    chat.see(machine, agents, true)
    expect([chat.gone, chat.row]).toEqual([true, null])
  })

  it('stays gone when a row with its pane comes back', () => {
    const chat = ready()
    chat.see(MACHINE, null)
    chat.see(MACHINE, [row(PANE, 'done')])
    expect([chat.gone, chat.row]).toEqual([true, null])
  })
})

type Pressed = Awaited<ReturnType<PressKey>>

describe('Chat, its keys', () => {
  it('sends a key to its pane, and gives no reason once sent', async () => {
    const pressKey = vi.fn(pressedOk)
    expect(await seen({ pressKey }).press('down')).toBeNull()
    expect(pressKey.mock.calls).toEqual([[PANE, 'down']])
  })

  it('sends keys pressed quickly in the order pressed, without waiting for an answer', async () => {
    const answers = [gate<Pressed>(), gate<Pressed>(), gate<Pressed>()]
    const pressKey = vi.fn<PressKey>()
    answers.forEach((answer) => pressKey.mockReturnValueOnce(answer.promise))
    const chat = seen({ pressKey })
    const pressed = [chat.press('down'), chat.press('down'), chat.press('enter')]
    expect(pressKey.mock.calls).toEqual([
      [PANE, 'down'],
      [PANE, 'down'],
      [PANE, 'enter'],
    ])
    answers[2].open(ok(null))
    answers[0].open(refused('nothing was sent', 'not_sent'))
    answers[1].open(ok(null))
    expect(await Promise.all(pressed)).toEqual(['nothing was sent', null, null])
  })

  it('sends a key while a send is on its way, without waiting for it', async () => {
    const answer = gate<{ status: 'ok'; data: null }>()
    const pressKey = vi.fn(pressedOk)
    const chat = seen({ sendText: () => answer.promise, pressKey })
    chat.draft = 'go on'
    const sent = chat.send()
    expect([chat.sending, chat.canPress]).toEqual([true, true])
    expect(await chat.press('enter')).toBeNull()
    expect(pressKey.mock.calls).toEqual([[PANE, 'enter']])
    answer.open(ok(null))
    await sent
  })

  it.each<[string, PressKey, string]>([
    ["cornercase's reason", () => Promise.resolve(refused('no pane 12')), 'no pane 12'],
    ['a key skipped', () => Promise.resolve(refused('nothing was sent', 'not_sent')), 'nothing was sent'],
    [
      'a key left unanswered',
      () => Promise.resolve(refused('it may have been sent, but no answer came back', 'unanswered')),
      'it may have been sent, but no answer came back',
    ],
    [
      'no machine',
      () => Promise.resolve(refused('no machine is connected', 'not_connected')),
      'no machine is connected',
    ],
    ['a rejected call', () => Promise.reject(new Error('IPC is gone')), 'IPC is gone'],
  ])('gives %s when a key fails', async (_, pressKey, failed) => {
    expect(await seen({ pressKey }).press('esc')).toBe(failed)
  })

  it.each<[string, string | null]>([
    ['done', 'done'],
    ['working', 'working'],
    ['asking on its screen', 'waiting'],
    ['without a status', null],
  ])('can press keys to an agent %s', (_, status) => {
    expect(seen({}, status).canPress).toBe(true)
  })

  it.each<[string, (chat: Chat) => void]>([
    ['before its row is seen', () => {}],
    ['once gone', (chat) => chat.see(MACHINE, [row(3, 'working')])],
    ['once another machine is active', (chat) => chat.see(2, [row(PANE, 'done')])],
  ])('sends no key, and taps nothing, %s', async (_, after) => {
    const pressKey = vi.fn(pressedOk)
    const haptics = still()
    const chat = fresh({ pressKey, haptics })
    after(chat)
    const pressed = [await chat.press('enter'), await chat.press('ctrl+c'), await chat.press('ctrl+c')]
    expect([chat.canPress, chat.interrupting, pressed]).toEqual([false, false, [null, null, null]])
    expect(pressKey).not.toHaveBeenCalled()
    expect(haptics.impactFeedback).not.toHaveBeenCalled()
  })

  it('taps lightly for every key sent and for arming Ctrl-C', async () => {
    const haptics = still()
    const chat = seen({ haptics })
    await chat.press('down')
    await chat.press('ctrl+c')
    await chat.press('ctrl+c')
    expect(vi.mocked(haptics.impactFeedback).mock.calls).toEqual([['light'], ['light'], ['light']])
  })

  it('sends the key when the tap fails', async () => {
    const pressKey = vi.fn(pressedOk)
    const chat = seen({ pressKey, haptics: { impactFeedback: () => Promise.reject(new Error('no plugin')) } })
    expect(await chat.press('tab')).toBeNull()
    expect(pressKey.mock.calls).toEqual([[PANE, 'tab']])
  })
})

describe('Chat, its Ctrl-C', () => {
  const armed = () => {
    const { clock, tick } = fakeClock()
    const pressKey = vi.fn(pressedOk)
    return { chat: seen({ pressKey, clock }), pressKey, tick }
  }

  it('arms on the first tap and sends nothing', async () => {
    const { chat, pressKey } = armed()
    expect(await chat.press('ctrl+c')).toBeNull()
    expect(chat.interrupting).toBe(true)
    expect(pressKey).not.toHaveBeenCalled()
  })

  it('sends ctrl+c once on a second tap within the window, and disarms', async () => {
    const { chat, pressKey, tick } = armed()
    await chat.press('ctrl+c')
    tick(INTERRUPT_WINDOW_MS - 1)
    expect(await chat.press('ctrl+c')).toBeNull()
    expect([chat.interrupting, pressKey.mock.calls]).toEqual([false, [[PANE, 'ctrl+c']]])
  })

  it('gives the reason when the confirmed ctrl+c fails', async () => {
    const chat = seen({ pressKey: () => Promise.resolve(refused('nothing was sent', 'not_sent')) })
    await chat.press('ctrl+c')
    expect(await chat.press('ctrl+c')).toBe('nothing was sent')
  })

  it('disarms once the window is over, and the next tap arms again', async () => {
    const { chat, pressKey, tick } = armed()
    await chat.press('ctrl+c')
    tick(INTERRUPT_WINDOW_MS)
    expect(chat.interrupting).toBe(false)
    await chat.press('ctrl+c')
    expect([chat.interrupting, pressKey.mock.calls]).toEqual([true, []])
  })

  it('disarms when another key is pressed, which is sent', async () => {
    const { chat, pressKey } = armed()
    await chat.press('ctrl+c')
    expect(await chat.press('down')).toBeNull()
    expect([chat.interrupting, pressKey.mock.calls]).toEqual([false, [[PANE, 'down']]])
  })

  it.each<[string, KeyName]>([
    ['another key', 'down'],
    ['the confirming tap', 'ctrl+c'],
  ])('forgets the window of an arming disarmed by %s', async (_, key) => {
    const { chat, tick } = armed()
    await chat.press('ctrl+c')
    tick(1000)
    await chat.press(key)
    tick(1000)
    await chat.press('ctrl+c')
    tick(INTERRUPT_WINDOW_MS - 1000)
    expect(chat.interrupting).toBe(true)
    tick(1000)
    expect(chat.interrupting).toBe(false)
  })

  it('sends one ctrl+c for three quick taps, and stays armed', async () => {
    const { chat, pressKey } = armed()
    await chat.press('ctrl+c')
    await chat.press('ctrl+c')
    await chat.press('ctrl+c')
    expect([chat.interrupting, pressKey.mock.calls]).toEqual([true, [[PANE, 'ctrl+c']]])
  })
})

describe('timers', () => {
  afterEach(() => {
    vi.useRealTimers()
  })

  it('runs a callback once its time has passed, unless cancelled', () => {
    vi.useFakeTimers()
    const kept = vi.fn()
    const cancelled = vi.fn()
    timers.after(INTERRUPT_WINDOW_MS, kept)
    timers.after(INTERRUPT_WINDOW_MS, cancelled)()
    vi.advanceTimersByTime(INTERRUPT_WINDOW_MS - 1)
    expect(kept).not.toHaveBeenCalled()
    vi.advanceTimersByTime(1)
    expect([kept.mock.calls.length, cancelled.mock.calls.length]).toEqual([1, 0])
  })

  it("tells the phone's time", () => {
    vi.useFakeTimers({ now: Date.parse('2026-11-09T21:30:00.000Z') })
    expect(timers.now()).toBe(Date.parse('2026-11-09T21:30:00.000Z'))
  })
})

const followed = (status: string | null, readLast: ReadLast = vi.fn(noRead)) => {
  const { clock, tick } = fakeClock()
  const chat = seen({ readLast, clock }, status)
  const stop = chat.follow()
  return { chat, stop, tick, readLast }
}

describe('Chat, while it is followed', () => {
  it('reads its last message again every 10 s while the agent works', async () => {
    const { tick, readLast } = followed('working')
    tick(FOLLOW_MS - 1)
    expect(readLast).not.toHaveBeenCalled()
    tick(1)
    await settle()
    tick(FOLLOW_MS)
    await settle()
    expect(readLast).toHaveBeenCalledTimes(2)
  })

  it.each(['waiting', 'done', 'shell', null])('reads nothing while the agent is %s', (status) => {
    const { tick, readLast } = followed(status)
    tick(FOLLOW_MS * 3)
    expect(readLast).not.toHaveBeenCalled()
  })

  it('reads nothing while it is not followed', () => {
    const readLast = vi.fn(noRead)
    const { clock, tick } = fakeClock()
    seen({ readLast, clock }, 'working')
    tick(FOLLOW_MS * 3)
    expect(readLast).not.toHaveBeenCalled()
  })

  it('keeps its pace when the same work is seen again', () => {
    const { chat, tick, readLast } = followed('working')
    tick(FOLLOW_MS / 2)
    chat.see(MACHINE, [row(PANE, 'working')])
    tick(FOLLOW_MS / 2)
    expect(readLast).toHaveBeenCalledTimes(1)
  })

  it('stops when the agent stops working, and starts again when it works again', async () => {
    const { chat, tick, readLast } = followed('working')
    tick(FOLLOW_MS)
    await settle()
    chat.see(MACHINE, [row(PANE, 'done')])
    tick(FOLLOW_MS * 3)
    expect(readLast).toHaveBeenCalledTimes(1)
    chat.see(MACHINE, [row(PANE, 'working')])
    tick(FOLLOW_MS)
    expect(readLast).toHaveBeenCalledTimes(2)
  })

  it('stops when it is told to', () => {
    const { stop, tick, readLast } = followed('working')
    stop()
    tick(FOLLOW_MS * 3)
    expect(readLast).not.toHaveBeenCalled()
  })

  it('stops when the agent is gone', () => {
    const { chat, tick, readLast } = followed('working')
    chat.see(MACHINE, [row(3, 'working')])
    tick(FOLLOW_MS * 3)
    expect(chat.gone).toBe(true)
    expect(readLast).not.toHaveBeenCalled()
  })

  it('reads once more, not once per beat, behind a slow read', async () => {
    const first = gate<ReturnType<typeof ok<LastMessage>>>()
    const readLast = vi.fn<ReadLast>().mockReturnValueOnce(first.promise).mockResolvedValue(ok(message('second')))
    const { chat, tick } = followed('working', readLast)
    tick(FOLLOW_MS)
    tick(FOLLOW_MS)
    tick(FOLLOW_MS)
    expect(readLast).toHaveBeenCalledTimes(1)
    first.open(ok(message('first')))
    await settle()
    await settle()
    expect(readLast).toHaveBeenCalledTimes(2)
    expect(chat.reading).toEqual({ kind: 'message', message: message('second') })
  })

  it('keeps the message shown while it reads it again', async () => {
    const later = gate<ReturnType<typeof ok<LastMessage>>>()
    const readLast = vi.fn<ReadLast>().mockResolvedValueOnce(ok(message('Half'))).mockReturnValueOnce(later.promise)
    const { chat, tick } = followed('working', readLast)
    tick(FOLLOW_MS)
    await settle()
    tick(FOLLOW_MS)
    await settle()
    expect([readLast.mock.calls.length, chat.reading]).toEqual([2, { kind: 'message', message: message('Half') }])
    later.open(ok(message('Half, and the rest')))
    await settle()
    expect(chat.reading).toEqual({ kind: 'message', message: message('Half, and the rest') })
  })
})

class FakePage implements Page {
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

const watched = (visibilityState: DocumentVisibilityState) => {
  const page = new FakePage(visibilityState)
  const stop = vi.fn()
  const start = vi.fn(() => stop)
  const leave = whileShown(page, start)
  return { page, start, stop, leave }
}

describe('whileShown', () => {
  it('starts at once on a page that shows', () => {
    const { start, stop } = watched('visible')
    expect([start.mock.calls.length, stop.mock.calls.length]).toEqual([1, 0])
  })

  it('waits for a hidden page to show', () => {
    const { page, start } = watched('hidden')
    expect(start).not.toHaveBeenCalled()
    page.turn('visible')
    expect(start).toHaveBeenCalledTimes(1)
  })

  it('stops when the page hides and starts again when it shows', () => {
    const { page, start, stop } = watched('visible')
    page.turn('hidden')
    page.turn('hidden')
    expect(stop).toHaveBeenCalledTimes(1)
    page.turn('visible')
    page.turn('visible')
    expect(start).toHaveBeenCalledTimes(2)
  })

  it('stops and stops listening when it is left', () => {
    const { page, start, stop, leave } = watched('visible')
    leave()
    page.turn('hidden')
    page.turn('visible')
    expect([start.mock.calls.length, stop.mock.calls.length, page.listening]).toEqual([1, 1, 0])
  })

  it('stops nothing when it is left while the page is hidden', () => {
    const { stop, leave } = watched('hidden')
    leave()
    expect(stop).not.toHaveBeenCalled()
  })
})
