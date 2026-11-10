import type { Answer } from './addMachine.svelte'
import type {
  AgentChoice_Serialize as AgentChoice,
  CommandError,
  Place,
  PlaceInfo_Serialize as PlaceInfo,
  ProjectChoice,
  StartChoices_Serialize as StartChoices,
  Started_Serialize as Started,
  commands,
} from './bindings'
import type { StartingIssue, Where } from './router.svelte'
import { messageOf } from './toasts'

export type StartCalls = Pick<
  typeof commands,
  'startProjects' | 'installedAgents' | 'startAgent' | 'startIssue'
>

export type Choices<T> = { kind: 'loading' } | { kind: 'failed'; reason: string } | { kind: 'ready'; items: T[] }

export type Memory = { agent: string | null; workspace: number | null }

export type WorkspaceOption = { id: number; project: number; label: string }

export type Reply<T> = { ok: true; data: T } | { ok: false; error: string }

export const WORKSPACE_EMPTY = 'Choose a workspace'
export const PLACE_EMPTY = 'Choose where to start'
export const BRANCH_EMPTY = 'Name the branch'
export const AGENT_EMPTY = 'Choose an agent'
export const PROMPT_EMPTY = 'Write what the agent should do'
export const AGENT_UNANSWERED =
  'cornercase did not answer, so the agent may have started: look at the board before trying again.'
export const CHOICES_UNANSWERED = 'cornercase did not answer.'

const remembered: Memory = { agent: null, workspace: null }

const reasonOf = (error: CommandError, unanswered: string) =>
  error.code === 'unanswered' ? unanswered : error.message

export const ask = async <T>(call: () => Promise<Answer<T>>, unanswered: string): Promise<Reply<T>> => {
  try {
    const answer = await call()
    if (answer.status === 'ok') {
      return { ok: true, data: answer.data }
    }
    return { ok: false, error: reasonOf(answer.error, unanswered) }
  } catch (error) {
    return { ok: false, error: messageOf(error) }
  }
}

const choicesOf = <T>(reply: Reply<T[]>): Choices<T> =>
  reply.ok ? { kind: 'ready', items: reply.data } : { kind: 'failed', reason: reply.error }

const projectsOf = (reply: Reply<StartChoices>): Reply<ProjectChoice[]> =>
  reply.ok ? { ok: true, data: reply.data.projects } : reply

const errorIf = (empty: boolean, error: string) => (empty ? error : null)

const worktreeOf = (project: number, branch: string): Place => ({ kind: 'worktree', project, branch })

export const placeKey = (place: PlaceInfo) =>
  place.workspace === null ? `project-${place.project}` : `workspace-${place.workspace}`

const placeOf = (place: PlaceInfo, branch: string): Place =>
  place.workspace === null ? worktreeOf(place.project, branch) : { kind: 'workspace', workspace: place.workspace }

const startedIn = ({ places, project, workspace }: Where) => {
  const opened = places.find((place) => place.workspace !== null && place.workspace === workspace)
  if (opened !== undefined || project !== null) {
    return opened ?? places.find((place) => place.worktree && place.project === project)
  }
  return places.length === 1 ? places[0] : undefined
}

const optionsOf = (projects: ProjectChoice[]): WorkspaceOption[] =>
  projects.flatMap((project) =>
    project.workspaces.map((workspace) => ({
      id: workspace.id,
      project: project.id,
      label: `${project.name} › ${workspace.name}`,
    })),
  )

export class StartForm {
  projects = $state<Choices<ProjectChoice>>({ kind: 'loading' })
  agents = $state<Choices<AgentChoice>>({ kind: 'loading' })
  project = $state<number | null>(null)
  workspace = $state<number | null>(null)
  agent = $state('')
  model = $state('')
  prompt = $state('')
  ownWorktree = $state(false)
  issue = $state.raw<StartingIssue | null>(null)
  branch = $state('')
  place = $state<string | null>(null)
  placeError = $state<string | null>(null)
  workspaceError = $state<string | null>(null)
  branchError = $state<string | null>(null)
  agentError = $state<string | null>(null)
  promptError = $state<string | null>(null)
  formError = $state<string | null>(null)
  sending = $state(false)
  #calls: StartCalls
  #onstarted: (started: Started, agent: string) => void
  #memory: Memory
  #boot: string | null = null
  #connected: boolean | null = null
  #projectsAsked = 0
  #agentsAsked = 0

  constructor(
    calls: StartCalls,
    onstarted: (started: Started, agent: string) => void,
    memory: Memory = remembered,
  ) {
    this.#calls = calls
    this.#onstarted = onstarted
    this.#memory = memory
  }

  get issueBranch() {
    return this.issue?.issue.branch ?? ''
  }

  get placeChoices(): Choices<PlaceInfo> {
    return { kind: 'ready', items: this.issue?.places ?? [] }
  }

  get chosenPlace() {
    return this.issue?.places.find((place) => placeKey(place) === this.place)
  }

  placeName(place: PlaceInfo) {
    if (!place.worktree) {
      return `${place.label} · new tab`
    }
    const branch = this.branch.trim() === '' ? this.issueBranch : this.branch.trim()
    return branch === '' ? `${place.label} · new worktree` : `${place.label} · new worktree on ${branch}`
  }

  get workspaceChoices(): Choices<WorkspaceOption> {
    const projects = this.projects
    return projects.kind === 'ready' ? { kind: 'ready', items: optionsOf(projects.items) } : projects
  }

