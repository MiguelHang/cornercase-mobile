import { describe, expect, it, vi } from 'vitest'
import { CLAIM } from './backSwipe'
import { fakeClock } from './chatFakes'
import { LONG_PRESS_MS, LongPress, OPEN_WIDTH, RowSwipe, STAY, rowGesture } from './rowSwipe'

const X = 300

const Y = 200

const OPEN = -OPEN_WIDTH

const pressed = (open = false) => {
  const swipe = new RowSwipe()
  swipe.press(1, [X, Y], open)
  return swipe
}

const moved = (dx: number, dy = 0, open = false) => pressed(open).move(1, [X + dx, Y + dy])

const dragged = (dx: number, open = false) => {
  const swipe = pressed(open)
  swipe.move(1, [X + Math.sign(dx) * CLAIM, Y])
  swipe.move(1, [X + dx, Y])
  return swipe
}

const lifted = (dx: number, open = false) => dragged(dx, open).lift(1)

describe('rowGesture', () => {
  it.each([
    ['ios', 'swipe'],
    ['android', 'hold'],
  ] as const)('on %s is a %s', (on, gesture) => {
    expect(rowGesture(on)).toBe(gesture)
  })
})

describe('RowSwipe', () => {
  describe('claiming', () => {
    it('waits while the finger has not moved at all', () => {
      expect(moved(0)).toEqual({ state: 'pending', offset: 0 })
    })

    it('waits while the finger moved left less than the threshold', () => {
      expect(moved(-(CLAIM - 1))).toEqual({ state: 'pending', offset: 0 })
    })

    it('claims a finger that moved left as far as the threshold', () => {
      expect(moved(-CLAIM)).toEqual({ state: 'claimed', offset: -CLAIM })
    })

    it('claims a finger that moved more than twice as far left as down', () => {
      expect(moved(-CLAIM, CLAIM / 2 - 1)).toEqual({ state: 'claimed', offset: -CLAIM })
    })

    it.each([
      ['a vertical scroll', 0, CLAIM * 4],
      ['a finger only twice as far left as down', -CLAIM, CLAIM / 2],
      ['a finger that moved right on a shut row', CLAIM, 0],
    ])('never claims %s', (_, dx, dy) => {
      const swipe = pressed()
      expect([swipe.move(1, [X + dx, Y + dy]), swipe.move(1, [X - OPEN_WIDTH, Y]), swipe.lift(1)]).toEqual([
        { state: 'cancelled', offset: 0 },
        { state: 'cancelled', offset: 0 },
        'none',
      ])
    })

    it('leaves an open row where it is on a vertical scroll', () => {
      expect(moved(0, CLAIM * 4, true)).toEqual({ state: 'cancelled', offset: OPEN })
    })

    it.each([
      ['a finger leaning left', -1, 0, false, true],
      ['a finger that has not moved', 0, 0, false, false],
      ['a finger going down', 0, 1, false, false],
      ['a finger leaning right on a shut row', 1, 0, false, false],
      ['a finger leaning right on an open row', 1, 0, true, true],
    ])('holds the touch for %s: %s', (_, dx, dy, open, holds) => {
      const swipe = pressed(open)
      swipe.move(1, [X + dx, Y + dy])
      expect(swipe.holds).toBe(holds)
    })

    it('holds a claimed touch', () => {
      expect(dragged(-OPEN_WIDTH).holds).toBe(true)
    })
  })

  describe('opening', () => {
    it('follows the finger to the left', () => {
      expect(dragged(-OPEN_WIDTH / 2).move(1, [X - OPEN_WIDTH / 2, Y])).toEqual({
        state: 'claimed',
        offset: -OPEN_WIDTH / 2,
      })
    })

    it('stops at the width of the button', () => {
      expect(dragged(-OPEN_WIDTH * 3).move(1, [X - OPEN_WIDTH * 3, Y])).toEqual({ state: 'claimed', offset: OPEN })
    })

    it.each([
      ['as far as the threshold', -OPEN_WIDTH * STAY, 'open'],
      ['short of the threshold', -OPEN_WIDTH * STAY + 1, 'shut'],
      ['all the way across', -X, 'open'],
    ] as const)('stays open after a drag %s: %s', (_, dx, ending) => {
      expect(lifted(dx)).toBe(ending)
    })

    it('opens no further when the finger goes left on an open row', () => {
      expect(dragged(-CLAIM * 3, true).move(1, [X - CLAIM * 3, Y])).toEqual({ state: 'claimed', offset: OPEN })
    })
  })

  describe('closing', () => {
    it('rests open when it starts open', () => {
      expect(moved(0, 0, true)).toEqual({ state: 'pending', offset: OPEN })
    })

    it('follows the finger to the right from the open place', () => {
      expect(moved(CLAIM, 0, true)).toEqual({ state: 'claimed', offset: OPEN + CLAIM })
    })

    it('stops when the row is shut', () => {
      expect(dragged(OPEN_WIDTH * 2, true).move(1, [X + OPEN_WIDTH * 2, Y])).toEqual({
        state: 'claimed',
        offset: 0,
      })
    })

    it.each([
      ['past the threshold', OPEN_WIDTH * STAY + 1, 'shut'],
      ['short of the threshold', OPEN_WIDTH * STAY, 'open'],
      ['to the left', -CLAIM, 'open'],
    ] as const)('after a drag %s: %s', (_, dx, ending) => {
      expect(lifted(dx, true)).toBe(ending)
    })
  })

  describe('a tap', () => {
    it.each([
      ['a shut row', false],
      ['an open row', true],
    ])('on %s is a tap', (_, open) => {
      expect(pressed(open).lift(1)).toBe('tap')
    })

    it('ends as nothing for a finger it never saw', () => {
      expect(pressed().lift(2)).toBe('none')
    })
  })

  describe('dropping the touch', () => {
    it('drops a second finger', () => {
      const swipe = pressed()
      expect([swipe.press(2, [X, Y], false), swipe.move(1, [X - OPEN_WIDTH, Y]), swipe.lift(1)]).toEqual([
        'cancelled',
        { state: 'cancelled', offset: 0 },
        'none',
      ])
    })

    it('ignores the moves of a finger it never saw', () => {
      expect(dragged(-CLAIM).move(2, [X - OPEN_WIDTH, Y])).toEqual({ state: 'claimed', offset: -CLAIM })
    })

    it.each([
      ['a shut row', false, 0],
      ['an open row', true, OPEN],
    ])('puts %s back where it was when cancelled', (_, open, offset) => {
      const swipe = dragged(open ? CLAIM * 2 : -CLAIM * 2, open)
      swipe.cancel()
      expect([swipe.holds, swipe.move(1, [X, Y]), swipe.lift(1)]).toEqual([
        false,
        { state: 'cancelled', offset },
        'none',
      ])
    })

    it('takes a new touch after a lift', () => {
      const swipe = pressed()
      swipe.lift(1)
      expect(swipe.press(2, [X, Y], false)).toBe('pending')
    })
  })
})

