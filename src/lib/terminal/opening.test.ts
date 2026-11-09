import { describe, expect, it, vi } from 'vitest'
import { Opening, type Opened } from './opening'

const gate = <T>() => {
  let open: (value: T) => void = () => {}
  const promise = new Promise<T>((resolve) => (open = resolve))
  return { promise, open }
}

const OK: Opened = { status: 'ok' }

const calls = (answer: () => Promise<Opened> = () => Promise.resolve(OK)) => ({
  open: vi.fn(answer),
  close: vi.fn(() => Promise.resolve()),
  failed: vi.fn(),
})

const settled = () => new Promise((resolve) => setTimeout(resolve, 0))

describe('Opening', () => {
  it('opens only once it listens', async () => {
    const listening = gate<void>()
    const made = calls()
    new Opening(listening.promise, made).open({ cols: 80, rows: 24 }, false)
    await settled()
    expect(made.open).not.toHaveBeenCalled()
    listening.open()
    await settled()
    expect(made.open.mock.calls).toEqual([[{ cols: 80, rows: 24 }, false]])
  })

  it('opens nothing when it left before it listened', async () => {
    const listening = gate<void>()
    const made = calls()
    const opening = new Opening(listening.promise, made)
    opening.open({ cols: 80, rows: 24 }, false)
    opening.leave()
    listening.open()
    await settled()
    expect(made.open).not.toHaveBeenCalled()
    expect(made.close).toHaveBeenCalledTimes(1)
  })

  it('closes again when it left while the terminal was opening', async () => {
    const answer = gate<Opened>()
    const made = calls(() => answer.promise)
    const opening = new Opening(Promise.resolve(), made)
    opening.open({ cols: 80, rows: 24 }, false)
    await settled()
    opening.leave()
    expect(made.close).toHaveBeenCalledTimes(1)
    answer.open(OK)
    await settled()
    expect(made.close).toHaveBeenCalledTimes(2)
  })

  it('opens one size at a time, skipping those overtaken while it waited', async () => {
    const answer = gate<Opened>()
    const made = calls(() => answer.promise)
    const opening = new Opening(Promise.resolve(), made)
    opening.open({ cols: 80, rows: 24 }, false)
    await settled()
    opening.open({ cols: 40, rows: 60 }, false)
    opening.open({ cols: 60, rows: 40 }, false)
    await settled()
    expect(made.open).toHaveBeenCalledTimes(1)
    made.open.mockResolvedValue(OK)
    answer.open(OK)
    await settled()
    expect(made.open.mock.calls).toEqual([[{ cols: 80, rows: 24 }, false], [{ cols: 60, rows: 40 }, false]])
  })

  it.each([false, true])('opens nothing new for the same size and the same fit (fit %s)', async (fit) => {
    const made = calls()
    const opening = new Opening(Promise.resolve(), made)
    opening.open({ cols: 80, rows: 24 }, fit)
    await settled()
    opening.open({ cols: 80, rows: 24 }, fit)
    await settled()
    expect(made.open.mock.calls).toEqual([[{ cols: 80, rows: 24 }, fit]])
  })

  it('opens again when the fit is turned on and off with the same size', async () => {
    const made = calls()
    const opening = new Opening(Promise.resolve(), made)
    opening.open({ cols: 80, rows: 24 }, false)
    await settled()
    opening.open({ cols: 80, rows: 24 }, true)
    await settled()
    opening.open({ cols: 80, rows: 24 }, false)
    await settled()
    expect(made.open.mock.calls).toEqual([
      [{ cols: 80, rows: 24 }, false],
      [{ cols: 80, rows: 24 }, true],
      [{ cols: 80, rows: 24 }, false],
    ])
  })

  it('opens the new size still fitted when the screen changes while fitted', async () => {
    const made = calls()
    const opening = new Opening(Promise.resolve(), made)
    opening.open({ cols: 80, rows: 24 }, true)
    await settled()
    opening.open({ cols: 40, rows: 60 }, true)
    await settled()
    expect(made.open.mock.calls).toEqual([
      [{ cols: 80, rows: 24 }, true],
      [{ cols: 40, rows: 60 }, true],
    ])
  })

  it('tells why the terminal did not open and still opens the next size', async () => {
    const made = calls(() => Promise.resolve({ status: 'error', error: 'not_connected' }))
    const opening = new Opening(Promise.resolve(), made)
    opening.open({ cols: 80, rows: 24 }, false)
    await settled()
    opening.open({ cols: 40, rows: 60 }, false)
    await settled()
    expect(made.failed.mock.calls).toEqual([['not_connected'], ['not_connected']])
  })

  it('tells a call that threw and goes on', async () => {
    const made = calls(() => Promise.reject(new Error('gone')))
    const opening = new Opening(Promise.resolve(), made)
    opening.open({ cols: 80, rows: 24 }, false)
    await settled()
    made.open.mockResolvedValue(OK)
    opening.open({ cols: 40, rows: 60 }, false)
    await settled()
    expect(made.failed.mock.calls).toEqual([[new Error('gone')]])
    expect(made.open).toHaveBeenCalledTimes(2)
  })
})
