import { render } from 'svelte/server'
import { describe, expect, it, vi } from 'vitest'
import comments from '../../../crates/core/tests/fixtures/control/issue-read/v1/comments.json'
import type {
  AgentChoice_Serialize,
  ConnectionView_Serialize,
  Place,
  ReadIssue_Serialize as ReadIssue,
} from '../../lib/bindings'
import type { StartingIssue } from '../../lib/router.svelte'
import { StartForm, type StartCalls } from '../../lib/startForm.svelte'
import AgentForm from './AgentForm.svelte'
import agentSource from './AgentForm.svelte?raw'
import pickerSource from './AgentPicker.svelte?raw'
import issueSource from './IssueForm.svelte?raw'
import projectSource from './ProjectPicker.svelte?raw'
import ShowIssues from './ShowIssues.svelte'
import showSource from './ShowIssues.svelte?raw'
import Start from './Start.svelte'
import startSource from './Start.svelte?raw'
import workspacePickerSource from './WorkspacePicker.svelte?raw'

const CONNECTED: ConnectionView_Serialize = { state: 'connected', route: 'ws://mac:7000' }

const answer = <T>(data: T) => Promise.resolve({ status: 'ok' as const, data })

const never = () => new Promise<never>(() => {})

const installed = (...names: string[]): AgentChoice_Serialize[] =>
  names.map((name) => ({ agent: name, command: name, installed: true }))

const CALLS: StartCalls = {
  startProjects: () =>
    answer({
      boot: 'boot-1',
      projects: [
        { id: 1, name: 'web', workspaces: [{ id: 12, name: 'main' }] },
        { id: 2, name: 'api', workspaces: [{ id: 9, name: 'Login' }] },
      ],
    }),
  installedAgents: () => answer(installed('claude', 'codex')),
  startAgent: never,
  startIssue: never,
}

const formOf = async (calls: Partial<StartCalls> = {}) => {
  const form = new StartForm({ ...CALLS, ...calls }, () => {}, { agent: null, workspace: null })
  await form.load()
  return form
}

const refusing = (message: string) => () =>
  Promise.resolve({ status: 'error' as const, error: { code: 'failed' as const, message } })

const READ = comments as ReadIssue

const COUPON: StartingIssue = { project: 2, issue: READ.issue, prompt: READ.prompt }

const screen = (
  form: StartForm,
  connection: ConnectionView_Serialize = CONNECTED,
  issue?: StartingIssue,
  workspace?: number,
) =>
  render(Start, {
    props: {
      connection,
      issue,
      workspace,
      backTo: 'Agents',
      onback: () => {},
      onstarted: () => {},
      onissues: () => {},
      form,
    },
  }).body

const onIssue = (form: StartForm, connection: ConnectionView_Serialize = CONNECTED) =>
  screen(form, connection, COUPON)

const showIssues = (form: StartForm, sendable = true) =>
  render(ShowIssues, { props: { form, sendable, onshow: () => {} } }).body

const agentForm = (form: StartForm, sendable = true) => render(AgentForm, { props: { form, sendable } }).body

const LABEL = /<label class="label[^"]*" for="[^"]+">([^<]+)<\/label>/g

const labels = (body: string) => [...body.matchAll(LABEL)].map(([, text]) => text)

const errorOf = (body: string, label: string) =>
  body.match(new RegExp(`>${label}</label>[^]*?<p class="error[^"]*" id="[^"]+">([^<]*)</p>`))?.[1]

const SUBMIT = /<button class="button primary[^"]*" type="submit"([^>]*)>([^]*?)<\/button>/

const submit = (body: string) => body.match(SUBMIT)

const label = (button: RegExpMatchArray | null) => button?.[2].replace(/<[^>]*>|\s/g, '')

const filled = async (calls: Partial<StartCalls> = {}) => {
  const form = await formOf(calls)
  form.project = 2
  form.workspace = 9
  form.prompt = 'fix the login'
  return form
}

const flat = (text: string) => text.replace(/\s+/g, ' ')

