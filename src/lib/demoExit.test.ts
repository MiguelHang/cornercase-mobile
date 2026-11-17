import { describe, expect, it, vi } from 'vitest'
import { DEMO_MACHINE } from './bindings'
import { gate, ok, refused } from './chatFakes'
import { DemoExit, type DemoCalls } from './demoExit'

const MAC = 3

const exiting = (leave: DemoCalls['demoLeave'] = () => Promise.resolve(ok(null))) => {
  const steps: string[] = []
  const screens = {
    root: vi.fn(() => void steps.push('root')),
    load: vi.fn(async () => void steps.push('load')),
    reload: vi.fn(async () => void steps.push('reload')),
    failed: vi.fn(),
  }
  const calls = { demoLeave: vi.fn(leave) }
  return { exit: new DemoExit(calls, screens), calls, screens, steps }
}

describe('DemoExit.leave', () => {
  it('goes back to the board before loading the machines and the connection again', async () => {
    const { exit, calls, steps } = exiting()
    await exit.leave()
    expect(calls.demoLeave).toHaveBeenCalledOnce()
    expect(steps).toEqual(['root', 'load'])
  })

  it('stays where it is and tells why when leaving is refused', async () => {
    const { exit, screens, steps } = exiting(() => Promise.resolve(refused('busy')))
    await exit.leave()
    expect(screens.failed).toHaveBeenCalledWith({ code: 'failed', message: 'busy' })
    expect(steps).toEqual([])
  })
})

describe('DemoExit.removed', () => {
  it('reopens on the board as at launch once the demo is deleted, so the previous machine shows', async () => {
    const { exit, steps } = exiting()
    const otherwise = vi.fn(() => Promise.resolve())
    await exit.removed(true, otherwise)
    expect(steps).toEqual(['root', 'load'])
    expect(otherwise).not.toHaveBeenCalled()
  })

  it('leaves the removal of a saved machine to its own steps', async () => {
    const { exit, steps } = exiting()
    const otherwise = vi.fn(() => Promise.resolve())
    await exit.removed(false, otherwise)
    expect(otherwise).toHaveBeenCalledOnce()
    expect(steps).toEqual([])
  })
})

describe('DemoExit.switched', () => {
  it('lists the machines again once another machine is joined from the demo, so its row goes', async () => {
    const { exit, steps } = exiting()
    const joined = gate<void>()
    const switching = exit.switched(DEMO_MACHINE, joined.promise)
    await Promise.resolve()
    expect(steps).toEqual([])
    joined.open()
    await switching
    expect(steps).toEqual(['reload'])
  })

  it('lists nothing again when switching between saved machines or from none', async () => {
    const { exit, screens } = exiting()
    await exit.switched(MAC, Promise.resolve())
    await exit.switched(null, Promise.resolve())
    expect(screens.reload).not.toHaveBeenCalled()
  })
})
