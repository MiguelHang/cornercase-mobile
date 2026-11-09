import type { Size_Deserialize } from '../bindings'

export type Opened = { status: 'ok' } | { status: 'error'; error: unknown }

export type Calls = {
  open: (size: Size_Deserialize) => Promise<Opened>
  close: () => Promise<unknown>
  failed: (error: unknown) => void
}

const keyOf = (size: Size_Deserialize) => `${size.cols}x${size.rows}`

export class Opening {
  #calls: Calls
  #turn: Promise<unknown>
  #wanted = ''
  #left = false

  constructor(listening: Promise<unknown>, calls: Calls) {
    this.#calls = calls
    this.#turn = listening
  }

  open(size: Size_Deserialize) {
    const key = keyOf(size)
    if (this.#left || key === this.#wanted) {
      return
    }
    this.#wanted = key
    this.#turn = this.#turn.then(() => this.#send(size, key)).catch(this.#calls.failed)
  }

  leave() {
    this.#left = true
    this.#calls.close().catch(this.#calls.failed)
  }

  async #send(size: Size_Deserialize, key: string) {
    if (this.#left || key !== this.#wanted) {
      return
    }
    const result = await this.#calls.open(size)
    if (this.#left) {
      await this.#calls.close()
    } else if (result.status === 'error') {
      this.#calls.failed(result.error)
    }
  }
}
