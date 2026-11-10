import type { AgentRow_Serialize as AgentRow } from './bindings'

export const SECTIONS = ['Waiting', 'Done', 'Working', 'Idle'] as const

export type Section = (typeof SECTIONS)[number]

export type Row = {
  kind: 'row'
  key: string
  row: AgentRow
  depth: number
  hasChildren: boolean
  folded: boolean
  hidden: number
  total: number
  attention: string | null
}

export type Item = { kind: 'header'; key: string; section: Section } | Row

export type Subagents = { children: Row[]; line: string }

type Tree = { roots: AgentRow[]; children: Map<number, AgentRow[]>; expanded: ReadonlySet<number> }

type Walk = (made: Tree, agent: AgentRow) => Item[]

const SECTION_OF: Record<string, Section> = {
  waiting: 'Waiting',
  failed: 'Waiting',
  done: 'Done',
  pending: 'Done',
  working: 'Working',
  shell: 'Working',
}

const NAMES: Record<string, string> = {
  working: 'Working',
  shell: 'Shell',
  waiting: 'Waiting',
  failed: 'Failed',
  done: 'Done',
  pending: 'Pending',
}

const MINUTE = 60_000

const HOUR = 60 * MINUTE

const DAY = 24 * HOUR

const MAX_DEPTH = 3

const ATTENTION = ['waiting', 'failed', 'done', 'pending']

const NO_PANES: ReadonlySet<number> = new Set()

const ORCHESTRATOR = 'orchestrator'

const COUNTED = ['waiting', 'failed', 'done', 'pending', 'working', 'shell', 'idle']

export const sectionOf = (status: string | null): Section => SECTION_OF[status ?? ''] ?? 'Idle'

export const statusName = (status: string | null | undefined) => NAMES[status ?? ''] ?? 'Idle'

const header = (section: Section): Item => ({ kind: 'header', key: `section:${section}`, section })

const tree = (rows: AgentRow[], expanded: ReadonlySet<number>): Tree => {
  const listed = new Set(rows.map((agent) => agent.pane))
  const children = new Map<number, AgentRow[]>()
  const roots: AgentRow[] = []
  for (const agent of rows) {
    if (agent.parent === null || !listed.has(agent.parent)) {
      roots.push(agent)
    } else {
      children.set(agent.parent, [...(children.get(agent.parent) ?? []), agent])
    }
  }
  return { roots, children, expanded }
}

const below = (made: Tree, agent: AgentRow) => made.children.get(agent.pane) ?? []

const descendants = (made: Tree, agent: AgentRow): AgentRow[] =>
  below(made, agent).flatMap((child) => [child, ...descendants(made, child)])

const loudest = (agents: AgentRow[]) =>
  ATTENTION.find((status) => agents.some((agent) => agent.status === status)) ?? null

const row = (agent: AgentRow, all: AgentRow[], folded: boolean): Row => {
  const hidden = folded ? all : []
  return {
    kind: 'row',
    key: `pane:${agent.pane}`,
    row: agent,
    depth: Math.min(agent.depth, MAX_DEPTH),
    hasChildren: all.length > 0,
    folded,
    hidden: hidden.length,
    total: all.length,
    attention: loudest(hidden),
  }
}

const branch = (made: Tree, agent: AgentRow): Item[] => {
  const all = descendants(made, agent)
  const folded = all.length > 0 && !made.expanded.has(agent.pane)
  const shown = folded ? [] : below(made, agent).flatMap((child) => branch(made, child))
  return [row(agent, all, folded), ...shown]
}

const found = (made: Tree, agent: AgentRow): Item[] => [
  row(agent, [], false),
  ...below(made, agent).flatMap((child) => found(made, child)),
]

const sectionItems = (made: Tree, section: Section, walk: Walk): Item[] => {
  const inside = made.roots.filter((agent) => sectionOf(agent.rank_status) === section)
  return inside.length === 0 ? [] : [header(section), ...inside.flatMap((agent) => walk(made, agent))]
}

