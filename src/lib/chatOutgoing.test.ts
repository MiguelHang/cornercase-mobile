import { describe, expect, it, vi } from 'vitest'
import type { ReadLast, SendText } from './chat.svelte'
import {
  MACHINE,
  PANE,
  row,
  message,
  ok,
  refused,
  gate,
  sentOk,
  attachedOk,
  fakeClock,
  type Calls,
  fresh,
  seen,
  ready,
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
  it('shows what you sent, sending, under the message it answers, with the field empty at once', async () => {
    const answer = gate<Sent>()
    const chat = await opened(() => answer.promise)
    chat.draft = 'fix thread.ts'
    const sent = chat.send()
    expect(chat.draft).toBe('')
    expect(chat.outgoing).toEqual({ text: 'fix thread.ts', chips: [], state: 'sending', reason: null, answered: SHOWN })
    answer.open(ok(null))
    await sent
    expect(chat.outgoing).toEqual({ text: 'fix thread.ts', chips: [], state: 'sent', reason: null, answered: SHOWN })
  })

  it('answers no message while it is not read yet', async () => {
    const chat = ready()
    chat.draft = 'go on'
    await chat.send()
    expect(chat.outgoing?.answered).toBeNull()
  })

  it('names the images that went with it', async () => {
    const chat = await attached()
    chat.draft = 'what is wrong here?'
    await chat.send()
    expect(chat.outgoing?.chips).toEqual([{ name: 'IMG_0042.jpg', size: '47 KiB' }])
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
    expect(chat.outgoing).toEqual({ text: 'fix thread.ts', chips: [], state: 'failed', reason, answered: SHOWN })
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

describe('Chat, Retry', () => {
  it('sends the same text again, under the message shown now', async () => {
    const sendText = sentOnce(refused('nothing was sent', 'not_sent'))
    const chat = await failedWith(sendText)
    expect(chat.canRetry).toBe(true)
    await chat.retry()
    expect(sendText.mock.calls).toEqual([
      [PANE, 'fix thread.ts', false],
      [PANE, 'fix thread.ts', false],
    ])
    expect(chat.outgoing).toEqual({ text: 'fix thread.ts', chips: [], state: 'sent', reason: null, answered: SHOWN })
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
  ])('drops the bubble, and touches nothing when it %s', async (_, outcome) => {
    const answer = gate<Sent>()
    const { clock, tick } = fakeClock()
    const chat = await opened(() => answer.promise, { attachImage: attachedOk, clock })
    await chat.attach()
    chat.draft = 'look'
    const sent = chat.send()
    chat.see(MACHINE, [row(3, 'working')])
    const left = chat.lastNews
    tick(5000)
    answer.open(outcome)
    await sent
    expect([chat.outgoing, chat.draft, chat.attachments.chips, chat.lastNews]).toEqual([null, '', [], left])
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
