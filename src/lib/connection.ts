import type {
  ConnectionChanged_Serialize,
  ConnectionState_Serialize,
  ConnectionView_Serialize,
  MachineView_Serialize,
  SessionChanged_Serialize,
  Snapshot_Serialize,
  Tab_Serialize,
} from './bindings'

export type Screen = {
  machine: number | null
  connection: ConnectionView_Serialize
  session: Snapshot_Serialize | null
}

export type Changed = ConnectionChanged_Serialize | SessionChanged_Serialize

export const screen = (state: ConnectionState_Serialize, session: Snapshot_Serialize | null): Screen => ({
  ...state,
  session,
})

export const connecting = (machine: number): Screen => ({ machine, connection: { state: 'connecting' }, session: null })

export const apply = (current: Screen, event: Changed): Screen => {
  if (event.machine !== current.machine) {
    return current
  }
  if ('connection' in event && event.connection.state === 'none') {
    return { machine: null, connection: event.connection, session: null }
  }
  if ('connection' in event) {
    return { ...current, connection: event.connection }
  }
  return { ...current, session: event.session }
}

export const countPanes = (session: Snapshot_Serialize | null) =>
  (session?.projects ?? [])
    .flatMap((project) => project.workspaces)
    .flatMap((workspace) => workspace.tabs)
    .reduce((panes, tab) => panes + tab.panes.length, 0)

export const describePanes = (session: Snapshot_Serialize | null) => {
  if (session === null) {
    return null
  }
  const panes = countPanes(session)
  return panes === 1 ? '1 pane' : `${panes} panes`
}

export type AgentTab = { tab: number; pane: number; name: string; agent: string; where: string }

const named = (item: { name: string; custom_name: string | null }) => item.custom_name ?? item.name

const withAgent = (tab: Tab_Serialize, where: string): AgentTab[] => {
  const pane = tab.panes.find((candidate) => candidate.agent !== null)
  if (pane === undefined || pane.agent === null) {
    return []
  }
  return [{ tab: tab.id, pane: pane.id, name: named(tab), agent: pane.agent, where }]
}

export const agentTabs = (session: Snapshot_Serialize | null): AgentTab[] =>
  (session?.projects ?? []).flatMap((project) =>
    project.workspaces.flatMap((workspace) =>
      workspace.tabs.flatMap((tab) => withAgent(tab, `${named(project)} · ${named(workspace)}`)),
    ),
  )

export const firstToConnect = (machines: MachineView_Serialize[], state: ConnectionState_Serialize) =>
  state.machine === null ? (machines[0]?.id ?? null) : null

const seconds = (ms: number) => Math.ceil(ms / 1000)

export type Tone = 'working' | 'done' | 'failed'

export type Status = { text: string; detail?: string; tone: Tone; pulsing: boolean }

const parsed = (route: string) => {
  try {
    return new URL(route)
  } catch {
    return null
  }
}

export const routeAddress = (route: string) => parsed(route)?.host || route.replace(/\?.*$/s, '')

export const retryLeft = (in_ms: number, elapsed: number) => Math.max(1, seconds(in_ms - elapsed))

const connectedTo = (machineName: string, route: string) =>
  machineName ? `${machineName} · via ${routeAddress(route)}` : `via ${routeAddress(route)}`

const failed = (text: string): Status => ({ text, tone: 'failed', pulsing: false })

export const status = (connection: ConnectionView_Serialize, machineName: string, elapsed = 0): Status => {
  switch (connection.state) {
    case 'none':
      return failed('Not connected')
    case 'connecting':
      return { text: 'Connecting…', tone: 'working', pulsing: true }
    case 'connected':
      return { text: connectedTo(machineName, connection.route), tone: 'done', pulsing: false }
    case 'retrying':
      return { ...failed(`Retrying in ${retryLeft(connection.in_ms, elapsed)} s`), detail: connection.error.message }
    case 'missing':
      return failed(`cornercase on the Mac lacks ${connection.missing.map((missing) => missing.name).join(', ')}: update it`)
    case 'unauthorized':
      return failed("The bridge rejected this phone's token")
    case 'incompatible':
      return failed(`Incompatible: ${connection.error.message}`)
    case 'offline':
      return failed('No network')
  }
}
