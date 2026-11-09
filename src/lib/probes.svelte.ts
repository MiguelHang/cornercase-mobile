import { send } from './addMachine.svelte'
import type { commands } from './bindings'
import { CHECKING, type Probe } from './machineRows'

export type ProbeCalls = Pick<typeof commands, 'machineProbe'>

export class Probes {
  #results = $state<Record<number, Probe>>({})
  #calls: ProbeCalls
  #active: number | null = null

  constructor(calls: ProbeCalls) {
    this.#calls = calls
  }

  of(machine: number): Probe {
    return this.#results[machine] ?? CHECKING
  }

  open(machines: number[], active: number | null) {
    this.#active = active
    return this.check(machines.filter((machine) => machine !== active))
  }

  follow(active: number | null) {
    const left = this.#active
    this.#active = active
    return left === null || left === active ? Promise.resolve() : this.check([left])
  }

  async check(machines: number[]) {
    const idle = machines.filter((machine) => this.#results[machine]?.state !== 'checking')
    await Promise.all(idle.map((machine) => this.#probe(machine)))
  }

  async #probe(machine: number) {
    this.#results[machine] = CHECKING
    const outcome = await send(() => this.#calls.machineProbe(machine))
    this.#results[machine] = outcome.kind === 'done' ? outcome.data : { state: 'failed', message: outcome.error }
  }
}
