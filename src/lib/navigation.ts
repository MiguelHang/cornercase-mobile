import { nameOf } from './attach.svelte'
import type { AgentRow_Serialize as AgentRow } from './bindings'
import { label } from './board'
import type { ChatOf, Route } from './router.svelte'

export type Lookups = {
  machineName: (id: number) => string | undefined
  agentRow: (chat: ChatOf) => AgentRow | null | undefined
  listed: () => AgentRow[]
}

type Title<R extends Route> = (route: R, lookups: Lookups) => string | null

type Titles = { [N in Route['name']]?: Title<Extract<Route, { name: N }>> }

const agentTitle: Title<Extract<Route, { name: 'agent' }>> = (route, lookups) => {
  const row = lookups.agentRow({ machine: route.machine, pane: route.pane })
  return row == null ? null : label(row, lookups.listed())
}

const TITLES: Titles = {
  home: () => 'Agents',
  machines: () => 'Machines',
  start: () => 'Start',
  changes: (route) => route.title,
  agent: agentTitle,
  machine: (route, lookups) => lookups.machineName(route.id) ?? null,
  terminal: (route) => label(route.row),
  change: (route) => nameOf(route.path),
}

export const titleOf = (route: Route | null, lookups: Lookups): string | null => {
  if (route === null) {
    return null
  }
  const title = TITLES[route.name] as Title<Route> | undefined
  return title === undefined ? null : title(route, lookups)
}

export const backLabel = (title: string | null) => (title === null ? 'Back' : `Back to ${title}`)

export const fitsLabel = (labelWidth: number, room: number) => labelWidth <= room
