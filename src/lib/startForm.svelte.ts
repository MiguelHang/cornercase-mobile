import type { Answer } from './addMachine.svelte'
import type {
  AgentChoice_Serialize as AgentChoice,
  CommandError,
  FlowStarted_Serialize as FlowStarted,
  Place,
  PlaceInfo_Serialize as PlaceInfo,
  ProjectChoice,
  StartAgents_Serialize as StartAgents,
  StartChoices_Serialize as StartChoices,
  Started_Serialize as Started,
  commands,
} from './bindings'
import type { StartingIssue, Where } from './router.svelte'
import { messageOf } from './toasts'

export type StartCalls = Pick<
  typeof commands,
  'startProjects' | 'installedAgents' | 'startAgent' | 'startIssue' | 'issueFlows' | 'startFlow'
>

export type Choices<T> = { kind: 'loading' } | { kind: 'failed'; reason: string } | { kind: 'ready'; items: T[] }

export type Memory = { agent: string | null; workspace: { boot: string; id: number } | null }

export type WorkspaceOption = { id: number; project: number; label: string }

export type Reply<T> = { ok: true; data: T } | { ok: false; error: string }

export type Keep = (place: PlaceInfo) => boolean

export const WORKSPACE_EMPTY = 'Choose a workspace'
export const PLACE_EMPTY = 'Choose where to start'
export const BRANCH_EMPTY = 'Name the branch'
export const AGENT_EMPTY = 'Choose an agent'
export const PROMPT_EMPTY = 'Write what the agent should do'
export const AGENT_UNANSWERED =
  'cornercase did not answer, so the agent may have started: look at the board before trying again.'
export const CHOICES_UNANSWERED = 'cornercase did not answer.'
export const PLACES_STALE = 'cornercase restarted: go back to the issue to see where it can start.'

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

const agentsOf = (reply: Reply<StartAgents>): Reply<AgentChoice[]> =>
  reply.ok ? { ok: true, data: reply.data.agents } : reply

const errorIf = (empty: boolean, error: string) => (empty ? error : null)

const everywhere: Keep = () => true

const naming = (answer: Answer<Started>, agent: string): Answer<FlowStarted> =>
  answer.status === 'ok' ? { status: 'ok', data: { started: answer.data, agent } } : answer

const worktreeOf = (project: number, branch: string): Place => ({ kind: 'worktree', project, branch })

export const placeKey = (place: PlaceInfo) =>
  place.workspace === null ? `project-${place.project}` : `workspace-${place.workspace}`

const placeOf = (place: PlaceInfo, branch: string): Place =>
  place.workspace === null ? worktreeOf(place.project, branch) : { kind: 'workspace', workspace: place.workspace }

const startedIn = ({ places, project, workspace }: Where) => {
  const opened = places.find((place) => place.workspace !== null && place.workspace === workspace)
  const own = places.find((place) => place.worktree && place.project === project)
  return opened ?? own ?? (places.length === 1 ? places[0] : undefined)
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
  #placesStale = $state(false)
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

  get boot() {
    return this.#boot
  }

  get issueBranch() {
    return this.issue?.issue.branch ?? ''
  }

  get placeChoices(): Choices<PlaceInfo> {
    return this.placesWhere(everywhere)
  }

  placesWhere(keep: Keep): Choices<PlaceInfo> {
    if (this.#placesStale) {
      return { kind: 'failed', reason: PLACES_STALE }
    }
    return { kind: 'ready', items: (this.issue?.places ?? []).filter(keep) }
  }

  defaultPlace(keep: Keep = everywhere) {
    const issue = this.issue
    const place = issue === null ? undefined : startedIn({ ...issue, places: issue.places.filter(keep) })
    return place === undefined ? null : placeKey(place)
  }

  get canStartIssue() {
    return !this.sending && this.issue !== null && !this.#placesStale
  }

  checkPlace() {
    const chosen = this.chosenPlace
    this.placeError = errorIf(chosen === undefined, PLACE_EMPTY)
    this.formError = null
    return chosen
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
    this.#placesStale = false
    this.project = issue.project
    this.branch = issue.issue.branch
    this.place = this.defaultPlace()
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
    const agents = choicesOf(agentsOf(await ask(() => this.#calls.installedAgents(), CHOICES_UNANSWERED)))
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
    const restarted = reply.ok && this.#restarted(reply.data.boot)
    if (restarted) {
      this.#forgetIds()
    }
    this.projects = choicesOf(projectsOf(reply))
    this.#pickWorkspace()
    return restarted
  }

  #restarted(boot: string | null) {
    const known = this.#boot
    this.#boot = boot ?? known
    return known !== null && boot !== null && boot !== known
  }

  #forgetIds() {
    this.workspace = null
    this.place = null
    this.#placesStale = this.issue !== null
  }

  async start() {
    if (this.sending) {
      return
    }
    const chosen = this.#checkAgent()
    const { agent, model, prompt } = this
    if (chosen === null) {
      return
    }
    const form = { place: this.#agentPlace(chosen), agent, model, prompt, accounts: [] }
    const boot = this.#boot
    if (await this.#launch(() => this.#calls.startAgent(form), agent)) {
      this.#memory.workspace = boot === null ? null : { boot, id: chosen.id }
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
    return chosen === undefined || refused ? null : chosen
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
    if (issue === null || !this.canStartIssue) {
      return
    }
    const chosen = this.checkPlace()
    this.agentError = errorIf(this.agent === '', AGENT_EMPTY)
    const { agent, model, branch, issueBranch } = this
    if (chosen === undefined || this.agentError !== null) {
      return
    }
    const { key, source } = issue.issue
    const place = placeOf(chosen, branch)
    const form = { place, agent, model, key, source, issue_branch: issueBranch, accounts: [] }
    await this.#launch(() => this.#calls.startIssue(form), agent)
  }

  async #launch(call: () => Promise<Answer<Started>>, agent: string) {
    const launched = await this.launch(async () => naming(await call(), agent), AGENT_UNANSWERED)
    if (launched) {
      this.#memory.agent = agent
    }
    return launched
  }

  async launch(call: () => Promise<Answer<FlowStarted>>, unanswered: string) {
    const reply = await this.#send(call, unanswered)
    if (reply.ok) {
      this.#onstarted(reply.data.started, reply.data.agent)
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
    if (last !== null && last.boot === this.#boot && ids.includes(last.id)) {
      return last.id
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
