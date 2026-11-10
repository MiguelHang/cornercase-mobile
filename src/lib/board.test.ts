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

const keys = (rows: AgentRow[], expanded?: ReadonlySet<number>) =>
  board.items(rows, expanded).map((item) => item.key)

const under = (pane: number, status: string | null, parent: number, depth: number) =>
  agent(pane, status, { parent, depth })

const TEAM = [
  agent(5, 'working', { rank_status: 'waiting' }),
  { ...under(7, 'done', 5, 1), rank_status: 'waiting' },
  under(9, 'waiting', 7, 2),
  under(12, 'working', 5, 1),
  agent(16, 'working'),
  under(17, 'working', 16, 1),
  under(18, 'pending', 16, 1),
  agent(20, 'idle'),
]

const FLOW = [
  agent(30, 'working', { role: 'orchestrator', title: 'Ship login', workspace_count: 1 }),
  { ...under(31, 'working', 30, 1), role: 'coder', title: 'Write the form' },
  { ...under(32, 'waiting', 30, 1), role: 'reviewer', title: null, agent: 'codex' },
  { ...under(33, 'working', 30, 1), tab_name: 'coder' },
  agent(40, 'working'),
  { ...under(41, 'working', 40, 1), role: 'coder' },
  agent(50, 'working', { role: 'coder', tab_name: 'coder' }),
]

const named = (pane: number, listed: AgentRow[] = FLOW) => {
  const found = FLOW.find((agent) => agent.pane === pane) ?? agent(pane, null)
  return [board.label(found, listed), board.place(found, listed)]
}

const children = (rows: AgentRow[], pane: number) =>
  board.subagents(rows, pane)?.children.map((item) => [item.row.pane, item.hasChildren, item.hidden, item.attention])

const shown = (rows: AgentRow[], expanded?: ReadonlySet<number>) =>
  board
    .items(rows, expanded)
    .flatMap((item) =>
      item.kind === 'row'
        ? [[item.row.pane, item.depth, item.hasChildren, item.folded, item.hidden, item.attention]]
        : [],
    )

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
      {
        kind: 'row',
        key: 'pane:1',
        row,
        depth: 0,
        hasChildren: false,
        folded: false,
        hidden: 0,
        attention: null,
      },
    ])
  })
})

