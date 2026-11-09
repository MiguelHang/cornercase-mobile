import { describe, expect, it } from 'vitest'
import type { AgentRow_Serialize as AgentRow } from './bindings'
import * as connection from './connection'
import { chatOf, openedTerminal, Router, routeFor } from './router.svelte'

const HOME = { name: 'home' } as const
const NO_MACHINES = { name: 'no-machines' } as const
const MACHINES = { name: 'machines' } as const
const MACHINE = { name: 'machine', id: 2 } as const
const AGENT = { name: 'agent', machine: 1, pane: 7 } as const
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

  it('keeps Machines on top when the active machine is removed and others remain', () => {
    const router = new Router(HOME)
    router.go(MACHINES)
    const removed = connection.apply(connection.connecting(1), { machine: 1, connection: { state: 'none' } })
    router.setBase(routeFor(removed, 1))
    expect(router.current).toEqual(MACHINES)
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

  it('goes to an agent from home and back', () => {
    const router = new Router(HOME)
    router.go(AGENT)
    expect(router.current).toEqual(AGENT)
    router.back()
    expect(router.current).toEqual(HOME)
  })
})

describe('Router.setBase with an agent or its terminal open', () => {
  it('stays on an agent while the connection settles on home', () => {
    const router = new Router(HOME)
    router.go(AGENT)
    router.setBase(HOME)
    expect(router.current).toEqual(AGENT)
    router.back()
    expect(router.current).toEqual(HOME)
  })

  it('leaves only the empty screen once the machine is gone', () => {
    const router = new Router(HOME)
    router.go(AGENT)
    router.setBase(NO_MACHINES)
    expect(router.current).toEqual(NO_MACHINES)
    router.back()
    expect(router.current).toEqual(NO_MACHINES)
  })

  it('leaves only home once a machine is added', () => {
    const router = new Router(NO_MACHINES)
    router.setBase(HOME)
    expect(router.current).toEqual(HOME)
    router.back()
    expect(router.current).toEqual(HOME)
  })

  it('stays on a terminal opened from an agent while the connection settles on home', () => {
    const router = new Router(HOME)
    router.go(AGENT)
    router.go(TERMINAL)
    router.setBase(HOME)
    expect(router.current).toBe(TERMINAL)
    router.back()
    expect(router.current).toEqual(AGENT)
  })

  it('drops a terminal once its machine is gone', () => {
    const router = new Router(HOME)
    router.go(AGENT)
    router.go(TERMINAL)
    router.setBase(NO_MACHINES)
    expect(router.current).toEqual(NO_MACHINES)
  })

  it('keeps the route it settles on when it is already there', () => {
    const first = { name: 'home' } as const
    const router = new Router(first)
    router.setBase({ name: 'home' })
    expect(router.current).toBe(first)
  })
})

describe('routeFor', () => {
  it('shows home with a machine', () => {
    expect(routeFor(connection.screen({ machine: 1, connection: CONNECTED }, null), 1)).toEqual(HOME)
  })

  it('shows home while connecting to a machine just added', () => {
    expect(routeFor(connection.connecting(3), 0)).toEqual(HOME)
  })

  it('shows the empty screen without a machine', () => {
    expect(routeFor(connection.screen({ machine: null, connection: { state: 'none' } }, null), 0)).toEqual(NO_MACHINES)
  })

  it('goes back to the empty screen once the last machine is removed', () => {
    const current = connection.screen({ machine: 1, connection: CONNECTED }, null)
    const removed = connection.apply(current, { machine: 1, connection: { state: 'none' } })
    expect(routeFor(removed, 0)).toEqual(NO_MACHINES)
  })

  it('stays home without a connection once the active machine is removed and others remain', () => {
    const current = connection.screen({ machine: 1, connection: CONNECTED }, null)
    const removed = connection.apply(current, { machine: 1, connection: { state: 'none' } })
    expect(routeFor(removed, 2)).toEqual(HOME)
  })
})

const on = (machine: number | null, agents: AgentRow[] | null): connection.Screen => ({
  machine,
  connection: CONNECTED,
  agents,
})

describe('chatOf, on Back', () => {
  it('keeps the machine the agent was opened on, after switching from Machines', () => {
    const router = new Router(HOME)
    router.go(AGENT)
    router.go(MACHINES)
    router.setBase(HOME)
    router.back()
    expect(chatOf(router.current)).toEqual({ machine: 1, pane: 7 })
  })
})

describe('openedTerminal', () => {
  it('opens the terminal on the machine it was opened on', () => {
    expect(openedTerminal(TERMINAL, on(1, null))).toBe(TERMINAL)
  })

  it('opens no terminal of another machine, so its tab id is not opened on the new one', () => {
    expect(openedTerminal(TERMINAL, on(2, [ROW]))).toBeUndefined()
    expect(openedTerminal(TERMINAL, on(null, null))).toBeUndefined()
  })

  it('opens nothing on another route', () => {
    expect(openedTerminal(AGENT, on(1, [ROW]))).toBeUndefined()
  })

  it('shows no terminal once the active machine switches under it', () => {
    const router = new Router(HOME)
    router.go(AGENT)
    router.go(TERMINAL)
    router.setBase(HOME)
    expect(router.current).toBe(TERMINAL)
    expect(openedTerminal(router.current, on(2, [ROW]))).toBeUndefined()
  })
})

describe('chatOf', () => {
  it("is the agent's pane and machine on its screen and under its terminal", () => {
    expect(chatOf(AGENT)).toEqual({ machine: 1, pane: 7 })
    expect(chatOf({ ...TERMINAL, machine: 2, row: { ...ROW, pane: 9 } })).toEqual({ machine: 2, pane: 9 })
  })

  it.each([HOME, NO_MACHINES, MACHINES, MACHINE])('is none elsewhere', (route) => {
    expect(chatOf(route)).toBeNull()
  })
})
