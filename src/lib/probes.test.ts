import { afterEach, describe, expect, it, vi } from 'vitest'
import type { CommandError, ProbeView } from './bindings'
import { Probes, type ProbeCalls } from './probes.svelte'

const ok = (data: ProbeView) => Promise.resolve({ status: 'ok' as const, data })

const ANSWERS: ProbeView = { state: 'answers', route: 'ws://studio.lan:9000/' }

const gate = () => {
  let open: (view: ProbeView) => void = () => {}
  const promise = new Promise<ProbeView>((resolve) => (open = resolve))
  return { promise: promise.then((data) => ({ status: 'ok' as const, data })), open }
}

const probes = (machineProbe: ProbeCalls['machineProbe'] = () => ok(ANSWERS)) => {
  const calls = { machineProbe: vi.fn(machineProbe) }
  return { probes: new Probes(calls), calls }
}

afterEach(() => {
  vi.useRealTimers()
})

describe('Probes', () => {
  it('probes each machine it is given once and keeps what each said', async () => {
    const views: Record<number, ProbeView> = { 2: ANSWERS, 3: { state: 'no_token' } }
    const { probes: made, calls } = probes((id) => ok(views[id]))
    await made.check([2, 3])
    expect(calls.machineProbe.mock.calls).toEqual([[2], [3]])
    expect([made.of(2), made.of(3)]).toEqual([ANSWERS, { state: 'no_token' }])
  })

  it('shows a machine as checking until its probe answers', async () => {
    const pending = gate()
    const { probes: made } = probes(() => pending.promise)
    const checked = made.check([2])
    expect(made.of(2)).toEqual({ state: 'checking' })
    pending.open(ANSWERS)
    await checked
    expect(made.of(2)).toEqual(ANSWERS)
  })

  it('goes back to checking on Check again and keeps the new answer', async () => {
    const pending = gate()
    const { probes: made, calls } = probes()
    await made.check([2])
    calls.machineProbe.mockImplementation(() => pending.promise)
    const again = made.check([2])
    expect(made.of(2)).toEqual({ state: 'checking' })
    pending.open({ state: 'unauthorized' })
    await again
    expect([made.of(2), calls.machineProbe.mock.calls.length]).toEqual([{ state: 'unauthorized' }, 2])
  })

  it('does not probe again a machine still being checked', async () => {
    const pending = gate()
    const { probes: made, calls } = probes(() => pending.promise)
    const first = made.check([2])
    await made.check([2])
    pending.open(ANSWERS)
    await first
    expect(calls.machineProbe).toHaveBeenCalledTimes(1)
  })

  it.each<[string, () => Promise<{ status: 'error'; error: CommandError }>, string]>([
    [
      'refused',
      () => Promise.resolve({ status: 'error', error: { code: 'unknown_machine', message: 'unknown machine' } }),
      'unknown machine',
    ],
    ['rejected', () => Promise.reject(new Error('IPC is gone')), 'IPC is gone'],
  ])('keeps the reason of a probe %s', async (_, machineProbe, message) => {
    const { probes: made } = probes(machineProbe)
    await made.check([2])
    expect(made.of(2)).toEqual({ state: 'failed', message })
  })

  it('shows a machine never probed as checking', () => {
    expect(probes().probes.of(9)).toEqual({ state: 'checking' })
  })

  it('probes on opening every machine but the active one', async () => {
    const { probes: made, calls } = probes()
    await made.open([1, 2, 3], 2)
    expect(calls.machineProbe.mock.calls).toEqual([[1], [3]])
  })

  it('probes every machine on opening without an active one', async () => {
    const { probes: made, calls } = probes()
    await made.open([1, 2], null)
    expect(calls.machineProbe.mock.calls).toEqual([[1], [2]])
  })

  it('probes the machine that stops being the active one, and only it', async () => {
    const { probes: made, calls } = probes()
    await made.open([1, 2, 3], 2)
    calls.machineProbe.mockClear()
    await made.follow(3)
    await made.follow(3)
    await made.follow(null)
    expect(calls.machineProbe.mock.calls).toEqual([[2], [3]])
  })

  it('probes nothing when a machine becomes active with none before', async () => {
    const { probes: made, calls } = probes()
    await made.open([1, 2], null)
    calls.machineProbe.mockClear()
    await made.follow(1)
    expect(calls.machineProbe).not.toHaveBeenCalled()
  })

  it('probes nothing by itself and leaves no timer behind', async () => {
    vi.useFakeTimers()
    const { probes: made, calls } = probes()
    expect(calls.machineProbe).not.toHaveBeenCalled()
    await made.check([2])
    await vi.advanceTimersByTimeAsync(60_000)
    expect([calls.machineProbe.mock.calls.length, vi.getTimerCount()]).toEqual([1, 0])
  })
})
