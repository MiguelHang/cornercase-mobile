import { describe, expect, it } from 'vitest'
import { revealBy, type Span } from './reveal'

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
