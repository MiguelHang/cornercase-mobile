import type { Answer } from './addMachine.svelte'
import type {
  AgentChoice_Serialize as AgentChoice,
  CommandError,
  ProjectChoice,
  StartChoices_Serialize as StartChoices,
  Started_Serialize as Started,
  commands,
} from './bindings'
import { messageOf } from './toasts'

export type StartCalls = Pick<typeof commands, 'startProjects' | 'installedAgents' | 'startAgent' | 'newWorkspace'>

export type Choices<T> = { kind: 'loading' } | { kind: 'failed'; reason: string } | { kind: 'ready'; items: T[] }

export type Memory = { agent: string | null }

type Reply<T> = { ok: true; data: T } | { ok: false; error: string }

export const PROJECT_EMPTY = 'Choose a project'
export const AGENT_EMPTY = 'Choose an agent'
export const PROMPT_EMPTY = 'Write what the agent should do'
export const NAME_EMPTY = 'Name the workspace'
export const AGENT_UNANSWERED =
  'cornercase did not answer, so the agent may have started: look at the board before trying again.'
export const WORKSPACE_UNANSWERED =
  'cornercase did not answer, so the workspace may exist already: look on the Mac before trying again.'
export const CHOICES_UNANSWERED = 'cornercase did not answer.'

const remembered: Memory = { agent: null }

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

export class StartForm {
  projects = $state<Choices<ProjectChoice>>({ kind: 'loading' })
  agents = $state<Choices<AgentChoice>>({ kind: 'loading' })
  project = $state<number | null>(null)
  agent = $state('')
  model = $state('')
  prompt = $state('')
  worktree = $state('')
  name = $state('')
  ownWorktree = $state(false)
  projectError = $state<string | null>(null)
  agentError = $state<string | null>(null)
  promptError = $state<string | null>(null)
  nameError = $state<string | null>(null)
  formError = $state<string | null>(null)
  sending = $state(false)
  #calls: StartCalls
  #onstarted: (started: Started, agent: string) => void
  #oncreated: (name: string) => void
  #memory: Memory
  #boot: string | null = null
  #connected: boolean | null = null
  #projectsAsked = 0
  #agentsAsked = 0

  constructor(
    calls: StartCalls,
    onstarted: (started: Started, agent: string) => void,
    oncreated: (name: string) => void,
    memory: Memory = remembered,
  ) {
    this.#calls = calls
    this.#onstarted = onstarted
    this.#oncreated = oncreated
    this.#memory = memory
  }

  get noAgents() {
    return this.agents.kind === 'ready' && this.agents.items.length === 0
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
    this.#pickProject()
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
    this.projectError = errorIf(this.project === null, PROJECT_EMPTY)
    this.agentError = errorIf(this.agent === '', AGENT_EMPTY)
    this.promptError = errorIf(this.prompt.trim() === '', PROMPT_EMPTY)
    this.formError = null
    const { project, agent, model, prompt, worktree } = this
    if (project === null || this.agentError !== null || this.promptError !== null) {
      return
    }
    const form = { project, agent, model, prompt, worktree }
    const reply = await this.#send(() => this.#calls.startAgent(form), AGENT_UNANSWERED)
    if (reply.ok) {
      this.#memory.agent = agent
      this.#onstarted(reply.data, agent)
    }
  }

  async create() {
    if (this.sending) {
      return
    }
    this.projectError = errorIf(this.project === null, PROJECT_EMPTY)
    this.nameError = errorIf(this.name.trim() === '', NAME_EMPTY)
    this.formError = null
    const { project, name, ownWorktree } = this
    if (project === null || this.nameError !== null) {
      return
    }
    const reply = await this.#send(
      () => this.#calls.newWorkspace({ project, name, worktree: ownWorktree }),
      WORKSPACE_UNANSWERED,
    )
    if (reply.ok) {
      this.#oncreated(name.trim())
    }
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

  #pickProject() {
    if (this.projects.kind !== 'ready') {
      return
    }
    const items = this.projects.items
    if (!items.some((choice) => choice.id === this.project)) {
      this.project = null
    }
    if (this.project === null && items.length === 1) {
      this.project = items[0].id
    }
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
