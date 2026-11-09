import type { Modes_Serialize, Size_Serialize } from '../bindings'
import type { InputEvent } from './keys'
import type { Point } from './pinch'

export const TAP = 10

export const STEPS = 25

export type Room = { up: number; down: number }

export type Grid = {
  modes: Modes_Serialize
  size: Size_Serialize
  cell: number
  line: number
  origin: Point
  room: Room
}

export type Wheel = { send: InputEvent[]; rowsUp: number }

export type Scroll = Wheel & { pan: number }

const STILL: Wheel = { send: [], rowsUp: 0 }

const within = (at: number, length: number) => Math.min(Math.max(0, Math.floor(at)), Math.max(0, length - 1))

const cellAt = ([x, y]: Point, { origin: [left, top], cell, line, size }: Grid) => ({
  x: within((x - left) / cell, size.cols),
  y: within((y - top) / line, size.rows),
})

const repeated = (event: InputEvent, steps: number): Wheel => ({ send: Array.from({ length: steps }, () => event), rowsUp: 0 })

export const wheel = (lines: number, at: Point, grid: Grid): Wheel => {
  const up = lines > 0
  const steps = Math.min(Math.abs(lines), STEPS)
  const { modes } = grid
  if (modes.mouse !== 'none') {
    return repeated({ mouse: { kind: up ? 'scroll-up' : 'scroll-down', ...cellAt(at, grid) } }, steps)
  }
  if (!modes.alternate_screen) return { send: [], rowsUp: up ? steps : -steps }
  return modes.alternate_scroll ? repeated({ key: { code: up ? 'up' : 'down' } }, steps) : STILL
}

const panned = (dy: number, { up, down }: Room) => (dy > 0 ? 0 - Math.min(dy, up) : Math.min(0 - dy, down))

export class Swipe {
  #fingers = new Set<number>()
  #spoiled = false
  #sliding = false
  #from: Point = [0, 0]
  #last: Point = [0, 0]
  #rest = 0

  press(id: number, point: Point) {
    this.#fingers.add(id)
    if (this.#fingers.size > 1) {
      this.#spoiled = true
      return
    }
    this.#spoiled = false
    this.#sliding = false
    this.#from = point
    this.#last = point
    this.#rest = 0
  }

  move(id: number, point: Point, grid: Grid): Scroll | null {
    if (this.#spoiled || !this.#fingers.has(id) || !this.#slid(point)) return null
    const dy = point[1] - this.#last[1]
    this.#last = point
    const pan = panned(dy, grid.room)
    this.#rest += (dy + pan) / grid.line
    const lines = Math.trunc(this.#rest)
    this.#rest -= lines
    return { pan, ...(lines === 0 ? STILL : wheel(lines, point, grid)) }
  }

  lift(id: number) {
    const tapped = this.#fingers.has(id) && !this.#spoiled && !this.#sliding
    this.#fingers.delete(id)
    return tapped
  }

  #slid([x, y]: Point) {
    const [fromX, fromY] = this.#from
    this.#sliding ||= Math.hypot(x - fromX, y - fromY) >= TAP
    return this.#sliding
  }
}
