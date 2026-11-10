import { describe, expect, it, vi } from 'vitest'
import type {
  CommandError,
  FlowChoice_Serialize as FlowChoice,
  FlowStarted_Serialize as FlowStarted,
  Flows_Serialize as Flows,
  IssueInfo_Serialize as IssueInfo,
  PlaceInfo_Serialize as PlaceInfo,
  StartChoices_Serialize as StartChoices,
} from './bindings'
import { FLOW_EMPTY, FLOW_UNANSWERED, FlowStart, type FlowCalls, type FlowMemory } from './flowStart.svelte'
import type { Where } from './router.svelte'
import { CHOICES_UNANSWERED, PLACE_EMPTY, PLACES_STALE, StartForm, type StartCalls } from './startForm.svelte'

type Answer = Promise<{ status: 'ok'; data: Flows } | { status: 'error'; error: CommandError }>

const reply = <T>(data: T) => Promise.resolve({ status: 'ok' as const, data })

const ok = (flows: FlowChoice[]): Answer => reply({ flows })

const refused = <T = Flows>(error: CommandError): Promise<{ status: 'ok'; data: T } | { status: 'error'; error: CommandError }> =>
  Promise.resolve({ status: 'error', error })

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

const inTurn = <T>(...answers: T[]) => {
  let left = answers
  return () => {
    const [answer, ...rest] = left
    left = rest
    return answer
  }
}

const ORCHESTRATOR: FlowStarted = {
  started: { project: 8, workspace: 9, tab: 14, pane: 11, warning: 'they run without asking, so check them' },
  agent: 'claude',
}

const booted = (boot: string): Promise<{ status: 'ok'; data: StartChoices }> => reply({ boot, projects: [] })

type Calls = Partial<FlowCalls & Pick<StartCalls, 'startProjects'>>

const built = (overrides: Calls, memory: FlowMemory) => {
  const calls = {
    issueFlows: vi.fn(overrides.issueFlows ?? (() => ok(SHOP))),
    startFlow: vi.fn(overrides.startFlow ?? (() => reply(ORCHESTRATOR))),
    startProjects: vi.fn(overrides.startProjects ?? (() => booted('boot-1'))),
    installedAgents: () => reply([]),
    startAgent: vi.fn(),
    startIssue: vi.fn(),
  }
  const onstarted = vi.fn()
  const form = new StartForm(calls, onstarted, { agent: null, workspace: null })
  return { form, flows: new FlowStart(form, calls, memory), calls, onstarted, memory }
}

const made = (issueFlows?: FlowCalls['issueFlows'], memory: FlowMemory = { flow: null }) =>
  built({ issueFlows }, memory)

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

const ISSUE_BRANCH = 'issue-482-login-form-forgets-the-email'

const ISSUE: IssueInfo = {
  source: 'github',
  key: '#482',
  title: 'Login form forgets the email',
  url: 'https://github.com/acme/shop/issues/482',
  state: 'open',
  closed: false,
  labels: [],
  author: 'mara',
  updated_at: '2026-11-09T10:12:00Z',
  branch: ISSUE_BRANCH,
}

const IN_SHOP: PlaceInfo = { project: 8, workspace: null, label: 'shop', worktree: true }
const IN_LOGIN: PlaceInfo = { project: 8, workspace: 22, label: 'shop › Login', worktree: false }
const IN_NOTES: PlaceInfo = { project: 3, workspace: null, label: 'notes', worktree: true }
const FROM_SHOP: Where = { project: 8, places: [IN_LOGIN, IN_SHOP], workspace: 22 }
const FROM_ACCOUNT: Where = { project: null, places: [IN_SHOP, IN_LOGIN, IN_NOTES], workspace: 22 }

const opened = async (overrides: Calls = {}, where: Where = FROM_SHOP) => {
  const made = built(overrides, { flow: null })
  made.form.forIssue({ ...where, issue: ISSUE, prompt: 'Work on #482' })
  await made.form.load()
  await made.flows.look()
  return made
}

