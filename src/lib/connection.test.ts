import { describe, expect, it } from 'vitest'
import type { AgentRow_Serialize, ConnectionView_Serialize, Kind, MissingView_Serialize } from './bindings'
import * as connection from './connection'

const error = (message: string) => ({ code: 'timeout' as const, message })

const RETRYING: ConnectionView_Serialize = { state: 'retrying', in_ms: 2000, error: error('timed out') }
const CONNECTED: ConnectionView_Serialize = { state: 'connected', route: 'ws://127.0.0.1:7000' }
const MISSING: ConnectionView_Serialize = {
  state: 'missing',
  missing: [
    { name: 'watch', kind: 'stream', reason: null, since: null },
    { name: 'input', kind: 'operation', reason: 'too old', since: null },
  ],
}
const lacks = (name: string, kind: Kind, since: string | null): MissingView_Serialize => ({
  name,
  kind,
  reason: null,
  since,
})
const LACKS_THE_CLI: ConnectionView_Serialize = {
  state: 'missing',
  missing: [lacks('app-routes', 'operation', '0.13.78'), lacks('cli-endpoint', 'feature', '0.13.78')],
}
const RUNNING: AgentRow_Serialize[] = [
  {
    boot: 'b1',
    pane: 31,
    status: 'working',
    agent: 'codex',
    since: null,
    parent: null,
    depth: 0,
    role: null,
    rank_status: 'working',
    title: null,
    project_name: 'api',
    workspace_name: 'main',
    tab_name: 'shell',
    project: 2,
    workspace: 6,
    tab: 3,
    workspace_count: 1,
  },
]
const ON_FIRST = connection.screen({ machine: 1, connection: CONNECTED }, RUNNING)

describe('firstToConnect', () => {
  const machine = (id: number) => ({ id, name: `mac ${id}`, routes: [], last_route: null })
  const IDLE = { machine: null, connection: { state: 'none' as const } }
  const listed = (active: number | null) => ({ machines: [machine(4), machine(2)], active })

  it('picks the saved active machine while none is connected', () => {
    expect(connection.firstToConnect(listed(2), IDLE)).toBe(2)
  })

  it('picks the first machine once the saved one is gone', () => {
    expect(connection.firstToConnect(listed(9), IDLE)).toBe(4)
  })

  it('picks the first machine while none was saved', () => {
    expect(connection.firstToConnect(listed(null), IDLE)).toBe(4)
  })

  it('picks nothing without machines', () => {
    expect(connection.firstToConnect({ machines: [], active: null }, IDLE)).toBeNull()
  })

  it('keeps the store already running', () => {
    expect(connection.firstToConnect(listed(4), { machine: 2, connection: CONNECTED })).toBeNull()
  })
})

describe('status', () => {
  const failing = (text: string): connection.Status => ({ text, tone: 'failed', pulsing: false })

  it.each<[string, ConnectionView_Serialize, connection.Status]>([
    ['none', { state: 'none' }, failing('Not connected')],
    ['connecting', { state: 'connecting' }, { text: 'Connecting…', tone: 'working', pulsing: true }],
    ['connected', CONNECTED, { text: 'MacBook · via 127.0.0.1:7000', tone: 'done', pulsing: false }],
    ['retrying', RETRYING, { ...failing('Retrying in 2 s'), detail: 'timed out' }],
    ['missing', MISSING, failing('cornercase on the Mac lacks watch, input: update it')],
    [
      'missing the /cli',
      LACKS_THE_CLI,
      failing('cornercase on the Mac lacks app-routes, cli-endpoint: update it to 0.13.78 or later'),
    ],
    [
      'unauthorized',
      { state: 'unauthorized' },
      failing("The bridge rejected this phone's token: scan its code again in Machines"),
    ],
    [
      'not_paired',
      { state: 'not_paired' },
      failing('This Mac no longer recognizes this phone: scan its code again in Machines'),
    ],
    ['incompatible', { state: 'incompatible', error: error('too new') }, failing('Incompatible: too new')],
    ['offline', { state: 'offline' }, failing('No network')],
  ])('shows %s', (_, view, shown) => {
    expect(connection.status(view, 'MacBook')).toEqual(shown)
  })

  it.each<[string, (string | null)[], string]>([
    ['the newest release', ['0.13.78', '0.14.2', '0.13.9'], 'update it to 0.14.2 or later'],
    ['a release by its numbers, not its text', ['0.9.1', '0.10.0'], 'update it to 0.10.0 or later'],
    ['the release of whatever has one', [null, '0.13.78'], 'update it to 0.13.78 or later'],
    ['no release when none is known', [null, null], 'update it'],
  ])('names %s that brings what is missing', (_, releases, told) => {
    const missing = releases.map((since, at) => lacks(`op-${at}`, 'operation', since))
    expect(connection.status({ state: 'missing', missing }, 'MacBook').text).toMatch(new RegExp(`: ${told}$`))
  })

  it('shows the route alone while the machine has no name', () => {
    expect(connection.status(CONNECTED, '').text).toBe('via 127.0.0.1:7000')
  })

  it('counts down while retrying', () => {
    expect(connection.status({ ...RETRYING, in_ms: 5000 }, 'MacBook', 2500).text).toBe('Retrying in 3 s')
  })

  it('sends the user to scan the code again only when the Mac no longer takes this phone', () => {
    const views: ConnectionView_Serialize[] = [CONNECTED, RETRYING, MISSING, { state: 'none' }, { state: 'offline' }]
    expect(views.map((view) => connection.status(view, 'MacBook').text).filter((text) => /scan/i.test(text))).toEqual([])
    for (const state of ['unauthorized', 'not_paired'] as const) {
      expect(connection.status({ state }, 'MacBook').text).toMatch(/scan its code again in Machines$/)
    }
  })
})

