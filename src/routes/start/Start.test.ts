import { render } from 'svelte/server'
import { describe, expect, it } from 'vitest'
import type { AgentChoice_Serialize, ConnectionView_Serialize } from '../../lib/bindings'
import { StartForm, type StartCalls } from '../../lib/startForm.svelte'
import AgentForm from './AgentForm.svelte'
import agentSource from './AgentForm.svelte?raw'
import projectSource from './ProjectPicker.svelte?raw'
import Start from './Start.svelte'
import startSource from './Start.svelte?raw'
import WorkspaceForm from './WorkspaceForm.svelte'
import workspaceSource from './WorkspaceForm.svelte?raw'

const CONNECTED: ConnectionView_Serialize = { state: 'connected', route: 'ws://mac:7000' }

const answer = <T>(data: T) => Promise.resolve({ status: 'ok' as const, data })

const never = () => new Promise<never>(() => {})

const installed = (...names: string[]): AgentChoice_Serialize[] =>
  names.map((name) => ({ agent: name, command: name, installed: true }))

const CALLS: StartCalls = {
  startProjects: () => answer({ boot: 'boot-1', projects: [{ id: 1, name: 'web' }, { id: 2, name: 'api' }] }),
  installedAgents: () => answer(installed('claude', 'codex')),
  startAgent: never,
  startIssue: never,
  newWorkspace: never,
}

const formOf = async (calls: Partial<StartCalls> = {}) => {
  const form = new StartForm({ ...CALLS, ...calls }, () => {}, () => {}, { agent: null })
  await form.load()
  return form
}

const refusing = (message: string) => () =>
  Promise.resolve({ status: 'error' as const, error: { code: 'failed' as const, message } })

const screen = (form: StartForm, connection: ConnectionView_Serialize = CONNECTED) =>
  render(Start, { props: { connection, backTo: 'Agents', onback: () => {}, onstarted: () => {}, oncreated: () => {}, form } }).body

const agentForm = (form: StartForm, sendable = true) => render(AgentForm, { props: { form, sendable } }).body

const workspaceForm = (form: StartForm, sendable = true) => render(WorkspaceForm, { props: { form, sendable } }).body

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
  form.prompt = 'fix the login'
  form.name = 'login'
  return form
}

const flat = (text: string) => text.replace(/\s+/g, ' ')

