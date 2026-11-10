import { describe, expect, it } from 'vitest'
import { Fit, type Asked, type Page } from './fit.svelte'

class FakePage implements Page {
  visibilityState: DocumentVisibilityState = 'visible'
  #listeners = new Set<() => void>()

  addEventListener(name: string, listener: () => void) {
    if (name === 'visibilitychange') this.#listeners.add(listener)
  }

  removeEventListener(name: string, listener: () => void) {
    if (name === 'visibilitychange') this.#listeners.delete(listener)
  }

  turn(state: DocumentVisibilityState) {
    this.visibilityState = state
    for (const listener of this.#listeners) listener()
  }

  get listening() {
    return this.#listeners.size
  }
}

const watched = (page = new FakePage()) => {
  const fit = new Fit()
  const stop = fit.watch(page)
  return { page, fit, stop }
}

const shown = ({ on, visible }: Fit) => ({ on, visible })

const SIZE = { cols: 40, rows: 50 }

const asked = (fit: Fit) => {
  const calls: unknown[][] = []
  const opening: Asked = {
    open: (size, fitted) => calls.push(['open', size, fitted]),
    pause: () => calls.push(['pause']),
  }
  fit.ask(opening, SIZE)
  return calls
}

describe('Fit', () => {
  it('starts on and asks for the fit', () => {
    const { fit } = watched()
    expect([shown(fit), asked(fit)]).toEqual([{ on: true, visible: true }, [['open', SIZE, true]]])
  })

  it('asks for the tab as it is once it is turned off', () => {
    const { fit } = watched()
    fit.toggle()
    expect([shown(fit), asked(fit)]).toEqual([{ on: false, visible: true }, [['open', SIZE, false]]])
  })

  it('turns on again when toggled again', () => {
    const { fit } = watched()
    fit.toggle()
    fit.toggle()
    expect([shown(fit), asked(fit)]).toEqual([{ on: true, visible: true }, [['open', SIZE, true]]])
  })

  it('starts on again in a new screen, whatever the last one was left at', () => {
    watched().fit.toggle()
    expect(watched().fit.on).toBe(true)
  })

  it('reads a hidden page as soon as it watches', () => {
    const page = new FakePage()
    page.visibilityState = 'hidden'
    const { fit } = watched(page)
    expect([shown(fit), asked(fit)]).toEqual([{ on: true, visible: false }, [['pause']]])
  })

  it('pauses the fit while the page is hidden, keeping it on', () => {
    const { page, fit } = watched()
    page.turn('hidden')
    expect([shown(fit), asked(fit)]).toEqual([{ on: true, visible: false }, [['pause']]])
  })

  it('keeps the tab as it is open while the page is hidden with the fit off', () => {
    const { page, fit } = watched()
    fit.toggle()
    page.turn('hidden')
    expect(asked(fit)).toEqual([['open', SIZE, false]])
  })

  it('asks for the fit again when the page is visible again', () => {
    const { page, fit } = watched()
    page.turn('hidden')
    page.turn('visible')
    expect([shown(fit), asked(fit)]).toEqual([{ on: true, visible: true }, [['open', SIZE, true]]])
  })

  it('turns off when the Mac takes the tab back', () => {
    const { fit } = watched()
    fit.unfitted()
    expect([shown(fit), asked(fit)]).toEqual([{ on: false, visible: true }, [['open', SIZE, false]]])
  })

  it('stays off when the tab is unfitted while it was off', () => {
    const { fit } = watched()
    fit.toggle()
    fit.unfitted()
    expect(fit.on).toBe(false)
  })

  it('stops listening once cleaned up', () => {
    const { page, fit, stop } = watched()
    expect(page.listening).toBe(1)
    stop()
    page.turn('hidden')
    expect(page.listening).toBe(0)
    expect(fit.visible).toBe(true)
  })
})
