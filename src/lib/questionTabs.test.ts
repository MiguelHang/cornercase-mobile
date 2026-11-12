import { describe, expect, it } from 'vitest'
import type { KeyName } from './keys'
import { arrowsTo } from './questionTabs'

describe('arrowsTo', () => {
  it.each<[string, number | null, number, KeyName[]]>([
    ['forward by one', 0, 1, ['right']],
    ['forward by two', 1, 3, ['right', 'right']],
    ['back by one', 2, 1, ['left']],
    ['back by three', 3, 0, ['left', 'left', 'left']],
    ['to the open one', 2, 2, []],
    ['without an open one', null, 2, []],
  ])('goes %s', (_, open, target, keys) => {
    expect(arrowsTo(open, target)).toEqual(keys)
  })
})
