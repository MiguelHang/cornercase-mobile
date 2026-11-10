import { describe, expect, it, vi } from 'vitest'
import type { Card_Serialize as Card, QuestionChanged_Serialize as QuestionChanged } from './bindings'
import { whileShown } from './chat.svelte'
import {
  MACHINE,
  PANE,
  TAB,
  row,
  ok,
  refused,
  pressedOk,
  still,
  fresh,
  settle,
  asked,
  fakeAsker,
  FakePage,
} from './chatFakes'
import { digitOf } from './keys'

const QUESTION = ['Which colour?', '❯ 1. Red', '  2. Green', '  3. Blue']

const MENU = ['Which colour?', '1. Red', '2. Green', '3. Blue']

const tabbed = (open: number | null): Card => ({
  tabs: ['Colour', 'Size', 'Shape', 'Submit'].map((label) => ({ label, answered: false })),
  open,
  blocks: [{ kind: 'text', text: 'Which colour?' }],
  options: [{ number: 1, label: 'Red', detail: null, selected: true }],
  help: ['Enter to select'],
})

const watching = async (status: string | null = 'waiting', calls: Parameters<typeof fresh>[0] = {}) => {
  const told = fakeAsker()
  const chat = fresh({ ...calls, asker: told.asker })
  chat.see(MACHINE, [row(3, 'working'), row(PANE, status)])
  const stop = chat.watchQuestion()
  await settle()
  return { chat, stop, ...told }
}

const turned = async (chat: ReturnType<typeof fresh>, status: string) => {
  chat.see(MACHINE, [row(PANE, status)])
  await settle()
}

describe('Chat, its question', () => {
  it('opens the question of its pane once its agent waits, listening first', async () => {
    const { asker, listening } = await watching()
    expect(asker.open.mock.calls).toEqual([[TAB, PANE]])
    expect(asker.listen.mock.invocationCallOrder[0]).toBeLessThan(asker.open.mock.invocationCallOrder[0])
    expect(listening()).toBe(1)
  })

  it.each(['working', 'done', null])('opens nothing while its agent is %s', async (status) => {
    const { asker, listening } = await watching(status)
    expect([asker.open.mock.calls.length, asker.listen.mock.calls.length, listening()]).toEqual([0, 0, 0])
  })

  it('opens nothing while nobody watches it', async () => {
    const { asker } = fakeAsker()
    const chat = fresh({ asker })
    chat.see(MACHINE, [row(PANE, 'waiting')])
    await settle()
    expect(asker.open).not.toHaveBeenCalled()
  })

  it('opens once the agent starts waiting, and only once', async () => {
    const { chat, asker } = await watching('working')
    await turned(chat, 'waiting')
    await turned(chat, 'waiting')
    expect(asker.open.mock.calls).toEqual([[TAB, PANE]])
  })

  it('shows the rows, the options, the colours and the card of the question told', async () => {
    const { chat, tell } = await watching()
    const changed = asked(MENU, { card: tabbed(0) })
    tell(changed)
    expect(chat.question).toEqual({
      rows: changed.rows,
      options: changed.options,
      colours: changed.colours,
      card: tabbed(0),
    })
    expect(chat.question?.options.map((choice) => choice.label)).toEqual(['Red', 'Green', 'Blue'])
  })

  it('follows the question as it changes', async () => {
    const { chat, tell } = await watching()
    tell(asked(MENU))
    tell(asked(QUESTION))
    expect(chat.question?.rows.map((runs) => runs[0].text)).toEqual(QUESTION)
  })

  it.each<[string, Partial<QuestionChanged>]>([
    ['another machine', { machine: MACHINE + 1 }],
    ['another tab', { tab: TAB + 1 }],
    ['another pane', { pane: PANE + 1 }],
  ])('ignores the question of %s', async (_, changes) => {
    const { chat, tell } = await watching()
    tell(asked(MENU, changes))
    expect(chat.question).toBeNull()
  })

  it.each<[string, Partial<QuestionChanged>]>([
    ['closed', { closed: true, rows: [], options: [] }],
    ['without rows', { rows: [], options: [] }],
  ])('drops the question once told %s', async (_, changes) => {
    const { chat, tell } = await watching()
    tell(asked(MENU))
    tell(asked(MENU, changes))
    expect(chat.question).toBeNull()
  })
})

