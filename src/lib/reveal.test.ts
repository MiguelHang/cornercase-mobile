import { describe, expect, it } from 'vitest'
import { revealBy, scrolledTo, under, type Span } from './reveal'

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

describe('scrolledTo', () => {
  it.each<[string, number, boolean, Span | null, number]>([
    ['keeps the scroll of the same question with its option in view', 80, false, { top: 150, bottom: 200 }, 80],
    ['scrolls the same question down to its option below', 80, false, { top: 280, bottom: 330 }, 110],
    ['keeps the scroll of the same question without an option', 80, false, null, 80],
    ['starts a new question at its top', 80, true, null, 0],
    ['starts a new question at its top with its option in view there', 80, true, { top: 60, bottom: 110 }, 0],
    ['never scrolls a new question above its top', 80, true, { top: 0, bottom: 10 }, 0],
    ['scrolls a new question from its top down to its option', 80, true, { top: 250, bottom: 300 }, 80],
  ])('%s', (_, top, fresh, item, to) => {
    expect(scrolledTo(top, fresh, VIEW, item, null)).toBe(to)
  })

  it.each<[string, number, boolean, Span, Span, number]>([
    [
      'scrolls a new question down to an option that fits under its start',
      80,
      true,
      { top: 50, bottom: 90 },
      { top: 150, bottom: 225 },
      5,
    ],
    [
      'keeps a new question from its start when its option does not fit under it',
      80,
      true,
      { top: 30, bottom: 70 },
      { top: 280, bottom: 330 },
      10,
    ],
    [
      'keeps a new question at its top when its start is right under the chips',
      80,
      true,
      { top: 20, bottom: 60 },
      { top: 300, bottom: 360 },
      0,
    ],
    [
      'keeps the start of the question in view as the card shrinks',
      0,
      false,
      { top: 110, bottom: 150 },
      { top: 250, bottom: 330 },
      10,
    ],
    [
      'keeps the start of the question in view half a pixel above the chips',
      0,
      false,
      { top: 99.5, bottom: 140 },
      { top: 250, bottom: 330 },
      0,
    ],
    [
      'still follows the option of a question swiped past its start',
      80,
      false,
      { top: 40, bottom: 80 },
      { top: 280, bottom: 330 },
      110,
    ],
    [
      'still scrolls up to an option above with the question kept',
      80,
      false,
      { top: 40, bottom: 80 },
      { top: 60, bottom: 90 },
      40,
    ],
  ])('%s', (_, top, fresh, lead, item, to) => {
    expect(scrolledTo(top, fresh, VIEW, item, lead)).toBe(to)
  })
})
