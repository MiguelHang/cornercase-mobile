import { describe, expect, it } from 'vitest'
import * as connection from './connection'
import { Router, routeFor } from './router.svelte'

const HOME = { name: 'home' } as const
const NO_MACHINES = { name: 'no-machines' } as const
const CONNECTED = { state: 'connected', route: 'ws://127.0.0.1:7000' } as const

describe('Router', () => {
  it('starts on the route it is given', () => {
    expect(new Router(NO_MACHINES).current).toEqual(NO_MACHINES)
  })

  it('goes to a route on top of the current one', () => {
    const router = new Router(NO_MACHINES)
    router.go(HOME)
    expect(router.current).toEqual(HOME)
  })

  it('goes back to the route below', () => {
    const router = new Router(NO_MACHINES)
    router.go(HOME)
    router.back()
    expect(router.current).toEqual(NO_MACHINES)
  })

  it('replaces the route on top and keeps the one below', () => {
    const router = new Router(NO_MACHINES)
    router.go(NO_MACHINES)
    router.replace(HOME)
    expect(router.current).toEqual(HOME)
    router.back()
    expect(router.current).toEqual(NO_MACHINES)
  })

  it('replaces the only route without leaving one below', () => {
    const router = new Router(NO_MACHINES)
    router.replace(HOME)
    router.back()
    expect(router.current).toEqual(HOME)
  })

  it('keeps the route on top when it is replaced by the same one', () => {
    const top = { name: 'home' } as const
    const router = new Router(NO_MACHINES)
    router.go(top)
    router.replace({ name: 'home' })
    expect(router.current).toBe(top)
    router.back()
    expect(router.current).toEqual(NO_MACHINES)
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