const inFlow = async (overrides: Calls = {}, where: Where = FROM_SHOP) => {
  const made = await opened(overrides, where)
  await made.flows.use('flow')
  return made
}

describe('FlowStart, as a way to start an issue', () => {
  it('opens in Agent and offers Flow once the flows of the new worktree s project are read', async () => {
    const answer = gate()
    const { form, flows, calls } = built({ issueFlows: () => answer.promise }, { flow: null })
    form.forIssue({ ...FROM_SHOP, issue: ISSUE, prompt: '' })
    await form.load()
    const look = flows.look()
    expect([flows.mode, flows.offered, form.place]).toEqual(['agent', false, 'workspace-22'])
    answer.open(SHOP)
    await look
    expect([flows.mode, flows.offered, form.place]).toEqual(['agent', true, 'workspace-22'])
    expect(calls.issueFlows.mock.calls).toEqual([[8]])
  })

  it.each<[string, Where, FlowCalls['issueFlows'], boolean, number]>([
    ['the list cannot be read', FROM_SHOP, () => refused({ code: 'failed', message: 'busy' }), true, 1],
    ['the project has no flows', FROM_SHOP, () => ok([]), false, 1],
    ['there is no new worktree', { project: 8, places: [IN_LOGIN], workspace: 22 }, () => ok(SHOP), false, 0],
  ])('offers Flow or not when %s', async (_, where, issueFlows, offered, asked) => {
    const { flows, calls } = await opened({ issueFlows }, where)
    expect(flows.offered).toBe(offered)
    expect(calls.issueFlows).toHaveBeenCalledTimes(asked)
  })

  it('offers only new worktrees in Flow, chosen by the same rule, and gives Agent its place back', async () => {
    const { form, flows } = await opened()
    form.formError = 'refused'
    form.placeError = PLACE_EMPTY
    await flows.use('agent')
    expect([form.place, form.formError]).toEqual(['workspace-22', 'refused'])
    await flows.use('flow')
    expect([flows.mode, form.place, form.formError, form.placeError]).toEqual(['flow', 'project-8', null, null])
    expect(flows.placeChoices).toEqual({ kind: 'ready', items: [IN_SHOP] })
    await flows.use('agent')
    expect([form.place, flows.placeChoices]).toEqual(['workspace-22', { kind: 'ready', items: [IN_LOGIN, IN_SHOP] }])
  })

  it('reads the flows of the project Flow moves to, and chooses again by the rule', async () => {
    const issueFlows = (project: number) => ok(project === 8 ? SHOP : [SECOND, flow('Arena'), QUICK])
    const { form, flows, calls } = await inFlow({ issueFlows }, FROM_ACCOUNT)
    expect([form.place, flows.flow]).toEqual([null, 'Ship it'])
    flows.flow = 'Quick'
    const moved = flows.moveTo('project-3')
    expect([flows.flows, flows.offered]).toEqual([{ kind: 'loading' }, true])
    await moved
    expect([form.place, flows.flow]).toEqual(['project-3', 'Arena'])
    await flows.moveTo('project-8')
    expect(flows.flow).toBe('Ship it')
    expect(calls.issueFlows.mock.calls).toEqual([[8], [3]])
  })

  it('keeps the place chosen in each mode, and reads no flows for a place chosen in Agent', async () => {
    const { form, flows, calls } = await inFlow({}, FROM_ACCOUNT)
    await flows.moveTo('project-3')
    await flows.use('agent')
    await flows.moveTo('project-8')
    await flows.use('flow')
    expect(form.place).toBe('project-3')
    await flows.use('agent')
    expect(form.place).toBe('project-8')
    expect(calls.issueFlows.mock.calls).toEqual([[8], [3]])
  })
})

