import { describe, expect, it } from 'vitest'
import type { AgentRow_Serialize as AgentRow, HistoryEntry_Serialize as Entry } from './bindings'
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
    const answer = failed ? ok(null) : refused('the machine is not reachable')
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

  it('marks a send sent once the machine has it', async () => {
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
    expect(said(chat.conversation)).toEqual(['gap', 'agent: Done.', 'gap', 'agent: Fixed.'])
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
      ['failed', 'the machine is not reachable'],
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

const told = (chat: ReturnType<typeof fresh>, status: string | null, agent = 'codex') => {
  const shown: AgentRow = { ...row(PANE, status), agent }
  chat.see(MACHINE, [shown])
}

const caughtUp = () => loaded(fakeHistory(stored), saying('Done.', 'Fixed.'))

describe('Chat, vouching for the conversation', () => {
  it('puts a gap before a first reading that is not the last message kept', async () => {
    const { chat } = await loaded(fakeHistory(stored), saying('Fixed.'))
    await chat.refresh()
    expect(said(chat.conversation)).toEqual(['gap', 'agent: Done.', 'gap', 'agent: Fixed.'])
  })

  it('adds no gap for messages read one after the other', async () => {
    const { chat } = await caughtUp()
    await chat.refresh()
    await chat.refresh()
    expect(said(chat.conversation)).toEqual(['gap', 'agent: Done.', 'agent: Fixed.'])
  })

  it('adds no gap for a turn followed on screen, from the send to its answer', async () => {
    const { chat } = await caughtUp()
    const stop = chat.follow()
    await chat.refresh()
    await sendOn(chat, 'fix the login')
    told(chat, 'working')
    told(chat, 'working')
    told(chat, 'done')
    await chat.refresh()
    stop()
    expect(said(chat.conversation)).toEqual(['gap', 'agent: Done.', 'you: fix the login', 'agent: Fixed.'])
  })

  it('adds no gap for a turn followed on screen that waits on a command and wakes up again', async () => {
    const { chat } = await caughtUp()
    await chat.refresh()
    await sendOn(chat, 'fix the login')
    told(chat, 'working')
    told(chat, 'shell')
    told(chat, 'working')
    told(chat, 'done')
    await chat.refresh()
    expect(said(chat.conversation)).toEqual(['gap', 'agent: Done.', 'you: fix the login', 'agent: Fixed.'])
  })

  it('adds no gap for the answer to a send made while done that starts once the agent is idle', async () => {
    const { chat } = await caughtUp()
    await chat.refresh()
    await sendOn(chat, 'fix the login')
    told(chat, 'idle')
    told(chat, 'working')
    await chat.refresh()
    expect(said(chat.conversation)).toEqual(['gap', 'agent: Done.', 'you: fix the login', 'agent: Fixed.'])
  })

  it('adds no gap for the answer to a send made before any reading', async () => {
    const { chat } = await loaded(fakeHistory(stored), saying('Fixed.'))
    await sendOn(chat, 'fix the login')
    told(chat, 'working')
    await chat.refresh()
    expect(said(chat.conversation)).toEqual(['gap', 'agent: Done.', 'you: fix the login', 'agent: Fixed.'])
  })

  it('adds no gap once the agent goes on after answering a question', async () => {
    const { chat } = await caughtUp()
    await chat.refresh()
    told(chat, 'waiting')
    told(chat, 'working')
    await chat.refresh()
    expect(said(chat.conversation)).toEqual(['gap', 'agent: Done.', 'agent: Fixed.'])
  })

  it.each([
    ['following starts again, back from the background', (chat: ReturnType<typeof fresh>) => chat.follow()],
    ['the streams of the machine are lost', (chat: ReturnType<typeof fresh>) => chat.lose()],
    ['the agent works on something sent from the machine', (chat: ReturnType<typeof fresh>) => told(chat, 'working')],
    ['the pane runs a command from the machine', (chat: ReturnType<typeof fresh>) => told(chat, 'shell')],
    ['another agent runs in the pane', (chat: ReturnType<typeof fresh>) => told(chat, 'done', 'claude')],
  ])('puts a gap before the next message once %s', async (_, lost) => {
    const { chat } = await caughtUp()
    await chat.refresh()
    lost(chat)
    await chat.refresh()
    expect(said(chat.conversation)).toEqual(['gap', 'agent: Done.', 'gap', 'agent: Fixed.'])
  })

  it('puts a gap before a message after a second turn it did not start', async () => {
    const { chat } = await loaded(fakeHistory(stored), saying('Done.', 'Fixed.', 'Tests pass.'))
    await chat.refresh()
    await sendOn(chat, 'fix the login')
    told(chat, 'working')
    told(chat, 'done')
    await chat.refresh()
    told(chat, 'working')
    await chat.refresh()
    expect(said(chat.conversation)).toEqual([
      'gap',
      'agent: Done.',
      'you: fix the login',
      'agent: Fixed.',
      'gap',
      'agent: Tests pass.',
    ])
  })

  it('puts a gap before a message after a turn from the machine that follows a send into a running turn', async () => {
    const { chat } = await loaded(fakeHistory(stored), saying('Done.', 'Fixed.', 'Tests pass.'))
    await chat.refresh()
    told(chat, 'working')
    await sendOn(chat, 'and the tests')
    told(chat, 'working')
    told(chat, 'done')
    await chat.refresh()
    told(chat, 'working')
    await chat.refresh()
    expect(said(chat.conversation)).toEqual([
      'gap',
      'agent: Done.',
      'you: and the tests',
      'agent: Fixed.',
      'gap',
      'agent: Tests pass.',
    ])
  })

  it('puts a gap before a message after a turn that a failed send did not start', async () => {
    const { chat } = await loaded(fakeHistory(stored), saying('Done.', 'Fixed.'), failingOnce())
    await chat.refresh()
    await sendOn(chat, 'fix the login')
    chat.edit()
    told(chat, 'working')
    await chat.refresh()
    expect(said(chat.conversation)).toEqual(['gap', 'agent: Done.', 'gap', 'agent: Fixed.'])
  })

  it('vouches for nothing after a send while the streams were lost', async () => {
    const answer = gate<Awaited<ReturnType<SendText>>>()
    const { chat } = await loaded(fakeHistory(stored), saying('Fixed.'), () => answer.promise)
    const sending = sendOn(chat, 'fix the login')
    chat.lose()
    answer.open(ok(null))
    await sending
    await chat.refresh()
    expect(said(chat.conversation)).toEqual(['gap', 'agent: Done.', 'you: fix the login', 'gap', 'agent: Fixed.'])
  })

  it('vouches for nothing after a reading that failed', async () => {
    const answers = [refused('no transcript'), ok(message('Fixed.'))]
    const { chat } = await loaded(fakeHistory(stored), () => Promise.resolve(answers.shift() ?? refused('no more')))
    await chat.refresh()
    await chat.refresh()
    expect(said(chat.conversation)).toEqual(['gap', 'agent: Done.', 'gap', 'agent: Fixed.'])
  })

  it('vouches for nothing after a reading that was on its way when the streams were lost', async () => {
    const first = gate<Awaited<ReturnType<ReadLast>>>()
    const later = saying('Fixed.')
    let asked = 0
    const { chat } = await loaded(fakeHistory(stored), () => (asked++ === 0 ? first.promise : later(PANE)))
    const reading = chat.refresh()
    chat.lose()
    first.open(ok(message('Done.')))
    await reading
    await chat.refresh()
    expect(said(chat.conversation)).toEqual(['gap', 'agent: Done.', 'gap', 'agent: Fixed.'])
  })
})