describe('routeAddress', () => {
  it.each([
    ['ws://192.168.1.20:7000/', '192.168.1.20:7000'],
    ['wss://mac.tail1234.ts.net:7443/cli', 'mac.tail1234.ts.net:7443'],
    ['wss://mac.local/', 'mac.local'],
    ['ws://[fe80::1]:7000/', '[fe80::1]:7000'],
    ['not a route?code=c0de', 'not a route'],
  ])('shows %s as %s', (route, address) => {
    expect(connection.routeAddress(route)).toBe(address)
  })
})

describe('retryLeft', () => {
  it.each([
    [1001, 0, 2],
    [1000, 0, 1],
    [4000, 1500, 3],
    [4000, 4000, 1],
    [4000, 9000, 1],
  ])('waits %d ms after %d ms gone as %d s', (in_ms, gone, left) => {
    expect(connection.retryLeft(in_ms, gone)).toBe(left)
  })
})

describe('screen', () => {
  it('starts from the state and the agents the commands return', () => {
    expect(ON_FIRST).toEqual({ machine: 1, connection: CONNECTED, agents: RUNNING })
  })

  it('starts connecting to a machine without agents', () => {
    expect(connection.connecting(2)).toEqual({ machine: 2, connection: { state: 'connecting' }, agents: null })
  })
})

describe('apply', () => {
  it('takes the connection of the active machine', () => {
    expect(connection.apply(ON_FIRST, { machine: 1, connection: RETRYING })).toEqual({ ...ON_FIRST, connection: RETRYING })
  })

  it('takes the agents of the active machine', () => {
    const later = [{ ...RUNNING[0], status: 'waiting' }]
    expect(connection.apply(ON_FIRST, { machine: 1, agents: later })).toEqual({ ...ON_FIRST, agents: later })
  })

  it('empties the agents when the server restarts', () => {
    expect(connection.apply(ON_FIRST, { machine: 1, agents: null }).agents).toBeNull()
  })

  it.each([
    ['a connection', { machine: 2, connection: RETRYING }],
    ['its agents', { machine: 2, agents: [] }],
  ])('drops %s from a store already let go', (_, event) => {
    expect(connection.apply(ON_FIRST, event)).toBe(ON_FIRST)
  })

  it('forgets the machine and its agents when it is removed', () => {
    expect(connection.apply(ON_FIRST, { machine: 1, connection: { state: 'none' } })).toEqual({
      machine: null,
      connection: { state: 'none' },
      agents: null,
    })
  })

  it('drops every event while no machine is active', () => {
    const idle = connection.screen({ machine: null, connection: { state: 'none' } }, null)
    expect(connection.apply(idle, { machine: 1, connection: CONNECTED })).toBe(idle)
  })
})
