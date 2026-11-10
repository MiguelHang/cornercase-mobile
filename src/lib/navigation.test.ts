import { describe, expect, it } from 'vitest'
import type { AgentRow_Serialize as AgentRow } from './bindings'
import { backLabel, fitsLabel, type Lookups, titleOf } from './navigation'
import type { ChatOf, Route } from './router.svelte'

const ROW: AgentRow = {
  boot: 'b1',
  agent: 'claude',
  title: 'Fix the login',
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
  parent: null,
  depth: 0,
  role: null,
  rank_status: null,
}

const asked: ChatOf[] = []

const LOOKUPS: Lookups = {
  machineName: (id) => (id === 2 ? 'Studio' : undefined),
  agentRow: (chat) => {
    asked.push(chat)
    return chat.pane === 7 ? ROW : undefined
  },
  listed: () => [],
}

describe('titleOf', () => {
  it.each<[string, Route, string]>([
    ['the board', { name: 'home' }, 'Agents'],
    ['Machines', { name: 'machines' }, 'Machines'],
    ['a machine by its name', { name: 'machine', id: 2 }, 'Studio'],
    ['the start screen', { name: 'start', machine: 1 }, 'Start'],
    ['an agent by its label', { name: 'agent', machine: 1, pane: 7 }, 'Fix the login'],
    ['a terminal by its agent', { name: 'terminal', machine: 1, row: { ...ROW, title: null } }, 'claude'],
    ['the changes by their title', { name: 'changes', machine: 1, workspace: 2, title: 'main' }, 'main'],
    ['a diff by its file name', { name: 'change', machine: 1, workspace: 2, path: 'src/lib/app.rs' }, 'app.rs'],
    ['the files by their title', { name: 'files', machine: 1, workspace: 2, title: 'main' }, 'main'],
    ['a file by its name', { name: 'file', machine: 1, workspace: 2, path: 'src/lib/files.ts' }, 'files.ts'],
    ['the issues', { name: 'issues', machine: 1 }, 'Issues'],
    ['an issue by its key', { name: 'issue', machine: 1, project: 4, key: 'ENG-42', source: 'linear' }, 'ENG-42'],
  ])('names %s', (_, route, title) => {
    expect(titleOf(route, LOOKUPS)).toBe(title)
  })

  it('looks the agent up by its machine and pane', () => {
    asked.length = 0
    titleOf({ name: 'agent', machine: null, pane: 9 }, LOOKUPS)
    expect(asked).toEqual([{ machine: null, pane: 9 }])
  })

  it.each<[string, Route | null]>([
    ['a machine that is no longer known', { name: 'machine', id: 5 }],
    ['an agent whose row is gone', { name: 'agent', machine: 1, pane: 9 }],
    ['the empty screen', { name: 'no-machines' }],
    ['no route', null],
  ])('is unknown for %s', (_, route) => {
    expect(titleOf(route, LOOKUPS)).toBeNull()
  })

  it("names a flow's member by its role, as its chat does", () => {
    const lead = { ...ROW, pane: 6, role: 'orchestrator' }
    const coder = { ...ROW, parent: 6, depth: 1, role: 'coder' }
    expect(titleOf({ name: 'agent', machine: 1, pane: 7 }, { ...LOOKUPS, agentRow: () => coder, listed: () => [lead, coder] })).toBe('coder')
  })

  it('is unknown for an agent whose lookup finds no row at all', () => {
    expect(titleOf({ name: 'agent', machine: 1, pane: 7 }, { ...LOOKUPS, agentRow: () => null })).toBeNull()
  })
})

describe('backLabel', () => {
  it('names the screen it goes back to', () => {
    expect(backLabel('Agents')).toBe('Back to Agents')
  })

  it('is plain Back when the title is unknown', () => {
    expect(backLabel(null)).toBe('Back')
  })

  it.each<[string, Route]>([
    ['a machine that is no longer known', { name: 'machine', id: 5 }],
    ['an agent whose row is gone', { name: 'agent', machine: 1, pane: 9 }],
  ])('is plain Back for %s', (_, route) => {
    expect(backLabel(titleOf(route, LOOKUPS))).toBe('Back')
  })
})

describe('fitsLabel', () => {
  it.each<[string, number, number, boolean]>([
    ['fits with room to spare', 60, 100, true],
    ['fits exactly at the limit', 100, 100, true],
    ['does not fit one pixel over', 101, 100, false],
    ['does not fit without room', 1, 0, false],
  ])('%s', (_, width, room, fits) => {
    expect(fitsLabel(width, room)).toBe(fits)
  })
})
