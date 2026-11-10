import { SvelteMap } from 'svelte/reactivity'
import type { FlowChoice_Serialize as FlowChoice } from './bindings'
import {
  CHOICES_UNANSWERED,
  ask,
  placeKey,
  type Choices,
  type Keep,
  type StartCalls,
  type StartForm,
} from './startForm.svelte'

export type FlowCalls = Pick<StartCalls, 'issueFlows' | 'startFlow'>

export type StartMode = 'agent' | 'flow'

export type FlowMemory = { flow: string | null }

export type FlowOption = { name: string; agents: string; description: string; problem: string | null }

export const FLOW_EMPTY = 'Choose a flow'
export const FLOW_UNANSWERED =
  'cornercase did not answer, so the flow may have started: look at the board before trying again.'

const ORCHESTRATOR = 'orchestrator'

const worktrees: Keep = (place) => place.worktree

const remembered: FlowMemory = { flow: null }

const runs = (flow: FlowChoice) => flow.problems.length === 0

const agentsOf = (flow: FlowChoice) => {
  const count = flow.roles.length
  return count === 1 ? '1 agent' : `${count} agents`
}

const optionOf = (flow: FlowChoice): FlowOption => ({
  name: flow.name,
  agents: agentsOf(flow),
  description: flow.description,
  problem: flow.problems[0] ?? null,
})

const nameKey = (name: string) => name.trim().toLowerCase()

const firstOfEachName = (flows: FlowChoice[]) =>
  flows.filter((flow, at) => flows.findIndex((other) => nameKey(other.name) === nameKey(flow.name)) === at)

const teamOf = (flow: FlowChoice) => {
  const leads = flow.roles.filter((role) => role.role === ORCHESTRATOR)
  const others = flow.roles.filter((role) => role.role !== ORCHESTRATOR)
  return [...leads, ...others].map((role) => `${role.role} · ${role.agent}`)
}

export class FlowStart {
  flows = $state.raw<Choices<FlowChoice>>({ kind: 'loading' })
  flow = $state<string | null>(null)
  flowError = $state<string | null>(null)
  #mode = $state<StartMode>('agent')
  #left: { [mode in StartMode]?: string | null } = {}
  #form: StartForm
  #calls: FlowCalls
  #memory: FlowMemory
  #read = new SvelteMap<number, FlowChoice[]>()
  #project: number | null = null
  #boot: string | null = null
  #asked = 0

  constructor(form: StartForm, calls: FlowCalls, memory: FlowMemory = remembered) {
    this.#form = form
    this.#calls = calls
    this.#memory = memory
  }

  get mode() {
    return this.#mode
  }

  get offered() {
    const flows = this.flows
    const listed = flows.kind === 'failed' || (flows.kind === 'ready' && flows.items.length > 0)
    const places = this.#form.placesWhere(worktrees)
    return this.#mode === 'flow' || (listed && places.kind === 'ready' && places.items.length > 0)
  }

  get placeChoices() {
    return this.#mode === 'flow' ? this.#form.placesWhere(worktrees) : this.#form.placeChoices
  }

  get options(): Choices<FlowOption> {
    const flows = this.flows
    return flows.kind === 'ready' ? { kind: 'ready', items: firstOfEachName(flows.items).map(optionOf) } : flows
  }

  get chosen() {
    const flows = this.flows
    return flows.kind === 'ready' ? flows.items.find((flow) => flow.name === this.flow && runs(flow)) : undefined
  }

  get team() {
    const chosen = this.chosen
    return chosen === undefined ? [] : teamOf(chosen)
  }

  async read(project: number, boot: string | null) {
    const restarted = this.#restarted(boot)
    if (project === this.#project && !restarted && this.flows.kind !== 'failed') {
      return
    }
    this.#project = project
    const known = this.#read.get(project)
    if (known === undefined) {
      await this.#ask(project)
    } else {
      this.#show(known)
    }
  }

  async retry() {
    if (this.#project !== null && this.#readable) {
      await this.#ask(this.#project)
    }
  }

  remember(name: string) {
    this.#memory.flow = name
  }

  async use(mode: StartMode) {
    const form = this.#form
    if (mode === this.#mode) {
      return
    }
    this.#left[this.#mode] = form.place
    this.#mode = mode
    form.place = this.#placeFor(mode)
    form.placeError = null
    form.formError = null
    this.flowError = null
    await this.look()
  }

  async moveTo(place: string | null) {
    this.#form.place = place
    if (this.#mode === 'flow') {
      await this.look()
    }
  }

  async look() {
    const place = this.#flowPlace()
    if (place !== undefined && this.#readable) {
      await this.read(place.project, this.#form.boot)
    }
  }

  async start() {
    const form = this.#form
    const issue = form.issue
    if (issue === null || !form.canStartIssue) {
      return
    }
    const place = form.checkPlace()
    const chosen = this.chosen
    this.flowError = chosen === undefined ? FLOW_EMPTY : null
    if (place === undefined || chosen === undefined) {
      return
    }
    const { key, source } = issue.issue
    const { branch, issueBranch } = form
    const run = { project: place.project, name: chosen.name, key, source, branch, issue_branch: issueBranch }
    if (await form.launch(() => this.#calls.startFlow(run), FLOW_UNANSWERED)) {
      this.remember(chosen.name)
    }
  }

  get #readable() {
    return this.#form.placesWhere(worktrees).kind === 'ready'
  }

  #placeFor(mode: StartMode) {
    const left = this.#left[mode]
    return left === undefined ? this.#form.defaultPlace(mode === 'flow' ? worktrees : undefined) : left
  }

  #flowPlace() {
    const key = this.#mode === 'flow' ? this.#form.place : this.#placeFor('flow')
    const places = (this.#form.issue?.places ?? []).filter(worktrees)
    return places.find((place) => placeKey(place) === key) ?? places[0]
  }

  #restarted(boot: string | null) {
    const known = this.#boot
    this.#boot = boot ?? known
    const restarted = known !== null && boot !== null && boot !== known
    if (restarted) {
      this.#read.clear()
    }
    return restarted
  }

  async #ask(project: number) {
    const asked = ++this.#asked
    this.flows = { kind: 'loading' }
    this.flow = null
    const reply = await ask(() => this.#calls.issueFlows(project), CHOICES_UNANSWERED)
    if (asked !== this.#asked) {
      return
    }
    if (reply.ok) {
      this.#read.set(project, reply.data.flows)
      this.#show(reply.data.flows)
    } else {
      this.flows = { kind: 'failed', reason: reply.error }
    }
  }

  #show(flows: FlowChoice[]) {
    this.#asked++
    this.flows = { kind: 'ready', items: flows }
    this.flow = this.#pick(flows)
  }

  #pick(flows: FlowChoice[]) {
    const open = flows.filter(runs)
    const last = open.find((flow) => flow.name === this.#memory.flow)
    return (last ?? open.find((flow) => flow.default) ?? open[0])?.name ?? null
  }
}
