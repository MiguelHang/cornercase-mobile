import { expanded, none, toggled, type Opened } from './folds'

export class Folds {
  #opened: Opened = $state.raw(none)

  expanded(machine: number, boot: string) {
    return expanded(this.#opened, machine, boot)
  }

  folded(machine: number, boot: string, pane: number) {
    return !this.expanded(machine, boot).has(pane)
  }

  toggle(machine: number, boot: string, pane: number) {
    this.#opened = toggled(this.#opened, machine, boot, pane)
  }
}

export const folds = new Folds()