describe('Start', () => {
  it('goes back to the agents from a bar with its title, and starts on Agent, with Issue as the only other action', async () => {
    const body = screen(await formOf())
    expect(body).toMatch(/<div class="bar ios svelte-[\w-]+ fixed topmost"[^]*?aria-label="Back to Agents"/)
    expect(body).toMatch(/<div class="center svelte-[\w-]+">(<!--[^>]*-->)*<h1 class="title svelte-[\w-]+">Start<\/h1>/)
    expect(body).not.toContain('class="top')
    expect(body).toMatch(
      /<div class="actions[^"]*" role="group" aria-label="What to start"><!--\[-->(<!--[^>]*-->)?<button[^>]*aria-pressed="true">Agent<\/button>(<!--[^>]*-->)?<button[^>]*aria-pressed="false">Issue<\/button>(<!--[^>]*-->)?<!--]--><\/div>/,
    )
    expect(labels(body)).toEqual(['Workspace', 'Agent', 'Model', 'Prompt'])
  })

  it('shows the projects and the agents loading while the rest can be filled', () => {
    const body = screen(new StartForm(CALLS, () => {}, { agent: null, workspace: null }))
    const reading = [...body.matchAll(/<p class="state[^"]*"><span class="spinner[^]*?<\/span>(?:<!---->)?([^<]*)</g)]
    expect(reading.map(([, text]) => text)).toEqual(['Reading the workspaces…', 'Reading the agents…'])
    expect(labels(body)).toEqual(['Model', 'Prompt'])
  })

  it('holds Start and says why while not connected', async () => {
    const form = await formOf()
    const body = screen(form, { state: 'connecting' })
    expect(body).toContain('Not connected to the Mac. What you type stays here until it connects.')
    expect(submit(body)?.[1]).toContain('disabled=""')
    expect(screen(form)).not.toContain('Not connected')
    expect(submit(screen(form))?.[1]).not.toContain('disabled')
  })
})

describe('Start, its agent form', () => {
  it('lists the workspaces to choose from under their projects, and the agents with the first one chosen', async () => {
    const body = agentForm(await formOf())
    expect(body).toMatch(
      /<option disabled="" selected="">Choose…<\/option>[^]*?<option value="12">web › main<\/option><option value="9">api › Login</,
    )
    expect(body).toMatch(/<option value="claude" selected="">claude<\/option><option value="codex">codex<\/option>/)
  })

  it('chooses the workspace of the agent it was opened from', async () => {
    const form = await formOf()
    const body = screen(form, CONNECTED, undefined, 9)
    expect(form.workspace).toBe(9)
    expect(body).toMatch(/<option value="12">web › main<\/option><option value="9" selected="">api › Login</)
  })

  it('chooses the only workspace', async () => {
    const projects = [
      { id: 5, name: 'web', workspaces: [{ id: 12, name: 'main' }] },
      { id: 2, name: 'api', workspaces: [] },
    ]
    const body = agentForm(await formOf({ startProjects: () => answer({ boot: null, projects }) }))
    expect(body).toMatch(/<select[^>]*>(?:<!--[^>]*-->)*<option value="12" selected="">web › main<\/option>/)
  })

  it('says why the agents could not be read, with Try again', async () => {
    const body = agentForm(await formOf({ installedAgents: refusing('account list failed') }))
    expect(body).toMatch(/<p class="label[^"]*">Agent<\/p>/)
    expect(body).toMatch(
      /<p class="state reason[^"]*">account list failed<\/p> <button class="button plain[^"]*"[^>]*>[^]*?Try again/,
    )
  })

  it('says when no agent is installed and holds Start', async () => {
    const body = agentForm(await formOf({ installedAgents: () => answer([]) }))
    expect(body).toContain('No agent is installed on this Mac.')
    expect(submit(body)?.[1]).toContain('disabled=""')
  })

  it('tells each refused field under it', async () => {
    const form = await formOf()
    form.agent = ''
    form.prompt = '   '
    await form.start()
    const body = agentForm(form)
    expect(['Workspace', 'Agent', 'Prompt', 'Model'].map((field) => errorOf(body, field))).toEqual([
      'Choose a workspace',
      'Choose an agent',
      'Write what the agent should do',
      '',
    ])
  })

  it('turns the button into a spinner with Starting… and holds the fields while it sends', async () => {
    const form = await filled()
    void form.start()
    const body = agentForm(form)
    expect(submit(body)?.[1]).toContain('aria-busy="true"')
    expect(submit(body)?.[2]).toContain('class="spinner')
    expect(label(submit(body))).toBe('Starting…')
    expect(body.match(/<(?:input|textarea|select)[^>]*disabled=""/g)).toHaveLength(5)
  })

  it('shows what the server says and keeps what was typed', async () => {
    const form = await filled({ startAgent: refusing('the branch exists') })
    await form.start()
    const body = agentForm(form)
    expect(body).toMatch(/<p class="problem[^"]*" role="alert">the branch exists<\/p>/)
    expect(body).toContain('>fix the login</textarea>')
    expect(label(submit(body))).toBe('Start')
  })
})

