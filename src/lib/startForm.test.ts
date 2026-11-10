import { describe, expect, it, vi } from 'vitest'
import type {
  CommandError,
  IssueInfo_Serialize as IssueInfo,
  ProjectChoice,
  StartChoices_Serialize as StartChoices,
  Started_Serialize as Started,
} from './bindings'
import {
  AGENT_EMPTY,
  AGENT_UNANSWERED,
  CHOICES_UNANSWERED,
  NAME_EMPTY,
  PROJECT_EMPTY,
  PROMPT_EMPTY,
  StartForm,
  WORKSPACE_UNANSWERED,
  ask,
  type Memory,
  type StartCalls,
} from './startForm.svelte'

const STARTED: Started = { project: 1, workspace: 2, tab: 3, pane: 4, warning: null }

const ok = <T>(data: T) => Promise.resolve({ status: 'ok' as const, data })

const refused = (error: CommandError) => Promise.resolve({ status: 'error' as const, error })

const failed = (message: string) => refused({ code: 'failed', message })

const UNANSWERED: CommandError = { code: 'unanswered', message: 'the request was left unanswered' }

const agent = (name: string) => ({ agent: name, command: name, installed: true })

const project = (id: number, name = `p${id}`): ProjectChoice => ({ id, name })

const choices = (projects: ProjectChoice[], boot: string | null = 'boot-1'): StartChoices => ({ boot, projects })

const gate = <T>() => {
  let open: (value: T) => void = () => {}
  const promise = new Promise<T>((resolve) => (open = resolve))
  return { promise, open }
}

const calls = (overrides: Partial<StartCalls> = {}) => ({
  startProjects: vi.fn(overrides.startProjects ?? (() => ok(choices([project(1), project(2)])))),
  installedAgents: vi.fn(overrides.installedAgents ?? (() => ok([agent('claude'), agent('codex')]))),
  startAgent: vi.fn(overrides.startAgent ?? (() => ok(STARTED))),
  startIssue: vi.fn(overrides.startIssue ?? (() => ok(STARTED))),
  newWorkspace: vi.fn(overrides.newWorkspace ?? (() => ok(STARTED))),
})

const made = (overrides: Partial<StartCalls> = {}, memory: Memory = { agent: null }) => {
  const onstarted = vi.fn()
  const oncreated = vi.fn()
  const commands = calls(overrides)
  const form = new StartForm(commands, onstarted, oncreated, memory)
  return { form, onstarted, oncreated, commands, memory }
}

const loaded = async (overrides: Partial<StartCalls> = {}, memory: Memory = { agent: null }) => {
  const built = made(overrides, memory)
  await built.form.load()
  return built
}

const filled = async (overrides: Partial<StartCalls> = {}) => {
  const built = await loaded(overrides)
  built.form.project = 2
  built.form.model = ' opus '
  built.form.prompt = 'fix the login\nand its tests'
  built.form.worktree = 'fix-login'
  built.form.name = 'login'
  return built
}

const TICKET: IssueInfo = {
  source: 'shortcut',
  key: 'sc-7',
  title: 'Refunds round the wrong way',
  state: 'In Progress',
  closed: false,
  labels: [],
  author: 'ada',
  updated_at: '2026-11-08T17:02:11Z',
  branch: 'ada/sc-7/refunds',
}

const fromIssue = async (overrides: Partial<StartCalls> = {}, memory: Memory = { agent: null }) => {
  const built = made(overrides, memory)
  built.form.forIssue({ project: 9, issue: TICKET, prompt: 'Work on sc-7: Refunds round the wrong way' })
  await built.form.load()
  return built
}

const issueFields = (form: StartForm) => [form.project, form.agent, form.model, form.branch]

const typed = (form: StartForm) => ({
  project: form.project,
  agent: form.agent,
  model: form.model,
  prompt: form.prompt,
  worktree: form.worktree,
  name: form.name,
  ownWorktree: form.ownWorktree,
})

