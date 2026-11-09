import { describe, expect, it, vi } from 'vitest'
import { tap, type HapticsApi } from './haptics'

type Feedback = HapticsApi['impactFeedback']

const plugin = (impactFeedback: Feedback) => ({ impactFeedback: vi.fn(impactFeedback) })

describe('tap', () => {
  it('asks the phone for a light impact', async () => {
    const api = plugin(() => Promise.resolve({ status: 'ok', data: null }))
    await tap(api)
    expect(api.impactFeedback.mock.calls).toEqual([['light']])
  })

  it('asks before it waits for the answer', () => {
    const api = plugin(() => new Promise(() => {}))
    void tap(api)
    expect(api.impactFeedback).toHaveBeenCalledOnce()
  })

  it.each<[string, Feedback]>([
    ['a rejection', () => Promise.reject(new Error('plugin haptics not found'))],
    ['a refusal that is not an Error', () => Promise.reject('haptics.impact_feedback not allowed')],
    [
      'a throw',
      () => {
        throw new Error('window.__TAURI_INTERNALS__ is undefined')
      },
    ],
  ])('swallows %s', async (_, impactFeedback) => {
    const api = plugin(impactFeedback)
    await expect(tap(api)).resolves.toBeUndefined()
    expect(api.impactFeedback).toHaveBeenCalledOnce()
  })
})
