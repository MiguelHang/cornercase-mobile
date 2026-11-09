import type { Screen } from './connection'

export type Route = { name: 'no-machines' } | { name: 'home' } | { name: 'machines' } | { name: 'machine'; id: number; adding?: boolean }

type Base = Extract<Route, { name: 'no-machines' | 'home' }>

export const routeFor = (screen: Screen, known: number): Base =>
  screen.machine === null && known === 0 ? { name: 'no-machines' } : { name: 'home' }

export class Router {
  #stack: Route[] = $state([])

  constructor(first: Base) {
    this.#stack = [first]
  }

  get current(): Route {
    return this.#stack[this.#stack.length - 1]
  }

  go(route: Route) {
    this.#stack = [...this.#stack, route]
  }

  setBase(base: Base) {
    const [first, ...above] = this.#stack
    const emptied = base.name === 'no-machines'
    if (first.name === base.name && (!emptied || above.length === 0)) {
      return
    }
    this.#stack = emptied ? [base] : [base, ...above]
  }

  back() {
    if (this.#stack.length > 1) {
      this.#stack = this.#stack.slice(0, -1)
    }
  }
}
