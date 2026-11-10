import type {
  Choice,
  Colours_Serialize as Colours,
  commands,
  QuestionChanged_Serialize as QuestionChanged,
  Run_Serialize as Run,
} from './bindings'

export type Question = { rows: Run[][]; options: Choice[]; colours: Colours }

export type Asker = {
  open: typeof commands.questionOpen
  close: typeof commands.questionClose
  listen: (heard: (changed: QuestionChanged) => void) => Promise<() => void>
}

type Opened = { tab: number; unlisten: Promise<() => void> }

const quietly = () => {}

export class Questions {
  question = $state<Question | null>(null)
  #machine: number | null
  #pane: number
  #asker: Asker
  #opened: Opened | null = null
  #turn: Promise<unknown> = Promise.resolve()

  constructor(machine: number | null, pane: number, asker: Asker) {
    this.#machine = machine
    this.#pane = pane
    this.#asker = asker
  }

  want(tab: number | null) {
    if (this.#opened !== null && this.#opened.tab !== tab) {
      this.#close(this.#opened)
    }
    if (tab !== null && this.#opened === null) {
      this.#open(tab)
    }
  }

  #open(tab: number) {
    const unlisten = this.#asker.listen((changed) => this.#hear(opened, changed))
    const opened = { tab, unlisten }
    this.#opened = opened
    this.#queue(async () => {
      await unlisten
      if (this.#opened === opened) {
        await this.#asker.open(tab, this.#pane)
      }
    })
  }

  #close(opened: Opened) {
    this.#opened = null
    this.question = null
    opened.unlisten.then((stop) => stop()).catch(quietly)
    this.#queue(() => this.#asker.close(opened.tab))
  }

  #queue(call: () => Promise<unknown>) {
    this.#turn = this.#turn.then(call).catch(quietly)
  }

  #hear(opened: Opened, changed: QuestionChanged) {
    const ours = changed.machine === this.#machine && changed.tab === opened.tab && changed.pane === this.#pane
    if (!ours || this.#opened !== opened) {
      return
    }
    const { rows, options, colours } = changed
    this.question = changed.closed || rows.length === 0 ? null : { rows, options, colours }
  }
}
