import type { Toast, ToastLevel } from './bindings'

export type Shown = Toast & { id: number; count: number; at: number }

export type Stack = { items: Shown[]; next: number }

const LIMIT = 3

const LIFETIME: Record<ToastLevel, number> = { info: 4000, error: 8000 }

export const empty: Stack = { items: [], next: 1 }

const endOf = (shown: Shown) => shown.at + LIFETIME[shown.level]

const alive = (items: Shown[], now: number) => items.filter((shown) => now < endOf(shown))

const same = (shown: Shown, toast: Toast) => shown.level === toast.level && shown.message === toast.message

const renewed = (shown: Shown, now: number): Shown => ({ ...shown, count: shown.count + 1, at: now })

export const push = (stack: Stack, toast: Toast, now: number): Stack => {
  const items = alive(stack.items, now)
  if (items.some((shown) => same(shown, toast))) {
    return { ...stack, items: items.map((shown) => (same(shown, toast) ? renewed(shown, now) : shown)) }
  }
  const added = { level: toast.level, message: toast.message, id: stack.next, count: 1, at: now }
  return { items: [...items, added].slice(-LIMIT), next: stack.next + 1 }
}

export const expire = (stack: Stack, now: number): Stack => ({ ...stack, items: alive(stack.items, now) })

export const dismiss = (stack: Stack, id: number): Stack => ({
  ...stack,
  items: stack.items.filter((shown) => shown.id !== id),
})

export const nextExpiry = (stack: Stack): number | null =>
  stack.items.length === 0 ? null : Math.min(...stack.items.map(endOf))

const hasMessage = (error: unknown): error is { message: string } =>
  typeof error === 'object' && error !== null && 'message' in error && typeof error.message === 'string'

export const messageOf = (error: unknown) => (hasMessage(error) ? error.message : String(error))

type Answer = { status: 'ok' } | { status: 'error'; error: { message: string } }

export const deliver = async (call: () => Promise<Answer>): Promise<string | null> => {
  try {
    const sent = await call()
    return sent.status === 'ok' ? null : sent.error.message
  } catch (error) {
    return messageOf(error)
  }
}
