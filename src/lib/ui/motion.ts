import { cubicOut, quadIn } from 'svelte/easing'
import { fade, fly } from 'svelte/transition'

export const ENTER_MS = 180
export const LEAVE_MS = 120
export const POP_MS = 110
export const RISE_PX = 14

export const enter = (reduced: boolean) => ({ duration: reduced ? 0 : ENTER_MS, y: RISE_PX, easing: cubicOut })

export const leave = (reduced: boolean) => ({ duration: reduced ? 0 : LEAVE_MS, easing: quadIn })

export const prefersReducedMotion = () =>
  typeof window !== 'undefined' &&
  typeof window.matchMedia === 'function' &&
  window.matchMedia('(prefers-reduced-motion: reduce)').matches

export const riseIn = (node: Element) => fly(node, enter(prefersReducedMotion()))

export const fadeOut = (node: Element) => fade(node, leave(prefersReducedMotion()))
