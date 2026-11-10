import { describe, expect, it } from 'vitest'
import { BoardSearch, boardSearch } from './boardSearch.svelte'

describe('BoardSearch', () => {
  it('starts empty', () => {
    expect(new BoardSearch().query(1)).toBe('')
  })

  it('keeps the query written for a machine', () => {
    const made = new BoardSearch()
    made.write(1, 'login')
    expect(made.query(1)).toBe('login')
  })

  it('shows another machine an empty query', () => {
    const made = new BoardSearch()
    made.write(1, 'login')
    expect(made.query(2)).toBe('')
  })

  it('drops the old query when another machine writes', () => {
    const made = new BoardSearch()
    made.write(1, 'login')
    made.write(2, 'shop')
    expect([made.query(1), made.query(2)]).toEqual(['', 'shop'])
  })

  it('shows a machine its query again when it comes back before another writes', () => {
    const made = new BoardSearch()
    made.write(1, 'login')
    const away = made.query(2)
    expect([away, made.query(1)]).toEqual(['', 'login'])
  })

  it('leaves the query empty once cleared', () => {
    const made = new BoardSearch()
    made.write(1, 'login')
    made.write(1, '')
    expect(made.query(1)).toBe('')
  })

  it('shares one store across the app', () => {
    boardSearch.write(3, 'billing')
    const kept = boardSearch.query(3)
    boardSearch.write(3, '')
    expect([boardSearch instanceof BoardSearch, kept, boardSearch.query(3)]).toEqual([true, 'billing', ''])
  })
})
