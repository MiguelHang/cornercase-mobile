import { describe, expect, it, vi } from 'vitest'
import type {
  CommandError,
  IssueInfo_Serialize as IssueInfo,
  ProjectChoice,
  StartChoices_Serialize as StartChoices,
  Started_Serialize as Started,
  WorkspaceChoice,
} from './bindings'
import {
  AGENT_EMPTY,
  AGENT_UNANSWERED,
  BRANCH_EMPTY,
  CHOICES_UNANSWERED,
  PROJECT_EMPTY,
  PROMPT_EMPTY,
  StartForm,
  WORKSPACE_EMPTY,
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

const workspace = (id: number, name = `w${id}`): WorkspaceChoice => ({ id, name })

const project = (id: number, name = `p${id}`, workspaces: WorkspaceChoice[] = []): ProjectChoice => ({
  id,
  name,
  workspaces,
})

const SESSION = [project(1, 'web', [workspace(11, 'main')]), project(2, 'api', [workspace(21), workspace(22, 'Login')])]

const FORGETFUL: Memory = { agent: null, workspace: null }

const choices = (projects: ProjectChoice[], boot: string | null = 'boot-1'): StartChoices => ({ boot, projects })

const gate = <T>() => {
  let open: (value: T) => void = () => {}
  const promise = new Promise<T>((resolve) => (open = resolve))
  return { promise, open }
}

const calls = (overrides: Partial<StartCalls> = {}) => ({
  startProjects: vi.fn(overrides.startProjects ?? (() => ok(choices(SESSION)))),
  installedAgents: vi.fn(overrides.installedAgents ?? (() => ok([agent('claude'), agent('codex')]))),
  startAgent: vi.fn(overrides.startAgent ?? (() => ok(STARTED))),
  startIssue: vi.fn(overrides.startIssue ?? (() => ok(STARTED))),
})

const made = (overrides: Partial<StartCalls> = {}, memory: Memory = { ...FORGETFUL }) => {
  const onstarted = vi.fn()
  const commands = calls(overrides)
  const form = new StartForm(commands, onstarted, memory)
  return { form, onstarted, commands, memory }
}

const loaded = async (overrides: Partial<StartCalls> = {}, memory: Memory = { ...FORGETFUL }) => {
  const built = made(overrides, memory)
  await built.form.load()
  return built
}

const filled = async (overrides: Partial<StartCalls> = {}) => {
  const built = await loaded(overrides)
  built.form.project = 2
  built.form.workspace = 22
  built.form.model = ' opus '
  built.form.prompt = 'fix the login\nand its tests'
  built.form.branch = 'fix-login'
  return built
}

const TICKET: IssueInfo = {
  source: 'shortcut',
  key: 'sc-7',
  title: 'Refunds round the wrong way',
  url: 'https://app.shortcut.com/acme/story/7',
  state: 'In Progress',
  closed: false,
  labels: [],
  author: 'ada',
  updated_at: '2026-11-08T17:02:11Z',
  branch: 'ada/sc-7/refunds',
}

const fromIssue = async (overrides: Partial<StartCalls> = {}, memory: Memory = { ...FORGETFUL }) => {
  const built = made(overrides, memory)
  built.form.forIssue({ project: 9, issue: TICKET, prompt: 'Work on sc-7: Refunds round the wrong way' })
  await built.form.load()
  return built
}

const issueFields = (form: StartForm) => [form.project, form.agent, form.model, form.branch]

const typed = (form: StartForm) => ({
  project: form.project,
  workspace: form.workspace,
  agent: form.agent,
  model: form.model,
  prompt: form.prompt,
  branch: form.branch,
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
    expect(form.projects).toEqual({ kind: 'ready', items: SESSION })
    expect(form.agents).toEqual({ kind: 'ready', items: [agent('claude'), agent('codex')] })
    expect([form.project, form.agent]).toEqual([null, 'claude'])
  })

  it('picks the only project', async () => {
    const { form } = await loaded({ startProjects: () => ok(choices([project(5)])) })
    expect(form.project).toBe(5)
  })

  it('picks the agent last started in the app while it is still installed', async () => {
    const { form } = await loaded({}, { agent: 'codex', workspace: null })
    const gone = await loaded({}, { agent: 'gemini', workspace: null })
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

describe('StartForm choosing a workspace', () => {
  const ONE = [project(1, 'web', [workspace(11, 'main')]), project(3)]

  it('lists every workspace as its project › its name, in the order of the session', async () => {
    const { form } = await loaded()
    expect(form.workspaceChoices).toEqual({
      kind: 'ready',
      items: [
        { id: 11, project: 1, label: 'web › main' },
        { id: 21, project: 2, label: 'api › w21' },
        { id: 22, project: 2, label: 'api › Login' },
      ],
    })
  })

  it('reads and fails with the projects', async () => {
    const { form } = made({ startProjects: () => failed('offline') })
    expect(form.workspaceChoices).toEqual({ kind: 'loading' })
    await form.load()
    expect(form.workspaceChoices).toEqual({ kind: 'failed', reason: 'offline' })
  })

  it.each<[string, StartCalls['startProjects'], boolean]>([
    ['projects without workspaces', () => ok(choices([project(1), project(2)])), true],
    ['some workspace', () => ok(choices(SESSION)), false],
    ['projects that could not be read', () => failed('offline'), false],
  ])('has no workspaces with %s', async (_, startProjects, none) => {
    const { form } = made({ startProjects })
    expect(form.noWorkspaces).toBe(false)
    await form.load()
    expect(form.noWorkspaces).toBe(none)
  })

  it.each<[string, ProjectChoice[], number | null, number | null, number | null]>([
    ['the one of whoever opened it, before the last used', SESSION, 22, 21, 21],
    ['the last used while it is still in the session', SESSION, 22, null, 22],
    ['the last used when the one of whoever opened it is gone', SESSION, 22, 99, 22],
    ['the only one when the last used is gone', ONE, 22, null, 11],
    ['the only one with none used before', ONE, null, null, 11],
    ['none among several with none used before', SESSION, null, null, null],
    ['none without workspaces', [project(1), project(2)], 22, 21, null],
  ])('picks %s', async (_, session, last, preset, picked) => {
    const { form } = made({ startProjects: () => ok(choices(session)) }, { agent: null, workspace: last })
    if (preset !== null) {
      form.prefer(preset)
    }
    await form.load()
    expect(form.workspace).toBe(picked)
  })

  it('takes the one of whoever opened it once the choices are read, unless it is gone', async () => {
    const { form } = await loaded({ startProjects: () => ok(choices(ONE)) })
    form.prefer(99)
    expect(form.workspace).toBe(11)
    const { form: several } = await loaded()
    several.prefer(21)
    expect(several.workspace).toBe(21)
  })

  it('empties a workspace that is gone after the server restarted, and keeps one still there', async () => {
    const answers = [ok(choices(SESSION)), ok(choices([project(2, 'api', [workspace(21), workspace(23)])], 'boot-2'))]
    const { form } = await loaded({ startProjects: () => answers.shift() ?? ok(choices(SESSION)) })
    form.workspace = 22
    await form.see(false)
    await form.see(true)
    expect(form.workspace).toBeNull()
    form.workspace = 21
    await form.see(false)
    await form.see(true)
    expect(form.workspace).toBe(21)
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
    expect(typed(form)).toEqual({ ...before, project: null, workspace: null })
  })

  it('keeps a project that is still there', async () => {
    const answers = [ok(choices([project(1), project(2)])), ok(choices([project(2)], 'boot-2'))]
    const { form } = await filled({ startProjects: () => answers.shift() ?? ok(choices([])) })
    await reconnect(form)
    expect(form.project).toBe(2)
  })
})

describe('StartForm start', () => {
  it('sends the agent to the chosen workspace and hands over what started, remembering both', async () => {
    const { form, commands, onstarted, memory } = await filled()
    form.agent = 'codex'
    await form.start()
    expect(commands.startAgent.mock.calls).toEqual([
      [
        {
          place: { kind: 'workspace', workspace: 22 },
          agent: 'codex',
          model: ' opus ',
          prompt: 'fix the login\nand its tests',
        },
      ],
    ])
    expect(onstarted.mock.calls).toEqual([[STARTED, 'codex']])
    expect(memory).toEqual({ agent: 'codex', workspace: 22 })
  })

  it("sends the agent to its own worktree on the branch, in the workspace's project", async () => {
    const { form, commands, memory } = await filled()
    form.workspace = 11
    form.ownWorktree = true
    await form.start()
    expect(commands.startAgent.mock.calls[0][0].place).toEqual({ kind: 'worktree', project: 1, branch: 'fix-login' })
    expect(memory.workspace).toBe(11)
  })

  type Refusal = 'workspaceError' | 'agentError' | 'promptError' | 'branchError'

  const ownWorktreeOn = (branch: string) => (form: StartForm) => {
    form.ownWorktree = true
    form.branch = branch
  }

  it.each<[string, (form: StartForm) => void, Refusal, string]>([
    ['no workspace', (form) => (form.workspace = null), 'workspaceError', WORKSPACE_EMPTY],
    ['a workspace no longer in the session', (form) => (form.workspace = 99), 'workspaceError', WORKSPACE_EMPTY],
    ['no agent', (form) => (form.agent = ''), 'agentError', AGENT_EMPTY],
    ['an empty prompt', (form) => (form.prompt = ''), 'promptError', PROMPT_EMPTY],
    ['a prompt of only spaces', (form) => (form.prompt = ' \n\t '), 'promptError', PROMPT_EMPTY],
    ['its own worktree without a branch', ownWorktreeOn(''), 'branchError', BRANCH_EMPTY],
    ['its own worktree on a branch of spaces', ownWorktreeOn('  '), 'branchError', BRANCH_EMPTY],
  ])('refuses %s under its field without sending', async (_, change, field, error) => {
    const { form, commands } = await filled()
    change(form)
    await form.start()
    expect(form[field]).toBe(error)
    expect(commands.startAgent).not.toHaveBeenCalled()
  })

  it('clears the errors of the fields once they are right', async () => {
    const { form } = await filled({ startAgent: () => failed('refused') })
    form.workspace = null
    form.ownWorktree = true
    form.branch = ''
    await form.start()
    form.workspace = 11
    form.branch = 'fix-login'
    await form.start()
    const errors = [form.workspaceError, form.agentError, form.promptError, form.branchError, form.formError]
    expect(errors).toEqual([null, null, null, null, 'refused'])
  })

  it('needs no branch while the agent works in the workspace', async () => {
    const { form, commands } = await filled()
    form.branch = ''
    await form.start()
    expect([form.branchError, commands.startAgent.mock.calls.length]).toEqual([null, 1])
  })

  it('keeps everything typed and shows what the server says', async () => {
    const { form, onstarted, memory } = await filled({ startAgent: () => failed('the branch exists') })
    const before = typed(form)
    await form.start()
    expect(typed(form)).toEqual(before)
    expect([form.formError, form.sending, memory]).toEqual(['the branch exists', false, FORGETFUL])
    expect(onstarted).not.toHaveBeenCalled()
  })

  it('cannot be sent again while it waits', async () => {
    const answer = gate<{ status: 'ok'; data: Started }>()
    const { form, commands } = await filled({ startAgent: () => answer.promise })
    const first = form.start()
    expect(form.sending).toBe(true)
    await form.start()
    answer.open({ status: 'ok', data: STARTED })
    await first
    expect(commands.startAgent).toHaveBeenCalledTimes(1)
  })

  it('says the agent may have started when the request is left unanswered, and does not try again', async () => {
    const { form, commands } = await filled({ startAgent: () => refused(UNANSWERED) })
    await form.start()
    expect([form.formError, form.sending]).toEqual([AGENT_UNANSWERED, false])
    expect(commands.startAgent).toHaveBeenCalledTimes(1)
  })
})

describe('StartForm choosing a project for its issues', () => {
  it('says to choose a project when none is chosen', async () => {
    const { form, commands } = await loaded()
    expect(form.chooseProject()).toBeNull()
    expect(form.projectError).toBe(PROJECT_EMPTY)
    expect(commands.startIssue).not.toHaveBeenCalled()
  })

  it('gives the chosen project and clears what it said before', async () => {
    const { form } = await loaded()
    form.chooseProject()
    form.project = 2
    expect(form.chooseProject()).toBe(2)
    expect(form.projectError).toBeNull()
  })
})

describe('StartForm from an issue', () => {
  const sent = (branch: string, agent = 'claude', model = '') => ({
    place: { kind: 'worktree', project: 9, branch },
    agent,
    model,
    key: 'sc-7',
    source: 'shortcut',
    issue_branch: 'ada/sc-7/refunds',
  })

  it("keeps the issue's project and fills the branch with the issue's, leaving the model empty", async () => {
    const memory = { agent: 'codex', workspace: null }
    const { form } = await fromIssue({ startProjects: () => ok(choices([project(5)])) }, memory)
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
