import type { Size_Deserialize } from '../bindings'

export type Opened = { status: 'ok' } | { status: 'error'; error: unknown }

export type Calls = {
  open: (size: Size_Deserialize, fit: boolean) => Promise<Opened>
  close: () => Promise<unknown>
  failed: (error: unknown) => void
}

const keyOf = (size: Size_Deserialize, fit: boolean) => `${size.cols}x${size.rows}${fit ? ' fit' : ''}`

export class Opening {
  #calls: Calls
  #turn: Promise<unknown>
  #wanted = ''
  #left = false

  constructor(listening: Promise<unknown>, calls: Calls) {
    this.#calls = calls
    this.#turn = listening
  }

  open(size: Size_Deserialize, fit: boolean) {
    const key = keyOf(size, fit)
    if (this.#left || key === this.#wanted) {
      return
    }
    this.#wanted = key
    this.#turn = this.#turn.then(() => this.#send(size, fit, key)).catch(this.#calls.failed)
  }

  leave() {
    this.#left = true
    this.#calls.close().catch(this.#calls.failed)
  }

  async #send(size: Size_Deserialize, fit: boolean, key: string) {
    if (this.#left || key !== this.#wanted) {
      return
    }
    const result = await this.#calls.open(size, fit)
    if (this.#left) {
      await this.#calls.close()
    } else if (result.status === 'error') {
      this.#calls.failed(result.error)
    }
  }
}
