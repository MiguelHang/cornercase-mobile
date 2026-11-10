import { none, queryFor, withQuery, type Searched } from './boardSearch'

export class BoardSearch {
  #searched: Searched = $state.raw(none)

  query(machine: number) {
    return queryFor(this.#searched, machine)
  }

  write(machine: number, query: string) {
    this.#searched = withQuery(machine, query)
  }
}

export const boardSearch = new BoardSearch()
