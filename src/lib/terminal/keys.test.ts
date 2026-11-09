import { describe, expect, it } from 'vitest'
import { pasted, physical, ROW, StickyCtrl, type Pressed } from './keys'

const press = (key: string, held: Partial<Pressed> = {}): Pressed => ({
  key,
  ctrlKey: false,
  altKey: false,
  shiftKey: false,
  metaKey: false,
  ...held,
})

const armed = () => {
  const ctrl = new StickyCtrl()
  ctrl.toggle()
  return ctrl
}

describe('ROW', () => {
  it('lists its buttons in order', () => {
    expect(ROW.map(({ label }) => label)).toEqual([
      'Esc',
      'Tab',
      'Ctrl',
      'Up',
      'Down',
      'Left',
      'Right',
      '|',
      '~',
      '/',
      '-',
      'Paste',
    ])
  })

  it.each([
    ['Esc', { key: { code: 'esc' } }],
    ['Tab', { key: { code: 'tab' } }],
    ['Up', { key: { code: 'up' } }],
    ['Down', { key: { code: 'down' } }],
    ['Left', { key: { code: 'left' } }],
    ['Right', { key: { code: 'right' } }],
    ['|', { text: '|' }],
    ['~', { text: '~' }],
    ['/', { text: '/' }],
    ['-', { text: '-' }],
  ])('sends %s as its event', (label, event) => {
    expect(ROW.find((button) => button.label === label)).toMatchObject({ kind: 'key', label, event })
  })

  it('draws the arrows and Paste as icons named for screen readers, so the row fits a narrow phone', () => {
    expect(ROW.filter((button) => button.icon !== undefined).map(({ label, icon }) => [label, icon])).toEqual([
      ['Up', 'arrow-up'],
      ['Down', 'arrow-down'],
      ['Left', 'arrow-left'],
      ['Right', 'arrow-right'],
      ['Paste', 'clipboard-paste'],
    ])
  })

  it('leaves Ctrl to its own handling', () => {
    expect(ROW.find((button) => button.label === 'Ctrl')).toEqual({ kind: 'ctrl', label: 'Ctrl' })
  })

  it('leaves Paste to its own handling and draws it as an icon, so the row fits a narrow phone', () => {
    expect(ROW.at(-1)).toEqual({ kind: 'paste', label: 'Paste', icon: 'clipboard-paste' })
  })
})

describe('pasted', () => {
  it('sends the clipboard as a paste', () => {
    expect(pasted('ls -la\n')).toEqual({ paste: 'ls -la\n' })
  })

  it('sends nothing for an empty clipboard', () => {
    expect(pasted('')).toBeNull()
  })
})

describe('physical', () => {
  it.each([
    ['ArrowUp', 'up'],
    ['ArrowDown', 'down'],
    ['ArrowLeft', 'left'],
    ['ArrowRight', 'right'],
    ['Escape', 'esc'],
    ['Tab', 'tab'],
    ['Enter', 'enter'],
    ['Backspace', 'backspace'],
    ['Delete', 'delete'],
    ['Home', 'home'],
    ['End', 'end'],
    ['PageUp', 'pageup'],
    ['PageDown', 'pagedown'],
    ['F1', 'f1'],
    ['F5', 'f5'],
    ['F12', 'f12'],
  ])('sends %s as the key %s', (key, code) => {
    expect(physical(press(key))).toEqual({ key: { code } })
  })

  it('keeps the modifiers of a function key', () => {
    expect(physical(press('F2', { shiftKey: true }))).toEqual({ key: { code: 'f2', mods: ['shift'] } })
  })

  it('sends Ctrl+c as c with ctrl', () => {
    expect(physical(press('c', { ctrlKey: true }))).toEqual({ key: { code: 'c', mods: ['ctrl'] } })
  })

  it('keeps every modifier held with a named key', () => {
    const held = { ctrlKey: true, altKey: true, shiftKey: true }
    expect(physical(press('ArrowLeft', held))).toEqual({ key: { code: 'left', mods: ['ctrl', 'alt', 'shift'] } })
  })

  it('sends Shift+Tab as tab with shift', () => {
    expect(physical(press('Tab', { shiftKey: true }))).toEqual({ key: { code: 'tab', mods: ['shift'] } })
  })

  it.each([
    ['a letter typed alone', press('a')],
    ['a letter with alt', press('a', { altKey: true })],
    ['a named key of a shortcut', press('ArrowUp', { metaKey: true })],
    ['a letter of a shortcut with ctrl', press('v', { ctrlKey: true, metaKey: true })],
    ['ctrl alone', press('Control', { ctrlKey: true })],
    ['a function key past the twelfth', press('F13')],
    ['a key it does not know', press('Insert')],
  ])('leaves out %s', (_, pressed) => {
    expect(physical(pressed)).toBeNull()
  })
})

describe('StickyCtrl', () => {
  it('starts unarmed and passes events as they are', () => {
    const ctrl = new StickyCtrl()
    expect([ctrl.armed, ctrl.apply([{ text: 'c' }])]).toEqual([false, [{ text: 'c' }]])
  })

  it('arms on the first press and disarms on the second', () => {
    const ctrl = new StickyCtrl()
    expect([ctrl.toggle(), ctrl.armed, ctrl.toggle(), ctrl.armed]).toEqual([true, true, false, false])
  })

  it('turns a typed character into the key with ctrl', () => {
    expect(armed().apply([{ text: 'c' }])).toEqual([{ key: { code: 'c', mods: ['ctrl'] } }])
  })

  it('adds ctrl to a key of the row', () => {
    expect(armed().apply([{ key: { code: 'left' } }])).toEqual([{ key: { code: 'left', mods: ['ctrl'] } }])
  })

  it('keeps the other modifiers of a key', () => {
    const event = { key: { code: 'tab', mods: ['shift' as const] } }
    expect(armed().apply([event])).toEqual([{ key: { code: 'tab', mods: ['ctrl', 'shift'] } }])
  })

  it('does not add ctrl twice', () => {
    const event = { key: { code: 'c', mods: ['ctrl' as const] } }
    expect(armed().apply([event])).toEqual([event])
  })

  it('applies ctrl only to the first character of a text typed at once', () => {
    expect(armed().apply([{ text: 'cat' }])).toEqual([{ key: { code: 'c', mods: ['ctrl'] } }, { text: 'at' }])
  })

  it('takes a character outside the basic plane whole', () => {
    expect(armed().apply([{ text: '😀!' }])).toEqual([{ key: { code: '😀', mods: ['ctrl'] } }, { text: '!' }])
  })

  it('applies ctrl only to the first of several events', () => {
    const events = [{ text: 'c' }, { key: { code: 'enter' } }]
    expect(armed().apply(events)).toEqual([{ key: { code: 'c', mods: ['ctrl'] } }, { key: { code: 'enter' } }])
  })

  it('passes a paste as it is', () => {
    expect(armed().apply([{ paste: 'ls' }])).toEqual([{ paste: 'ls' }])
  })

  it('lets go after the next key', () => {
    const ctrl = armed()
    ctrl.apply([{ text: 'c' }])
    expect([ctrl.armed, ctrl.apply([{ text: 'c' }])]).toEqual([false, [{ text: 'c' }]])
  })

  it('stays armed when nothing was sent', () => {
    const ctrl = armed()
    expect([ctrl.apply([]), ctrl.armed]).toEqual([[], true])
  })
})
