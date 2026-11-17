import { describe, expect, it, vi } from 'vitest'
import type { AttachImage, PasteAttached } from './attach.svelte'
import { staysOnTheMachine, type Chat, type ReadLast, type SendText } from './chat.svelte'
import {
  MACHINE,
  PANE,
  row,
  message,
  ok,
  refused,
  gate,
  sentOk,
  pressedOk,
  attachedOk,
  pastedOk,
  discardedOk,
  image,
  fakeClock,
  type Calls,
  fresh,
  seen,
  settle,
  PASTED,
  attached,
} from './chatFakes'

type Sent = Awaited<ReturnType<SendText>>

const SHOWN = message('Which file should I fix?')

const opened = async (sendText: SendText, calls: Calls = {}) => {
  const chat = seen({ readLast: () => Promise.resolve(ok(SHOWN)), sendText, ...calls })
  await chat.refresh()
  return chat
}

const sentOnce = (answer: Sent) => vi.fn<SendText>().mockResolvedValueOnce(answer).mockResolvedValue(ok(null))

const failedWith = async (sendText: SendText, draft = 'fix thread.ts', calls: Calls = {}) => {
  const chat = await opened(sendText, calls)
  chat.draft = draft
  await chat.send()
  return chat
}

describe('Chat, what you sent', () => {
  it('shows what you sent, sending, with the field empty at once', async () => {
    const answer = gate<Sent>()
    const chat = await opened(() => answer.promise)
    chat.draft = 'fix thread.ts'
    const sent = chat.send()
    expect(chat.draft).toBe('')
    expect(chat.outgoing).toEqual({ text: 'fix thread.ts', chips: [], state: 'sending', reason: null })
    answer.open(ok(null))
    await sent
    expect(chat.outgoing).toEqual({ text: 'fix thread.ts', chips: [], state: 'sent', reason: null })
  })

  it('names the images that went with it', async () => {
    const chat = await attached()
    chat.draft = 'what is wrong here?'
    await chat.send()
    expect(chat.outgoing?.chips.map((chip) => [chip.name, chip.size])).toEqual([['IMG_0042.jpg', '47 KiB']])
  })

  it.each<[string, SendText, string]>([
    ["cornercase's refusal", () => Promise.resolve(refused('pane 12 is waiting')), 'pane 12 is waiting'],
    ['a skipped job', () => Promise.resolve(refused('nothing was sent', 'not_sent')), 'nothing was sent'],
    [
      'a lost answer',
      () => Promise.resolve(refused('it may have been sent, but no answer came back', 'unanswered')),
      'it may have been sent, but no answer came back',
    ],
    ['no machine', () => Promise.resolve(refused('no machine is connected', 'not_connected')), 'no machine is connected'],
    ['a rejected call', () => Promise.reject(new Error('IPC is gone')), 'IPC is gone'],
  ])('is not sent after %s, with its reason', async (_, sendText, reason) => {
    const chat = await failedWith(sendText)
    expect(chat.outgoing).toEqual({ text: 'fix thread.ts', chips: [], state: 'failed', reason })
    expect(chat.draft).toBe('')
  })

  it('sends one at a time, while the field takes text', async () => {
    const answer = gate<Sent>()
    const sendText = vi.fn(() => answer.promise)
    const chat = await opened(sendText)
    chat.draft = 'first'
    const sent = chat.send()
    chat.draft = 'second'
    expect(chat.canSend).toBe(false)
    await chat.send()
    answer.open(ok(null))
    await sent
    expect(sendText.mock.calls).toEqual([[PANE, 'first', false]])
    expect([chat.draft, chat.outgoing?.text, chat.canSend]).toEqual(['second', 'first', true])
  })

  it('shows the next send in place of the last one', async () => {
    const chat = await failedWith(sentOnce(refused('nothing was sent', 'not_sent')))
    chat.draft = 'something else'
    await chat.send()
    expect([chat.outgoing?.text, chat.outgoing?.state]).toEqual(['something else', 'sent'])
  })
})

type Pasted = Awaited<ReturnType<PasteAttached>>

const PASTE = ok({ pane: PANE, path: '/tmp/IMG_0042.jpg', bytes: 48213 })

