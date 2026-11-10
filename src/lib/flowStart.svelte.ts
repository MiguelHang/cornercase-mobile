import { SvelteMap } from 'svelte/reactivity'
import type { FlowChoice_Serialize as FlowChoice } from './bindings'
import { CHOICES_UNANSWERED, ask, type Choices, type StartCalls } from './startForm.svelte'

export type FlowCalls = Pick<StartCalls, 'issueFlows'>

export type FlowMemory = { flow: string | null }

export type FlowOption = { name: string; agents: string; description: string; problem: string | null }

const ORCHESTRATOR = 'orchestrator'

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

const teamOf = (flow: FlowChoice) => {
  const leads = flow.roles.filter((role) => role.role === ORCHESTRATOR)
  const others = flow.roles.filter((role) => role.role !== ORCHESTRATOR)
  return [...leads, ...others].map((role) => `${role.role} · ${role.agent}`)
}

export class FlowStart {
  flows = $state.raw<Choices<FlowChoice>>({ kind: 'loading' })
  flow = $state<string | null>(null)
  #calls: FlowCalls
  #memory: FlowMemory
  #read = new SvelteMap<number, FlowChoice[]>()
  #project: number | null = null
  #boot: string | null = null
  #asked = 0

  constructor(calls: FlowCalls, memory: FlowMemory = remembered) {
    this.#calls = calls
    this.#memory = memory
  }

  get options(): Choices<FlowOption> {
    const flows = this.flows
    return flows.kind === 'ready' ? { kind: 'ready', items: flows.items.map(optionOf) } : flows
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
    if (this.#project !== null) {
      await this.#ask(this.#project)
    }
  }

  remember(name: string) {
    this.#memory.flow = name
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