describe('ask', () => {
  it('hands the answer over', async () => {
    expect(await ask(() => ok(7), AGENT_UNANSWERED)).toEqual({ ok: true, data: 7 })
  })

  it.each<[string, () => Promise<never>, string]>([
    ['what the server says', () => failed('not a repository') as Promise<never>, 'not a repository'],
    ['its own text for an unanswered request', () => refused(UNANSWERED) as Promise<never>, AGENT_UNANSWERED],
    ['the message of a rejected call', () => Promise.reject(new Error('IPC is gone')), 'IPC is gone'],
  ])('tells %s', async (_, call, error) => {
    expect(await ask(call, AGENT_UNANSWERED)).toEqual({ ok: false, error })
  })
})

describe('StartForm choices', () => {
  it('lists the projects and the installed agents, and picks the first agent', async () => {
    const { form } = await loaded()
    expect(form.projects).toEqual({ kind: 'ready', items: [project(1), project(2)] })
    expect(form.agents).toEqual({ kind: 'ready', items: [agent('claude'), agent('codex')] })
    expect([form.project, form.agent]).toEqual([null, 'claude'])
  })

  it('picks the only project', async () => {
    const { form } = await loaded({ startProjects: () => ok(choices([project(5)])) })
    expect(form.project).toBe(5)
  })

  it('picks the agent last started in the app while it is still installed', async () => {
    const { form } = await loaded({}, { agent: 'codex' })
    const gone = await loaded({}, { agent: 'gemini' })
    expect([form.agent, gone.form.agent]).toEqual(['codex', 'claude'])
  })

  it('shows the projects while the agents are still loading', async () => {
    const agents = gate<{ status: 'ok'; data: ReturnType<typeof agent>[] }>()
    const { form } = made({ installedAgents: () => agents.promise })
    const loading = form.load()
    await vi.waitFor(() => expect(form.projects.kind).toBe('ready'))
    expect(form.agents.kind).toBe('loading')
    agents.open({ status: 'ok', data: [agent('claude')] })
    await loading
    expect(form.agents.kind).toBe('ready')
  })

  it('says why the agents could not be read and reads them again on Try again', async () => {
    const answers = [failed('account list failed'), ok([agent('claude')])]
    const { form } = await loaded({ installedAgents: () => answers.shift() ?? ok([]) })
    expect(form.agents).toEqual({ kind: 'failed', reason: 'account list failed' })
    await form.loadAgents()
    expect([form.agents.kind, form.agent]).toEqual(['ready', 'claude'])
  })

  it('says why the projects could not be read', async () => {
    const { form } = await loaded({ startProjects: () => refused({ code: 'not_connected', message: 'offline' }) })
    expect(form.projects).toEqual({ kind: 'failed', reason: 'offline' })
  })

  it('shows the projects loading again on Try again', async () => {
    const again = gate<{ status: 'ok'; data: StartChoices }>()
    const answers = [failed('offline'), again.promise]
    const { form } = await loaded({ startProjects: () => answers.shift() ?? ok(choices([])) })
    const reading = form.loadProjects()
    expect(form.projects.kind).toBe('loading')
    again.open({ status: 'ok', data: choices([project(1)]) })
    await reading
    expect(form.projects).toEqual({ kind: 'ready', items: [project(1)] })
  })

  it.each<[string, CommandError, string]>([
    ['its own text when cornercase does not answer', UNANSWERED, CHOICES_UNANSWERED],
    ['what the app says when nothing was sent', { code: 'not_sent', message: 'nothing was sent' }, 'nothing was sent'],
  ])('tells %s about the agents', async (_, error, reason) => {
    const { form } = await loaded({ installedAgents: () => refused(error) })
    expect(form.agents).toEqual({ kind: 'failed', reason })
  })

  it('knows when no agent is installed and refuses to start', async () => {
    const { form, commands } = await loaded({ installedAgents: () => ok([]) })
    form.project = 1
    form.prompt = 'fix it'
    await form.start()
    expect([form.noAgents, form.agent, form.agentError]).toEqual([true, '', AGENT_EMPTY])
    expect(commands.startAgent).not.toHaveBeenCalled()
  })

  it('keeps only the latest answer of the agents', async () => {
    const first = gate<{ status: 'ok'; data: ReturnType<typeof agent>[] }>()
    const answers = [first.promise, ok([agent('codex')])]
    const { form } = made({ installedAgents: () => answers.shift() ?? ok([]) })
    const stale = form.loadAgents()
    await form.loadAgents()
    first.open({ status: 'ok', data: [agent('claude')] })
    await stale
    expect([form.agents, form.agent]).toEqual([{ kind: 'ready', items: [agent('codex')] }, 'codex'])
  })
})

