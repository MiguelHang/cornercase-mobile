import { describe, expect, it, vi } from 'vitest'
import { oneAtATime } from './join'

const gate = () => {
  let open: () => void = () => {}
  let fail: (error: Error) => void = () => {}
  const promise = new Promise<void>((resolve, reject) => {
    open = resolve
    fail = reject
  })
  return { promise, open, fail }
}

describe('oneAtATime', () => {
  it('runs a second call made while the first is pending as the first', async () => {
    const first = gate()
    const run = vi.fn(() => first.promise)
    const join = oneAtATime(run)
    const calls = [join(1), join(2)]
    expect(calls[1]).toBe(calls[0])
    first.open()
    await Promise.all(calls)
    expect(run.mock.calls).toEqual([[1]])
  })

  it('runs again once the pending call has settled', async () => {
    const run = vi.fn(() => Promise.resolve())
    const join = oneAtATime(run)
    await join(1)
    await join(2)
    expect(run.mock.calls).toEqual([[1], [2]])
  })

  it('runs again after a pending call failed, which every caller sees', async () => {
    const first = gate()
    const run = vi.fn<(machine: number) => Promise<void>>(() => first.promise)
    const join = oneAtATime(run)
    const calls = [join(1), join(1)]
    first.fail(new Error('refused'))
    await expect(calls[1]).rejects.toThrow('refused')
    await expect(calls[0]).rejects.toThrow('refused')
    run.mockResolvedValue(undefined)
    await join(3)
    expect(run.mock.calls).toEqual([[1], [3]])
  })
})