const twoImages = (): AttachImage => {
  let last = 0
  return () => Promise.resolve(ok(image(++last)))
}

const holdingTwo = async (calls: Calls) => {
  const chat = await opened(calls.sendText ?? sentOk, { attachImage: twoImages(), ...calls })
  await chat.attach()
  await chat.attach()
  return chat
}

describe('Chat, the images kept on the phone', () => {
  it('pastes each one in the order attached, then forces the text past them', async () => {
    const calls: unknown[][] = []
    const pasteAttached = vi.fn<PasteAttached>((pane, id) => (calls.push(['paste', pane, id]), pastedOk(pane, id)))
    const sendText = vi.fn<SendText>((...args) => (calls.push(['send', ...args]), sentOk(...args)))
    const chat = await holdingTwo({ pasteAttached, sendText })
    chat.draft = 'what is wrong here?'
    await chat.send()
    expect(calls).toEqual([
      ['paste', PANE, 1],
      ['paste', PANE, 2],
      ['send', PANE, 'what is wrong here?', true],
    ])
    expect([chat.outgoing?.state, chat.outgoing?.chips.length, chat.attachments.chips]).toEqual(['sent', 2, []])
  })

  it('shows one bubble, sending, while the images are pasted', async () => {
    const pasted = gate<Pasted>()
    const sendText = vi.fn(sentOk)
    const chat = await holdingTwo({ pasteAttached: () => pasted.promise, sendText })
    chat.draft = 'look'
    const sent = chat.send()
    expect([chat.outgoing?.text, chat.outgoing?.state, chat.outgoing?.chips.length]).toEqual(['look', 'sending', 2])
    expect(chat.attachments.chips).toEqual([])
    expect(chat.canSend).toBe(false)
    pasted.open(PASTE)
    await sent
    expect(sendText).toHaveBeenCalledTimes(1)
  })

  it('pastes an image sent alone, then sends the empty text, which presses Enter', async () => {
    const pasteAttached = vi.fn(pastedOk)
    const sendText = vi.fn(sentOk)
    const chat = await opened(sendText, { pasteAttached })
    await chat.attach()
    await chat.send()
    expect(pasteAttached.mock.calls).toEqual([[PANE, 1]])
    expect(sendText.mock.calls).toEqual([[PANE, '', true]])
  })

  it('pastes nothing for text alone', async () => {
    const pasteAttached = vi.fn(pastedOk)
    const chat = await failedWith(sentOk, 'go on', { pasteAttached })
    expect([chat.outgoing?.state, pasteAttached.mock.calls]).toEqual(['sent', []])
  })

  it('sends no text once a paste fails, and says why', async () => {
    const pasteAttached = vi.fn<PasteAttached>(() => Promise.resolve(refused('pane 12 is busy')))
    const sendText = vi.fn(sentOk)
    const chat = await holdingTwo({ pasteAttached, sendText })
    chat.draft = 'look'
    await chat.send()
    expect(pasteAttached).toHaveBeenCalledTimes(1)
    expect(sendText).not.toHaveBeenCalled()
    expect([chat.outgoing?.state, chat.outgoing?.reason, chat.attachments.chips.length]).toEqual([
      'failed',
      'pane 12 is busy',
      2,
    ])
  })
})

const halfWay = async (calls: Calls = {}) => {
  const pasteAttached = vi.fn<PasteAttached>(pastedOk).mockResolvedValueOnce(PASTE).mockResolvedValueOnce(refused('pane 12 is busy'))
  const sendText = vi.fn(sentOk)
  const chat = await holdingTwo({ pasteAttached, sendText, ...calls })
  chat.draft = 'look'
  await chat.send()
  return { chat, pasteAttached, sendText }
}

const pastedMarks = (chat: Chat) => chat.attachments.chips.map((chip) => [chip.id, chip.pasted])

