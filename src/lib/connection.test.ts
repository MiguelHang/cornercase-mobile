import { describe, expect, it } from 'vitest'
import type { ConnectionView_Serialize, Snapshot_Serialize, Tab_Serialize, Workspace_Serialize } from './bindings'
import * as connection from './connection'

const pane = (id: number) => ({
  id,
  agent: null,
  status: null,
  program: null,
  title: null,
  path: null,
  parent: null,
  active: false,
  dialog: false,
  at_prompt: null,
})

const tab = (id: number, panes: number): Tab_Serialize => ({
  id,
  name: `tab ${id}`,
  custom_name: null,
  status: null,
  active: false,
  panes: Array.from({ length: panes }, (_, index) => pane(id * 10 + index)),
})

const workspace = (id: number, tabs: Tab_Serialize[]): Workspace_Serialize => ({
  id,
  name: `workspace ${id}`,
  custom_name: null,
  path: null,
  branch: null,
  worktree: false,
  tabs,
})

const snapshot = (...workspaces: Workspace_Serialize[][]): Snapshot_Serialize => ({
  boot: 'boot',
  revision: 1,
  version: '0.1.0',
  groups: [],
  projects: workspaces.map((list, index) => ({
    id: index,
    name: `project ${index}`,
    custom_name: null,
    path: null,
    group: null,
    workspaces: list,
  })),
})

const error = (message: string) => ({ code: 'timeout' as const, message })

const RETRYING: ConnectionView_Serialize = { state: 'retrying', in_ms: 2000, error: error('timed out') }
const CONNECTED: ConnectionView_Serialize = { state: 'connected', route: 'ws://127.0.0.1:7000' }
const MISSING: ConnectionView_Serialize = {
  state: 'missing',
  missing: [
    { name: 'watch', kind: 'stream', reason: null },
    { name: 'input', kind: 'operation', reason: 'too old' },
  ],
}
const SESSION = snapshot([workspace(1, [tab(1, 2)])])
const ON_FIRST = connection.screen({ machine: 1, connection: CONNECTED }, SESSION)

describe('countPanes', () => {
  it('counts nothing without a session', () => {
    expect(connection.countPanes(null)).toBe(0)
  })

  it('counts nothing in a session without projects', () => {
    expect(connection.countPanes(snapshot())).toBe(0)
  })

  it('adds the panes of every tab of every workspace of every project', () => {
    const first = [workspace(1, [tab(1, 2), tab(2, 1)]), workspace(2, [])]
    const second = [workspace(3, [tab(3, 3), tab(4, 0)])]
    expect(connection.countPanes(snapshot(first, second))).toBe(6)
  })
})

describe('describePanes', () => {
  it('says nothing without a session', () => {
    expect(connection.describePanes(null)).toBeNull()
  })

  it.each([
    [0, '0 panes'],
    [1, '1 pane'],
    [3, '3 panes'],
  ])('tells %d panes as %s', (panes, text) => {
    expect(connection.describePanes(snapshot([workspace(1, [tab(1, panes)])]))).toBe(text)
  })
})

describe('firstToConnect', () => {
  const machine = (id: number) => ({ id, name: `mac ${id}`, routes: [], last_route: null })
  const IDLE = { machine: null, connection: { state: 'none' as const } }

  it('picks the first machine while none is active', () => {
    expect(connection.firstToConnect([machine(4), machine(2)], IDLE)).toBe(4)
  })

  it('picks nothing without machines', () => {
    expect(connection.firstToConnect([], IDLE)).toBeNull()
  })

  it('keeps the store already running', () => {
    expect(connection.firstToConnect([machine(4), machine(2)], { machine: 2, connection: CONNECTED })).toBeNull()
  })
})

