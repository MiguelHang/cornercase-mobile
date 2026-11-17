import { DEMO_MACHINE, type commands } from './bindings'

export type DemoCalls = Pick<typeof commands, 'demoLeave'>

export type Screens = {
  root: () => void
  load: () => Promise<void>
  reload: () => Promise<unknown>
  failed: (error: unknown) => void
}

export class DemoExit {
  #calls: DemoCalls
  #screens: Screens

  constructor(calls: DemoCalls, screens: Screens) {
    this.#calls = calls
    this.#screens = screens
  }

  async leave() {
    const left = await this.#calls.demoLeave()
    if (left.status === 'error') {
      this.#screens.failed(left.error)
      return
    }
    await this.#reopen()
  }

  removed(demo: boolean, otherwise: () => Promise<void>) {
    return demo ? this.#reopen() : otherwise()
  }

  async switched(from: number | null, joined: Promise<void>) {
    await joined
    if (from === DEMO_MACHINE) {
      await this.#screens.reload()
    }
  }

  async #reopen() {
    this.#screens.root()
    await this.#screens.load()
  }
}
