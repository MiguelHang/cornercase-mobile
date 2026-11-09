import type { commands } from './bindings'
import { checkUrl, fieldOf, nameFor, type Field } from './machineForm'
import { messageOf } from './toasts'

export type MachineAdd = typeof commands.machineAdd

export type Outcome = { kind: 'added'; machine: number } | { kind: Field; error: string }

export const send = async (add: MachineAdd, url: string, name: string): Promise<Outcome> => {
  try {
    const added = await add(url, name)
    return added.status === 'ok'
      ? { kind: 'added', machine: added.data }
      : { kind: fieldOf(added.error), error: added.error.message }
  } catch (error) {
    return { kind: 'form', error: messageOf(error) }
  }
}

export class AddMachine {
  url = $state('')
  name = $state('')
  urlError = $state<string | null>(null)
  formError = $state<string | null>(null)
  busy = $state(false)
  #add: MachineAdd
  #onadded: (machine: number) => void

  constructor(add: MachineAdd, onadded: (machine: number) => void) {
    this.#add = add
    this.#onadded = onadded
  }

  async submit() {
    if (this.busy) {
      return
    }
    const checked = checkUrl(this.url)
    this.urlError = checked.ok ? null : checked.error
    this.formError = null
    if (checked.ok) {
      this.busy = true
      this.#settle(await send(this.#add, checked.url, nameFor(this.name, this.url)))
    }
  }

  #settle(outcome: Outcome) {
    if (outcome.kind === 'added') {
      this.#onadded(outcome.machine)
      return
    }
    this.busy = false
    if (outcome.kind === 'url') {
      this.urlError = outcome.error
    } else {
      this.formError = outcome.error
    }
  }
}
