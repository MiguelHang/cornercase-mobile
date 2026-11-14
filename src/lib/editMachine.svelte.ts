import { untrack } from 'svelte'
import { ERRORS, send, type Answer } from './addMachine.svelte'
import type { Direction, MachineView_Serialize, MergeView, Toast, commands } from './bindings'
import { checkName, checkUrl, routesText, type Field } from './machineForm'

export type EditCalls = Pick<
  typeof commands,
  'machineRename' | 'machineMerge' | 'machineRouteRemove' | 'machineRouteMove' | 'machineRemove'
>

export class EditMachine {
  name = $state('')
  url = $state('')
  nameError = $state<string | null>(null)
  urlError = $state<string | null>(null)
  formError = $state<string | null>(null)
  adding = $state(false)
  deleting = $state(false)
  busy = $state(false)
  #calls: EditCalls
  #machine: () => MachineView_Serialize
  #onchanged: () => Promise<unknown>
  #onremoved: () => Promise<unknown>
  #tell: (toast: Toast) => void

  constructor(
    calls: EditCalls,
    machine: () => MachineView_Serialize,
    onchanged: () => Promise<unknown>,
    onremoved: () => Promise<unknown>,
    tell: (toast: Toast) => void,
  ) {
    this.#calls = calls
    this.#machine = machine
    this.#onchanged = onchanged
    this.#onremoved = onremoved
    this.#tell = tell
    this.name = untrack(machine).name
  }

  get unchanged() {
    return this.name.trim() === this.#machine().name
  }

  async rename(taken: readonly string[]) {
    if (this.busy || this.unchanged) {
      return
    }
    const name = checkName(this.name, taken)
    if (!name.ok) {
      this.nameError = name.error
      return
    }
    await this.#run(() => this.#calls.machineRename(this.#machine().id, name.name), 'name')
  }

  async move(url: string, direction: Direction) {
    if (!this.busy) {
      await this.#run(() => this.#calls.machineRouteMove(this.#machine().id, url, direction), 'form')
    }
  }

  async remove(url: string) {
    if (!this.busy && this.#machine().routes.length > 1) {
      await this.#run(() => this.#calls.machineRouteRemove(this.#machine().id, url), 'form')
    }
  }

  open() {
    this.adding = true
  }

  close() {
    if (!this.busy) {
      this.adding = false
      this.url = ''
      this.urlError = null
    }
  }

  async addRoute() {
    if (this.busy) {
      return
    }
    const url = checkUrl(this.url)
    if (!url.ok) {
      this.urlError = url.error
      return
    }
    const machine = this.#machine()
    const added = await this.#run(() => this.#calls.machineMerge(machine.id, url.url), 'url', (merged) =>
      this.#merged(machine.name, merged),
    )
    if (added) {
      this.close()
    }
  }

  async fill(url: string) {
    if (!this.busy) {
      this.url = url
      await this.addRoute()
    }
  }

  askToDelete() {
    if (!this.busy) {
      this.deleting = true
    }
  }

  keep() {
    if (!this.busy) {
      this.deleting = false
      this.formError = null
    }
  }

  async deleteMachine() {
    if (this.deleting && !this.busy) {
      await this.#run(() => this.#calls.machineRemove(this.#machine().id), 'form', this.#onremoved)
    }
  }

  #merged(name: string, merged: MergeView) {
    this.#tell({ level: 'info', message: routesText(name, merged) })
    return this.#onchanged()
  }

  async #run<T>(call: () => Promise<Answer<T>>, own: Field, after: (data: T) => Promise<unknown> = this.#onchanged) {
    this.busy = true
    this.nameError = null
    this.urlError = null
    this.formError = null
    const outcome = await send(call)
    if (outcome.kind === 'done') {
      try {
        await after(outcome.data)
      } finally {
        this.busy = false
      }
      return true
    }
    this.busy = false
    this[ERRORS[outcome.kind === own ? own : 'form']] = outcome.error
    return false
  }
}
