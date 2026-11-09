import { describe, expect, it, vi } from 'vitest'
import { GAP, Keyboard, seen, whileClosed, type Screen, type Viewport } from './keyboard.svelte'

const FULL = 900

const KEYBOARD = 340

class Target {
  #listeners = new Map<string, Set<() => void>>()

  addEventListener(name: string, listener: () => void) {
    this.#listeners.set(name, (this.#listeners.get(name) ?? new Set()).add(listener))
  }

  removeEventListener(name: string, listener: () => void) {
    this.#listeners.get(name)?.delete(listener)
  }

  fire(name: string) {
    for (const listener of this.#listeners.get(name) ?? []) listener()
  }

  get listening() {
    return [...this.#listeners.values()].reduce((count, listeners) => count + listeners.size, 0)
  }
}

class FakeViewport extends Target implements Viewport {
  height = FULL
  offsetTop = 0
}

class FakeScreen extends Target implements Screen {
  innerHeight = FULL
  visualViewport: FakeViewport | null

  constructor(viewport: FakeViewport | null = new FakeViewport()) {
    super()
    this.visualViewport = viewport
  }
}

const watched = (screen = new FakeScreen()) => {
  const keyboard = new Keyboard(screen)
  const stop = keyboard.watch()
  return { screen, viewport: screen.visualViewport ?? new FakeViewport(), keyboard, stop }
}

const shown = ({ height, offset, covered, open }: Keyboard) => ({ height, offset, covered, open })

describe('Keyboard', () => {
  it('reads the visible height as soon as it watches', () => {
    expect(shown(watched().keyboard)).toEqual({ height: FULL, offset: 0, covered: false, open: false })
  })

  it('follows the keyboard as it opens', () => {
    const { viewport, keyboard } = watched()
    viewport.height = FULL - KEYBOARD
    viewport.fire('resize')
    expect(shown(keyboard)).toEqual({ height: FULL - KEYBOARD, offset: 0, covered: true, open: true })
  })

  it('follows the offset when the page scrolls under it', () => {
    const { viewport, keyboard } = watched()
    viewport.offsetTop = 120
    viewport.fire('scroll')
    expect(keyboard.offset).toBe(120)
  })

  it('goes back to the full height as the keyboard closes', () => {
    const { viewport, keyboard } = watched()
    viewport.height = FULL - KEYBOARD
    viewport.fire('resize')
    viewport.height = FULL
    viewport.fire('resize')
    expect(shown(keyboard)).toEqual({ height: FULL, offset: 0, covered: false, open: false })
  })

  it('follows a turn of the screen', () => {
    const { screen, viewport, keyboard } = watched()
    screen.innerHeight = 400
    viewport.height = 400
    screen.fire('resize')
    expect(shown(keyboard)).toEqual({ height: 400, offset: 0, covered: false, open: false })
  })

  it('takes the full height without a visual viewport', () => {
    const { screen, keyboard } = watched(new FakeScreen(null))
    screen.innerHeight = 700
    screen.fire('resize')
    expect(shown(keyboard)).toEqual({ height: 700, offset: 0, covered: false, open: false })
  })

  it('stops listening once it is unmounted', () => {
    const { screen, viewport, stop } = watched()
    stop()
    expect([screen.listening, viewport.listening]).toEqual([0, 0])
  })

  it('stays where it was after it is unmounted', () => {
    const { viewport, keyboard, stop } = watched()
    stop()
    viewport.height = FULL - KEYBOARD
    viewport.fire('resize')
    expect(keyboard.height).toBe(FULL)
  })
})

describe('seen', () => {
  const screen = (height: number): Screen => {
    const viewport = new FakeViewport()
    viewport.height = height
    return new FakeScreen(viewport)
  }

  it.each([
    ['open once the visible height is short of the screen by more than the gap', FULL - GAP - 1, true],
    ['closed while the visible height is short of the screen by the gap', FULL - GAP, false],
  ])('calls the keyboard %s', (_, height, open) => {
    expect(seen(screen(height)).open).toBe(open)
  })

  it.each([
    ['covered by a bar shorter than the gap, as a hardware keyboard shows', FULL - GAP, true],
    ['covered by the keyboard', FULL - GAP - 1, true],
    ['uncovered at the full height', FULL, false],
  ])('calls the screen %s', (_, height, covered) => {
    expect(seen(screen(height)).covered).toBe(covered)
  })
})

describe('whileClosed', () => {
  it.each([
    ['passes a value on while the keyboard is closed', false, [[7]]],
    ['drops a value while the keyboard is open', true, []],
  ])('%s', (_, open, calls) => {
    const then = vi.fn()
    whileClosed({ open }, then)(7)
    expect(then.mock.calls).toEqual(calls)
  })
})
