import { quadIn } from 'svelte/easing'
import { describe, expect, it, vi } from 'vitest'
import type { ConnectionView_Serialize } from './bindings'
import { screen, type Screen } from './connection'
import { dismiss, exit, LAUNCH_TIMEOUT_MS, LEAVE_EASING, ready, type After, type Layer } from './launch'
import { LEAVE_MS } from './ui/motion'

const error = (message: string) => ({ code: 'timeout' as const, message })

const NO_MACHINE: Screen = { machine: null, connection: { state: 'none' }, agents: null }

const on = (connection: ConnectionView_Serialize, agents: Screen['agents'] = null): Screen => ({
  machine: 1,
  connection,
  agents,
})

const CONNECTED: ConnectionView_Serialize = { state: 'connected', route: 'ws://127.0.0.1:7000' }

const fakeLayer = (finished: Promise<unknown> = Promise.resolve()) => {
  const removed = vi.fn()
  const animate = vi.fn(() => ({ finished }))
  const layer: Layer = { style: { pointerEvents: '' }, animate, remove: removed }
  return { layer, animate, removed }
}

const fakeTimer = () => {
  const pending: { ms: number; then: () => void; cancelled: boolean }[] = []
  const after: After = (ms, then) => {
    const timer = { ms, then, cancelled: false }
    pending.push(timer)
    return () => (timer.cancelled = true)
  }
  const fire = () => pending.filter((timer) => !timer.cancelled).forEach((timer) => timer.then())
  return { after, pending, fire }
}

describe('ready', () => {
  it('waits for the list of machines, whatever the screen holds', () => {
    expect(ready(false, NO_MACHINE)).toBe(false)
    expect(ready(false, on(CONNECTED, []))).toBe(false)
  })

  it('is ready with no active machine once the list is read or failed', () => {
    expect(ready(true, NO_MACHINE)).toBe(true)
  })

  it('waits while the active machine connects', () => {
    expect(ready(true, on({ state: 'connecting' }))).toBe(false)
  })

  it('waits for the first list of agents once connected', () => {
    expect(ready(true, on(CONNECTED))).toBe(false)
    expect(ready(true, on(CONNECTED, []))).toBe(true)
  })

  it.each<[string, ConnectionView_Serialize]>([
    ['retrying', { state: 'retrying', in_ms: 1000, error: error('refused') }],
    ['offline', { state: 'offline' }],
    ['unauthorized', { state: 'unauthorized' }],
    ['missing', { state: 'missing', missing: [{ name: 'watch', kind: 'stream', reason: null }] }],
    ['incompatible', { state: 'incompatible', error: error('too new') }],
    ['none', { state: 'none' }],
  ])('is ready when the connection ends %s', (_, connection) => {
    expect(ready(true, on(connection))).toBe(true)
  })

  it('is ready at once when the machine was already connected with its agents', () => {
    expect(ready(true, screen({ machine: 1, connection: CONNECTED }, []))).toBe(true)
    expect(ready(true, screen({ machine: 1, connection: CONNECTED }, null))).toBe(false)
  })
})

const bezier = (s: number, first: number, second: number) =>
  3 * (1 - s) ** 2 * s * first + 3 * (1 - s) * s ** 2 * second + s ** 3

describe('dismiss', () => {
  it('fades with the curve of quadIn, the leave easing of Motion', () => {
    const [x1, y1, x2, y2] = (LEAVE_EASING.match(/^cubic-bezier\((.*)\)$/)?.[1] ?? '').split(',').map(Number)
    for (let step = 0; step <= 20; step += 1) {
      const s = step / 20
      const x = bezier(s, x1 ?? Number.NaN, x2 ?? Number.NaN)
      expect(bezier(s, y1 ?? Number.NaN, y2 ?? Number.NaN)).toBeCloseTo(quadIn(x), 2)
    }
  })

  it('fades the layer out over the leave time, then removes it', async () => {
    const { layer, animate, removed } = fakeLayer()
    await dismiss(layer, false)
    expect(animate).toHaveBeenCalledWith([{ opacity: 1 }, { opacity: 0 }], {
      duration: LEAVE_MS,
      easing: LEAVE_EASING,
      fill: 'forwards',
    })
    expect(removed).toHaveBeenCalledOnce()
  })

  it('lets touches through while it fades', () => {
    const { layer, removed } = fakeLayer(new Promise(() => {}))
    void dismiss(layer, false)
    expect(layer.style.pointerEvents).toBe('none')
    expect(removed).not.toHaveBeenCalled()
  })

  it('removes the layer at once with Reduce Motion', () => {
    const { layer, animate, removed } = fakeLayer()
    void dismiss(layer, true)
    expect(animate).not.toHaveBeenCalled()
    expect(removed).toHaveBeenCalledOnce()
    expect(layer.style.pointerEvents).toBe('none')
  })

  it('removes the layer when the fade is cancelled', async () => {
    const { layer, removed } = fakeLayer(Promise.reject(new Error('aborted')))
    await dismiss(layer, false)
    expect(removed).toHaveBeenCalledOnce()
  })

  it('does nothing without a layer', async () => {
    await expect(dismiss(null, false)).resolves.toBeUndefined()
  })
})

describe('exit', () => {
  it('leaves when the time runs out, without being ready', () => {
    const go = vi.fn()
    const timer = fakeTimer()
    const launch = exit(go, timer.after)
    launch.when(false)
    expect(go).not.toHaveBeenCalled()
    expect(timer.pending.map((pending) => pending.ms)).toEqual([LAUNCH_TIMEOUT_MS])
    timer.fire()
    expect(go).toHaveBeenCalledOnce()
  })

  it('leaves once when ready and stops waiting for the time', () => {
    const go = vi.fn()
    const timer = fakeTimer()
    const launch = exit(go, timer.after)
    launch.when(true)
    launch.when(true)
    timer.fire()
    expect(go).toHaveBeenCalledOnce()
    expect(timer.pending[0]?.cancelled).toBe(true)
  })

  it('does not leave again when ready after the time ran out', () => {
    const go = vi.fn()
    const timer = fakeTimer()
    const launch = exit(go, timer.after)
    timer.fire()
    launch.when(true)
    expect(go).toHaveBeenCalledOnce()
  })

  it('stops waiting when stopped', () => {
    const go = vi.fn()
    const timer = fakeTimer()
    exit(go, timer.after).stop()
    timer.fire()
    expect(go).not.toHaveBeenCalled()
  })

  it('waits the real timeout by default', () => {
    vi.useFakeTimers()
    try {
      const go = vi.fn()
      exit(go)
      vi.advanceTimersByTime(LAUNCH_TIMEOUT_MS - 1)
      expect(go).not.toHaveBeenCalled()
      vi.advanceTimersByTime(1)
      expect(go).toHaveBeenCalledOnce()
    } finally {
      vi.useRealTimers()
    }
  })
})