describe('Chat, a send that fails half-way through its images', () => {
  it('sends no text, and keeps the image pasted marked as on the machine and the other one waiting', async () => {
    const { chat, sendText } = await halfWay()
    expect(sendText).not.toHaveBeenCalled()
    expect([chat.outgoing?.state, chat.outgoing?.reason]).toEqual(['failed', 'pane 12 is busy'])
    expect(pastedMarks(chat)).toEqual([
      [1, true],
      [2, false],
    ])
  })

  it('pastes only the image missing on Retry, then forces the text past both', async () => {
    const { chat, pasteAttached, sendText } = await halfWay()
    await chat.retry()
    expect(pasteAttached.mock.calls).toEqual([
      [PANE, 1],
      [PANE, 2],
      [PANE, 2],
    ])
    expect(sendText.mock.calls).toEqual([[PANE, 'look', true]])
    expect([chat.outgoing?.state, chat.attachments.chips]).toEqual(['sent', []])
  })

  it('gives the text back on Edit, and keeps both images as they are', async () => {
    const { chat } = await halfWay()
    chat.edit()
    expect([chat.draft, chat.outgoing]).toEqual(['look', null])
    expect(pastedMarks(chat)).toEqual([
      [1, true],
      [2, false],
    ])
  })
})

describe('Chat, removing an image', () => {
  it('forgets one not pasted yet, with nothing to tell', async () => {
    const discard = vi.fn(discardedOk)
    const chat = await holdingTwo({ discard })
    expect(await chat.remove(2)).toBeNull()
    expect(discard.mock.calls).toEqual([[[2]]])
    expect(chat.attachments.chips.map((chip) => chip.id)).toEqual([1])
  })

  it('tells why it could not be forgotten', async () => {
    const chat = await holdingTwo({ discard: () => Promise.reject(new Error('IPC is gone')) })
    expect(await chat.remove(1)).toEqual({ level: 'error', message: 'IPC is gone' })
  })

  it("tells one already pasted stays in the agent's input, and forces the next send past it", async () => {
    const discard = vi.fn(discardedOk)
    const { chat, sendText } = await halfWay({ discard })
    expect(await chat.remove(1)).toEqual({ level: 'info', message: staysOnTheMachine('codex') })
    expect(staysOnTheMachine('codex')).toBe("The image stays in codex's input on the machine")
    expect(discard).not.toHaveBeenCalled()
    await chat.remove(2)
    await chat.retry()
    chat.draft = 'and now?'
    await chat.send()
    expect(sendText.mock.calls).toEqual([
      [PANE, 'look', true],
      [PANE, 'and now?', false],
    ])
  })
})

describe('Chat, Enter with images attached', () => {
  it('presses Enter alone, pasting nothing and keeping every image', async () => {
    const pasteAttached = vi.fn(pastedOk)
    const pressKey = vi.fn(pressedOk)
    const discard = vi.fn(discardedOk)
    const chat = await holdingTwo({ pasteAttached, pressKey, discard })
    expect(await chat.press('enter')).toBeNull()
    expect([pressKey.mock.calls, pasteAttached.mock.calls, discard.mock.calls]).toEqual([[[PANE, ['enter']]], [], []])
    expect(chat.attachments.chips.length).toBe(2)
  })
})

describe('Chat, left', () => {
  it('forgets every image kept on the phone, as Back does', async () => {
    const discard = vi.fn(discardedOk)
    const chat = await holdingTwo({ discard })
    chat.leave()
    expect([chat.gone, chat.attachments.chips]).toEqual([true, []])
    expect(discard.mock.calls).toEqual([[[1, 2]]])
  })

  it('forgets them once, however often it is left', async () => {
    const discard = vi.fn(discardedOk)
    const chat = await holdingTwo({ discard })
    chat.leave()
    chat.leave()
    expect(discard).toHaveBeenCalledTimes(1)
  })

  it('forgets an image the picker brings back after it was left, so it neither shows nor counts', async () => {
    const picked = gate<Awaited<ReturnType<AttachImage>>>()
    const discard = vi.fn(discardedOk)
    const chat = await opened(sentOk, { attachImage: () => picked.promise, discard })
    const attaching = chat.attach()
    chat.leave()
    picked.open(ok(image(1)))
    await attaching
    expect(chat.attachments.chips).toEqual([])
    expect(discard.mock.calls).toEqual([[[1]]])
  })
})

