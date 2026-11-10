import type { AgentRow_Serialize as AgentRow } from './bindings'
import type { Screen } from './connection'

export type Route =
  | { name: 'no-machines' }
  | { name: 'home' }
  | { name: 'agent'; machine: number | null; pane: number; starting?: true }
  | { name: 'terminal'; machine: number; row: AgentRow }
  | { name: 'machines' }
  | { name: 'machine'; id: number; adding?: boolean }
  | { name: 'start'; machine: number }
  | { name: 'changes'; machine: number; workspace: number; title: string }
  | { name: 'change'; machine: number; workspace: number; path: string }
  | { name: 'files'; machine: number; workspace: number; title: string }
  | { name: 'file'; machine: number; workspace: number; path: string }

type Base = Extract<Route, { name: 'no-machines' | 'home' }>

export const routeFor = (screen: Screen, known: number): Base =>
  screen.machine === null && known === 0 ? { name: 'no-machines' } : { name: 'home' }

export const openedTerminal = (route: Route, screen: Screen) =>
  route.name === 'terminal' && route.machine === screen.machine ? route : undefined

export type ChatOf = { machine: number | null; pane: number }

export const chatOf = (route: Route): ChatOf | null => {
  if (route.name === 'agent') {
    return { machine: route.machine, pane: route.pane }
  }
  return route.name === 'terminal' ? { machine: route.machine, pane: route.row.pane } : null
}

const overMachines = (route: Route) => route.name === 'machines' || route.name === 'machine'

type InWorkspace = Extract<Route, { name: 'changes' | 'change' | 'files' | 'file' }>

const inWorkspace = (route: Route): route is InWorkspace =>
  route.name === 'changes' || route.name === 'change' || route.name === 'files' || route.name === 'file'

const overChat = (route: Route) => overMachines(route) || inWorkspace(route)

export type WorkspaceOf = { machine: number; workspace: number }

export class Router {
  #stack: Route[] = $state([])

  constructor(first: Base) {
    this.#stack = [first]
  }

  get current(): Route {
    return this.#stack[this.#stack.length - 1]
  }

  get below(): Route | null {
    return this.#stack.at(-2) ?? null
  }

  get opened(): ChatOf | null {
    const under = this.#stack.findLast((route) => !overChat(route))
    return under === undefined ? null : chatOf(under)
  }

  get reviewed(): WorkspaceOf | null {
    return this.#workspaceOf('changes', 'change')
  }

  get browsed(): WorkspaceOf | null {
    return this.#workspaceOf('files', 'file')
  }

  #workspaceOf(...names: InWorkspace['name'][]): WorkspaceOf | null {
    const above = this.#stack.slice(this.#stack.findLastIndex((route) => !inWorkspace(route)) + 1)
    const found = above.findLast((route): route is InWorkspace => inWorkspace(route) && names.includes(route.name))
    return found === undefined ? null : { machine: found.machine, workspace: found.workspace }
  }

  go(route: Route) {
    this.#stack = [...this.#stack, route]
  }

  replace(route: Route) {
    this.#stack = [...this.#stack.slice(0, Math.max(1, this.#stack.length - 1)), route]
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
