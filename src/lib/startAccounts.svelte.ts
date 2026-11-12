import { SvelteMap } from 'svelte/reactivity'
import type {
  AccountChoice_Serialize as AccountChoice,
  AccountPick,
  AgentAccounts_Serialize as AgentAccounts,
  Login_Serialize as Login,
} from './bindings'

export type AccountOption = { value: string; name: string }

const MARKS: Partial<Record<Login, string>> = {
  'signed-out': ' · signed out',
  failed: ' · could not check',
}

const optionOf = (choice: AccountChoice): AccountOption => ({
  value: choice.name,
  name: `${choice.label}${MARKS[choice.login] ?? ''}`,
})

const named = (accounts: AgentAccounts, name: string | undefined) =>
  accounts.options.find((option) => option.name === name)

export class StartAccounts {
  #known = $state.raw<AgentAccounts[]>([])
  #chosen = new SvelteMap<string, string>()

  read(known: AgentAccounts[]) {
    this.#known = known
  }

  forget() {
    this.#chosen.clear()
  }

  optionsOf(agent: string): AccountOption[] {
    return this.#of(agent)?.options.map(optionOf) ?? []
  }

  chosen(agent: string) {
    return this.#picked(agent)?.name ?? ''
  }

  choose(agent: string, name: string) {
    this.#chosen.set(agent, name)
  }

  signedOut(agent: string) {
    return this.#picked(agent)?.login === 'signed-out'
  }

  picks(agents: string[]): AccountPick[] {
    return agents.flatMap((agent) => {
      const accounts = this.#of(agent)
      const account = this.chosen(agent)
      return accounts === undefined || account === accounts.default ? [] : [{ agent, account }]
    })
  }

  #of(agent: string) {
    return this.#known.find((accounts) => accounts.agent === agent)
  }

  #picked(agent: string) {
    const accounts = this.#of(agent)
    if (accounts === undefined) {
      return undefined
    }
    return named(accounts, this.#chosen.get(agent)) ?? named(accounts, accounts.default)
  }
}