const holding = () => {
  const onheld = vi.fn()
  const { clock, tick, pending } = fakeClock()
  const press = new LongPress(onheld, clock)
  press.press(1, [X, Y])
  return { press, onheld, tick, pending }
}

describe('LongPress', () => {
  it('fires once the finger stays down for the whole wait', () => {
    const { onheld, tick } = holding()
    tick(LONG_PRESS_MS - 1)
    expect(onheld).not.toHaveBeenCalled()
    tick(1)
    expect(onheld).toHaveBeenCalledOnce()
  })

  it('says it fired when the finger lifts after it', () => {
    const { press, tick } = holding()
    tick(LONG_PRESS_MS)
    expect(press.lift(1)).toBe(true)
  })

  it('never fires once the finger lifts before the wait', () => {
    const { press, onheld, tick, pending } = holding()
    tick(LONG_PRESS_MS - 1)
    expect(press.lift(1)).toBe(false)
    tick(LONG_PRESS_MS)
    expect([onheld.mock.calls.length, pending()]).toEqual([0, 0])
  })

  it.each([
    ['as far as the threshold', CLAIM, 0, 0],
    ['down as far as the threshold', 0, CLAIM, 0],
    ['less than the threshold', CLAIM - 1, 0, 1],
  ])('after a finger moved %s fires %s times', (_, dx, dy, times) => {
    const { press, onheld, tick } = holding()
    press.move(1, [X + dx, Y + dy])
    tick(LONG_PRESS_MS)
    expect(onheld).toHaveBeenCalledTimes(times)
  })

  it('ignores the moves of another finger', () => {
    const { press, onheld, tick } = holding()
    press.move(2, [X + CLAIM * 4, Y])
    tick(LONG_PRESS_MS)
    expect([onheld.mock.calls.length, press.lift(2), press.lift(1)]).toEqual([1, false, true])
  })

  it('still says it fired when the finger moves after it', () => {
    const { press, tick } = holding()
    tick(LONG_PRESS_MS)
    press.move(1, [X + CLAIM * 4, Y])
    expect(press.lift(1)).toBe(true)
  })

  it('gives up when a second finger comes down', () => {
    const { press, onheld, tick } = holding()
    press.press(2, [X, Y])
    tick(LONG_PRESS_MS)
    expect([onheld.mock.calls.length, press.lift(1), press.lift(2)]).toEqual([0, false, false])
  })

  it('never fires once cancelled', () => {
    const { press, onheld, tick, pending } = holding()
    press.cancel()
    tick(LONG_PRESS_MS)
    expect([onheld.mock.calls.length, pending()]).toEqual([0, 0])
  })

  it('starts over on the next press', () => {
    const { press, onheld, tick } = holding()
    tick(LONG_PRESS_MS)
    press.lift(1)
    press.press(2, [X, Y])
    tick(LONG_PRESS_MS - 1)
    expect([onheld.mock.calls.length, press.lift(2)]).toEqual([1, false])
  })
})
