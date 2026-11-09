import type { AgentTab, Screen } from './connection'

export type Route =
  | { name: 'no-machines' }
  | { name: 'home' }
  | { name: 'terminal'; machine: number; boot: string; tab: AgentTab }

export const routeFor = (screen: Screen): Route => (screen.machine === null ? { name: 'no-machines' } : { name: 'home' })

export class Router {
  #stack: Route[] = $state([])

  constructor(first: Route) {
    this.#stack = [first]
  }

  get current(): Route {
    return this.#stack[this.#stack.length - 1]
  }

  go(route: Route) {
    this.#stack = [...this.#stack, route]
  }

  replace(route: Route) {
    if (this.#stack[0].name !== route.name) {
      this.#stack = [route]
    }
  }

  back() {
    if (this.#stack.length > 1) {
      this.#stack = this.#stack.slice(0, -1)
    }
  }
}
