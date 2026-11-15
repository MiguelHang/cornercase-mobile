import { describe, expect, it, vi } from 'vitest'
import { fromEnd, keepFromEnd } from './fromEnd'

describe('the distance from the end', () => {
  const scrolled = (scrollTop: number) => {
    const scrollTo = vi.fn()
    return {
      box: {
        scrollTop,
        clientHeight: 400,
        scrollHeight: 2000,
        scrollTo,
      } as unknown as Element,
      scrollTo,
    }
  }

  it('is what is left below the view', () => {
    expect(fromEnd(scrolled(1000).box)).toBe(600)
    expect(fromEnd(scrolled(1600).box)).toBe(0)
  })

  it('is kept by scrolling to the same distance from the end', () => {
    const { box, scrollTo } = scrolled(0)
    keepFromEnd(box, 600)
    expect(scrollTo).toHaveBeenCalledWith({ top: 1000 })
  })
})