describe('Chat, closing its question', () => {
  it.each(['working', 'done'])('closes the question and drops it once its agent is %s', async (status) => {
    const { chat, asker, tell, listening } = await watching()
    tell(asked(MENU))
    await turned(chat, status)
    expect([chat.question, asker.close.mock.calls, listening()]).toEqual([null, [[TAB, PANE]], 0])
  })

  it('closes the question once its agent is gone', async () => {
    const { chat, asker, tell } = await watching()
    tell(asked(MENU))
    chat.see(MACHINE, [row(3, 'working')])
    await settle()
    expect([chat.gone, chat.question, asker.close.mock.calls]).toEqual([true, null, [[TAB, PANE]]])
  })

  it('closes the question once nobody watches it, and hears no more', async () => {
    const { chat, stop, asker, tell } = await watching()
    stop()
    await settle()
    tell(asked(MENU))
    expect([chat.question, asker.close.mock.calls]).toEqual([null, [[TAB, PANE]]])
  })

  it('opens the question again once its agent waits again', async () => {
    const { chat, asker } = await watching()
    await turned(chat, 'working')
    await turned(chat, 'waiting')
    expect(asker.open.mock.calls).toEqual([
      [TAB, PANE],
      [TAB, PANE],
    ])
    expect(asker.close.mock.calls).toEqual([[TAB, PANE]])
  })

  it('closes and opens again in the order asked, even before an open has settled', async () => {
    const told = fakeAsker()
    const calls: string[] = []
    told.asker.open.mockImplementation(() => (calls.push('open'), Promise.resolve(ok(null))))
    told.asker.close.mockImplementation(() => (calls.push('close'), Promise.resolve()))
    const chat = fresh({ asker: told.asker })
    chat.see(MACHINE, [row(PANE, 'waiting')])
    chat.watchQuestion()
    chat.see(MACHINE, [row(PANE, 'working')])
    chat.see(MACHINE, [row(PANE, 'waiting')])
    await settle()
    expect(calls).toEqual(['close', 'open'])
  })

  it('closes while the page hides and opens again once it shows', async () => {
    const page = new FakePage('visible')
    const { asker } = fakeAsker()
    const chat = fresh({ asker })
    chat.see(MACHINE, [row(PANE, 'waiting')])
    whileShown(page, () => chat.watchQuestion())
    await settle()
    page.turn('hidden')
    await settle()
    expect(asker.close.mock.calls).toEqual([[TAB, PANE]])
    page.turn('visible')
    await settle()
    expect(asker.open.mock.calls).toEqual([
      [TAB, PANE],
      [TAB, PANE],
    ])
  })
})

describe('Chat, a question it cannot open', () => {
  it('shows no question when cornercase refuses it, and tells no reason', async () => {
    const told = fakeAsker()
    told.asker.open.mockResolvedValue(refused('no tab 6'))
    const chat = fresh({ asker: told.asker })
    chat.see(MACHINE, [row(PANE, 'waiting')])
    expect(chat.watchQuestion()).toBeInstanceOf(Function)
    await settle()
    expect(chat.question).toBeNull()
  })

  it('opens nothing when it cannot listen, and still closes', async () => {
    const told = fakeAsker()
    told.asker.listen.mockRejectedValue(new Error('no events'))
    const chat = fresh({ asker: told.asker })
    chat.see(MACHINE, [row(PANE, 'waiting')])
    const stop = chat.watchQuestion()
    await settle()
    stop()
    await settle()
    expect([told.asker.open.mock.calls.length, told.asker.close.mock.calls]).toEqual([0, [[TAB, PANE]]])
  })
})

