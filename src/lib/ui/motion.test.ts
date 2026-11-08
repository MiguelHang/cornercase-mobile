import { cubicOut, quadIn } from 'svelte/easing'
import { afterEach, describe, expect, it, vi } from 'vitest'
import tokens from '../theme/tokens.css?raw'
import { ENTER_MS, LEAVE_MS, POP_MS, RISE_PX, enter, fadeOut, leave, prefersReducedMotion, riseIn } from './motion'

const token = (name: string) => tokens.match(new RegExp(`${name}:\\s*(\\d+)ms;`))?.[1]

const stubMotion = (reduced: boolean) => {
  vi.stubGlobal('window', { matchMedia: (query: string) => ({ matches: reduced && query.includes('reduce') }) })
  vi.stubGlobal('getComputedStyle', () => ({ opacity: '1', transform: 'none' }))
}

afterEach(() => {
  vi.unstubAllGlobals()
})

describe('motion', () => {
  it('enters rising 14 px in 180 ms with cubicOut', () => {
    expect(enter(false)).toEqual({ duration: 180, y: 14, easing: cubicOut })
  })

  it('leaves in 120 ms with quadIn', () => {
    expect(leave(false)).toEqual({ duration: 120, easing: quadIn })
  })

  it('takes no time with reduced motion', () => {
    expect(enter(true).duration).toBe(0)
    expect(leave(true).duration).toBe(0)
  })

  it('reads no reduced motion without a window', () => {
    expect(typeof window).toBe('undefined')
    expect(prefersReducedMotion()).toBe(false)
  })

  it('reads no reduced motion from a window without matchMedia', () => {
    vi.stubGlobal('window', {})
    expect(prefersReducedMotion()).toBe(false)
  })

  it.each([true, false])('reads reduced motion %s from the media query', (reduced) => {
    stubMotion(reduced)
    expect(prefersReducedMotion()).toBe(reduced)
  })

  it.each([
    [false, ENTER_MS, LEAVE_MS],
    [true, 0, 0],
  ])('runs the transitions with reduced motion %s', (reduced, entering, leaving) => {
    stubMotion(reduced)
    const node = {} as Element
    expect(riseIn(node)).toMatchObject({ duration: entering, easing: cubicOut })
    expect(fadeOut(node)).toMatchObject({ duration: leaving, easing: quadIn })
  })

  it('rises from 14 px below', () => {
    stubMotion(false)
    expect(riseIn({} as Element).css?.(0, 1)).toContain(`translate(0px, ${RISE_PX}px)`)
  })

  it.each([
    ['--enter', ENTER_MS],
    ['--leave', LEAVE_MS],
    ['--pop', POP_MS],
  ])('matches %s in tokens.css', (name, ms) => {
    expect(token(name)).toBe(String(ms))
  })
})
