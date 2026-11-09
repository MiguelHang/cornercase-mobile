import type { CommandError, MatchView, commands } from './bindings'
import { ALREADY_SAVED, checkName, checkUrl, fieldOf, nameFor, type Field, type Offer } from './machineForm'
import { messageOf } from './toasts'

export type MachineCalls = Pick<typeof commands, 'machineAdd' | 'machineMatch' | 'machineRouteAdd'>

export type Answer<T> = { status: 'ok'; data: T } | { status: 'error'; error: CommandError }

export type Refusal = { kind: Field; error: string }

export type Outcome<T> = { kind: 'done'; data: T } | Refusal

export const ERRORS = { url: 'urlError', name: 'nameError', form: 'formError' } as const

export const send = async <T>(call: () => Promise<Answer<T>>): Promise<Outcome<T>> => {
  try {
    const answer = await call()
    return answer.status === 'ok'
      ? { kind: 'done', data: answer.data }
      : { kind: fieldOf(answer.error), error: answer.error.message }
  } catch (error) {
    return { kind: 'form', error: messageOf(error) }
  }
}

export class AddMachine {
  url = $state('')
  name = $state('')
  urlError = $state<string | null>(null)
  nameError = $state<string | null>(null)
  formError = $state<string | null>(null)
  offer = $state<Offer | null>(null)
  busy = $state(false)
  #calls: MachineCalls
  #onadded: (machine: number) => void
  #onrouted: (machine: number) => void

  constructor(calls: MachineCalls, onadded: (machine: number) => void, onrouted: (machine: number) => void) {
    this.#calls = calls
    this.#onadded = onadded
    this.#onrouted = onrouted
  }

  get held() {
    return this.busy || this.offer !== null
  }

  async submit() {
    if (this.held) {
      return
    }
    const url = checkUrl(this.url)
    const name = checkName(nameFor(this.name, this.url))
    this.urlError = url.ok ? null : url.error
    this.nameError = url.ok && !name.ok ? name.error : null
    this.formError = null
    if (url.ok && name.ok) {
      this.busy = true
      await this.#matched(url.url, name.name, await send(() => this.#calls.machineMatch(url.url, name.name)))
    }
  }

  async fill(url: string) {
    if (this.held) {
      return
    }
    this.url = url
    await this.submit()
  }

  async addRoute() {
    const offer = this.offer
    if (this.busy || offer === null) {
      return
    }
    this.busy = true
    const outcome = await send(() => this.#calls.machineRouteAdd(offer.machine, offer.url))
    this.#settle(outcome, () => this.#onrouted(offer.machine))
  }

  async addNew() {
    const offer = this.offer
    if (this.busy || offer === null) {
      return
    }
    const name = checkName(offer.name, offer.why === 'name' ? [offer.saved] : [])
    if (!name.ok) {
      this.#refuse({ kind: 'name', error: name.error })
      return
    }
    this.busy = true
    await this.#add(offer.url, name.name)
  }

  back() {
    if (!this.busy) {
      this.offer = null
    }
  }

  async #matched(url: string, name: string, outcome: Outcome<MatchView>) {
    if (outcome.kind !== 'done') {
      this.#refuse(outcome)
      return
    }
    const { found, checked } = outcome.data
    if (found.kind === 'none') {
      await this.#add(url, name)
      return
    }
    this.busy = false
    if (found.kind === 'already_saved') {
      this.formError = ALREADY_SAVED
    } else if (found.kind === 'same') {
      this.offer = { url, name, machine: found.machine, saved: found.name, why: found.why, checked }
    }
  }

  async #add(url: string, name: string) {
    this.#settle(await send(() => this.#calls.machineAdd(url, name)), (machine) => this.#onadded(machine))
  }

  #settle<T>(outcome: Outcome<T>, done: (data: T) => void) {
    if (outcome.kind === 'done') {
      done(outcome.data)
      return
    }
    this.#refuse(outcome)
  }

  #refuse(refusal: Refusal) {
    this.busy = false
    this.offer = null
    this[ERRORS[refusal.kind]] = refusal.error
  }
}
