import type { ConnectionView_Serialize, MachineView_Serialize, ProbeView } from './bindings'
import { routeAddress, spaced, status, via, type Status, type Tone } from './connection'

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
      return row(spaced(' ', 'Answers', via(probe.route)), 'done')
    case 'not_paired':
      return row('Not paired', 'failed')
    case 'no_token':
      return row('No secret', 'failed')
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

const RESCANNABLE = new Set(['not_paired', 'no_token'])

export const rescannable = (active: ConnectionView_Serialize | null, probe: Probe) =>
  RESCANNABLE.has((active ?? probe).state)
