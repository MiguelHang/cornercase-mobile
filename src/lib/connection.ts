import type {
  AgentRow_Serialize,
  AgentsChanged_Serialize,
  ConnectionChanged_Serialize,
  ConnectionState_Serialize,
  ConnectionView_Serialize,
  MachineList_Serialize,
  MissingView_Serialize,
} from './bindings'

export type Screen = {
  machine: number | null
  connection: ConnectionView_Serialize
  agents: AgentRow_Serialize[] | null
}

export type Changed = ConnectionChanged_Serialize | AgentsChanged_Serialize

export const screen = (state: ConnectionState_Serialize, agents: AgentRow_Serialize[] | null): Screen => ({
  ...state,
  agents,
})

export const connecting = (machine: number): Screen => ({
  machine,
  connection: { state: 'connecting' },
  agents: null,
})

export const apply = (current: Screen, event: Changed): Screen => {
  if (event.machine !== current.machine) {
    return current
  }
  if ('agents' in event) {
    return { ...current, agents: event.agents }
  }
  if (event.connection.state === 'none') {
    return { machine: null, connection: event.connection, agents: null }
  }
  return { ...current, connection: event.connection }
}

export const firstToConnect = ({ machines, active }: MachineList_Serialize, state: ConnectionState_Serialize) => {
  if (state.machine !== null) {
    return null
  }
  return machines.some((machine) => machine.id === active) ? active : (machines[0]?.id ?? null)
}

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

const release = (version: string) => version.split('.').map(Number)

const later = (one: string, other: string) => {
  const [mine, theirs] = [release(one), release(other)]
  const differs = mine.findIndex((part, at) => part !== (theirs[at] ?? 0))
  return differs === -1 ? false : mine[differs] > (theirs[differs] ?? 0)
}

const newest = (missing: MissingView_Serialize[]) =>
  missing.reduce<string | null>(
    (found, { since }) => (since !== null && (found === null || later(since, found)) ? since : found),
    null,
  )

const lacking = (missing: MissingView_Serialize[]) => {
  const since = newest(missing)
  const names = missing.map(({ name }) => name).join(', ')
  return `cornercase on the Mac lacks ${names}: update it${since === null ? '' : ` to ${since} or later`}`
}

type Failed = Exclude<ConnectionView_Serialize, { state: 'connecting' | 'connected' | 'retrying' }>

const failure = (connection: Failed) => {
  switch (connection.state) {
    case 'none':
      return 'Not connected'
    case 'missing':
      return lacking(connection.missing)
    case 'unauthorized':
      return "The bridge rejected this phone's token: scan its code again in Machines"
    case 'not_paired':
      return 'This Mac no longer recognizes this phone: scan its code again in Machines'
    case 'incompatible':
      return `Incompatible: ${connection.error.message}`
    case 'offline':
      return 'No network'
  }
}

export const status = (connection: ConnectionView_Serialize, machineName: string, elapsed = 0): Status => {
  switch (connection.state) {
    case 'connecting':
      return { text: 'Connecting…', tone: 'working', pulsing: true }
    case 'connected':
      return { text: connectedTo(machineName, connection.route), tone: 'done', pulsing: false }
    case 'retrying':
      return { ...failed(`Retrying in ${retryLeft(connection.in_ms, elapsed)} s`), detail: connection.error.message }
    default:
      return failed(failure(connection))
  }
}
