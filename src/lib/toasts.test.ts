import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import type { Toast } from './bindings'
import * as toasts from './toasts'
import { Toasts } from './toasts.svelte'

const INFO: Toast = { level: 'info', message: 'Machine added' }
const ERROR: Toast = { level: 'error', message: 'Connection refused' }

const pushed = (...steps: [Toast, number][]) =>
  steps.reduce((stack, [toast, now]) => toasts.push(stack, toast, now), toasts.empty)

const messages = (stack: toasts.Stack) => stack.items.map((shown) => shown.message)

const errors = (store: Toasts) => store.items.filter((shown) => shown.level === 'error').map((shown) => shown.message)

describe('push', () => {
  it('shows a new toast once, from now', () => {
    expect(pushed([INFO, 100]).items).toEqual([{ ...INFO, id: 1, count: 1, at: 100 }])
  })

  it('gives each new toast a greater id', () => {
    const stack = pushed([INFO, 0], [ERROR, 0])
    expect(stack.items.map((shown) => shown.id)).toEqual([1, 2])
  })

  it('keeps three toasts and drops the oldest', () => {
    const stack = pushed(...['a', 'b', 'c', 'd'].map((message): [Toast, number] => [{ level: 'info', message }, 0]))
    expect(messages(stack)).toEqual(['b', 'c', 'd'])
  })

  it('merges a toast still shown into a counter and renews its time', () => {
    const stack = pushed([INFO, 0], [ERROR, 10], [INFO, 3000])
    expect(stack.items).toEqual([
      { ...INFO, id: 1, count: 2, at: 3000 },
      { ...ERROR, id: 2, count: 1, at: 10 },
    ])
  })

  it('does not merge the same text at another level', () => {
    const stack = pushed([INFO, 0], [{ level: 'error', message: INFO.message }, 0])
    expect(stack.items.map((shown) => shown.count)).toEqual([1, 1])
  })

  it('shows a toast again as a new one once the first has expired', () => {
    expect(pushed([INFO, 0], [INFO, 4000]).items).toEqual([{ ...INFO, id: 2, count: 1, at: 4000 }])
  })
})

describe('expire', () => {
  it.each([
    [INFO, 3999, 1],
    [INFO, 4000, 0],
    [ERROR, 7999, 1],
    [ERROR, 8000, 0],
  ])('keeps a %o toast %i ms after it arrived: %i left', (toast, now, left) => {
    expect(toasts.expire(pushed([toast, 0]), now).items).toHaveLength(left)
  })

  it('counts from the last time a merged toast arrived', () => {
    const stack = pushed([INFO, 0], [INFO, 3000])
    expect(toasts.expire(stack, 4000).items).toHaveLength(1)
    expect(toasts.expire(stack, 7000).items).toHaveLength(0)
  })
})

describe('dismiss', () => {
  it('removes only the toast with that id', () => {
    expect(messages(toasts.dismiss(pushed([INFO, 0], [ERROR, 0]), 1))).toEqual([ERROR.message])
  })
})

describe('nextExpiry', () => {
  it('is the soonest end among the toasts shown', () => {
    expect(toasts.nextExpiry(pushed([ERROR, 0], [INFO, 1000]))).toBe(5000)
  })

  it('is none without toasts', () => {
    expect(toasts.nextExpiry(toasts.empty)).toBeNull()
  })
})

describe('messageOf', () => {
  it.each([
    [{ code: 'connect', message: 'Connection refused' }, 'Connection refused'],
    [new Error('IPC failed'), 'IPC failed'],
    ['plain text', 'plain text'],
    [{ code: 'connect' }, '[object Object]'],
    [null, 'null'],
  ])('reads %o as %s', (error, message) => {
    expect(toasts.messageOf(error)).toBe(message)
  })
})

describe('deliver', () => {
  it.each<[string, () => ReturnType<Parameters<typeof toasts.deliver>[0]>, string | null]>([
    ['nothing once sent', () => Promise.resolve({ status: 'ok' }), null],
    [
      'the reason of a refusal',
      () => Promise.resolve({ status: 'error', error: { message: 'the agent is asking a question' } }),
      'the agent is asking a question',
    ],
    ['the message of a rejected call', () => Promise.reject(new Error('no window')), 'no window'],
  ])('gives %s', async (_, call, failed) => {
    expect(await toasts.deliver(call)).toBe(failed)
  })
})

describe('Toasts', () => {
  beforeEach(() => {
    vi.useFakeTimers()
  })

  afterEach(() => {
    vi.useRealTimers()
  })

  it('removes each toast when its time is up', () => {
    const store = new Toasts()
    store.push(ERROR)
    vi.advanceTimersByTime(1000)
    store.push(INFO)
    vi.advanceTimersByTime(3999)
    expect(store.items).toHaveLength(2)
    vi.advanceTimersByTime(1)
    expect(errors(store)).toEqual([ERROR.message])
    vi.advanceTimersByTime(3000)
    expect(store.items).toEqual([])
  })

  it('dismisses a toast before its time', () => {
    const store = new Toasts()
    store.push(INFO)
    store.dismiss(store.items[0].id)
    expect(store.items).toEqual([])
  })

  it('reads the time from the clock it is given', () => {
    const store = new Toasts(() => 42)
    store.push(INFO)
    expect(store.items[0].at).toBe(42)
  })

  it('shows a failed command as an error', () => {
    const store = new Toasts()
    store.failed({ code: 'unknown_machine', message: 'No such machine' })
    expect(errors(store)).toEqual(['No such machine'])
  })

  it('shows a rejected promise as an error', async () => {
    const store = new Toasts()
    await Promise.reject(new Error('IPC failed')).catch(store.failed)
    expect(errors(store)).toEqual(['IPC failed'])
  })
})
