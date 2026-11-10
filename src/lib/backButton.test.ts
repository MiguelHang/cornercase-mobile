import { describe, expect, it } from 'vitest'
import { backPressed, handled, watchBackButton, type BackPress, type Listen, type Unregistering } from './backButton'
import type { Route } from './router.svelte'

const settled = () => new Promise((resolve) => setTimeout(resolve, 0))

class FakeBackButton {
  handlers: (() => void)[] = []
  unregistered = 0
  #registered: (listener: Unregistering) => void = () => {}
  #refused: (error: unknown) => void = () => {}

  listen: Listen = (handler) => {
    this.handlers.push(handler)
    return new Promise((resolve, reject) => {
      this.#registered = resolve
      this.#refused = reject
    })
  }

  register(unregister = async () => {}) {
    this.#registered({
      unregister: () => {
        this.unregistered += 1
        return unregister()
      },
    })
  }

  refuse(error: unknown) {
    this.#refused(error)
  }

  press() {
    for (const handler of this.handlers) handler()
  }
}

class Answers {
  press: BackPress = 'back'
  heard: string[] = []

  watching = {
    pressed: () => this.press,
    onback: () => this.heard.push('back'),
    onscan: () => this.heard.push('scan'),
    failed: (error: unknown) => this.heard.push(`failed: ${String(error)}`),
  }
}

const watched = () => {
  const button = new FakeBackButton()
  const answers = new Answers()
  const stop = watchBackButton(answers.watching, button.listen)
  return { button, answers, stop }
}

describe('backPressed', () => {
  const below: Route = { name: 'home' }

  it.each([
    ['leaves iOS alone', 'ios', below, false, 'none'],
    ['leaves iOS alone with the scanner open', 'ios', below, true, 'none'],
    ['cancels the scanner on Android', 'android', below, true, 'scanner'],
    ['cancels the scanner on Android over the first screen', 'android', null, true, 'scanner'],
    ['goes back on Android over another screen', 'android', below, false, 'back'],
    ['leaves the first screen on Android to the system', 'android', null, false, 'system'],
  ] as const)('%s', (_, on, under, scanning, press) => {
    expect(backPressed(on, under, scanning)).toBe(press)
  })
})

describe('handled', () => {
  it.each([
    ['none', false],
    ['system', false],
    ['scanner', true],
    ['back', true],
  ] as const)('takes the back button for %s: %s', (press, takes) => {
    expect(handled(press)).toBe(takes)
  })
})

describe('watchBackButton', () => {
  describe('a press', () => {
    it.each([
      ['goes back', 'back', ['back']],
      ['cancels the scanner', 'scanner', ['scan']],
      ['does nothing on the first screen', 'system', []],
      ['does nothing off Android', 'none', []],
    ] as const)('%s', (_, press, heard) => {
      const { button, answers } = watched()
      button.register()
      answers.press = press
      button.press()
      expect(answers.heard).toEqual(heard)
    })

    it('asks what to do when pressed, not when watched', () => {
      const { button, answers } = watched()
      answers.press = 'scanner'
      button.press()
      answers.press = 'back'
      button.press()
      expect(answers.heard).toEqual(['scan', 'back'])
    })

    it('listens once', () => {
      expect(watched().button.handlers).toHaveLength(1)
    })
  })

  describe('stopping', () => {
    it('unregisters the listener', async () => {
      const { button, stop } = watched()
      button.register()
      stop()
      await settled()
      expect(button.unregistered).toBe(1)
    })

    it('unregisters a listener registered after it stopped', async () => {
      const { button, stop } = watched()
      stop()
      button.register()
      await settled()
      expect(button.unregistered).toBe(1)
    })

    it('ignores a press that comes before the listener is gone', () => {
      const { button, answers, stop } = watched()
      stop()
      button.press()
      expect(answers.heard).toEqual([])
    })

    it('says when the listener cannot be removed', async () => {
      const { button, answers, stop } = watched()
      button.register(() => Promise.reject(new Error('gone')))
      stop()
      await settled()
      expect(answers.heard).toEqual(['failed: Error: gone'])
    })
  })

  describe('a listener that cannot be registered', () => {
    it('says so once, even when stopped', async () => {
      const { button, answers, stop } = watched()
      button.refuse(new Error('not allowed'))
      stop()
      await settled()
      expect([answers.heard, button.unregistered]).toEqual([['failed: Error: not allowed'], 0])
    })
  })
})
