import { describe, expect, it } from 'vitest'
import { Fit, type Page } from './fit.svelte'

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

const shown = ({ on, visible, wanted }: Fit) => ({ on, visible, wanted })

describe('Fit', () => {
  it('starts off', () => {
    expect(shown(watched().fit)).toEqual({ on: false, visible: true, wanted: false })
  })

  it('wants the fit once it is turned on', () => {
    const { fit } = watched()
    fit.toggle()
    expect(shown(fit)).toEqual({ on: true, visible: true, wanted: true })
  })

  it('turns off when toggled again', () => {
    const { fit } = watched()
    fit.toggle()
    fit.toggle()
    expect(shown(fit)).toEqual({ on: false, visible: true, wanted: false })
  })

  it('reads a hidden page as soon as it watches', () => {
    const page = new FakePage()
    page.visibilityState = 'hidden'
    const { fit } = watched(page)
    fit.toggle()
    expect(shown(fit)).toEqual({ on: true, visible: false, wanted: false })
  })

  it('stops wanting the fit while the page is hidden, keeping it on', () => {
    const { page, fit } = watched()
    fit.toggle()
    page.turn('hidden')
    expect(shown(fit)).toEqual({ on: true, visible: false, wanted: false })
  })

  it('wants the fit again when the page is visible again', () => {
    const { page, fit } = watched()
    fit.toggle()
    page.turn('hidden')
    page.turn('visible')
    expect(shown(fit)).toEqual({ on: true, visible: true, wanted: true })
  })

  it('turns off when the Mac takes the tab back', () => {
    const { fit } = watched()
    fit.toggle()
    fit.unfitted()
    expect(shown(fit)).toEqual({ on: false, visible: true, wanted: false })
  })

  it('stays off when the tab is unfitted while it was off', () => {
    const { fit } = watched()
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
