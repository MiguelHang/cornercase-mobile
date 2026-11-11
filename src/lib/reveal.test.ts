import { describe, expect, it } from 'vitest'
import { revealBy, under, type Span } from './reveal'

const VIEW: Span = { top: 100, bottom: 300 }

describe('revealBy', () => {
  it.each<[string, Span, number]>([
    ['leaves one in view alone', { top: 150, bottom: 200 }, 0],
    ['leaves one filling the view alone', { top: 100, bottom: 300 }, 0],
    ['scrolls up to one above', { top: 60, bottom: 110 }, -40],
    ['scrolls down to one below', { top: 280, bottom: 330 }, 30],
    ['scrolls down to one far below', { top: 500, bottom: 550 }, 250],
    ['shows the top of one taller than the view', { top: 250, bottom: 600 }, 150],
  ])('%s', (_, item, by) => {
    expect(revealBy(VIEW, item)).toBe(by)
  })
})

describe('under', () => {
  it.each<[string, Span | null, Span]>([
    ['keeps the view without a header', null, VIEW],
    ['starts the view below a header stuck to its top', { top: 100, bottom: 144 }, { top: 144, bottom: 300 }],
    ['keeps the top of the view over a header scrolled above it', { top: 40, bottom: 84 }, VIEW],
  ])('%s', (_, header, seen) => {
    expect(under(VIEW, header)).toEqual(seen)
  })

  it('scrolls up to an option hidden under the header', () => {
    expect(revealBy(under(VIEW, { top: 100, bottom: 144 }), { top: 120, bottom: 160 })).toBe(-24)
  })
})
