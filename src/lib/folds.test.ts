import { describe, expect, it } from 'vitest'
import { Folds, folds } from './folds.svelte'

const BOOT = 'boot-1'

const toggled = (...steps: [number, string, number][]) => {
  const made = new Folds()
  for (const [machine, boot, pane] of steps) {
    made.toggle(machine, boot, pane)
  }
  return made
}

describe('Folds', () => {
  it('keeps every pane folded until it is unfolded', () => {
    const made = new Folds()
    expect([made.folded(1, BOOT, 5), [...made.expanded(1, BOOT)]]).toEqual([true, []])
  })

  it('unfolds a pane and folds it again', () => {
    const made = toggled([1, BOOT, 5], [1, BOOT, 7])
    const unfolded = [made.folded(1, BOOT, 5), [...made.expanded(1, BOOT)]]
    made.toggle(1, BOOT, 5)
    expect([unfolded, made.folded(1, BOOT, 5), [...made.expanded(1, BOOT)]]).toEqual([
      [false, [5, 7]],
      true,
      [7],
    ])
  })

  it('keeps each machine apart', () => {
    const made = toggled([1, BOOT, 5], [2, BOOT, 9])
    expect([[...made.expanded(1, BOOT)], [...made.expanded(2, BOOT)], made.folded(2, BOOT, 5)]).toEqual([
      [5],
      [9],
      true,
    ])
  })

  it('forgets what was unfolded when the server boots again', () => {
    const made = toggled([1, BOOT, 5])
    expect([made.folded(1, 'boot-2', 5), [...made.expanded(1, 'boot-2')]]).toEqual([true, []])
  })

  it('starts the new boot empty and does not bring back the old one', () => {
    const made = toggled([1, BOOT, 5], [1, 'boot-2', 7])
    expect([[...made.expanded(1, 'boot-2')], [...made.expanded(1, BOOT)]]).toEqual([[7], []])
  })

  it('keeps the other machines when one boots again', () => {
    const made = toggled([1, BOOT, 5], [2, BOOT, 9], [1, 'boot-2', 7])
    expect([...made.expanded(2, BOOT)]).toEqual([9])
  })

  it('does not change a set it handed out', () => {
    const made = toggled([1, BOOT, 5])
    const before = made.expanded(1, BOOT)
    made.toggle(1, BOOT, 7)
    expect([[...before], [...made.expanded(1, BOOT)]]).toEqual([[5], [5, 7]])
  })

  it('shares one store across the app', () => {
    folds.toggle(3, BOOT, 11)
    const kept = [...folds.expanded(3, BOOT)]
    folds.toggle(3, BOOT, 11)
    expect([folds instanceof Folds, kept, folds.folded(3, BOOT, 11)]).toEqual([true, [11], true])
  })
})
