import { describe, expect, it } from 'vitest'
import { BASE, clamp, limits, Zooms, type Limits } from './zoom'

const PLACE = { machine: 7, boot: 'b1', tab: 3 }

const WIDE: Limits = { min: 6.5, max: 26 }

describe('limits', () => {
  it.each<[string, number, Limits]>([
    ['a pane that fits at the base font keeps it as the least', 40, { min: BASE, max: 2 * BASE }],
    ['a pane exactly as wide as the screen keeps the base font', 50, { min: BASE, max: 2 * BASE }],
    ['a pane wider than the screen shrinks until it fits', 200, { min: 3.25, max: 2 * BASE }],
  ])('%s', (_, cols, expected) => {
    expect(limits(cols, 400, 8)).toEqual(expected)
  })

  it('counts from the base font it is given', () => {
    expect(limits(100, 400, 8, 10)).toEqual({ min: 5, max: 20 })
  })
})

describe('clamp', () => {
  it.each<[string, number, number]>([
    ['below the least as the least', 2, 6.5],
    ['between the limits as it is', 13, 13],
    ['above the most as the most', 40, 26],
  ])('keeps a size %s', (_, size, expected) => {
    expect(clamp(size, WIDE)).toBe(expected)
  })
})

describe('Zooms', () => {
  it('starts a tab it never saw fitted to the screen', () => {
    expect(new Zooms().recall(PLACE, WIDE)).toBe(6.5)
  })

  it('recalls the size it remembered for the tab', () => {
    const zooms = new Zooms()
    zooms.remember(PLACE, 20)
    expect(zooms.recall({ ...PLACE }, WIDE)).toBe(20)
  })

  it('recalls a remembered size within the limits of now', () => {
    const zooms = new Zooms()
    zooms.remember(PLACE, 40)
    expect(zooms.recall(PLACE, WIDE)).toBe(26)
  })

  it.each<[string, Partial<typeof PLACE>]>([
    ['another boot of the server', { boot: 'b2' }],
    ['another tab', { tab: 4 }],
    ['another machine', { machine: 8 }],
  ])('does not recall the size of the same tab id after %s', (_, other) => {
    const zooms = new Zooms()
    zooms.remember(PLACE, 20)
    expect(zooms.recall({ ...PLACE, ...other }, WIDE)).toBe(6.5)
  })
})
