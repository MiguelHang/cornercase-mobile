import type { Size_Deserialize } from '../bindings'
import type { Opening } from './opening'

export type Asked = Pick<Opening, 'open' | 'pause'>

export type Page = Pick<EventTarget, 'addEventListener' | 'removeEventListener'> & Pick<Document, 'visibilityState'>

export class Fit {
  #on = $state(false)
  #visible = $state(true)

  get on() {
    return this.#on
  }

  get visible() {
    return this.#visible
  }

  toggle() {
    this.#on = !this.#on
  }

  ask(opening: Asked, size: Size_Deserialize) {
    if (this.#on && !this.#visible) {
      opening.pause()
    } else {
      opening.open(size, this.#on)
    }
  }

  unfitted() {
    this.#on = false
  }

  watch(page: Page) {
    const read = () => {
      this.#visible = page.visibilityState === 'visible'
    }
    read()
    page.addEventListener('visibilitychange', read)
    return () => page.removeEventListener('visibilitychange', read)
  }
}
