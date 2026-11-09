import type { Point } from './terminal/pinch'

export const EDGE = 20

export const CLAIM = 10

export const SHARE = 0.35

export const FLING = 0.5

export const HOLD = 300

export type EdgeState = 'pending' | 'claimed' | 'cancelled'

export type Step = { state: EdgeState; shift: number }

export type Ending = 'back' | 'settle' | 'tap' | 'none'

type Sample = { x: number; at: number }

const claimed = ([dx, dy]: Point): EdgeState => (dx > Math.abs(dy) ? 'claimed' : 'cancelled')

export class BackSwipe {
  #fingers = new Set<number>()
  #state: EdgeState = 'cancelled'
  #from: Point = [0, 0]
  #pressed = 0
  #shift = 0
  #before: Sample = { x: 0, at: 0 }
  #last: Sample = { x: 0, at: 0 }

  press(id: number, point: Point, at: number) {
    this.#fingers.add(id)
    if (this.#fingers.size > 1) return this.#drop()
    this.#state = point[0] <= EDGE ? 'pending' : 'cancelled'
    this.#from = point
    this.#pressed = at
    this.#shift = 0
    this.#last = { x: point[0], at }
    return this.#state
  }

  move(id: number, [x, y]: Point, at: number): Step {
    if (!this.#fingers.has(id)) return this.#step()
    this.#expire(at)
    const moved: Point = [x - this.#from[0], y - this.#from[1]]
    if (this.#state === 'pending' && Math.hypot(...moved) >= CLAIM) this.#state = claimed(moved)
    this.#before = this.#last
    this.#last = { x, at }
    this.#shift = this.#state === 'claimed' ? Math.max(0, moved[0]) : 0
    return this.#step()
  }

  lift(id: number, at: number, width: number): Ending {
    if (!this.#fingers.delete(id)) return 'none'
    this.#expire(at)
    const { state, shift } = this.#step()
    this.#drop()
    if (state === 'pending') return 'tap'
    if (state !== 'claimed') return 'none'
    return shift >= width * SHARE || this.#speed(at) >= FLING ? 'back' : 'settle'
  }

  cancel() {
    this.#fingers.clear()
    this.#drop()
  }

  #drop(): EdgeState {
    this.#state = 'cancelled'
    this.#shift = 0
    return this.#state
  }

  #expire(at: number) {
    if (this.#state === 'pending' && at - this.#pressed > HOLD) this.#drop()
  }

  #speed(at: number) {
    return (this.#last.x - this.#before.x) / (at - this.#before.at)
  }

  #step(): Step {
    return { state: this.#state, shift: this.#shift }
  }
}
