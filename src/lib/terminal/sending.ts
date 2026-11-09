import type { InputEvent } from './keys'
import type { PaneState } from './pane.svelte'

export type Sent = { status: 'ok' } | { status: 'error'; error: unknown }

export type Sends = {
  input: (pane: number, events: InputEvent[]) => Promise<Sent>
  scroll: (pane: number, rowsUp: number) => Promise<Sent>
  failed: (error: unknown) => void
}

export type Shown = { readonly current: PaneState }

export class Sending {
  readonly #shown: Shown
  readonly #calls: Sends

  constructor(shown: Shown, calls: Sends) {
    this.#shown = shown
    this.#calls = calls
  }

  input(events: InputEvent[]) {
    const pane = this.#pane()
    if (pane !== null && events.length > 0) this.#settle(this.#calls.input(pane, events))
  }

  scroll(rowsUp: number) {
    const pane = this.#pane()
    if (pane !== null && rowsUp !== 0) this.#settle(this.#calls.scroll(pane, rowsUp))
  }

  #pane() {
    const { pane, closed } = this.#shown.current
    return closed ? null : pane
  }

  #settle(sent: Promise<Sent>) {
    const { failed } = this.#calls
    sent.then((answer) => answer.status === 'error' && failed(answer.error)).catch(failed)
  }
}
