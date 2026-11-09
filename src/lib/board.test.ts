import { describe, expect, it } from 'vitest'
import type { AgentRow_Serialize as AgentRow } from './bindings'
import * as board from './board'

const NOW = Date.parse('2026-11-09T12:00:00.000Z')

const SECOND = 1000

const MINUTE = 60 * SECOND

const HOUR = 60 * MINUTE

const agent = (pane: number, status: string | null, more: Partial<AgentRow> = {}): AgentRow => ({
  boot: 'b1',
  project: 1,
  workspace: 2,
  tab: 3,
  pane,
  project_name: 'shop',
  workspace_name: 'login',
  tab_name: 'tab',
  workspace_count: 2,
  agent: 'claude',
  title: `Agent ${pane}`,
  status,
  since: null,
  parent: null,
  depth: 0,
  role: null,
  rank_status: status,
  ...more,
})

const ago = (millis: number) => new Date(NOW - millis).toISOString()

const keys = (rows: AgentRow[]) => board.items(rows).map((item) => item.key)

describe('sectionOf', () => {
  it.each([
    ['waiting', 'Waiting'],
    ['failed', 'Waiting'],
    ['done', 'Done'],
    ['pending', 'Done'],
    ['working', 'Working'],
    ['shell', 'Working'],
    ['idle', 'Idle'],
    ['something new', 'Idle'],
    [null, 'Idle'],
  ])('puts %o in %s', (status, section) => {
    expect(board.sectionOf(status)).toBe(section)
  })
})

describe('items', () => {
  it('shows the sections in the order Waiting, Done, Working, Idle', () => {
    const rows = [agent(1, 'idle'), agent(2, 'working'), agent(3, 'done'), agent(4, 'waiting')]
    expect(keys(rows)).toEqual([
      'section:Waiting',
      'pane:4',
      'section:Done',
      'pane:3',
      'section:Working',
      'pane:2',
      'section:Idle',
      'pane:1',
    ])
  })

  it('keeps the order of the rows inside a section', () => {
    const rows = [agent(7, 'failed'), agent(5, 'waiting'), agent(6, 'failed')]
    expect(keys(rows)).toEqual(['section:Waiting', 'pane:7', 'pane:5', 'pane:6'])
  })

  it('leaves out the sections without agents', () => {
    expect(keys([agent(1, 'pending'), agent(2, null)])).toEqual(['section:Done', 'pane:1', 'section:Idle', 'pane:2'])
  })

  it('is empty without agents', () => {
    expect(board.items([])).toEqual([])
  })

  it('gives every item its own key', () => {
    const rows = ['waiting', 'failed', 'done', 'pending', 'working', 'shell', 'idle', null].map((status, index) =>
      agent(index + 1, status),
    )
    const all = keys(rows)
    expect(new Set(all).size).toBe(all.length)
  })

  it('carries the section of a header and the row of an agent', () => {
    const row = agent(1, 'working')
    expect(board.items([row])).toEqual([
      { kind: 'header', key: 'section:Working', section: 'Working' },
      { kind: 'row', key: 'pane:1', row },
    ])
  })
})

describe('elapsed', () => {
  it.each([
    [0, 'now'],
    [59 * SECOND, 'now'],
    [60 * SECOND, '1 min'],
    [59 * MINUTE + 59 * SECOND, '59 min'],
    [60 * MINUTE, '1 h'],
    [23 * HOUR + 59 * MINUTE, '23 h'],
    [24 * HOUR, '1 d'],
    [73 * HOUR, '3 d'],
    [-5 * MINUTE, 'now'],
  ])('says how long %i ms is: %s', (millis, text) => {
    expect(board.elapsed(ago(millis), NOW)).toBe(text)
  })

  it.each([null, 'not a time'])('says nothing for %o', (since) => {
    expect(board.elapsed(since, NOW)).toBe('')
  })
})

describe('place', () => {
  it('names the project and the workspace when the project has several', () => {
    expect(board.place(agent(1, null))).toBe('shop › login')
  })

  it('names only the project when it has one workspace', () => {
    expect(board.place(agent(1, null, { workspace_count: 1 }))).toBe('shop')
  })
})

describe('label', () => {
  it('is the title of the agent', () => {
    expect(board.label(agent(1, null, { title: 'Fix login' }))).toBe('Fix login')
  })

  it.each([null, undefined])('is the name of the agent with %o as its title', (title) => {
    expect(board.label(agent(1, null, { title, agent: 'codex' }))).toBe('codex')
  })
})

describe('statusName', () => {
  it.each<[string | null | undefined, string]>([
    ['working', 'Working'],
    ['shell', 'Shell'],
    ['waiting', 'Waiting'],
    ['failed', 'Failed'],
    ['done', 'Done'],
    ['pending', 'Pending'],
    ['idle', 'Idle'],
    ['thinking', 'Idle'],
    [null, 'Idle'],
    [undefined, 'Idle'],
  ])('calls %o %s', (status, name) => {
    expect(board.statusName(status)).toBe(name)
  })
})