describe('describe', () => {
  it.each<[string, ConnectionView_Serialize, string]>([
    ['none', { state: 'none' }, 'Not connected'],
    ['connecting', { state: 'connecting' }, 'Connecting…'],
    ['connected', CONNECTED, 'Connected through ws://127.0.0.1:7000'],
    ['retrying', RETRYING, 'Retrying in 2 s: timed out'],
    ['unauthorized', { state: 'unauthorized' }, "Not authorized: scan the bridge's code again"],
    ['incompatible', { state: 'incompatible', error: error('cornercase is too old') }, 'Incompatible: cornercase is too old'],
  ])('tells %s', (_, view, text) => {
    expect(connection.describe(view)).toBe(text)
  })

  it('names every request cornercase lacks', () => {
    expect(connection.describe(MISSING)).toBe('cornercase lacks watch, input: update it on the Mac')
  })

  it.each([
    [1000, 1],
    [1001, 2],
    [1999, 2],
    [32000, 32],
  ])('rounds %d ms up to %d s', (in_ms, seconds) => {
    expect(connection.describe({ ...RETRYING, in_ms })).toBe(`Retrying in ${seconds} s: timed out`)
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
    ['unauthorized', { state: 'unauthorized' }, failing("The bridge rejected this phone's token")],
    ['incompatible', { state: 'incompatible', error: error('too new') }, failing('Incompatible: too new')],
  ])('shows %s', (_, view, shown) => {
    expect(connection.status(view, 'MacBook')).toEqual(shown)
  })

  it('counts down while retrying', () => {
    expect(connection.status({ ...RETRYING, in_ms: 5000 }, 'MacBook', 2500).text).toBe('Retrying in 3 s')
  })

  it('never sends the user to a code to scan', () => {
    expect(connection.status({ state: 'unauthorized' }, 'MacBook').text).not.toMatch(/scan|QR/i)
  })
})

describe('routeAddress', () => {
  it.each([
    ['ws://192.168.1.20:7000/?token=secret', '192.168.1.20:7000'],
    ['wss://mac.tail1234.ts.net:7443/bridge?token=secret', 'mac.tail1234.ts.net:7443'],
    ['wss://mac.local/?token=secret', 'mac.local'],
    ['ws://[fe80::1]:7000/?token=secret', '[fe80::1]:7000'],
    ['not a route?token=secret', 'not a route'],
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
    expect(connection.retryLeft(in_ms, 100, 100 + gone)).toBe(left)
  })
})

describe('screen', () => {
  it('starts from the state and the session the commands return', () => {
    expect(ON_FIRST).toEqual({ machine: 1, connection: CONNECTED, session: SESSION })
  })

  it('starts connecting to a machine without a session', () => {
    expect(connection.connecting(2)).toEqual({ machine: 2, connection: { state: 'connecting' }, session: null })
  })
})

describe('apply', () => {
  it('takes the connection of the active machine', () => {
    expect(connection.apply(ON_FIRST, { machine: 1, connection: RETRYING })).toEqual({ ...ON_FIRST, connection: RETRYING })
  })

  it('takes the session of the active machine', () => {
    const later = snapshot([workspace(1, [tab(1, 3)])])
    expect(connection.apply(ON_FIRST, { machine: 1, session: later })).toEqual({ ...ON_FIRST, session: later })
  })

  it('empties the session when the server restarts', () => {
    expect(connection.apply(ON_FIRST, { machine: 1, session: null }).session).toBeNull()
  })

  it.each([
    ['a connection', { machine: 2, connection: RETRYING }],
    ['a session', { machine: 2, session: null }],
  ])('drops %s from a store already let go', (_, event) => {
    expect(connection.apply(ON_FIRST, event)).toBe(ON_FIRST)
  })

  it('forgets the machine and its session when it is removed', () => {
    expect(connection.apply(ON_FIRST, { machine: 1, connection: { state: 'none' } })).toEqual({
      machine: null,
      connection: { state: 'none' },
      session: null,
    })
  })

  it('drops every event while no machine is active', () => {
    const idle = connection.screen({ machine: null, connection: { state: 'none' } }, null)
    expect(connection.apply(idle, { machine: 1, connection: CONNECTED })).toBe(idle)
  })
})

describe('noticeAfter', () => {
  it('clears the notice once connected', () => {
    expect(connection.noticeAfter('Retrying in 4 s: timed out', ON_FIRST)).toBeNull()
  })

  it.each([
    ['connecting', connection.connecting(1)],
    ['retrying', { ...ON_FIRST, connection: RETRYING }],
  ])('keeps the notice while %s', (_, current) => {
    expect(connection.noticeAfter('the bridge did not answer', current)).toBe('the bridge did not answer')
  })
})