  get noAgents() {
    return this.agents.kind === 'ready' && this.agents.items.length === 0
  }

  get noWorkspaces() {
    const choices = this.workspaceChoices
    return choices.kind === 'ready' && choices.items.length === 0
  }

  forIssue(issue: StartingIssue) {
    this.issue = issue
    this.project = issue.project
    this.branch = issue.issue.branch
    const place = startedIn(issue)
    this.place = place === undefined ? null : placeKey(place)
  }

  prefer(workspace: number) {
    this.workspace = workspace
    this.#pickWorkspace()
  }

  async load() {
    await Promise.all([this.loadProjects(), this.loadAgents()])
  }

  async loadProjects() {
    this.projects = { kind: 'loading' }
    await this.refresh()
  }

  async loadAgents() {
    const asked = ++this.#agentsAsked
    this.agents = { kind: 'loading' }
    const agents = choicesOf(await ask(() => this.#calls.installedAgents(), CHOICES_UNANSWERED))
    if (asked === this.#agentsAsked) {
      this.agents = agents
      this.#pickAgent()
    }
  }

  async refresh() {
    const agentsFailed = this.agents.kind === 'failed'
    if ((await this.#readProjects()) || agentsFailed) {
      await this.loadAgents()
    }
  }

  async see(connected: boolean) {
    const back = connected && this.#connected === false
    this.#connected = connected
    if (back) {
      await this.refresh()
    }
  }

  async #readProjects() {
    const asked = ++this.#projectsAsked
    const reply = await ask(() => this.#calls.startProjects(), CHOICES_UNANSWERED)
    if (asked !== this.#projectsAsked) {
      return false
    }
    this.projects = choicesOf(projectsOf(reply))
    this.#pickWorkspace()
    return reply.ok && this.#restarted(reply.data.boot)
  }

  #restarted(boot: string | null) {
    const known = this.#boot
    this.#boot = boot ?? known
    return known !== null && boot !== null && boot !== known
  }

  async start() {
    if (this.sending) {
      return
    }
    const place = this.#checkAgent()
    const { agent, model, prompt, workspace } = this
    if (place === null) {
      return
    }
    const form = { place, agent, model, prompt }
    if (await this.#launch(() => this.#calls.startAgent(form), agent)) {
      this.#memory.workspace = workspace
    }
  }

  #checkAgent() {
    const chosen = this.#chosenWorkspace()
    this.workspaceError = errorIf(chosen === undefined, WORKSPACE_EMPTY)
    this.agentError = errorIf(this.agent === '', AGENT_EMPTY)
    this.promptError = errorIf(this.prompt.trim() === '', PROMPT_EMPTY)
    this.branchError = errorIf(this.ownWorktree && this.branch.trim() === '', BRANCH_EMPTY)
    this.formError = null
    const refused = [this.agentError, this.promptError, this.branchError].some((error) => error !== null)
    return chosen === undefined || refused ? null : this.#agentPlace(chosen)
  }

  #agentPlace(chosen: WorkspaceOption): Place {
    return this.ownWorktree
      ? worktreeOf(chosen.project, this.branch)
      : { kind: 'workspace', workspace: chosen.id }
  }

  #chosenWorkspace() {
    const choices = this.workspaceChoices
    return choices.kind === 'ready' ? choices.items.find((choice) => choice.id === this.workspace) : undefined
  }

  async startIssue() {
    const issue = this.issue
    if (this.sending || issue === null) {
      return
    }
    const chosen = this.chosenPlace
    this.placeError = errorIf(chosen === undefined, PLACE_EMPTY)
    this.agentError = errorIf(this.agent === '', AGENT_EMPTY)
    this.formError = null
    const { agent, model, branch, issueBranch } = this
    if (chosen === undefined || this.agentError !== null) {
      return
    }
    const { key, source } = issue.issue
    const form = { place: placeOf(chosen, branch), agent, model, key, source, issue_branch: issueBranch }
    await this.#launch(() => this.#calls.startIssue(form), agent)
  }

  async #launch(call: () => Promise<Answer<Started>>, agent: string) {
    const reply = await this.#send(call, AGENT_UNANSWERED)
    if (reply.ok) {
      this.#memory.agent = agent
      this.#onstarted(reply.data, agent)
    }
    return reply.ok
  }

  async #send<T>(call: () => Promise<Answer<T>>, unanswered: string) {
    this.sending = true
    const reply = await ask(call, unanswered)
    if (!reply.ok) {
      this.sending = false
      this.formError = reply.error
    }
    return reply
  }

  #pickWorkspace() {
    const choices = this.workspaceChoices
    if (choices.kind !== 'ready') {
      return
    }
    const ids = choices.items.map((choice) => choice.id)
    if (this.workspace !== null && ids.includes(this.workspace)) {
      return
    }
    this.workspace = this.#fallbackWorkspace(ids)
  }

  #fallbackWorkspace(ids: number[]) {
    const last = this.#memory.workspace
    if (last !== null && ids.includes(last)) {
      return last
    }
    return ids.length === 1 ? ids[0] : null
  }

  #pickAgent() {
    if (this.agents.kind !== 'ready') {
      return
    }
    const names = this.agents.items.map((choice) => choice.agent)
    if (names.includes(this.agent)) {
      return
    }
    const last = this.#memory.agent
    this.agent = last !== null && names.includes(last) ? last : (names[0] ?? '')
  }
}
