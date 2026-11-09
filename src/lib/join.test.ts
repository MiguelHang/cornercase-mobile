import { describe, expect, it, vi } from 'vitest'
import { latestWins } from './join'

const gate = () => {
  let open: () => void = () => {}
  let fail: (error: Error) => void = () => {}
  const promise = new Promise<void>((resolve, reject) => {
    open = resolve
    fail = reject
  })
  return { promise, open, fail }
}

const started = () => new Promise((resolve) => setTimeout(resolve))

describe('latestWins', () => {
  it('gives a second call to the same target the pending promise', async () => {
    const first = gate()
    const run = vi.fn(() => first.promise)
    const join = latestWins(run)
    const calls = [join(1), join(1)]
    expect(calls[1]).toBe(calls[0])
    first.open()
    await Promise.all(calls)
    expect(run.mock.calls).toEqual([[1]])
  })

  it('ends on the last of two calls to different targets made at once', async () => {
    let active = 0
    const join = latestWins(async (machine: number) => {
      active = machine
    })
    await Promise.all([join(1), join(2)])
    expect(active).toBe(2)
  })

  it('starts a different target only once the pending call has settled', async () => {
    const first = gate()
    const run = vi.fn<(machine: number) => Promise<void>>(() => first.promise)
    const join = latestWins(run)
    const calls = [join(1)]
    await started()
    run.mockResolvedValue(undefined)
    calls.push(join(2))
    await started()
    expect(run.mock.calls).toEqual([[1]])
    first.open()
    await Promise.all(calls)
    expect(run.mock.calls).toEqual([[1], [2]])
  })

  it('drops a waiting call that a later one replaced, which then settles as done', async () => {
    const first = gate()
    const run = vi.fn<(machine: number) => Promise<void>>(() => first.promise)
    const join = latestWins(run)
    const calls = [join(1)]
    await started()
    run.mockResolvedValue(undefined)
    calls.push(join(2), join(3))
    first.open()
    await Promise.all(calls)
    expect(run.mock.calls).toEqual([[1], [3]])
  })

  it('runs the next target after a pending call failed, which its callers see', async () => {
    const first = gate()
    const run = vi.fn<(machine: number) => Promise<void>>(() => first.promise)
    const join = latestWins(run)
    const failed = [join(1), join(1)]
    await started()
    run.mockResolvedValue(undefined)
    const next = join(2)
    first.fail(new Error('refused'))
    await expect(failed[0]).rejects.toThrow('refused')
    await expect(failed[1]).rejects.toThrow('refused')
    await next
    expect(run.mock.calls).toEqual([[1], [2]])
  })

  it('runs the same target again once its call has settled', async () => {
    const run = vi.fn(() => Promise.resolve())
    const join = latestWins(run)
    await join(1)
    await join(1)
    expect(run.mock.calls).toEqual([[1], [1]])
  })
})