describe('Start, where its agent works', () => {
  const WORKTREE: Place = { kind: 'worktree', project: 2, branch: 'fix-login' }

  const note = (body: string) => flat(body).match(/<p class="note[^"]*">([^<]*)<\/p>/)?.[1]

  it('starts in a new tab in the workspace with the Own worktree switch off', async () => {
    const body = agentForm(await formOf())
    expect(body).toMatch(/">Own worktree<\/span> <input class="toggle[^"]*" type="checkbox" role="switch"\/>/)
    expect(labels(body)).toEqual(['Workspace', 'Agent', 'Model', 'Prompt'])
    expect(note(body)).toBe('The agent starts in a new tab in this workspace.')
  })

  it('asks for a branch once the switch is on, and says where the agent works', async () => {
    const form = await formOf()
    form.ownWorktree = true
    form.branch = 'fix-login'
    const body = agentForm(form)
    expect(body).toMatch(/role="switch" checked=""\/>/)
    expect(labels(body)).toEqual(['Workspace', 'Branch', 'Agent', 'Model', 'Prompt'])
    expect(body).toContain('value="fix-login"')
    expect(note(body)).toBe("The agent works in its own worktree on this branch, made from the workspace's project.")
  })

  it('tells a missing branch under it', async () => {
    const form = await filled()
    form.ownWorktree = true
    await form.start()
    expect(errorOf(agentForm(form), 'Branch')).toBe('Name the branch')
  })

  it.each<[string, boolean, string, Place]>([
    ['the workspace with the switch off', false, '', { kind: 'workspace', workspace: 9 }],
    ["a worktree of the workspace's project with it on", true, 'fix-login', WORKTREE],
  ])('sends %s', async (_, own, branch, place) => {
    const startAgent = vi.fn(never)
    const form = await filled({ startAgent })
    form.ownWorktree = own
    form.branch = branch
    void form.start()
    expect(startAgent.mock.calls).toEqual([[{ place, agent: 'claude', model: '', prompt: 'fix the login' }]])
  })

  it('says no workspace is open and holds Start without any', async () => {
    const projects = [{ id: 1, name: 'web', workspaces: [] }]
    const body = agentForm(await formOf({ startProjects: () => answer({ boot: null, projects }) }))
    expect(body).toContain('No workspace is open on this Mac.')
    expect(submit(body)?.[1]).toContain('disabled=""')
  })
})

describe('Start, its way to the issues', () => {
  it('chooses the project and offers Show issues', async () => {
    const body = showIssues(await formOf())
    expect(labels(body)).toEqual(['Project'])
    expect(body).toContain("Pick one of the project's issues to start an agent on it.")
    expect(label(submit(body))).toBe('Showissues')
    expect(submit(body)?.[1]).not.toContain('disabled')
  })

  it('says to choose a project instead of showing the issues of none', async () => {
    const form = await formOf()
    form.chooseProject()
    expect(errorOf(showIssues(form), 'Project')).toBe('Choose a project')
    expect(flat(showSource)).toContain(
      'const show = () => { const project = form.chooseProject() if (project !== null) { onshow(project) } }',
    )
  })

  it('holds Show issues while not connected', async () => {
    expect(submit(showIssues(await formOf(), false))?.[1]).toContain('disabled=""')
  })
})

describe('Start, on an issue', () => {
  it('shows only the issue form, with no other action to switch to and no prompt to write', async () => {
    const body = onIssue(await formOf())
    expect(body).not.toContain('aria-label="What to start"')
    expect(body).not.toContain('Show issues')
    expect(labels(body)).toEqual(['Agent', 'Model', 'Branch'])
    expect(body).not.toContain('<textarea')
    expect(label(submit(body))).toBe('Start')
  })

  it("heads the form with the issue's key, its project and its title", async () => {
    const body = onIssue(await formOf())
    expect(body).toMatch(/<span class="key[^"]*">#41<\/span> (?:<!--[^>]*-->)*<span class="project[^"]*">api<\/span>/)
    expect(body).toMatch(/<p class="name[^"]*">Checkout forgets the coupon after a refresh<\/p>/)
  })

  it("fills the branch with the issue's and says where the agent works", async () => {
    const body = flat(onIssue(await formOf()))
    expect(body).toMatch(/placeholder="issue-41-checkout-forgets-the-coupon"[^>]*value="issue-41-checkout-forgets-the-coupon"/)
    expect(body.match(/>Branch<\/label>[^]*<p class="note[^"]*">([^<]*)<\/p>/)?.[1]).toBe(
      "The agent works in its own worktree on this branch. If the project's folder is not the root of a git repository, it starts in a new tab there instead.",
    )
  })

  it('folds the prompt cornercase will use, to read and not to change', async () => {
    const body = onIssue(await formOf())
    expect(body).toMatch(
      /<details class="prompt[^"]*"><summary class="summary[^"]*">Prompt cornercase will use<\/summary> <p class="said[^"]*">https:\/\/github.com\/acme\/shop\/issues\/41<\/p><\/details>/,
    )
  })

  it('turns the button into a spinner with Starting… and holds the fields while it sends', async () => {
    const form = await formOf()
    const body = () => screen(form)
    onIssue(form)
    void form.startIssue()
    expect(submit(body())?.[1]).toContain('aria-busy="true"')
    expect(label(submit(body()))).toBe('Starting…')
    expect(body().match(/<(?:input|select)[^>]*disabled=""/g)).toHaveLength(3)
  })

  it('shows what the server says and keeps the branch typed', async () => {
    const form = await formOf({ startIssue: refusing('no issue #41 in GitHub') })
    onIssue(form)
    form.branch = 'coupon-fix'
    await form.startIssue()
    const body = screen(form)
    expect(body).toMatch(/<p class="problem[^"]*" role="alert">no issue #41 in GitHub<\/p>/)
    expect(body).toContain('value="coupon-fix"')
    expect(label(submit(body))).toBe('Start')
  })

  it('holds Start while not connected', async () => {
    const body = onIssue(await formOf(), { state: 'connecting' })
    expect(body).toContain('Not connected to the Mac.')
    expect(submit(body)?.[1]).toContain('disabled=""')
  })

  it('hands over the agent it started on the issue', async () => {
    const onstarted = vi.fn()
    const started = { project: 2, workspace: 3, tab: 4, pane: 5, warning: null }
    const form = new StartForm({ ...CALLS, startIssue: () => answer(started) }, onstarted, {
      agent: null,
      workspace: null,
    })
    await form.load()
    onIssue(form)
    await form.startIssue()
    expect(onstarted.mock.calls).toEqual([[started, 'claude']])
  })
})

