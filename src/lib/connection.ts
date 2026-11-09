import type {
  ConnectionChanged_Serialize,
  ConnectionState_Serialize,
  ConnectionView_Serialize,
  SessionChanged_Serialize,
  Snapshot_Serialize,
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

const seconds = (ms: number) => Math.ceil(ms / 1000)

export const describe = (connection: ConnectionView_Serialize): string => {
  switch (connection.state) {
    case 'none':
      return 'Not connected'
    case 'connecting':
      return 'Connecting…'
    case 'connected':
      return `Connected through ${connection.route}`
    case 'retrying':
      return `Retrying in ${seconds(connection.in_ms)} s: ${connection.error.message}`
    case 'missing':
      return `cornercase lacks ${connection.missing.map((missing) => missing.name).join(', ')}: update it on the Mac`
    case 'unauthorized':
      return "Not authorized: scan the bridge's code again"
    case 'incompatible':
      return `Incompatible: ${connection.error.message}`
  }
}
