import { describe, expect, it } from 'vitest'
import { ShownCtrl } from './ctrl.svelte'
import { Typing } from './typing'

const typed = (data: string) => ({ inputType: 'insertText', data, isComposing: false, dataTransfer: null })

describe('ShownCtrl', () => {
  it('shows Ctrl armed once it is pressed', () => {
    const ctrl = new ShownCtrl()
    expect([ctrl.armed, ctrl.toggle(), ctrl.armed]).toEqual([false, true, true])
  })

  it('shows Ctrl let go once pressed again', () => {
    const ctrl = new ShownCtrl()
    ctrl.toggle()
    expect([ctrl.toggle(), ctrl.armed]).toEqual([false, false])
  })

  it('shows Ctrl let go once the keyboard types the key it applies to', () => {
    const ctrl = new ShownCtrl()
    ctrl.toggle()
    const step = new Typing(ctrl).beforeinput(typed('c'))
    expect([step.send, ctrl.armed]).toEqual([[{ key: { code: 'c', mods: ['ctrl'] } }], false])
  })

  it('stays armed through a step that sends nothing', () => {
    const ctrl = new ShownCtrl()
    ctrl.toggle()
    expect([ctrl.apply([]), ctrl.armed]).toEqual([[], true])
  })
})
