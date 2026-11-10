import { describe, expect, it } from 'vitest'
import type { HistoryEntry_Serialize as Entry } from './bindings'
import type { ReadLast, SendText } from './chat.svelte'
import { MACHINE, PANE, fakeHistory, fresh, gate, message, ok, refused, row, sentOk, settle } from './chatFakes'

const stored: Entry[] = [
  { kind: 'gap', id: 0, seen: 5 },
  { kind: 'agent', id: 1, text: 'Done.', written: null, turn_over: true, seen: 5 },
]

const saying =
  (...said: string[]): ReadLast =>
  () =>
    Promise.resolve(ok(message(said.shift() ?? '')))

const failingOnce = (): SendText => {
  let failed = false
  return () => {
    const answer = failed ? ok(null) : refused('the Mac is not reachable')
    failed = true
    return Promise.resolve(answer)
  }
}

const opened = (history = fakeHistory(stored), readLast: ReadLast = saying(), sendText: SendText = sentOk) => {
  const chat = fresh({ history: history.history, readLast, sendText })
  chat.see(MACHINE, [row(PANE, 'done')])
  return { chat, ...history }
}

const loaded = async (...args: Parameters<typeof opened>) => {
  const open = opened(...args)
  await settle()
  return open
}

const said = (entries: Entry[]) =>
  entries.map((entry) => (entry.kind === 'gap' ? 'gap' : `${entry.kind}: ${entry.text}`))

const yours = (entries: Entry[]) => entries.filter((entry) => entry.kind === 'you')

const sendOn = async (chat: ReturnType<typeof fresh>, text: string) => {
  chat.draft = text
  await chat.send()
}

describe('Chat, opening the conversation', () => {
  it('loads the pane on its machine and boot as soon as its row arrives', async () => {
    const { chat, history } = opened()
    expect(history.open).toHaveBeenCalledWith(MACHINE, 'b1', PANE)
    expect(chat.conversation).toEqual([])
    await settle()
    expect(chat.conversation).toEqual(stored)
  })

  it('loads it once, however many rows come after', async () => {
    const { chat, history } = await loaded()
    chat.see(MACHINE, [row(PANE, 'working')])
    chat.see(MACHINE, [row(PANE, 'done')])
    expect(history.open).toHaveBeenCalledTimes(1)
  })

  it('waits for the row of an agent that is still starting', () => {
    const { history } = fakeHistory()
    const chat = fresh({ history })
    chat.see(MACHINE, [row(3, 'working')], true)
    expect(history.open).not.toHaveBeenCalled()
    chat.see(MACHINE, [row(3, 'working'), row(PANE, null)])
    expect(history.open).toHaveBeenCalledWith(MACHINE, 'b1', PANE)
  })

  it('writes nothing back when it only loaded', async () => {
    const { kept } = await loaded()
    expect(kept).toEqual([])
  })
})

describe('Chat, recording the conversation', () => {
  it('keeps a send, its answer and the next send in the order they happened', async () => {
    const { chat, kept } = await loaded(fakeHistory(), saying('Fixed the login.'))
    await sendOn(chat, 'fix the login')
    await chat.refresh()
    await sendOn(chat, 'and the tests')
    expect(said(chat.conversation)).toEqual(['gap', 'you: fix the login', 'agent: Fixed the login.', 'you: and the tests'])
    expect(kept.at(-1)).toEqual(chat.conversation)
  })

  it('marks a send sent once the Mac has it', async () => {
    const { chat } = await loaded()
    await sendOn(chat, 'fix the login')
    expect(yours(chat.conversation)).toEqual([
      { kind: 'you', id: 2, text: 'fix the login', chips: [], state: 'sent', reason: null, seen: 0 },
    ])
  })

  it('keeps the names of the images sent with it', async () => {
    const { chat } = await loaded()
    await chat.attach()
    await chat.send()
    expect(yours(chat.conversation).map((entry) => entry.chips)).toEqual([['IMG_0042.jpg']])
  })

  it('replaces a growing message in its place', async () => {
    const { chat } = await loaded(fakeHistory(), saying('Reading thread.ts', 'Reading thread.ts and chat.svelte.ts'))
    await chat.refresh()
    await chat.refresh()
    expect(said(chat.conversation)).toEqual(['gap', 'agent: Reading thread.ts and chat.svelte.ts'])
  })

  it('keeps the same message once, however often it is read', async () => {
    const { chat, kept } = await loaded(fakeHistory(), saying('Fixed.', 'Fixed.'))
    await chat.refresh()
    await chat.refresh()
    expect(said(chat.conversation)).toEqual(['gap', 'agent: Fixed.'])
    expect(kept).toHaveLength(1)
  })

  it('keeps nothing from a reading that failed', async () => {
    const { chat, kept } = await loaded(fakeHistory(stored), () => Promise.resolve(refused('no transcript')))
    await chat.refresh()
    expect(chat.conversation).toEqual(stored)
    expect(kept).toEqual([])
  })
})

