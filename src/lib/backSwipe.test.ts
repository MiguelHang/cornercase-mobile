import { describe, expect, it } from 'vitest'
import { BackSwipe, CLAIM, EDGE, FLING, HOLD, SHARE } from './backSwipe'

const WIDTH = 400

const FAR = WIDTH * SHARE

const X = 5

const Y = 300

const pressed = (x = X) => {
  const swipe = new BackSwipe()
  swipe.press(1, [x, Y], 0)
  return swipe
}

const claimed = () => {
  const swipe = pressed()
  swipe.move(1, [X + CLAIM, Y], 20)
  return swipe
}

const moved = (dx: number, dy = 0, at = 20) => pressed().move(1, [X + dx, Y + dy], at)

const flung = (dx: number, liftAt: number) => {
  const swipe = claimed()
  swipe.move(1, [X + CLAIM + dx, Y], 40)
  return swipe.lift(1, liftAt, WIDTH)
}

const STILL = { state: 'cancelled', shift: 0 }

describe('BackSwipe', () => {
  describe('the edge', () => {
    it.each([
      ['at the left side', 0, 'pending'],
      ['at the edge', EDGE, 'pending'],
      ['one pixel past the edge', EDGE + 1, 'cancelled'],
    ])('takes a touch %s', (_, x, state) => {
      expect(new BackSwipe().press(1, [x, Y], 0)).toBe(state)
    })

    it('never claims a touch away from the edge', () => {
      const swipe = pressed(EDGE + 1)
      expect([swipe.move(1, [EDGE + 200, Y], 20), swipe.lift(1, 40, WIDTH)]).toEqual([STILL, 'none'])
    })
  })

  describe('claiming', () => {
    it('waits while the finger moved less than the threshold', () => {
      expect(moved(CLAIM - 1)).toEqual({ state: 'pending', shift: 0 })
    })

    it('claims a finger that moved right as far as the threshold', () => {
      expect(moved(CLAIM)).toEqual({ state: 'claimed', shift: CLAIM })
    })

    it('claims a finger that moved more right than down', () => {
      expect(moved(CLAIM, CLAIM - 1)).toEqual({ state: 'claimed', shift: CLAIM })
    })

    it.each([
      ['down', 0, CLAIM],
      ['up', 0, -CLAIM],
      ['left', -CLAIM, 0],
      ['as much down as right', CLAIM, CLAIM],
      ['more up than right', CLAIM - 1, -CLAIM],
    ])('cancels a finger that moved %s', (_, dx, dy) => {
      expect(moved(dx, dy)).toEqual(STILL)
    })

    it('stays cancelled when the finger turns right', () => {
      const swipe = pressed()
      swipe.move(1, [X, Y + CLAIM], 20)
      expect(swipe.move(1, [X + 100, Y + CLAIM], 40)).toEqual(STILL)
    })
  })

  describe('the shift', () => {
    it('follows the finger once claimed', () => {
      expect(claimed().move(1, [X + 60, Y + 30], 40)).toEqual({ state: 'claimed', shift: 60 })
    })

    it('is never negative', () => {
      expect(claimed().move(1, [0, Y], 40)).toEqual({ state: 'claimed', shift: 0 })
    })
  })

  describe('a finger that stays still', () => {
    it('can still claim at the end of the wait', () => {
      expect(moved(CLAIM, 0, HOLD).state).toBe('claimed')
    })

    it('cancels after the wait', () => {
      expect(moved(CLAIM, 0, HOLD + 1)).toEqual(STILL)
    })

    it('is a tap when it lifts by the end of the wait', () => {
      expect(pressed().lift(1, HOLD, WIDTH)).toBe('tap')
    })

    it('is nothing when it lifts after the wait', () => {
      expect(pressed().lift(1, HOLD + 1, WIDTH)).toBe('none')
    })

    it('keeps the claim when it rests after claiming', () => {
      expect(claimed().move(1, [X + 30, Y], HOLD + 500)).toEqual({ state: 'claimed', shift: 30 })
    })
  })

  describe('lifting', () => {
    it('is a tap where the finger touched', () => {
      expect(pressed().lift(1, 50, WIDTH)).toBe('tap')
    })

    it('is a tap after a move below the threshold', () => {
      const swipe = pressed()
      swipe.move(1, [X + CLAIM - 1, Y], 20)
      expect(swipe.lift(1, 40, WIDTH)).toBe('tap')
    })

    it('goes back past the share of the width', () => {
      const swipe = claimed()
      swipe.move(1, [X + FAR, Y], 2000)
      expect(swipe.lift(1, 4000, WIDTH)).toBe('back')
    })

    it('settles short of the share of the width', () => {
      const swipe = claimed()
      swipe.move(1, [X + FAR - 1, Y], 2000)
      expect(swipe.lift(1, 4000, WIDTH)).toBe('settle')
    })

    it.each([
      ['goes back with a fling as fast as the limit', 20, 'back'],
      ['settles with a fling slower than the limit', 21, 'settle'],
    ])('%s', (_, pause, ending) => {
      expect(flung(FLING * 20, 20 + pause)).toBe(ending)
    })

    it('measures the fling from the move before the last', () => {
      const swipe = pressed()
      swipe.move(1, [X + CLAIM, Y], 200)
      swipe.move(1, [X + 2 * CLAIM, Y], 210)
      expect(swipe.lift(1, 200 + CLAIM / FLING, WIDTH)).toBe('back')
    })

    it('measures the first move from the touch', () => {
      const swipe = new BackSwipe()
      swipe.press(1, [X, Y], 100)
      swipe.move(1, [X + CLAIM, Y], 110)
      expect(swipe.lift(1, 100 + CLAIM / FLING, WIDTH)).toBe('back')
    })

    it('settles a fling to the left', () => {
      expect(flung(-CLAIM, 41)).toBe('settle')
    })

    it('is nothing for a finger it never saw', () => {
      expect(claimed().lift(9, 40, WIDTH)).toBe('none')
    })

    it('lets the next touch start again', () => {
      const swipe = claimed()
      swipe.lift(1, 40, WIDTH)
      swipe.press(2, [X, Y], 100)
      expect(swipe.move(2, [X + CLAIM, Y], 120)).toEqual({ state: 'claimed', shift: CLAIM })
    })
  })

  describe('a second finger', () => {
    it('cancels a waiting touch', () => {
      const swipe = pressed()
      expect([swipe.press(2, [X, Y], 10), swipe.move(1, [X + CLAIM, Y], 20)]).toEqual(['cancelled', STILL])
    })

    it('cancels a claimed touch', () => {
      const swipe = claimed()
      swipe.press(2, [200, Y], 30)
      expect(swipe.move(1, [X + 60, Y], 40)).toEqual(STILL)
    })

    it('leaves nothing to lift', () => {
      const swipe = claimed()
      swipe.press(2, [200, Y], 30)
      expect([swipe.lift(2, 40, WIDTH), swipe.lift(1, 50, WIDTH)]).toEqual(['none', 'none'])
    })

    it('lets a new touch start once both lift', () => {
      const swipe = pressed()
      swipe.press(2, [200, Y], 10)
      swipe.lift(2, 20, WIDTH)
      swipe.lift(1, 30, WIDTH)
      expect(swipe.press(3, [X, Y], 40)).toBe('pending')
    })
  })

  describe('cancel', () => {
    it('stops a claimed touch', () => {
      const swipe = claimed()
      swipe.cancel()
      expect([swipe.move(1, [X + 60, Y], 40), swipe.lift(1, 50, WIDTH)]).toEqual([STILL, 'none'])
    })

    it('forgets the fingers it had', () => {
      const swipe = claimed()
      swipe.cancel()
      expect(swipe.press(2, [X, Y], 40)).toBe('pending')
    })
  })

  it('ignores a finger it never saw', () => {
    const swipe = pressed()
    expect([swipe.move(9, [X + 100, Y], 20), swipe.move(1, [X + CLAIM, Y], 30)]).toEqual([
      { state: 'pending', shift: 0 },
      { state: 'claimed', shift: CLAIM },
    ])
  })
})
