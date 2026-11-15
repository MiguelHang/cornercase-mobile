import { describe, expect, it } from 'vitest'
import { dropped, followKeys, FOLLOWS, KEYBOARD_TOP, KeyboardTop } from './keyboardTop.svelte'

const told = (detail: unknown) => ({ detail }) as unknown as Event

class Window {
  heard: ((event: Event) => void) | null = null

  addEventListener(name: string, listener: (event: Event) => void) {
    if (name === KEYBOARD_TOP) this.heard = listener
  }

  removeEventListener(name: string, listener: (event: Event) => void) {
    if (name === KEYBOARD_TOP && this.heard === listener) this.heard = null
  }
}

describe('KeyboardTop', () => {
  it('knows nothing until the app tells it where the keys begin', () => {
    expect(new KeyboardTop().top).toBeNull()
  })

  it('takes the top the app tells, and forgets it when told null', () => {
    const window = new Window()
    const top = new KeyboardTop()
    top.watch(window)
    window.heard?.(told(512.5))
    expect(top.top).toBe(512.5)
    window.heard?.(told(null))
    expect(top.top).toBeNull()
  })

  it('takes anything but a number as nothing', () => {
    const window = new Window()
    const top = new KeyboardTop()
    top.watch(window)
    window.heard?.(told(512))
    window.heard?.(told('600'))
    expect(top.top).toBeNull()
    window.heard?.(told(Number.NaN))
    expect(top.top).toBeNull()
  })

  it('stops listening when asked', () => {
    const window = new Window()
    const stop = new KeyboardTop().watch(window)
    stop()
    expect(window.heard).toBeNull()
  })
})

describe('dropped', () => {
  it('is how far below what the page sees the keys begin, while a finger takes them down', () => {
    expect(dropped(420, 610)).toBe(190)
  })

  it('is nothing otherwise', () => {
    expect(dropped(420, null)).toBe(0)
    expect(dropped(420, 400)).toBe(0)
    expect(dropped(420, 420)).toBe(0)
  })

  it('leaves alone a fraction of a point between how the app and the page measure', () => {
    expect(dropped(420, 420.4)).toBe(0)
    expect(dropped(420, 421)).toBe(0)
    expect(dropped(420, 421.5)).toBe(1.5)
  })
})

describe('followKeys', () => {
  it('tells the app that the keys may follow the finger, until it is undone', () => {
    const root = { dataset: {} as DOMStringMap }
    const undo = followKeys(root)
    expect(root.dataset.keyboard).toBe(FOLLOWS)
    undo()
    expect(root.dataset).toEqual({})
  })
})
