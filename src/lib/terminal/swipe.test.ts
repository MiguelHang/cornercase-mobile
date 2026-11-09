import { describe, expect, it } from 'vitest'
import type { Modes_Serialize } from '../bindings'
import { empty } from './pane.svelte'
import type { Point } from './pinch'
import { STEPS, Swipe, TAP, wheel, type Grid } from './swipe'

const LINE = 20

const grid = (held: Partial<Grid> = {}, modes: Partial<Modes_Serialize> = {}): Grid => ({
  modes: { ...empty.modes, ...modes },
  size: { cols: 80, rows: 24 },
  cell: 10,
  line: LINE,
  origin: [0, 0],
  room: { up: 0, down: 0 },
  ...held,
})

const MOUSE = { mouse: 'press' } as const

const ALTERNATE = { alternate_screen: true, alternate_scroll: true }

const pressed = (at: Point = [100, 100]) => {
  const swipe = new Swipe()
  swipe.press(1, at)
  return swipe
}

const slid = (dy: number, held: Partial<Grid> = {}, modes: Partial<Modes_Serialize> = {}) =>
  pressed().move(1, [100, 100 + dy], grid(held, modes))

describe('Swipe', () => {
  describe('tap', () => {
    it('is a finger lifted where it touched', () => {
      expect(pressed().lift(1)).toBe(true)
    })

    it('is a finger that moved less than the threshold', () => {
      const swipe = pressed()
      const steps = [swipe.move(1, [100, 100 + TAP - 1], grid()), swipe.lift(1)]
      expect(steps).toEqual([null, true])
    })

    it('is not a finger that moved as far as the threshold', () => {
      const swipe = pressed()
      swipe.move(1, [100 + TAP, 100], grid())
      expect(swipe.lift(1)).toBe(false)
    })

    it('stays a swipe when the finger comes back to where it touched', () => {
      const swipe = pressed()
      swipe.move(1, [100, 100 + TAP], grid())
      swipe.move(1, [100, 100], grid())
      expect(swipe.lift(1)).toBe(false)
    })

    it('is not a finger it never saw', () => {
      expect(pressed().lift(9)).toBe(false)
    })
  })

  describe('lines', () => {
    it('count from where the finger touched once it slides', () => {
      expect(slid(LINE)).toEqual({ pan: 0, send: [], rowsUp: 1 })
    })

    it.each([
      ['down scrolls the history up', 2 * LINE, 2],
      ['up scrolls the history down', -2 * LINE, -2],
    ])('move naturally: a finger sliding %s', (_, dy, rowsUp) => {
      expect(slid(dy)?.rowsUp).toBe(rowsUp)
    })

    it('keep the rest of a line for the next move', () => {
      const swipe = pressed()
      const moves = [swipe.move(1, [100, 130], grid()), swipe.move(1, [100, 140], grid())]
      expect(moves.map((move) => move?.rowsUp)).toEqual([1, 1])
    })

    it('take the rest back first when the finger turns', () => {
      const swipe = pressed()
      swipe.move(1, [100, 130], grid())
      expect(swipe.move(1, [100, 115], grid())).toEqual({ pan: 0, send: [], rowsUp: 0 })
    })

    it('scroll the other way once the rest is taken back', () => {
      const swipe = pressed()
      swipe.move(1, [100, 130], grid())
      expect(swipe.move(1, [100, 95], grid())?.rowsUp).toBe(-1)
    })

    it('start without a rest on the next swipe', () => {
      const swipe = pressed()
      swipe.move(1, [100, 130], grid())
      swipe.lift(1)
      swipe.press(1, [100, 100])
      expect(swipe.move(1, [100, 110], grid())?.rowsUp).toBe(0)
    })

    it('scroll the history at most a step limit in one move', () => {
      expect(slid(40 * LINE)?.rowsUp).toBe(STEPS)
    })

    it('turn the wheel at most a step limit in one move', () => {
      expect(slid(40 * LINE, {}, MOUSE)?.send).toHaveLength(STEPS)
    })
  })

  describe('a grid taller than the screen', () => {
    it('moves first before the history', () => {
      expect(slid(3 * LINE, { room: { up: LINE, down: 0 } })).toEqual({ pan: -LINE, send: [], rowsUp: 2 })
    })

    it('moves alone while it has room', () => {
      expect(slid(3 * LINE, { room: { up: 5 * LINE, down: 0 } })).toEqual({ pan: -3 * LINE, send: [], rowsUp: 0 })
    })

    it('moves down first when the finger slides up', () => {
      expect(slid(-3 * LINE, { room: { up: 0, down: LINE } })).toEqual({ pan: LINE, send: [], rowsUp: -2 })
    })

    it('does not move towards an edge it has no room before', () => {
      expect(slid(3 * LINE, { room: { up: 0, down: 5 * LINE } })).toEqual({ pan: 0, send: [], rowsUp: 3 })
    })
  })

  describe('two fingers', () => {
    const pinched = () => {
      const swipe = pressed()
      swipe.press(2, [200, 100])
      return swipe
    }

    it('are not a swipe', () => {
      const swipe = pinched()
      expect([swipe.move(1, [100, 200], grid()), swipe.move(2, [200, 200], grid())]).toEqual([null, null])
    })

    it('are not a tap', () => {
      const swipe = pinched()
      expect([swipe.lift(2), swipe.lift(1)]).toEqual([false, false])
    })

    it('leave no swipe to the finger that stays', () => {
      const swipe = pinched()
      swipe.lift(2)
      expect(swipe.move(1, [100, 200], grid())).toBeNull()
    })

    it('let a new touch swipe once both lift', () => {
      const swipe = pinched()
      swipe.lift(2)
      swipe.lift(1)
      swipe.press(3, [100, 100])
      expect(swipe.move(3, [100, 100 + LINE], grid())?.rowsUp).toBe(1)
    })
  })

  it('ignores a finger it never saw', () => {
    expect(pressed().move(9, [100, 300], grid())).toBeNull()
  })
})

