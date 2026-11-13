import { CLAIM, decided, leans, type EdgeState } from './backSwipe'
import { timers, type Clock } from './chat.svelte'
import type { Platform } from './platform'
import type { Point } from './terminal/pinch'

export const OPEN_WIDTH = 88

export const STAY = 0.5

export const LONG_PRESS_MS = 500

export type RowStep = { state: EdgeState; offset: number }

export type RowEnding = 'open' | 'shut' | 'tap' | 'none'

export type RowGesture = 'swipe' | 'hold'

export const rowGesture = (on: Platform): RowGesture => (on === 'ios' ? 'swipe' : 'hold')

const sideways = ([dx, dy]: Point, open: boolean) => leans([-dx, dy]) || (open && leans([dx, dy]))

const far = (from: Point, [x, y]: Point) => Math.hypot(x - from[0], y - from[1]) >= CLAIM

const clamped = (offset: number) => Math.min(0, Math.max(-OPEN_WIDTH, offset))

export class RowSwipe {
  #fingers: number[] = []
  #state: EdgeState = 'cancelled'
  #open = false
  #from: Point = [0, 0]
  #moved: Point = [0, 0]
  #offset = 0

  press(id: number, point: Point, open: boolean): EdgeState {
    this.#fingers = [...this.#fingers, id]
    if (this.#fingers.length > 1) return this.#drop()
    this.#state = 'pending'
    this.#open = open
    this.#from = point
    this.#moved = [0, 0]
    this.#offset = this.#rest()
    return this.#state
  }

  move(id: number, [x, y]: Point): RowStep {
    if (!this.#fingers.includes(id)) return this.#step()
    const moved: Point = [x - this.#from[0], y - this.#from[1]]
    this.#moved = moved
    if (this.#state === 'pending') this.#state = decided(moved, (point) => sideways(point, this.#open))
    if (this.#state === 'claimed') this.#offset = clamped(this.#rest() + moved[0])
    return this.#step()
  }

  lift(id: number): RowEnding {
    if (!this.#fingers.includes(id)) return 'none'
    this.#fingers = this.#fingers.filter((finger) => finger !== id)
    const { state, offset } = this.#step()
    this.#drop()
    if (state === 'pending') return 'tap'
    if (state !== 'claimed') return 'none'
    return offset <= -OPEN_WIDTH * STAY ? 'open' : 'shut'
  }

  get holds() {
    return this.#state === 'claimed' || (this.#state === 'pending' && sideways(this.#moved, this.#open))
  }

  cancel() {
    this.#fingers = []
    this.#drop()
  }

  #rest() {
    return this.#open ? -OPEN_WIDTH : 0
  }

  #drop(): EdgeState {
    this.#state = 'cancelled'
    this.#offset = this.#rest()
    return this.#state
  }

  #step(): RowStep {
    return { state: this.#state, offset: this.#offset }
  }
}

export class LongPress {
  #onheld: () => void
  #clock: Pick<Clock, 'after'>
  #finger: number | null = null
  #from: Point = [0, 0]
  #held = false
  #stop: () => void = () => undefined

  constructor(onheld: () => void, clock: Pick<Clock, 'after'> = timers) {
    this.#onheld = onheld
    this.#clock = clock
  }

  press(id: number, point: Point) {
    if (this.#finger !== null) return this.cancel()
    this.#finger = id
    this.#from = point
    this.#held = false
    this.#stop = this.#clock.after(LONG_PRESS_MS, () => this.#hold())
  }

  move(id: number, point: Point) {
    if (id === this.#finger && !this.#held && far(this.#from, point)) this.cancel()
  }

  lift(id: number) {
    if (id !== this.#finger) return false
    const held = this.#held
    this.cancel()
    return held
  }

  cancel() {
    this.#stop()
    this.#finger = null
    this.#held = false
  }

  #hold() {
    this.#held = true
    this.#onheld()
  }
}
