import type {
  AgentRow_Serialize,
  AgentsChanged_Serialize,
  ConnectionChanged_Serialize,
  ConnectionState_Serialize,
  ConnectionView_Serialize,
  MachineList_Serialize,
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
      return failed("The bridge rejected this phone's token: scan its code again in Machines")
    case 'incompatible':
      return failed(`Incompatible: ${connection.error.message}`)
    case 'offline':
      return failed('No network')
  }
}
