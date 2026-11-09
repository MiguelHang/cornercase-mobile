import { describe, expect, it, vi } from 'vitest'
import { anchor, roomIn, slide } from './touch'

const box = (scrollTop: number, scrollHeight = 1000, clientHeight = 600) => ({ scrollTop, scrollHeight, clientHeight })

const moves = () => ({ send: vi.fn(), scroll: vi.fn() })

describe('roomIn', () => {
  it.each([
    ['all below at the top', 0, { up: 0, down: 400 }],
    ['some on both sides in the middle', 150, { up: 150, down: 250 }],
    ['all above at the bottom', 400, { up: 400, down: 0 }],
  ])('leaves a taller grid %s', (_, scrollTop, room) => {
    expect(roomIn(box(scrollTop))).toEqual(room)
  })

  it('leaves no room in a grid that fits', () => {
    expect(roomIn(box(0, 500, 600))).toEqual({ up: 0, down: 0 })
  })
})

describe('slide', () => {
  it('moves the grid by its pan', () => {
    const scrolled = box(150)
    slide(scrolled, { pan: -40, send: [], rowsUp: 0 }, moves())
    expect(scrolled.scrollTop).toBe(110)
  })

  it('sends the wheel to the program and scrolls the history', () => {
    const made = moves()
    slide(box(0), { pan: 0, send: [{ key: { code: 'up' } }], rowsUp: 2 }, made)
    expect([made.send.mock.calls, made.scroll.mock.calls]).toEqual([[[[{ key: { code: 'up' } }]]], [[2]]])
  })

  it('sends and scrolls nothing for a move inside a line', () => {
    const made = moves()
    slide(box(0), { pan: 0, send: [], rowsUp: 0 }, made)
    expect([made.send, made.scroll].map((call) => call.mock.calls.length)).toEqual([0, 0])
  })
})

describe('anchor', () => {
  it('shows the last row of a taller grid', () => {
    const anchored = box(0)
    anchor(anchored)
    expect(anchored.scrollTop).toBe(400)
  })

  it('leaves a grid that fits at its top', () => {
    const anchored = box(30, 500, 600)
    anchor(anchored)
    expect(anchored.scrollTop).toBe(0)
  })
})
