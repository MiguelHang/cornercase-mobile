import { describe, expect, it, vi } from 'vitest'
import type { CommandError, FlowChoice_Serialize as FlowChoice, Flows_Serialize as Flows } from './bindings'
import { FlowStart, type FlowCalls, type FlowMemory } from './flowStart.svelte'
import { CHOICES_UNANSWERED } from './startForm.svelte'

type Answer = Promise<{ status: 'ok'; data: Flows } | { status: 'error'; error: CommandError }>

const ok = (flows: FlowChoice[]): Answer => Promise.resolve({ status: 'ok', data: { flows } })

const refused = (error: CommandError): Answer => Promise.resolve({ status: 'error', error })

const gate = () => {
  let open: (flows: FlowChoice[]) => void = () => {}
  const promise: Answer = new Promise((resolve) => (open = (flows) => resolve({ status: 'ok', data: { flows } })))
  return { promise, open }
}

const role = (name: string, agent: string) => ({ role: name, agent })

const flow = (name: string, overrides: Partial<FlowChoice> = {}): FlowChoice => ({
  name,
  description: `${name}, described`,
  default: false,
  problems: [],
  roles: [role('orchestrator', 'claude'), role('coder', 'codex')],
  ...overrides,
})

const SHIP = flow('Ship it', {
  default: true,
  roles: [role('coder', 'codex'), role('orchestrator', 'claude'), role('tester', 'claude')],
})
const QUICK = flow('Quick')
const SECOND = flow('Second opinion', { problems: ['`summary` is in no stage', 'no reviewer'] })
const SHOP = [SECOND, QUICK, SHIP]

const inTurn = (...answers: Answer[]) => {
  let left = answers
  return () => {
    const [answer, ...rest] = left
    left = rest
    return answer
  }
}

const made = (issueFlows: FlowCalls['issueFlows'] = () => ok(SHOP), memory: FlowMemory = { flow: null }) => {
  const calls = { issueFlows: vi.fn(issueFlows) }
  return { flows: new FlowStart(calls, memory), calls, memory }
}

describe('FlowStart, reading a project s flows', () => {
  it('waits for the list, then shows it in cornercase s order', async () => {
    const answer = gate()
    const { flows, calls } = made(() => answer.promise)
    const read = flows.read(8, 'boot-1')
    expect(flows.flows).toEqual({ kind: 'loading' })
    answer.open(SHOP)
    await read
    expect(calls.issueFlows).toHaveBeenCalledWith(8)
    expect(flows.flows).toEqual({ kind: 'ready', items: SHOP })
  })

  it('says each flow s name, agents and description, and the first problem of one that cannot run', async () => {
    const { flows } = made(() => ok([SECOND, flow('Solo', { roles: [role('coder', 'codex')] })]))
    await flows.read(8, 'boot-1')
    expect(flows.options).toEqual({
      kind: 'ready',
      items: [
        { name: 'Second opinion', agents: '2 agents', description: 'Second opinion, described', problem: SECOND.problems[0] },
        { name: 'Solo', agents: '1 agent', description: 'Solo, described', problem: null },
      ],
    })
  })

  it('reads a project once while the form lives', async () => {
    const { flows, calls } = made()
    await flows.read(8, 'boot-1')
    flows.flow = 'Quick'
    await flows.read(8, 'boot-1')
    await flows.read(3, 'boot-1')
    await flows.read(8, 'boot-1')
    expect(calls.issueFlows.mock.calls).toEqual([[8], [3]])
    expect(flows.flows).toEqual({ kind: 'ready', items: SHOP })
  })

  it('keeps the choice while the project stays the same', async () => {
    const { flows } = made()
    await flows.read(8, 'boot-1')
    flows.flow = 'Quick'
    await flows.read(8, 'boot-1')
    expect(flows.flow).toBe('Quick')
  })

  it('asks once for a project it is still reading', async () => {
    const answer = gate()
    const { flows, calls } = made(() => answer.promise)
    const first = flows.read(8, 'boot-1')
    const again = flows.read(8, 'boot-1')
    answer.open(SHOP)
    await Promise.all([first, again])
    expect(calls.issueFlows).toHaveBeenCalledTimes(1)
  })

  it('ignores the late answer of a project left for another', async () => {
    const late = gate()
    const { flows, calls } = made((project) => (project === 8 ? late.promise : ok([QUICK])))
    const left = flows.read(8, 'boot-1')
    await flows.read(3, 'boot-1')
    late.open(SHOP)
    await left
    expect(flows.flows).toEqual({ kind: 'ready', items: [QUICK] })
    expect(flows.flow).toBe('Quick')
    await flows.read(8, 'boot-1')
    expect(calls.issueFlows.mock.calls).toEqual([[8], [3], [8]])
  })

  it('ignores the late answer of a project left for one already read', async () => {
    const late = gate()
    const { flows } = made((project) => (project === 3 ? late.promise : ok(SHOP)))
    await flows.read(8, 'boot-1')
    const left = flows.read(3, 'boot-1')
    await flows.read(8, 'boot-1')
    late.open([QUICK])
    await left
    expect(flows.flows).toEqual({ kind: 'ready', items: SHOP })
    expect(flows.flow).toBe('Ship it')
  })
})

