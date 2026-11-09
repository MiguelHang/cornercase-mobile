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
    const missing = [
      { name: 'watch', kind: 'stream' as const, reason: null },
      { name: 'input', kind: 'operation' as const, reason: 'too old' },
    ]
    expect(connection.describe({ state: 'missing', missing })).toBe('cornercase lacks watch, input: update it on the Mac')
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

  it('drops every event while no machine is active', () => {
    const idle = connection.screen({ machine: null, connection: { state: 'none' } }, null)
    expect(connection.apply(idle, { machine: 1, connection: CONNECTED })).toBe(idle)
  })
})