describe('StartForm when the connection comes back', () => {
  const reconnect = async (form: StartForm) => {
    await form.see(false)
    await form.see(true)
  }

  const reads = (commands: ReturnType<typeof calls>) => [
    commands.startProjects.mock.calls.length,
    commands.installedAgents.mock.calls.length,
  ]

  it('reads nothing more while it stays connected', async () => {
    const { form, commands } = await loaded()
    await form.see(true)
    await form.see(true)
    expect(reads(commands)).toEqual([1, 1])
  })

  it('reads the projects again, and the agents only when the server restarted', async () => {
    const boots = ['boot-1', 'boot-1', null, 'boot-2']
    const { form, commands } = await loaded({ startProjects: () => ok(choices([project(1)], boots.shift())) })
    await form.see(true)
    await reconnect(form)
    await reconnect(form)
    expect(reads(commands)).toEqual([3, 1])
    await reconnect(form)
    expect(reads(commands)).toEqual([4, 2])
  })

  it('reads them again after a restart while it opened not connected', async () => {
    const boots = ['boot-1', 'boot-2']
    const { form, commands } = await loaded({ startProjects: () => ok(choices([project(1)], boots.shift())) })
    await form.see(false)
    await form.see(true)
    expect(reads(commands)).toEqual([2, 2])
  })

  it('reads the agents again on the same boot when they had failed', async () => {
    const answers = [refused({ code: 'not_connected', message: 'offline' }), ok([agent('claude')])]
    const { form, commands } = await loaded({ installedAgents: () => answers.shift() ?? ok([]) })
    await reconnect(form)
    expect(reads(commands)).toEqual([2, 2])
    expect(form.agents).toEqual({ kind: 'ready', items: [agent('claude')] })
  })

  it('empties only a project that is gone', async () => {
    const answers = [ok(choices([project(1), project(2)])), ok(choices([project(1), project(3)], 'boot-2'))]
    const { form } = await filled({ startProjects: () => answers.shift() ?? ok(choices([])) })
    const before = typed(form)
    await reconnect(form)
    expect(typed(form)).toEqual({ ...before, project: null })
  })

  it('keeps a project that is still there', async () => {
    const answers = [ok(choices([project(1), project(2)])), ok(choices([project(2)], 'boot-2'))]
    const { form } = await filled({ startProjects: () => answers.shift() ?? ok(choices([])) })
    await reconnect(form)
    expect(form.project).toBe(2)
  })
})

