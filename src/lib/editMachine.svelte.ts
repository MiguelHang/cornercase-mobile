import { ERRORS, send, type Answer } from './addMachine.svelte'
import type { Direction, MachineView_Serialize, commands } from './bindings'
import { checkName, checkUrl, type Field } from './machineForm'

export type EditCalls = Pick<
  typeof commands,
  'machineRename' | 'machineRouteAdd' | 'machineRouteRemove' | 'machineRouteMove'
>

export class EditMachine {
  name = $state('')
  url = $state('')
  nameError = $state<string | null>(null)
  urlError = $state<string | null>(null)
  formError = $state<string | null>(null)
  adding = $state(false)
  busy = $state(false)
  #calls: EditCalls
  #machine: () => MachineView_Serialize
  #onchanged: () => Promise<unknown>

  constructor(calls: EditCalls, machine: () => MachineView_Serialize, onchanged: () => Promise<unknown>) {
    this.#calls = calls
    this.#machine = machine
    this.#onchanged = onchanged
    this.name = machine().name
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
    if (await this.#run(() => this.#calls.machineRouteAdd(this.#machine().id, url.url), 'url')) {
      this.close()
    }
  }

  async fill(url: string) {
    if (!this.busy) {
      this.url = url
      await this.addRoute()
    }
  }

  async #run(call: () => Promise<Answer<null>>, own: Field) {
    this.busy = true
    this.nameError = null
    this.urlError = null
    this.formError = null
    const outcome = await send(call)
    if (outcome.kind === 'done') {
      try {
        await this.#onchanged()
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
