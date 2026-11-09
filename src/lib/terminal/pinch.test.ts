import { describe, expect, it } from 'vitest'
import { Pinch } from './pinch'

const pinching = () => {
  const pinch = new Pinch()
  pinch.press(1, [0, 0])
  pinch.press(2, [10, 0])
  return pinch
}

describe('Pinch', () => {
  it('starts when a second finger touches', () => {
    const pinch = new Pinch()
    expect([pinch.press(1, [0, 0]), pinch.press(2, [0, 10])]).toEqual([false, true])
  })

  it('does not start with two fingers on the same point', () => {
    const pinch = new Pinch()
    pinch.press(1, [4, 4])
    expect(pinch.press(2, [4, 4])).toBe(false)
  })

  it('scales by how much farther apart the fingers are than when it started', () => {
    expect(pinching().move(2, [20, 0])).toBe(2)
  })

  it('scales down as the fingers come closer', () => {
    expect(pinching().move(1, [5, 0])).toBe(0.5)
  })

  it('gives no scale with one finger', () => {
    const pinch = new Pinch()
    pinch.press(1, [0, 0])
    expect(pinch.move(1, [5, 5])).toBeNull()
  })

  it('ignores a finger it never saw', () => {
    expect(pinching().move(9, [50, 0])).toBeNull()
  })

  it('stops when a finger lifts', () => {
    const pinch = pinching()
    pinch.lift(2)
    expect(pinch.move(1, [5, 0])).toBeNull()
  })

  it('stops when a third finger touches', () => {
    const pinch = pinching()
    expect([pinch.press(3, [0, 10]), pinch.move(2, [20, 0])]).toEqual([false, null])
  })

  it('starts again from where the fingers are when one comes back', () => {
    const pinch = pinching()
    pinch.lift(2)
    pinch.press(2, [5, 0])
    expect(pinch.move(2, [10, 0])).toBe(2)
  })
})