describe('Chat, Retry', () => {
  it('sends the same text again', async () => {
    const sendText = sentOnce(refused('nothing was sent', 'not_sent'))
    const chat = await failedWith(sendText)
    expect(chat.canRetry).toBe(true)
    await chat.retry()
    expect(sendText.mock.calls).toEqual([
      [PANE, 'fix thread.ts', false],
      [PANE, 'fix thread.ts', false],
    ])
    expect(chat.outgoing).toEqual({ text: 'fix thread.ts', chips: [], state: 'sent', reason: null })
  })

  it('forces the text past the images still in the input, and drops them once sent', async () => {
    const sendText = sentOnce(refused('nothing was sent', 'not_sent'))
    const chat = await opened(sendText)
    await chat.attach()
    chat.draft = 'what is wrong here?'
    await chat.send()
    await chat.retry()
    expect(sendText.mock.calls).toEqual([
      [PANE, 'what is wrong here?', true],
      [PANE, 'what is wrong here?', true],
    ])
    expect([chat.outgoing?.state, chat.outgoing?.chips.length, chat.attachments.chips]).toEqual(['sent', 1, []])
  })

  it('fails again with the new reason', async () => {
    const sendText = vi
      .fn<SendText>()
      .mockResolvedValueOnce(refused('nothing was sent', 'not_sent'))
      .mockResolvedValue(refused('no machine is connected', 'not_connected'))
    const chat = await failedWith(sendText)
    await chat.retry()
    expect([chat.outgoing?.state, chat.outgoing?.reason]).toEqual(['failed', 'no machine is connected'])
  })

  it('sends nothing unless the send failed', async () => {
    const sendText = vi.fn(sentOk)
    const chat = await failedWith(sendText)
    expect(chat.canRetry).toBe(false)
    await chat.retry()
    expect(sendText).toHaveBeenCalledTimes(1)
  })

  it('rests while the agent asks on its screen', async () => {
    const sendText = sentOnce(refused('nothing was sent', 'not_sent'))
    const chat = await failedWith(sendText)
    chat.see(MACHINE, [row(PANE, 'waiting')])
    expect(chat.canRetry).toBe(false)
    await chat.retry()
    expect(sendText).toHaveBeenCalledTimes(1)
  })

  it('rests while an image is being attached', async () => {
    const picked = gate<typeof PASTED>()
    const sendText = sentOnce(refused('nothing was sent', 'not_sent'))
    const chat = await failedWith(sendText, 'look', { attachImage: () => picked.promise })
    const attaching = chat.attach()
    expect(chat.canRetry).toBe(false)
    await chat.retry()
    picked.open(PASTED)
    await attaching
    expect(sendText).toHaveBeenCalledTimes(1)
  })
})

describe('Chat, Edit', () => {
  const failing = () => Promise.resolve(refused('nothing was sent', 'not_sent'))

  it('gives the text back to an empty field and takes the bubble away', async () => {
    const chat = await failedWith(failing)
    chat.edit()
    expect([chat.draft, chat.outgoing]).toEqual(['fix thread.ts', null])
  })

  it('puts the text before what was typed since, after a blank line', async () => {
    const chat = await failedWith(failing)
    chat.draft = 'and the tests'
    chat.edit()
    expect(chat.draft).toBe('fix thread.ts\n\nand the tests')
  })

  it('leaves the field as it is for an image sent alone, and keeps its chip', async () => {
    const chat = await opened(failing)
    await chat.attach()
    await chat.send()
    expect([chat.outgoing?.text, chat.outgoing?.state]).toEqual(['', 'failed'])
    chat.draft = 'typed'
    chat.edit()
    expect([chat.draft, chat.outgoing, chat.attachments.chips.length]).toEqual(['typed', null, 1])
  })

  it.each<[string, SendText]>([
    ['sending', () => new Promise<Sent>(() => {})],
    ['sent', sentOk],
  ])('does nothing while %s', async (_, sendText) => {
    const chat = await opened(sendText)
    chat.draft = 'go on'
    void chat.send()
    await settle()
    chat.edit()
    expect([chat.draft, chat.outgoing?.text]).toEqual(['', 'go on'])
  })
})

