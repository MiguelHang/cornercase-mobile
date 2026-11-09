import { StickyCtrl, type InputEvent } from './keys'

export class ShownCtrl extends StickyCtrl {
  #shown = $state(false)

  override get armed() {
    return this.#shown
  }

  override toggle() {
    this.#shown = super.toggle()
    return this.#shown
  }

  override apply(events: InputEvent[]) {
    const applied = super.apply(events)
    this.#shown = super.armed
    return applied
  }
}
