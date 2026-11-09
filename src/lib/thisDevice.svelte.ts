import { send, type Outcome } from './addMachine.svelte'
import type { commands } from './bindings'

export type DeviceCalls = Pick<typeof commands, 'identityFingerprint'>

export class ThisDevice {
  shown = $state<Outcome<string> | null>(null)
  #calls: DeviceCalls

  constructor(calls: DeviceCalls) {
    this.#calls = calls
  }

  async load() {
    this.shown = await send(() => this.#calls.identityFingerprint())
  }
}
