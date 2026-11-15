export const KEYBOARD_TOP = 'keyboardtop'
export const FOLLOWS = 'follows'
const SLACK = 1

type Heard = Pick<EventTarget, 'addEventListener' | 'removeEventListener'>
type Marked = { dataset: DOMStringMap }

const topOf = (event: Event) => {
  const { detail } = event as CustomEvent<unknown>
  return typeof detail === 'number' && Number.isFinite(detail) ? detail : null
}

export const dropped = (height: number, top: number | null) => {
  const drop = (top ?? height) - height
  return drop > SLACK ? drop : 0
}

export const followKeys = (root: Marked) => {
  root.dataset.keyboard = FOLLOWS
  return () => {
    delete root.dataset.keyboard
  }
}

export class KeyboardTop {
  #top = $state<number | null>(null)

  get top() {
    return this.#top
  }

  watch(target: Heard) {
    const heard = (event: Event) => (this.#top = topOf(event))
    target.addEventListener(KEYBOARD_TOP, heard)
    return () => target.removeEventListener(KEYBOARD_TOP, heard)
  }
}