const pruned = (rows: AgentRow[], wanted: string[]) => {
  const made = tree(rows, NO_PANES)
  const hit = (agent: AgentRow) => [agent, ...descendants(made, agent)].some((one) => matches(one, wanted))
  return rows.filter(hit)
}

const closed = (made: Tree, agent: AgentRow) => {
  const hidden = descendants(made, agent)
  return row(agent, hidden, hidden.length > 0)
}

const counted = (count: number) => `${count} ${count === 1 ? 'subagent' : 'subagents'}`

const tally = (agents: AgentRow[]) =>
  COUNTED.flatMap((status) => {
    const count = agents.filter((agent) => statusName(agent.status).toLowerCase() === status).length
    return count === 0 ? [] : [`${count} ${status}`]
  }).join(', ')

export const subagents = (rows: AgentRow[], pane: number): Subagents | null => {
  const made = tree(rows, NO_PANES)
  const children = made.children.get(pane) ?? []
  if (children.length === 0) {
    return null
  }
  const all = children.flatMap((child) => [child, ...descendants(made, child)])
  return { children: children.map((child) => closed(made, child)), line: `${counted(all.length)} · ${tally(all)}` }
}

export const hiddenName = (hidden: number, attention: string | null) => {
  const what = `${counted(hidden)} hidden`
  if (attention === null) {
    return what
  }
  const status = statusName(attention).toLowerCase()
  return hidden === 1 ? `${what}, ${status}` : `${what}, at least one ${status}`
}

export const disclosureName = (count: number, folded: boolean) => `${folded ? 'Show' : 'Hide'} ${counted(count)}`

export const items = (rows: AgentRow[], expanded: ReadonlySet<number> = NO_PANES, query = ''): Item[] => {
  const wanted = terms(query)
  const made = tree(wanted.length === 0 ? rows : pruned(rows, wanted), expanded)
  const walk = wanted.length === 0 ? branch : found
  return SECTIONS.flatMap((section) => sectionItems(made, section, walk))
}

export const span = (millis: number) => {
  if (millis < MINUTE) {
    return 'now'
  }
  if (millis < HOUR) {
    return `${Math.floor(millis / MINUTE)} min`
  }
  return millis < DAY ? `${Math.floor(millis / HOUR)} h` : `${Math.floor(millis / DAY)} d`
}

export const elapsed = (since: string | null, now: number) => {
  const start = Date.parse(since ?? '')
  return Number.isNaN(start) ? '' : span(now - start)
}

const member = (agent: AgentRow, listed: AgentRow[]) =>
  agent.role !== null &&
  agent.role !== ORCHESTRATOR &&
  (listed.find((other) => other.pane === agent.parent)?.role ?? null) !== null

const named = (agent: AgentRow) => agent.title ?? agent.agent

const where = (agent: AgentRow) =>
  agent.workspace_count > 1 ? `${agent.project_name} › ${agent.workspace_name}` : agent.project_name

export const place = (agent: AgentRow, listed: AgentRow[] = []) => {
  if (member(agent, listed)) {
    return named(agent)
  }
  return agent.role === ORCHESTRATOR ? `${where(agent)} · ${ORCHESTRATOR}` : where(agent)
}

export const label = (agent: AgentRow, listed: AgentRow[] = []) =>
  (member(agent, listed) ? agent.role : null) ?? named(agent)

const normalized = (text: string) =>
  text
    .toLowerCase()
    .normalize('NFD')
    .replace(/\p{M}/gu, '')

export const terms = (query: string) =>
  normalized(query)
    .split(/\s+/)
    .filter((term) => term !== '')

export const matches = (agent: AgentRow, wanted: string[]) => {
  const fields = [agent.title, agent.agent, agent.project_name, agent.workspace_name, agent.role]
  const words = fields.map((field) => normalized(field ?? ''))
  return wanted.every((term) => words.some((word) => word.includes(term)))
}