describe('items of a tree', () => {
  it('folds every agent with children until it is unfolded', () => {
    expect(keys(TEAM)).toEqual(['section:Waiting', 'pane:5', 'section:Working', 'pane:16', 'section:Idle', 'pane:20'])
  })

  it('counts the hidden agents of every level and shows the loudest of them', () => {
    expect(shown(TEAM)).toEqual([
      [5, 0, true, true, 3, 'waiting'],
      [16, 0, true, true, 2, 'pending'],
      [20, 0, false, false, 0, null],
    ])
  })

  it.each([
    [['failed', 'done', 'pending'], 'failed'],
    [['pending', 'done'], 'done'],
    [['failed', 'waiting'], 'waiting'],
    [['working', 'pending'], 'pending'],
  ])('shows %o hidden as %s', (statuses, attention) => {
    const rows = [agent(1, 'working'), ...statuses.map((status, index) => under(index + 2, status, 1, 1))]
    expect(shown(rows)[0]).toEqual([1, 0, true, true, statuses.length, attention])
  })

  it('says nothing but the count when the hidden agents only work or idle', () => {
    const rows = [agent(1, 'done'), under(2, 'working', 1, 1), under(3, 'shell', 2, 2), under(4, 'idle', 1, 1)]
    expect(shown(rows)).toEqual([[1, 0, true, true, 3, null]])
  })

  it('puts the whole branch in the section of its root', () => {
    expect(keys(TEAM, new Set([5, 7, 16]))).toEqual([
      'section:Waiting',
      'pane:5',
      'pane:7',
      'pane:9',
      'pane:12',
      'section:Working',
      'pane:16',
      'pane:17',
      'pane:18',
      'section:Idle',
      'pane:20',
    ])
  })

  it('keeps a folded child folded under an unfolded root', () => {
    expect(shown(TEAM, new Set([5]))).toEqual([
      [5, 0, true, false, 0, null],
      [7, 1, true, true, 1, 'waiting'],
      [12, 1, false, false, 0, null],
      [16, 0, true, true, 2, 'pending'],
      [20, 0, false, false, 0, null],
    ])
  })

  it('keeps the order whether a branch is folded or not', () => {
    const unfolded = keys(TEAM, new Set([5, 7, 16]))
    const gone = ['pane:9', 'pane:17', 'pane:18']
    expect(keys(TEAM, new Set([5]))).toEqual(unfolded.filter((key) => !gone.includes(key)))
  })

  it('hides a new child under a folded parent and only raises its count', () => {
    const grown = [...TEAM, under(21, 'done', 16, 1)]
    expect([keys(grown), shown(grown)[1]]).toEqual([keys(TEAM), [16, 0, true, true, 3, 'done']])
  })

  it('stops indenting after three levels', () => {
    const chain = [agent(1, 'working'), ...[2, 3, 4, 5].map((pane) => under(pane, 'working', pane - 1, pane - 1))]
    expect(shown(chain, new Set([1, 2, 3, 4])).map(([, depth]) => depth)).toEqual([0, 1, 2, 3, 3])
  })

  it('lists an agent whose parent is not listed as a root', () => {
    expect(keys([under(3, 'working', 99, 1)])).toEqual(['section:Working', 'pane:3'])
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

describe('label and place of a flow', () => {
  it('names a member by its role and puts its title below', () => {
    expect(named(31)).toEqual(['coder', 'Write the form'])
  })

  it('puts the name of the agent below the role when the member has no title', () => {
    expect(named(32)).toEqual(['reviewer', 'codex'])
  })

  it('keeps the title of the orchestrator and adds its role to the place', () => {
    expect(named(30)).toEqual(['Ship login', 'shop · orchestrator'])
  })

  it.each([
    ['a member whose parent has no role', 41],
    ['a member left as a root', 50],
  ])('shows %s as before', (_, pane) => {
    expect(named(pane)).toEqual([`Agent ${pane}`, 'shop › login'])
  })

  it('never takes the role from the name of the tab', () => {
    expect(named(33)).toEqual(['Agent 33', 'shop › login'])
  })

  it('shows a member as before when its parent is not listed', () => {
    expect([named(31, FLOW.slice(1)), named(31, [])]).toEqual([
      ['Write the form', 'shop › login'],
      ['Write the form', 'shop › login'],
    ])
  })
})

describe('subagents', () => {
  it('counts the subagents of every level by status', () => {
    expect(board.subagents(TEAM, 5)?.line).toBe('3 subagents · 1 waiting, 1 done, 1 working')
  })

  it('lists the direct children with the attention and count of what is below them', () => {
    expect(children(TEAM, 5)).toEqual([
      [7, true, 1, 'waiting'],
      [12, false, 0, null],
    ])
  })

  it('says it in the singular for one subagent', () => {
    expect(board.subagents(TEAM, 7)?.line).toBe('1 subagent · 1 waiting')
  })

  it('puts the statuses that need attention first and calls the rest idle', () => {
    const rows = [agent(1, 'working'), under(2, 'working', 1, 1), under(3, null, 1, 1), under(4, 'waiting', 2, 2)]
    expect(board.subagents(rows, 1)?.line).toBe('3 subagents · 1 waiting, 1 working, 1 idle')
  })

  it('says 3 subagents · 1 waiting, 2 working', () => {
    const rows = [agent(1, 'working'), under(2, 'working', 1, 1), under(3, 'working', 2, 2), under(4, 'waiting', 1, 1)]
    expect(board.subagents(rows, 1)?.line).toBe('3 subagents · 1 waiting, 2 working')
  })

  it.each([
    ['an agent without children', 9],
    ['a pane that is not listed', 99],
  ])('is nothing for %s', (_, pane) => {
    expect(board.subagents(TEAM, pane)).toBeNull()
  })
})

describe('hiddenName', () => {
  it.each<[string, number, string | null, string]>([
    ['the count alone when nothing hidden needs attention', 3, null, '3 subagents hidden'],
    ['one hidden by its status', 1, 'waiting', '1 subagent hidden, waiting'],
    ['the loudest status of several as one of them at least', 3, 'failed', '3 subagents hidden, at least one failed'],
    ['pending by its name', 2, 'pending', '2 subagents hidden, at least one pending'],
  ])('says %s', (_, hidden, attention, name) => {
    expect(board.hiddenName(hidden, attention)).toBe(name)
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
