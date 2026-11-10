export type Searched = { machine: number | null; query: string }

export const none: Searched = { machine: null, query: '' }

export const queryFor = (searched: Searched, machine: number) => (searched.machine === machine ? searched.query : '')

export const withQuery = (machine: number, query: string): Searched => ({ machine, query })