describe('Start wiring', () => {
  it('reads the choices on opening and again when the connection comes back', () => {
    const script = flat(startSource)
    expect(script).toContain('onMount(() => { void form.load() })')
    expect(script).toContain("const sendable = $derived(connection.state === 'connected')")
    expect(script).toContain('$effect(() => { const connected = sendable untrack(() => void form.see(connected)) })')
  })

  it('hands over what started, goes back on Back and switches between the agent form and the issues', () => {
    const script = flat(startSource)
    expect(script).toContain('form = new StartForm(commands, (started, agent) => onstarted(started, agent)),')
    expect(script).toContain('<NavBar {backTo} title="Start" {onback} topmost />')
    expect(script).toContain('onclick={() => (action = name)}')
    expect(script).toContain("{:else if action === 'agent'} <AgentForm {form} {sendable} />")
    expect(script).toContain('{:else} <ShowIssues {form} {sendable} onshow={onissues} />')
  })

  it('turns into the issue form when it opens on an issue', () => {
    const script = flat(startSource)
    expect(script).toContain(
      'untrack(() => { if (issue !== undefined) { form.forIssue(issue) } if (workspace !== undefined) { form.prefer(workspace) } })',
    )
    expect(script).toContain('{#if form.issue === null} <div class="actions"')
    expect(script).toContain('{#if form.issue !== null} <IssueForm {form} {sendable} />')
  })

  it('sends each form with its own action', () => {
    expect(flat(agentSource)).toContain('void form.start()')
    expect(flat(issueSource)).toContain('onsubmit={() => void form.startIssue()}')
  })

  it('reads again what failed on Try again', () => {
    expect(flat(pickerSource)).toContain('onretry={() => void form.loadAgents()}')
    expect(flat(projectSource)).toContain('onretry={() => void form.loadProjects()}')
    expect(flat(workspacePickerSource)).toContain('onretry={() => void form.loadProjects()}')
  })
})
