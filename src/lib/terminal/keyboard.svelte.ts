import { untrack } from 'svelte'

export const GAP = 100

type Listened = Pick<EventTarget, 'addEventListener' | 'removeEventListener'>

export type Viewport = Listened & Pick<VisualViewport, 'height' | 'offsetTop'>

export type Screen = Listened & { innerHeight: number; visualViewport?: Viewport | null }

export type Seen = { height: number; offset: number; open: boolean }

export const seen = ({ innerHeight, visualViewport }: Screen): Seen => {
  if (!visualViewport) return { height: innerHeight, offset: 0, open: false }
  const { height, offsetTop } = visualViewport
  return { height, offset: offsetTop, open: innerHeight - height > GAP }
}

const listened = (target: Listened, names: string[], listener: () => void) => {
  for (const name of names) target.addEventListener(name, listener)
  return () => {
    for (const name of names) target.removeEventListener(name, listener)
  }
}

export class Keyboard {
  readonly #screen: Screen
  #seen: Seen = $state.raw({ height: 0, offset: 0, open: false })

  constructor(screen: Screen) {
    this.#screen = screen
  }

  get height() {
    return this.#seen.height
  }

  get offset() {
    return this.#seen.offset
  }

  get open() {
    return this.#seen.open
  }

  watch() {
    const read = () => {
      this.#seen = seen(this.#screen)
    }
    read()
    const viewport = this.#screen.visualViewport
    const stops = [listened(this.#screen, ['resize'], read)]
    if (viewport) stops.push(listened(viewport, ['resize', 'scroll'], read))
    return () => {
      for (const stop of stops) stop()
    }
  }
}

export const whileClosed =
  <T>(keyboard: Pick<Keyboard, 'open'>, then: (value: T) => void) =>
  (value: T) => {
    if (!untrack(() => keyboard.open)) then(value)
  }