describe('FlowStart, starting a flow', () => {
  const sent = (name: string, branch: string) => ({
    project: 8,
    name,
    key: '#482',
    source: 'github',
    branch,
    issue_branch: ISSUE_BRANCH,
  })

  it.each([
    ["the issue's branch", ISSUE_BRANCH],
    ['a branch typed over it', 'fix/login-email'],
  ])('sends the chosen flow to a new worktree on %s, opens its orchestrator and remembers it', async (_, branch) => {
    const { form, flows, calls, onstarted, memory } = await inFlow()
    flows.flow = 'Quick'
    form.branch = branch
    await flows.start()
    expect(calls.startFlow.mock.calls).toEqual([[sent('Quick', branch)]])
    expect(onstarted.mock.calls).toEqual([[ORCHESTRATOR.started, 'claude']])
    expect([memory.flow, form.sending, form.formError]).toEqual(['Quick', true, null])
    expect(calls.startIssue).not.toHaveBeenCalled()
  })

  it('refuses no place and no flow under their fields without sending', async () => {
    const { form, flows, calls } = await inFlow({ issueFlows: () => ok([SECOND]) }, FROM_ACCOUNT)
    await flows.start()
    expect([form.placeError, flows.flowError]).toEqual([PLACE_EMPTY, FLOW_EMPTY])
    expect(calls.startFlow).not.toHaveBeenCalled()
  })

  it('clears what it refused once a place is chosen', async () => {
    const { form, flows, calls } = await inFlow({}, FROM_ACCOUNT)
    await flows.start()
    await flows.moveTo('project-8')
    await flows.start()
    expect([form.placeError, flows.flowError]).toEqual([null, null])
    expect(calls.startFlow.mock.calls).toEqual([[sent('Ship it', ISSUE_BRANCH)]])
  })

  it('cannot be sent again while it waits', async () => {
    let open = () => {}
    const waiting = new Promise<{ status: 'ok'; data: FlowStarted }>(
      (resolve) => (open = () => resolve({ status: 'ok', data: ORCHESTRATOR })),
    )
    const { form, flows, calls } = await inFlow({ startFlow: () => waiting })
    const first = flows.start()
    expect(form.sending).toBe(true)
    await flows.start()
    open()
    await first
    expect(calls.startFlow).toHaveBeenCalledTimes(1)
  })

  it.each<[string, CommandError, string]>([
    [
      'the reason cornercase refused it',
      { code: 'failed', message: 'workspace 17 already runs a flow, and a workspace runs one at a time' },
      'workspace 17 already runs a flow, and a workspace runs one at a time',
    ],
    ['that nothing was sent when the queue skipped it', { code: 'not_sent', message: 'nothing was sent' }, 'nothing was sent'],
    [
      'that the flow may have started when the request is left unanswered',
      { code: 'unanswered', message: 'the request was left unanswered' },
      FLOW_UNANSWERED,
    ],
  ])('keeps every field and says %s, without sending again', async (_, error, said) => {
    const { form, flows, calls, onstarted, memory } = await inFlow({ startFlow: () => refused<FlowStarted>(error) })
    await flows.start()
    expect([form.formError, form.sending, form.place, flows.flow, memory.flow]).toEqual([said, false, 'project-8', 'Ship it', null])
    expect(onstarted).not.toHaveBeenCalled()
    expect(calls.startFlow).toHaveBeenCalledTimes(1)
  })

  it('refuses to start after cornercase restarts, and reads the flows again', async () => {
    const { form, flows, calls } = await inFlow({ startProjects: inTurn(booted('boot-1'), booted('boot-2')) })
    await form.refresh()
    await flows.start()
    expect(flows.placeChoices).toEqual({ kind: 'failed', reason: PLACES_STALE })
    expect(calls.startFlow).not.toHaveBeenCalled()
    await flows.look()
    expect(calls.issueFlows.mock.calls).toEqual([[8], [8]])
  })

  it('sends nothing without an issue', async () => {
    const { flows, calls } = made()
    await flows.use('flow')
    await flows.start()
    expect([flows.flowError, calls.issueFlows.mock.calls, calls.startFlow.mock.calls]).toEqual([null, [], []])
  })
})
