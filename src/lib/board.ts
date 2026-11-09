import type { AgentRow } from './bindings'

export const SECTIONS = ['Waiting', 'Done', 'Working', 'Idle'] as const

export type Section = (typeof SECTIONS)[number]

export type Item = { kind: 'header'; key: string; section: Section } | { kind: 'row'; key: string; row: AgentRow }

const SECTION_OF: Record<string, Section> = {
  waiting: 'Waiting',
  failed: 'Waiting',
  done: 'Done',
  pending: 'Done',
  working: 'Working',
  shell: 'Working',
}

const MINUTE = 60_000

const HOUR = 60 * MINUTE

const DAY = 24 * HOUR

export const sectionOf = (status: string | null | undefined): Section => SECTION_OF[status ?? ''] ?? 'Idle'

const header = (section: Section): Item => ({ kind: 'header', key: `section:${section}`, section })

const row = (agent: AgentRow): Item => ({ kind: 'row', key: `pane:${agent.pane}`, row: agent })

const sectionItems = (rows: AgentRow[], section: Section): Item[] => {
  const inside = rows.filter((agent) => sectionOf(agent.status) === section)
  return inside.length === 0 ? [] : [header(section), ...inside.map(row)]
}

export const items = (rows: AgentRow[]): Item[] => SECTIONS.flatMap((section) => sectionItems(rows, section))

const span = (millis: number) => {
  if (millis < MINUTE) {
    return 'now'
  }
  if (millis < HOUR) {
    return `${Math.floor(millis / MINUTE)} min`
  }
  return millis < DAY ? `${Math.floor(millis / HOUR)} h` : `${Math.floor(millis / DAY)} d`
}

export const elapsed = (since: string | null | undefined, now: number) => {
  const start = Date.parse(since ?? '')
  return Number.isNaN(start) ? '' : span(now - start)
}

export const place = (agent: AgentRow) =>
  agent.workspace_count > 1 ? `${agent.project_name} › ${agent.workspace_name}` : agent.project_name

export const label = (agent: AgentRow) => agent.title ?? agent.agent
