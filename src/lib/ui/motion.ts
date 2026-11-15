import { cubicOut, quadIn, quintOut } from 'svelte/easing'
import { fade, fly } from 'svelte/transition'

export const ENTER_MS = 180
export const LEAVE_MS = 120
export const POP_MS = 110
export const RISE_PX = 14
export const PULSE_MS = 1400
export const SHEET_IN_MS = 280
export const SHEET_OUT_MS = 200

export const enter = (reduced: boolean) => ({ duration: reduced ? 0 : ENTER_MS, y: RISE_PX, easing: cubicOut })

export const leave = (reduced: boolean) => ({ duration: reduced ? 0 : LEAVE_MS, easing: quadIn })

export const prefersReducedMotion = () =>
  typeof window !== 'undefined' &&
  typeof window.matchMedia === 'function' &&
  window.matchMedia('(prefers-reduced-motion: reduce)').matches

export const riseIn = (node: Element) => fly(node, enter(prefersReducedMotion()))

export const fadeOut = (node: Element) => fade(node, leave(prefersReducedMotion()))

export const dropIn = (node: Element) => fly(node, { ...enter(prefersReducedMotion()), y: -RISE_PX })

const lasting = (ms: number) => (prefersReducedMotion() ? 0 : ms)

export const sheetIn = (node: Element) =>
  fly(node, { duration: lasting(SHEET_IN_MS), y: '100%', opacity: 1, easing: quintOut })

export const sheetOut = (node: Element) =>
  fly(node, { duration: lasting(SHEET_OUT_MS), y: '100%', opacity: 1, easing: cubicOut })

export const veilIn = (node: Element) => fade(node, { duration: lasting(SHEET_IN_MS), easing: cubicOut })

export const veilOut = (node: Element) => fade(node, { duration: lasting(SHEET_OUT_MS), easing: cubicOut })
