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

  get wanted() {
    return this.#on && this.#visible
  }

  toggle() {
    this.#on = !this.#on
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