describe('StartForm start', () => {
  it('sends the form and hands over what started, remembering the agent', async () => {
    const { form, commands, onstarted, memory } = await filled()
    form.agent = 'codex'
    await form.start()
    expect(commands.startAgent.mock.calls).toEqual([
      [{ project: 2, agent: 'codex', model: ' opus ', prompt: 'fix the login\nand its tests', worktree: 'fix-login' }],
    ])
    expect(onstarted.mock.calls).toEqual([[STARTED, 'codex']])
    expect(memory.agent).toBe('codex')
  })

  it.each<[string, (form: StartForm) => void, 'projectError' | 'agentError' | 'promptError', string]>([
    ['no project', (form) => (form.project = null), 'projectError', PROJECT_EMPTY],
    ['no agent', (form) => (form.agent = ''), 'agentError', AGENT_EMPTY],
    ['an empty prompt', (form) => (form.prompt = ''), 'promptError', PROMPT_EMPTY],
    ['a prompt of only spaces', (form) => (form.prompt = ' \n\t '), 'promptError', PROMPT_EMPTY],
  ])('refuses %s under its field without sending', async (_, change, field, error) => {
    const { form, commands } = await filled()
    change(form)
    await form.start()
    expect(form[field]).toBe(error)
    expect(commands.startAgent).not.toHaveBeenCalled()
  })

  it('clears the errors of the fields once they are right', async () => {
    const { form } = await filled({ startAgent: () => failed('refused') })
    form.project = null
    await form.start()
    form.project = 1
    await form.start()
    const errors = [form.projectError, form.agentError, form.promptError, form.formError]
    expect(errors).toEqual([null, null, null, 'refused'])
  })

  it('keeps everything typed and shows what the server says', async () => {
    const { form, onstarted, memory } = await filled({ startAgent: () => failed('the branch exists') })
    const before = typed(form)
    await form.start()
    expect(typed(form)).toEqual(before)
    expect([form.formError, form.sending, memory.agent]).toEqual(['the branch exists', false, null])
    expect(onstarted).not.toHaveBeenCalled()
  })

  it('cannot be sent again while it waits', async () => {
    const answer = gate<{ status: 'ok'; data: Started }>()
    const { form, commands } = await filled({ startAgent: () => answer.promise })
    const first = form.start()
    expect(form.sending).toBe(true)
    await form.start()
    await form.create()
    answer.open({ status: 'ok', data: STARTED })
    await first
    expect([commands.startAgent.mock.calls.length, commands.newWorkspace.mock.calls.length]).toEqual([1, 0])
  })

  it('says the agent may have started when the request is left unanswered, and does not try again', async () => {
    const { form, commands } = await filled({ startAgent: () => refused(UNANSWERED) })
    await form.start()
    expect([form.formError, form.sending]).toEqual([AGENT_UNANSWERED, false])
    expect(commands.startAgent).toHaveBeenCalledTimes(1)
  })
})

describe('StartForm create', () => {
  it('sends the workspace and hands over what was made with its trimmed name', async () => {
    const { form, commands, oncreated } = await filled()
    form.name = ' -login '
    form.ownWorktree = true
    await form.create()
    expect(commands.newWorkspace.mock.calls).toEqual([[{ project: 2, name: ' -login ', worktree: true }]])
    expect(oncreated.mock.calls).toEqual([['-login']])
  })

  it.each<[string, string]>([
    ['an empty name', ''],
    ['a name of only spaces', '   '],
  ])('refuses %s under its field without sending', async (_, name) => {
    const { form, commands } = await filled()
    form.name = name
    await form.create()
    expect(form.nameError).toBe(NAME_EMPTY)
    expect(commands.newWorkspace).not.toHaveBeenCalled()
  })

  it('refuses no project under its field', async () => {
    const { form, commands } = await filled()
    form.project = null
    await form.create()
    expect(form.projectError).toBe(PROJECT_EMPTY)
    expect(commands.newWorkspace).not.toHaveBeenCalled()
  })

  it('keeps everything typed and shows what the server says', async () => {
    const { form, oncreated } = await filled({ newWorkspace: () => failed('not a git repository') })
    const before = typed(form)
    await form.create()
    expect(typed(form)).toEqual(before)
    expect([form.formError, form.sending]).toEqual(['not a git repository', false])
    expect(oncreated).not.toHaveBeenCalled()
  })

  it('says the workspace may exist when the request is left unanswered', async () => {
    const { form, commands } = await filled({ newWorkspace: () => refused(UNANSWERED) })
    await form.create()
    expect(form.formError).toBe(WORKSPACE_UNANSWERED)
    expect(commands.newWorkspace).toHaveBeenCalledTimes(1)
  })
})