describe('Chat, a send on its way when the agent goes', () => {
  it.each<[string, Sent]>([
    ['succeeds', ok(null)],
    ['fails', refused('nothing was sent', 'not_sent')],
  ])('drops the bubble and touches nothing when it %s, its image already off the phone', async (_, outcome) => {
    const answer = gate<Sent>()
    const { clock, tick } = fakeClock()
    const discard = vi.fn(discardedOk)
    const chat = await opened(() => answer.promise, { attachImage: attachedOk, discard, clock })
    await chat.attach()
    chat.draft = 'look'
    const sent = chat.send()
    chat.see(MACHINE, [row(3, 'working')])
    const left = chat.lastNews
    tick(5000)
    answer.open(outcome)
    await sent
    expect([chat.outgoing, chat.draft, chat.attachments.chips, chat.lastNews]).toEqual([null, '', [], left])
    expect(discard).not.toHaveBeenCalled()
  })

  it('pastes none of the images left once it is gone, and forgets them', async () => {
    const pasted = gate<Pasted>()
    const pasteAttached = vi.fn<PasteAttached>(() => pasted.promise)
    const sendText = vi.fn(sentOk)
    const discard = vi.fn(discardedOk)
    const chat = await holdingTwo({ pasteAttached, sendText, discard })
    const sent = chat.send()
    chat.see(MACHINE, [row(3, 'working')])
    pasted.open(PASTE)
    await sent
    expect([pasteAttached.mock.calls.length, sendText.mock.calls.length]).toEqual([1, 0])
    expect(discard.mock.calls).toEqual([[[2]]])
  })
})

describe('Chat, its last news', () => {
  const clocked = () => {
    const { clock, tick } = fakeClock()
    tick(1000)
    return { clock, tick }
  }

  it('starts when it opens', () => {
    expect(fresh(clocked()).lastNews).toBe(1000)
  })

  it('tells the time by the clock it was given', () => {
    const { clock, tick } = clocked()
    const chat = fresh({ clock })
    tick(500)
    expect(chat.now()).toBe(1500)
  })

  it('comes with a change of status, not with the same one again', () => {
    const { clock, tick } = clocked()
    const chat = seen({ clock }, 'working')
    tick(500)
    chat.see(MACHINE, [row(PANE, 'working')])
    expect(chat.lastNews).toBe(1000)
    tick(500)
    chat.see(MACHINE, [row(PANE, 'waiting')])
    expect(chat.lastNews).toBe(2000)
  })

  it('keeps the status heard across a row hidden by a drop', () => {
    const { clock, tick } = clocked()
    const chat = seen({ clock }, 'working')
    tick(500)
    chat.see(MACHINE, null)
    chat.see(MACHINE, [row(PANE, 'working')])
    expect(chat.lastNews).toBe(1000)
  })

  it('comes with a reading of another text, not with the same text again', async () => {
    const { clock, tick } = clocked()
    const readLast = vi
      .fn<ReadLast>()
      .mockResolvedValueOnce(ok(message('Reading thread.ts')))
      .mockResolvedValueOnce(ok({ ...message('Reading thread.ts'), turn_over: false }))
      .mockResolvedValue(ok(message('Reading thread.ts and its tests')))
    const chat = seen({ readLast, clock }, 'working')
    const times = []
    for (const later of [500, 500, 500]) {
      tick(later)
      await chat.refresh()
      times.push(chat.lastNews)
    }
    expect(times).toEqual([1500, 1500, 2500])
  })

  it('takes no news from a reading that fails', async () => {
    const { clock, tick } = clocked()
    const chat = seen({ readLast: () => Promise.resolve(refused('no message yet')), clock })
    tick(500)
    await chat.refresh()
    expect(chat.lastNews).toBe(1000)
  })

  it('comes with a confirmed send, not with a failed one', async () => {
    const { clock, tick } = clocked()
    const chat = seen({ sendText: sentOnce(refused('nothing was sent', 'not_sent')), clock })
    chat.draft = 'go on'
    tick(500)
    await chat.send()
    expect(chat.lastNews).toBe(1000)
    tick(500)
    await chat.retry()
    expect(chat.lastNews).toBe(2000)
  })
})
