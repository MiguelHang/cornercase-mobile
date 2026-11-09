import { describe, expect, it } from 'vitest'
import type { AgentRow_Serialize as AgentRow } from './bindings'
import * as connection from './connection'
import { Router, chatPane, routeFor } from './router.svelte'

const HOME = { name: 'home' } as const
const NO_MACHINES = { name: 'no-machines' } as const
const AGENT = { name: 'agent', pane: 7 } as const
const ROW: AgentRow = {
  boot: 'b1',
  agent: 'claude',
  title: null,
  status: 'working',
  since: null,
  pane: 7,
  tab: 3,
  tab_name: 'fix login',
  workspace: 2,
  workspace_name: 'main',
  workspace_count: 1,
  project: 1,
  project_name: 'app',
}
const TERMINAL = { name: 'terminal', machine: 1, row: ROW } as const
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

  it('stays on the only route when going back', () => {
    const router = new Router(HOME)
    router.back()
    expect(router.current).toEqual(HOME)
  })

  it('goes to an agent from home and back', () => {
    const router = new Router(HOME)
    router.go(AGENT)
    expect(router.current).toEqual(AGENT)
    router.back()
    expect(router.current).toEqual(HOME)
  })
})

describe('Router.settle', () => {
  it('stays on an agent while the connection settles on home', () => {
    const router = new Router(HOME)
    router.go(AGENT)
    router.settle(HOME)
    expect(router.current).toEqual(AGENT)
    router.back()
    expect(router.current).toEqual(HOME)
  })

  it('leaves only the empty screen once the machine is gone', () => {
    const router = new Router(HOME)
    router.go(AGENT)
    router.settle(NO_MACHINES)
    expect(router.current).toEqual(NO_MACHINES)
    router.back()
    expect(router.current).toEqual(NO_MACHINES)
  })

  it('leaves only home once a machine is added', () => {
    const router = new Router(NO_MACHINES)
    router.settle(HOME)
    expect(router.current).toEqual(HOME)
    router.back()
    expect(router.current).toEqual(HOME)
  })

  it('stays on a terminal opened from an agent while the connection settles on home', () => {
    const router = new Router(HOME)
    router.go(AGENT)
    router.go(TERMINAL)
    router.settle(HOME)
    expect(router.current).toBe(TERMINAL)
    router.back()
    expect(router.current).toEqual(AGENT)
  })

  it('drops a terminal once its machine is gone', () => {
    const router = new Router(HOME)
    router.go(AGENT)
    router.go(TERMINAL)
    router.settle(NO_MACHINES)
    expect(router.current).toEqual(NO_MACHINES)
  })

  it('keeps the route it settles on when it is already there', () => {
    const first = { name: 'home' } as const
    const router = new Router(first)
    router.settle({ name: 'home' })
    expect(router.current).toBe(first)
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

describe('chatPane', () => {
  it("is the agent's pane on its screen and under its terminal", () => {
    expect(chatPane(AGENT)).toBe(7)
    expect(chatPane({ ...TERMINAL, row: { ...ROW, pane: 9 } })).toBe(9)
  })

  it.each([HOME, NO_MACHINES])('is none elsewhere', (route) => {
    expect(chatPane(route)).toBeNull()
  })
})