describe('StartForm from an issue', () => {
  const sent = (branch: string, agent = 'claude', model = '') => ({
    project: 9,
    agent,
    model,
    key: 'sc-7',
    source: 'shortcut',
    branch,
    issue_branch: 'ada/sc-7/refunds',
  })

  it("keeps the issue's project and fills the branch with the issue's, leaving the model empty", async () => {
    const { form } = await fromIssue({ startProjects: () => ok(choices([project(5)])) }, { agent: 'codex' })
    expect(issueFields(form)).toEqual([9, 'codex', '', 'ada/sc-7/refunds'])
    expect(form.issueBranch).toBe('ada/sc-7/refunds')
    expect(form.issue?.prompt).toBe('Work on sc-7: Refunds round the wrong way')
  })

  it.each<[string, string, string]>([
    ["the issue's branch", 'ada/sc-7/refunds', 'ada/sc-7/refunds'],
    ['a branch typed over it', 'refunds-fix', 'refunds-fix'],
  ])('sends the issue from its tracker with %s, remembering the agent', async (_, branch, expected) => {
    const { form, commands, onstarted, memory } = await fromIssue()
    form.agent = 'codex'
    form.model = 'gpt-5'
    form.branch = branch
    await form.startIssue()
    expect(commands.startIssue.mock.calls).toEqual([[sent(expected, 'codex', 'gpt-5')]])
    expect(commands.startAgent).not.toHaveBeenCalled()
    expect(onstarted.mock.calls).toEqual([[STARTED, 'codex']])
    expect(memory.agent).toBe('codex')
  })

  it('needs no prompt but refuses no agent under its field without sending', async () => {
    const { form, commands } = await fromIssue()
    form.agent = ''
    await form.startIssue()
    expect([form.agentError, form.promptError]).toEqual([AGENT_EMPTY, null])
    expect(commands.startIssue).not.toHaveBeenCalled()
  })

  it('keeps every field and shows what the server says', async () => {
    const { form, onstarted, memory } = await fromIssue({ startIssue: () => failed('no issue sc-7 in Shortcut') })
    form.branch = 'refunds-fix'
    await form.startIssue()
    expect(issueFields(form)).toEqual([9, 'claude', '', 'refunds-fix'])
    expect([form.formError, form.sending, memory.agent]).toEqual(['no issue sc-7 in Shortcut', false, null])
    expect(onstarted).not.toHaveBeenCalled()
  })

  it('clears what the server said once the agent is right', async () => {
    const answers = [failed('refused'), ok(STARTED)]
    const { form } = await fromIssue({ startIssue: () => answers.shift() ?? ok(STARTED) })
    await form.startIssue()
    await form.startIssue()
    expect([form.agentError, form.formError]).toEqual([null, null])
  })

  it('says the agent may have started when the request is left unanswered, and does not try again', async () => {
    const { form, commands } = await fromIssue({ startIssue: () => refused(UNANSWERED) })
    await form.startIssue()
    expect([form.formError, form.sending]).toEqual([AGENT_UNANSWERED, false])
    expect(commands.startIssue).toHaveBeenCalledTimes(1)
  })

  it('cannot be sent again while it waits', async () => {
    const answer = gate<{ status: 'ok'; data: Started }>()
    const { form, commands } = await fromIssue({ startIssue: () => answer.promise })
    const first = form.startIssue()
    expect(form.sending).toBe(true)
    await form.startIssue()
    await form.start()
    answer.open({ status: 'ok', data: STARTED })
    await first
    expect([commands.startIssue.mock.calls.length, commands.startAgent.mock.calls.length]).toEqual([1, 0])
  })

  it('sends nothing without an issue', async () => {
    const { form, commands } = await filled()
    await form.startIssue()
    expect([form.agentError, form.formError, form.sending]).toEqual([null, null, false])
    expect(commands.startIssue).not.toHaveBeenCalled()
  })
})
