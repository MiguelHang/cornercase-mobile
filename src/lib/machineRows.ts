import type { ConnectionView_Serialize, MachineView_Serialize, ProbeView } from './bindings'
import { routeAddress, status, type Status, type Tone } from './connection'

export type Probe = { state: 'checking' } | { state: 'failed'; message: string } | ProbeView

export type Failure = { address: string; message: string }

export type RowStatus = Status & { failures: Failure[] }

export const CHECKING: Probe = { state: 'checking' }

const row = (text: string, tone: Tone, failures: Failure[] = []): RowStatus => ({
  text,
  tone,
  pulsing: false,
  failures,
})

const probed = (probe: Probe): RowStatus => {
  switch (probe.state) {
    case 'checking':
      return { ...row('Checking…', 'working'), pulsing: true }
    case 'answers':
      return row(`Answers via ${routeAddress(probe.route)}`, 'done')
    case 'unauthorized':
      return row('Token rejected', 'failed')
    case 'no_token':
      return row('No token: scan its code again', 'failed')
    case 'no_answer':
      return row(
        'No answer',
        'failed',
        probe.failures.map(({ route, error }) => ({ address: routeAddress(route), message: error.message })),
      )
    case 'failed':
      return { ...row('No answer', 'failed'), detail: probe.message }
  }
}

export const rowStatus = (
  machine: MachineView_Serialize,
  active: ConnectionView_Serialize | null,
  probe: Probe,
  elapsed = 0,
): RowStatus => (active === null ? probed(probe) : { ...status(active, machine.name, elapsed), failures: [] })