describe('wheel', () => {
  it.each([
    ['the history up when the program does not read the mouse', {}, 3, 3],
    ['the history down when the program does not read the mouse', {}, -3, -3],
    ['nothing on the alternate screen without alternate scroll', { alternate_screen: true }, 3, 0],
  ])('scrolls %s', (_, modes, lines, rowsUp) => {
    expect(wheel(lines, [0, 0], grid({}, modes))).toEqual({ send: [], rowsUp })
  })

  it.each([
    ['scroll-up', 2],
    ['scroll-down', -2],
  ])('sends the wheel as %s once per line to a program that reads the mouse', (kind, lines) => {
    const event = { mouse: { kind, x: 0, y: 0 } }
    expect(wheel(lines, [0, 0], grid({}, MOUSE))).toEqual({ send: [event, event], rowsUp: 0 })
  })

  it.each([
    ['up', 2],
    ['down', -2],
  ])('sends the arrow %s once per line on the alternate screen with alternate scroll', (code, lines) => {
    const event = { key: { code } }
    expect(wheel(lines, [0, 0], grid({}, ALTERNATE))).toEqual({ send: [event, event], rowsUp: 0 })
  })

  it('sends the wheel even on the alternate screen when the program reads the mouse', () => {
    const modes = { ...ALTERNATE, ...MOUSE }
    expect(wheel(1, [0, 0], grid({}, modes)).send).toEqual([{ mouse: { kind: 'scroll-up', x: 0, y: 0 } }])
  })

  it.each([
    ['the cell under the finger', [0, 0], [35, 47], { x: 3, y: 2 }],
    ['the cell of a grid moved on the screen', [100, -40], [135, 47], { x: 3, y: 4 }],
    ['the first cell for a finger before the grid', [0, 0], [-20, -20], { x: 0, y: 0 }],
    ['the last cell for a finger past the grid', [0, 0], [5000, 5000], { x: 79, y: 23 }],
  ] as [string, Point, Point, { x: number; y: number }][])('points the wheel at %s', (_, origin, at, cell) => {
    expect(wheel(1, at, grid({ origin }, MOUSE)).send).toEqual([{ mouse: { kind: 'scroll-up', ...cell } }])
  })
})