describe('Chat, a reading or a send while the conversation loads', () => {
  it('adds a reading that came first after what was loaded', async () => {
    const load = gate<undefined>()
    const { chat, kept } = opened(fakeHistory(stored, load.promise), saying('Fixed.'))
    await chat.refresh()
    expect(chat.conversation).toEqual([])
    load.open(undefined)
    await settle()
    expect(said(chat.conversation)).toEqual(['gap', 'agent: Done.', 'agent: Fixed.'])
    expect(kept).toEqual([chat.conversation])
  })

  it('adds nothing for a reading the load already had', async () => {
    const load = gate<undefined>()
    const { chat, kept } = opened(fakeHistory(stored, load.promise), saying('Done.'))
    await chat.refresh()
    load.open(undefined)
    await settle()
    expect(chat.conversation).toEqual(stored)
    expect(kept).toEqual([])
  })

  it('adds a send that came first after what was loaded, with its state', async () => {
    const load = gate<undefined>()
    const { chat } = opened(fakeHistory(stored, load.promise))
    await sendOn(chat, 'fix the login')
    load.open(undefined)
    await settle()
    expect(said(chat.conversation)).toEqual(['gap', 'agent: Done.', 'you: fix the login'])
    expect(yours(chat.conversation).map((entry) => entry.state)).toEqual(['sent'])
  })
})

describe('Chat, a send that failed', () => {
  it('keeps the reason it failed', async () => {
    const { chat } = await loaded(fakeHistory(stored), saying(), failingOnce())
    await sendOn(chat, 'fix the login')
    expect(yours(chat.conversation).map((entry) => [entry.state, entry.reason])).toEqual([
      ['failed', 'the Mac is not reachable'],
    ])
  })

  it('updates the same entry on Retry', async () => {
    const { chat, kept } = await loaded(fakeHistory(stored), saying(), failingOnce())
    await sendOn(chat, 'fix the login')
    await chat.retry()
    expect(yours(chat.conversation)).toEqual([
      { kind: 'you', id: 2, text: 'fix the login', chips: [], state: 'sent', reason: null, seen: 0 },
    ])
    expect(kept.map((entries) => yours(entries).map((entry) => entry.state))).toEqual([
      ['sending'],
      ['failed'],
      ['sending'],
      ['sent'],
    ])
  })

  it('removes its entry on Edit', async () => {
    const { chat, kept } = await loaded(fakeHistory(stored), saying(), failingOnce())
    await sendOn(chat, 'fix the login')
    chat.edit()
    expect(chat.conversation).toEqual(stored)
    expect(kept.at(-1)).toEqual(stored)
  })
})

describe('Chat, once it is left', () => {
  it('writes nothing more for a send still on its way', async () => {
    const answer = gate<Awaited<ReturnType<SendText>>>()
    const { chat, kept } = await loaded(fakeHistory(stored), saying(), () => answer.promise)
    const sending = sendOn(chat, 'fix the login')
    await settle()
    expect(kept).toHaveLength(1)
    chat.leave()
    answer.open(ok(null))
    await sending
    expect(kept).toHaveLength(1)
  })

  it('takes nothing from a load that ends after it', async () => {
    const load = gate<undefined>()
    const { chat, kept } = opened(fakeHistory(stored, load.promise), saying('Fixed.'))
    await chat.refresh()
    chat.leave()
    load.open(undefined)
    await settle()
    expect(chat.conversation).toEqual([])
    expect(kept).toEqual([])
  })
})
