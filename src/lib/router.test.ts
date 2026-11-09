import { describe, expect, it } from 'vitest'
import * as connection from './connection'
import { Router, routeFor } from './router.svelte'

const HOME = { name: 'home' } as const
const NO_MACHINES = { name: 'no-machines' } as const
const MACHINES = { name: 'machines' } as const
const MACHINE = { name: 'machine', id: 2 } as const
const CONNECTED = { state: 'connected', route: 'ws://127.0.0.1:7000' } as const

describe('Router', () => {
  it('starts on the route it is given', () => {
    expect(new Router(NO_MACHINES).current).toEqual(NO_MACHINES)
  })

  it('goes to a route on top of the current one', () => {
    const router = new Router(HOME)
    router.go(MACHINES)
    expect(router.current).toEqual(MACHINES)
  })

  it('goes back to the route below', () => {
    const router = new Router(HOME)
    router.go(MACHINES)
    router.back()
    expect(router.current).toEqual(HOME)
  })

  it('swaps the base and keeps the routes on top of it', () => {
    const router = new Router(NO_MACHINES)
    router.go(MACHINES)
    router.setBase(HOME)
    expect(router.current).toEqual(MACHINES)
    router.back()
    expect(router.current).toEqual(HOME)
  })

  it('keeps the base as it is when it is set to one of the same name', () => {
    const base = { name: 'home' } as const
    const router = new Router(base)
    router.go(MACHINES)
    router.setBase({ name: 'home' })
    expect(router.current).toEqual(MACHINES)
    router.back()
    expect(router.current).toBe(base)
  })

  it('replaces the only route without leaving one below', () => {
    const router = new Router(NO_MACHINES)
    router.setBase(HOME)
    expect(router.current).toEqual(HOME)
    router.back()
    expect(router.current).toEqual(HOME)
  })

  it('empties the stack when the base becomes the empty screen', () => {
    const router = new Router(HOME)
    router.go(MACHINES)
    router.go(MACHINE)
    router.setBase(NO_MACHINES)
    expect(router.current).toEqual(NO_MACHINES)
    router.back()
    expect(router.current).toEqual(NO_MACHINES)
  })

  it('empties the stack even when the empty screen is already its base', () => {
    const router = new Router(NO_MACHINES)
    router.go(MACHINES)
    router.setBase(NO_MACHINES)
    expect(router.current).toEqual(NO_MACHINES)
  })

  it('keeps the empty screen as it is when it is set again', () => {
    const base = { name: 'no-machines' } as const
    const router = new Router(base)
    router.setBase({ name: 'no-machines' })
    expect(router.current).toBe(base)
  })

  it('goes back one route at a time down to the base', () => {
    const router = new Router(HOME)
    router.go(MACHINES)
    router.go(MACHINE)
    router.back()
    expect(router.current).toEqual(MACHINES)
    router.back()
    expect(router.current).toEqual(HOME)
    router.back()
    expect(router.current).toEqual(HOME)
  })

  it('stays on the only route when going back', () => {
    const router = new Router(HOME)
    router.back()
    expect(router.current).toEqual(HOME)
  })
})

describe('routeFor', () => {
  it('shows home with a machine', () => {
    expect(routeFor(connection.screen({ machine: 1, connection: CONNECTED }, null))).toEqual(HOME)
  })

  it('shows home while connecting to a machine just added', () => {
    expect(routeFor(connection.connecting(3))).toEqual(HOME)
  })

  it('shows the empty screen without a machine', () => {
    expect(routeFor(connection.screen({ machine: null, connection: { state: 'none' } }, null))).toEqual(NO_MACHINES)
  })

  it('goes back to the empty screen once the active machine is removed', () => {
    const current = connection.screen({ machine: 1, connection: CONNECTED }, null)
    const removed = connection.apply(current, { machine: 1, connection: { state: 'none' } })
    expect(routeFor(removed)).toEqual(NO_MACHINES)
  })
})
