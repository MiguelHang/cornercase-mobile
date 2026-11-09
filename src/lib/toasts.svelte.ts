import type { Toast } from './bindings'
import { dismiss, empty, expire, messageOf, nextExpiry, push, type Stack } from './toasts'

export class Toasts {
  #stack: Stack = $state.raw(empty)
  #timer: ReturnType<typeof setTimeout> | undefined
  #now: () => number

  constructor(now: () => number = Date.now) {
    this.#now = now
  }

  get items() {
    return this.#stack.items
  }

  push(toast: Toast) {
    this.#set(push(this.#stack, toast, this.#now()))
  }

  dismiss(id: number) {
    this.#set(dismiss(this.#stack, id))
  }

  failed = (error: unknown) => {
    this.push({ level: 'error', message: messageOf(error) })
  }

  #set(stack: Stack) {
    this.#stack = stack
    clearTimeout(this.#timer)
    const at = nextExpiry(stack)
    if (at !== null) {
      this.#timer = setTimeout(() => this.#set(expire(this.#stack, this.#now())), at - this.#now())
    }
  }
}
