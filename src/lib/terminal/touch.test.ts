import { describe, expect, it, vi } from 'vitest'
import { anchor, ended, followed, pinned, roomAt, roomIn, slide, Tap } from './touch'
import { BASE } from './zoom'

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

describe('pinned', () => {
  const FITS = { min: BASE, max: 2 * BASE }

  it.each([
    ['holds a fitted grid at the base', true, BASE, FITS, true],
    ['holds a fitted grid pinched back to just over the base', true, BASE + 0.2, FITS, true],
    ['frees a fitted grid zoomed in', true, BASE + 1, FITS, false],
    ['frees a fitted grid zoomed in by half a pixel', true, BASE + 0.5, FITS, false],
    ['frees a passive grid at the base', false, BASE, FITS, false],
    ['frees a passive grid fitted to the width', false, 2 * BASE, { min: 2 * BASE, max: 2 * BASE }, false],
  ])('%s', (_, fitted, size, limits, held) => {
    expect(pinned(fitted, size, limits)).toBe(held)
  })
})

describe('roomAt', () => {
  it('leaves no room in a held grid taller than the box', () => {
    expect(roomAt(box(400), true)).toEqual({ up: 0, down: 0 })
  })

  it('leaves the room of the box to a free grid', () => {
    expect(roomAt(box(150), false)).toEqual({ up: 150, down: 250 })
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

describe('Tap', () => {
  const lifted = (tapped: boolean) => {
    const tap = new Tap()
    tap.lifted(tapped)
    return tap
  }

  it('takes the click that follows a finger lifted as a tap', () => {
    expect(lifted(true).clicked()).toBe(true)
  })

  it('takes the click of a swipe or a pinch as nothing', () => {
    expect(lifted(false).clicked()).toBe(false)
  })

  it('takes only the first click after a tap', () => {
    const tap = lifted(true)
    expect([tap.clicked(), tap.clicked()]).toEqual([true, false])
  })

  it('takes a click of its own, with no finger lifted, as nothing', () => {
    expect(new Tap().clicked()).toBe(false)
  })

  it('drops the tap of a cancelled touch', () => {
    const tap = lifted(true)
    tap.cancelled()
    expect(tap.clicked()).toBe(false)
  })

  it('forgets an earlier tap once a swipe lifts', () => {
    const tap = lifted(true)
    tap.lifted(false)
    expect(tap.clicked()).toBe(false)
  })
})

describe('the edge of the screen', () => {
  it.each([
    ['follows a finger with the edge idle', 'idle', true],
    ['leaves a finger waiting at the edge', 'pending', false],
    ['leaves a finger the swipe back claimed', 'claimed', false],
  ] as const)('%s', (_, edge, follows) => {
    expect(followed(edge)).toBe(follows)
  })

  it.each([
    ['a still finger with the edge idle is a tap', 'idle', true, true],
    ['a finger that slid with the edge idle is no tap', 'idle', false, false],
    ['a still finger waiting at the edge is a tap', 'pending', true, true],
    ['a finger the swipe back claimed is no tap', 'claimed', true, false],
  ] as const)('%s', (_, edge, still, tapped) => {
    const tap = new Tap()
    ended(tap, edge, still)
    expect(tap.clicked()).toBe(tapped)
  })

  it('forgets an earlier tap once the swipe back lifts', () => {
    const tap = new Tap()
    tap.lifted(true)
    ended(tap, 'claimed', true)
    expect(tap.clicked()).toBe(false)
  })
})