describe('Start', () => {
  it('goes back to the agents from a bar with its title, and starts on Agent', async () => {
    const body = screen(await formOf())
    expect(body).toMatch(/<div class="bar ios svelte-[\w-]+ fixed topmost"[^]*?aria-label="Back to Agents"/)
    expect(body).toMatch(/<div class="center svelte-[\w-]+">(<!--[^>]*-->)*<h1 class="title svelte-[\w-]+">Start<\/h1>/)
    expect(body).not.toContain('class="top')
    expect(body).toMatch(/aria-pressed="true">Agent<\/button><button[^>]*aria-pressed="false">Workspace<\/button>/)
    expect(labels(body)).toEqual(['Project', 'Agent', 'Model', 'Prompt', 'Branch'])
  })

  it('says what happens without a branch, under the branch', async () => {
    const note = flat(screen(await formOf())).match(/>Branch<\/label>[^]*<p class="note[^"]*">([^<]*)<\/p>/)?.[1]
    expect(note).toBe(
      "With a branch, the agent works in its own worktree. Without one, it starts in a new tab in the project's folder.",
    )
  })

  it('shows the projects and the agents loading while the rest can be filled', () => {
    const body = screen(new StartForm(CALLS, () => {}, () => {}, { agent: null }))
    const reading = [...body.matchAll(/<p class="state[^"]*"><span class="spinner[^]*?<\/span>(?:<!---->)?([^<]*)</g)]
    expect(reading.map(([, text]) => text)).toEqual(['Reading the projects…', 'Reading the agents…'])
    expect(labels(body)).toEqual(['Model', 'Prompt', 'Branch'])
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
  it('lists the projects to choose from, and the agents with the first one chosen', async () => {
    const body = agentForm(await formOf())
    expect(body).toMatch(
      /<option disabled="" selected="">Choose…<\/option>[^]*?<option value="1">web<\/option><option value="2">api</,
    )
    expect(body).toMatch(/<option value="claude" selected="">claude<\/option><option value="codex">codex<\/option>/)
  })

  it('chooses the only project', async () => {
    const body = agentForm(await formOf({ startProjects: () => answer({ boot: null, projects: [{ id: 5, name: 'web' }] }) }))
    expect(body).toMatch(/<select[^>]*>(?:<!--[^>]*-->)*<option value="5" selected="">web<\/option>/)
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
    expect(['Project', 'Agent', 'Prompt', 'Model'].map((field) => errorOf(body, field))).toEqual([
      'Choose a project',
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

describe('Start, its workspace form', () => {
  it('has the project, the Own worktree switch off, the name and Create workspace', async () => {
    const body = workspaceForm(await formOf())
    expect(labels(body)).toEqual(['Project', 'Name'])
    expect(body).toMatch(/">Own worktree<\/span> <input class="toggle[^"]*" type="checkbox" role="switch"\/>/)
    expect(label(submit(body))).toBe('Createworkspace')
  })

  it('asks for a branch once the switch is on', async () => {
    const form = await formOf()
    form.ownWorktree = true
    const body = workspaceForm(form)
    expect(body).toMatch(/role="switch" checked=""\/>/)
    expect(labels(body)).toEqual(['Project', 'Branch'])
  })

  it('tells a missing name under it', async () => {
    const form = await formOf()
    await form.create()
    expect(errorOf(workspaceForm(form), 'Name')).toBe('Name the workspace')
  })

  it('turns the button into a spinner with Creating… while it sends', async () => {
    const form = await filled()
    void form.create()
    const body = workspaceForm(form)
    expect(submit(body)?.[1]).toContain('aria-busy="true"')
    expect(label(submit(body))).toBe('Creating…')
    expect(body.match(/<(?:input|select)[^>]*disabled=""/g)).toHaveLength(3)
  })

  it('shows what the server says', async () => {
    const form = await filled({ newWorkspace: refusing('not a git repository') })
    await form.create()
    expect(workspaceForm(form)).toMatch(/<p class="problem[^"]*" role="alert">not a git repository<\/p>/)
  })

  it('holds Create workspace while not connected', async () => {
    expect(submit(workspaceForm(await formOf(), false))?.[1]).toContain('disabled=""')
  })
})

describe('Start wiring', () => {
  it('reads the choices on opening and again when the connection comes back', () => {
    const script = flat(startSource)
    expect(script).toContain('onMount(() => { void form.load() })')
    expect(script).toContain("const sendable = $derived(connection.state === 'connected')")
    expect(script).toContain('$effect(() => { const connected = sendable untrack(() => void form.see(connected)) })')
  })

  it('hands over what started or was made, goes back on Back and switches between the two forms', () => {
    const script = flat(startSource)
    expect(script).toContain('(started, agent) => onstarted(started, agent),')
    expect(script).toContain('(name) => oncreated(name),')
    expect(script).toContain('<NavBar {backTo} title="Start" {onback} topmost />')
    expect(script).toContain('onclick={() => (action = name)}')
    expect(script).toContain("{#if action === 'agent'} <AgentForm {form} {sendable} /> {:else}")
    expect(script).toContain('{:else} <WorkspaceForm {form} {sendable} />')
  })

  it('sends each form with its own action', () => {
    expect(flat(agentSource)).toContain('void form.start()')
    expect(flat(workspaceSource)).toContain('void form.create()')
  })

  it('reads again what failed on Try again', () => {
    expect(flat(agentSource)).toContain('onretry={() => void form.loadAgents()}')
    expect(flat(projectSource)).toContain('onretry={() => void form.loadProjects()}')
  })
})