describe('FlowStart, when the list cannot be read', () => {
  it('says cornercase s reason', async () => {
    const { flows } = made(() => refused({ code: 'failed', message: 'no project 8' }))
    await flows.read(8, 'boot-1')
    expect(flows.flows).toEqual({ kind: 'failed', reason: 'no project 8' })
    expect(flows.options).toEqual({ kind: 'failed', reason: 'no project 8' })
    expect([flows.flow, flows.chosen, flows.team]).toEqual([null, undefined, []])
  })

  it('says cornercase did not answer', async () => {
    const { flows } = made(() => refused({ code: 'unanswered', message: 'the request was left unanswered' }))
    await flows.read(8, 'boot-1')
    expect(flows.flows).toEqual({ kind: 'failed', reason: CHOICES_UNANSWERED })
  })

  it('says why a call threw', async () => {
    const { flows } = made(() => Promise.reject(new Error('the bridge is gone')))
    await flows.read(8, 'boot-1')
    expect(flows.flows).toEqual({ kind: 'failed', reason: 'the bridge is gone' })
  })

  it('tries again after a failure', async () => {
    const { flows, calls } = made(inTurn(refused({ code: 'failed', message: 'busy' }), ok(SHOP)))
    await flows.read(8, 'boot-1')
    await flows.retry()
    expect(flows.flows).toEqual({ kind: 'ready', items: SHOP })
    expect(flows.flow).toBe('Ship it')
    expect(calls.issueFlows).toHaveBeenCalledTimes(2)
  })

  it('reads again a project that failed when it is asked for again', async () => {
    const { flows } = made(inTurn(refused({ code: 'failed', message: 'busy' }), ok(SHOP)))
    await flows.read(8, 'boot-1')
    await flows.read(8, 'boot-1')
    expect(flows.flows).toEqual({ kind: 'ready', items: SHOP })
  })

  it('has nothing to try again before a project is asked for', async () => {
    const { flows, calls } = made()
    await flows.retry()
    expect(calls.issueFlows).not.toHaveBeenCalled()
    expect(flows.flows).toEqual({ kind: 'loading' })
  })
})

describe('FlowStart, choosing a flow', () => {
  it('chooses the project s default that can run', async () => {
    const { flows } = made()
    await flows.read(8, 'boot-1')
    expect(flows.flow).toBe('Ship it')
    expect(flows.chosen).toEqual(SHIP)
  })

  it('chooses the last flow started from the app while it is listed and can run', async () => {
    const memory = { flow: 'Quick' }
    const { flows } = made(undefined, memory)
    await flows.read(8, 'boot-1')
    expect(flows.flow).toBe('Quick')
  })

  it.each([
    ['is gone', 'Arena'],
    ['cannot run', 'Second opinion'],
  ])('passes over a last flow that %s', async (_, last) => {
    const { flows } = made(undefined, { flow: last })
    await flows.read(8, 'boot-1')
    expect(flows.flow).toBe('Ship it')
  })

  it('passes over a default that cannot run for the first that can', async () => {
    const broken = flow('Ship it', { default: true, problems: ['codex is not installed'] })
    const { flows } = made(() => ok([SECOND, broken, QUICK, flow('Arena')]))
    await flows.read(8, 'boot-1')
    expect(flows.flow).toBe('Quick')
  })

  it.each([
    ['no flows', []],
    ['only flows that cannot run', [SECOND]],
  ])('chooses nothing with %s', async (_, listed: FlowChoice[]) => {
    const { flows } = made(() => ok(listed))
    await flows.read(8, 'boot-1')
    expect([flows.flow, flows.chosen, flows.team]).toEqual([null, undefined, []])
  })

  it('chooses again by the rule in another project', async () => {
    const { flows } = made((project) => ok(project === 8 ? SHOP : [SECOND, flow('Arena'), QUICK]))
    await flows.read(8, 'boot-1')
    flows.flow = 'Quick'
    await flows.read(3, 'boot-1')
    expect(flows.flow).toBe('Arena')
    await flows.read(8, 'boot-1')
    expect(flows.flow).toBe('Ship it')
  })

  it('remembers the flow started for the next form', async () => {
    const memory = { flow: null }
    const first = made(undefined, memory)
    first.flows.remember('Quick')
    const next = made(undefined, memory)
    await next.flows.read(8, 'boot-1')
    expect(memory).toEqual({ flow: 'Quick' })
    expect(next.flows.flow).toBe('Quick')
  })

  it('cannot choose a flow with problems by hand', async () => {
    const { flows } = made()
    await flows.read(8, 'boot-1')
    flows.flow = 'Second opinion'
    expect([flows.chosen, flows.team]).toEqual([undefined, []])
  })

  it('shows the chosen team with the orchestrator first', async () => {
    const { flows } = made()
    await flows.read(8, 'boot-1')
    expect(flows.team).toEqual(['orchestrator · claude', 'coder · codex', 'tester · claude'])
    flows.flow = 'Quick'
    expect(flows.team).toEqual(['orchestrator · claude', 'coder · codex'])
  })
})

describe('FlowStart, after cornercase restarts', () => {
  it('drops what it read and reads the project again', async () => {
    const { flows, calls } = made()
    await flows.read(8, 'boot-1')
    await flows.read(3, 'boot-1')
    await flows.read(3, 'boot-2')
    await flows.read(8, 'boot-2')
    expect(calls.issueFlows.mock.calls).toEqual([[8], [3], [3], [8]])
  })

  it('keeps what it read when the boot is not known', async () => {
    const { flows, calls } = made()
    await flows.read(8, null)
    await flows.read(8, 'boot-1')
    await flows.read(8, null)
    await flows.read(3, 'boot-1')
    await flows.read(8, 'boot-1')
    expect(calls.issueFlows.mock.calls).toEqual([[8], [3]])
  })
})