describe('Chat, its options', () => {
  it('presses the digit of the option on its pane, with a light tap', async () => {
    const pressKey = vi.fn(pressedOk)
    const haptics = still()
    const chat = fresh({ pressKey, haptics })
    chat.see(MACHINE, [row(PANE, 'waiting')])
    expect(await chat.pressOption(2)).toBeNull()
    expect(pressKey.mock.calls).toEqual([[PANE, ['2']]])
    expect(vi.mocked(haptics.impactFeedback).mock.calls).toEqual([['light']])
  })

  it('gives the reason the option was not pressed', async () => {
    const chat = fresh({ pressKey: () => Promise.resolve(refused('no pane 12')) })
    chat.see(MACHINE, [row(PANE, 'waiting')])
    expect(await chat.pressOption(1)).toBe('no pane 12')
  })

  it.each([10, 2.5])('presses nothing for an option numbered %s', async (number) => {
    const pressKey = vi.fn(pressedOk)
    const chat = fresh({ pressKey })
    chat.see(MACHINE, [row(PANE, 'waiting')])
    expect([await chat.pressOption(number), pressKey.mock.calls.length]).toEqual([null, 0])
  })
})

describe('Chat, the tabs of its question', () => {
  const tabsOf = async (open: number | null, answer: typeof pressedOk = pressedOk) => {
    const pressKey = vi.fn(answer)
    const haptics = still()
    const { chat, tell } = await watching('waiting', { pressKey, haptics })
    tell(asked(MENU, { card: tabbed(open) }))
    return { chat, pressKey, haptics }
  }

  it('presses the arrows to the tab on its pane in one call, with a light tap', async () => {
    const { chat, pressKey, haptics } = await tabsOf(1)
    expect(await chat.openTab(3)).toBeNull()
    expect(pressKey.mock.calls).toEqual([[PANE, ['right', 'right']]])
    expect(vi.mocked(haptics.impactFeedback).mock.calls).toEqual([['light']])
  })

  it('presses the arrows back to an earlier tab', async () => {
    const { chat, pressKey } = await tabsOf(2)
    await chat.openTab(0)
    expect(pressKey.mock.calls).toEqual([[PANE, ['left', 'left']]])
  })

  it('gives the reason the arrows were not pressed', async () => {
    const { chat } = await tabsOf(0, () => Promise.resolve(refused('no pane 12')))
    expect(await chat.openTab(1)).toBe('no pane 12')
  })

  it.each<[string, number | null, number]>([
    ['the open tab', 2, 2],
    ['a tab while none is open', null, 1],
  ])('presses nothing and taps nothing for %s', async (_, open, target) => {
    const { chat, pressKey, haptics } = await tabsOf(open)
    expect(await chat.openTab(target)).toBeNull()
    expect([pressKey.mock.calls.length, vi.mocked(haptics.impactFeedback).mock.calls.length]).toEqual([0, 0])
  })

  it('presses nothing once its agent is gone', async () => {
    const { chat, pressKey } = await tabsOf(0)
    chat.see(MACHINE, [row(3, 'working')])
    await settle()
    expect([await chat.openTab(2), pressKey.mock.calls.length]).toEqual([null, 0])
  })

  it('presses nothing without a question', async () => {
    const pressKey = vi.fn(pressedOk)
    const { chat } = await watching('waiting', { pressKey })
    expect([await chat.openTab(1), pressKey.mock.calls.length]).toEqual([null, 0])
  })
})

describe('digitOf', () => {
  it.each<[number, string | null]>([
    [1, '1'],
    [9, '9'],
    [0, '0'],
    [10, null],
    [-1, null],
  ])('names %s as %s', (number, digit) => {
    expect(digitOf(number)).toBe(digit)
  })
})
