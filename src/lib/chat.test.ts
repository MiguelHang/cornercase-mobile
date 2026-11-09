import { describe, expect, it, vi } from 'vitest'
import type { AgentRow_Serialize as AgentRow, CommandError, LastMessage_Serialize as LastMessage } from './bindings'
import { Chat, deliver, read, touches, type ReadLast, type SendText } from './chat.svelte'

const PANE = 12

const row = (pane: number, status: string | null): AgentRow => ({
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

const message = (text: string): LastMessage => ({ pane: PANE, agent: 'claude', text, written: null, turn_over: true })

const ok = <T>(data: T) => ({ status: 'ok' as const, data })

const refused = (message: string) => ({ status: 'error' as const, error: { code: 'failed', message } as CommandError })

const gate = <T>() => {
  let open: (value: T) => void = () => {}
  const promise = new Promise<T>((resolve) => (open = resolve))
  return { promise, open }
}

const noRead: ReadLast = () => Promise.resolve(ok(message('')))

const sentOk: SendText = () => Promise.resolve(ok(null))

const ready = (sendText: SendText = sentOk, status: string | null = 'done', readLast: ReadLast = noRead) => {
  const chat = new Chat(PANE, readLast, sendText)
  chat.see([row(3, 'working'), row(PANE, status)])
  return chat
}

const settle = () => new Promise((resolve) => setTimeout(resolve))

describe('read', () => {
  it('hands the message over', async () => {
    expect(await read(() => Promise.resolve(ok(message('Done.'))), PANE)).toEqual({
      kind: 'message',
      message: message('Done.'),
    })
  })

  it.each<[string, ReadLast, string]>([
    ['a refusal', () => Promise.resolve(refused('no message yet')), 'no message yet'],
    ['a rejected call', () => Promise.reject(new Error('IPC is gone')), 'IPC is gone'],
  ])('tells %s as unreadable, with its reason', async (_, readLast, reason) => {
    expect(await read(readLast, PANE)).toEqual({ kind: 'unreadable', reason })
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
    expect(await deliver(sendText, PANE, 'hi')).toBe(failed)
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
    expect([chat.draft, chat.sending, chat.error, chat.reading, chat.canSend]).toEqual([
      '',
      false,
      null,
      { kind: 'loading' },
      false,
    ])
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
    const chat = new Chat(PANE, noRead, sentOk)
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
    expect(sendText.mock.calls).toEqual([[PANE, '- not a flag\n']])
    expect([chat.draft, chat.sending, chat.error]).toEqual(['', false, null])
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

  it('keeps the text and the reason when the send fails, and gives the reason', async () => {
    const chat = ready(() => Promise.resolve(refused('nothing was sent')))
    chat.draft = 'go on'
    expect(await chat.send()).toBe('nothing was sent')
    expect([chat.draft, chat.sending, chat.error]).toEqual(['go on', false, 'nothing was sent'])
  })

  it('gives no reason once sent', async () => {
    const chat = ready()
    chat.draft = 'carry on'
    expect(await chat.send()).toBeNull()
  })

  it('forgets the last reason on the next send', async () => {
    const sendText = vi.fn<SendText>().mockResolvedValueOnce(refused('nothing was sent')).mockResolvedValue(ok(null))
    const chat = ready(sendText)
    chat.draft = 'go on'
    await chat.send()
    const again = chat.send()
    expect(chat.error).toBeNull()
    await again
    expect(chat.draft).toBe('')
  })

  it.each([
    ['blank text', ' ', 'done'],
    ['an agent asking on its screen', 'go on', 'waiting'],
  ])('does not send %s, and keeps it', async (_, draft, status) => {
    const sendText = vi.fn(sentOk)
    const chat = ready(sendText, status)
    chat.draft = draft
    expect(await chat.send()).toBeNull()
    expect(sendText).not.toHaveBeenCalled()
    expect(chat.draft).toBe(draft)
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
    chat.see([row(3, 'working')])
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
    chat.see([row(PANE, 'working')])
    expect([chat.row?.status, chat.gone]).toEqual(['working', false])
  })

  it('waits while the agents are not known, before its row is seen', () => {
    const chat = new Chat(PANE, noRead, sentOk)
    chat.see(null)
    expect([chat.row, chat.gone]).toEqual([null, false])
    chat.see([row(PANE, 'done')])
    expect([chat.row?.status, chat.gone]).toEqual(['done', false])
  })

  it.each<[string, AgentRow[] | null]>([
    ['its row is missing', [row(3, 'working')]],
    ['the agents stop being known once its row was seen', null],
  ])('is gone when %s, and loses the draft', (_, agents) => {
    const chat = ready()
    chat.draft = 'go on'
    chat.see(agents)
    expect([chat.gone, chat.row, chat.draft, chat.canSend]).toEqual([true, null, '', false])
  })

  it('is gone when the agents are known without its row', () => {
    const chat = new Chat(PANE, noRead, sentOk)
    chat.see([row(3, 'working')])
    expect(chat.gone).toBe(true)
  })

  it('stays gone when a row with its pane comes back', () => {
    const chat = ready()
    chat.see(null)
    chat.see([row(PANE, 'done')])
    expect([chat.gone, chat.row]).toEqual([true, null])
  })
})
