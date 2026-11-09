import { describe, expect, it } from 'vitest'
import { StickyCtrl } from './keys'
import { SENTINEL, Typing, type Keydown, type Typed } from './typing'

const press = (key: string, held: Partial<Keydown> = {}): Keydown => ({
  key,
  ctrlKey: false,
  altKey: false,
  shiftKey: false,
  metaKey: false,
  isComposing: false,
  ...held,
})

const typed = (inputType: string, data: string | null = null, held: Partial<Typed> = {}): Typed => ({
  inputType,
  data,
  isComposing: false,
  dataTransfer: null,
  ...held,
})

const carrying = (text: string) => ({ dataTransfer: { getData: (format: string) => (format === 'text/plain' ? text : '') } })

const typing = () => new Typing(new StickyCtrl())

const armed = () => {
  const ctrl = new StickyCtrl()
  ctrl.toggle()
  return { ctrl, typing: new Typing(ctrl) }
}

const sent = (...send: unknown[]) => ({ send, prevent: true, restore: true })

const passed = { send: [], prevent: false, restore: false }

const swallowed = { send: [], prevent: true, restore: true }

describe('SENTINEL', () => {
  it('leaves a character in the field for a delete to remove', () => {
    expect(SENTINEL).toHaveLength(1)
  })
})

describe('beforeinput', () => {
  it.each([
    ['a typed key', 'insertText', 'a'],
    ['a prediction', 'insertText', 'hello '],
    ['a replacement', 'insertReplacementText', 'world'],
  ])('sends %s as text', (_, inputType, data) => {
    expect(typing().beforeinput(typed(inputType, data))).toEqual(sent({ text: data }))
  })

  it('reads a replacement carried by its data transfer', () => {
    const replaced = typed('insertReplacementText', null, carrying('word'))
    expect(typing().beforeinput(replaced)).toEqual(sent({ text: 'word' }))
  })

  it.each([
    ['a line break', 'insertLineBreak', 'enter'],
    ['a paragraph', 'insertParagraph', 'enter'],
    ['a delete', 'deleteContentBackward', 'backspace'],
  ])('sends %s as the key %s', (_, inputType, code) => {
    expect(typing().beforeinput(typed(inputType))).toEqual(sent({ key: { code } }))
  })

  it('sends a delete with the field left at its sentinel', () => {
    const input = typing()
    input.beforeinput(typed('deleteContentBackward'))
    expect(input.beforeinput(typed('deleteContentBackward'))).toEqual(sent({ key: { code: 'backspace' } }))
  })

  it('sends a native paste as a paste', () => {
    expect(typing().beforeinput(typed('insertFromPaste', 'ls -la\n'))).toEqual(sent({ paste: 'ls -la\n' }))
  })

  it('reads a native paste carried by its data transfer', () => {
    expect(typing().beforeinput(typed('insertFromPaste', null, carrying('pwd')))).toEqual(sent({ paste: 'pwd' }))
  })

  it.each([
    ['an empty paste', typed('insertFromPaste', null, carrying(''))],
    ['an empty insert', typed('insertText', '')],
    ['an insert without data', typed('insertText')],
    ['a word delete', typed('deleteWordBackward')],
    ['a drop', typed('insertFromDrop', 'file')],
  ])('swallows %s and sends nothing', (_, event) => {
    expect(typing().beforeinput(event)).toEqual(swallowed)
  })

  it.each([
    ['composition text', typed('insertCompositionText', 'に')],
    ['an insert while composing', typed('insertText', 'に', { isComposing: true })],
  ])('lets %s through', (_, event) => {
    expect(typing().beforeinput(event)).toEqual(passed)
  })
})

describe('keydown', () => {
  it.each([
    ['Enter', 'enter'],
    ['Backspace', 'backspace'],
    ['ArrowUp', 'up'],
    ['Escape', 'esc'],
    ['Tab', 'tab'],
  ])('sends %s as the key %s', (key, code) => {
    expect(typing().keydown(press(key))).toEqual(sent({ key: { code } }))
  })

  it('sends Ctrl+c of a hardware keyboard', () => {
    expect(typing().keydown(press('c', { ctrlKey: true }))).toEqual(sent({ key: { code: 'c', mods: ['ctrl'] } }))
  })

  it.each([
    ['a letter', press('a')],
    ['a shortcut', press('v', { metaKey: true })],
    ['a key while composing', press('Enter', { isComposing: true })],
  ])('lets %s through', (_, pressed) => {
    expect(typing().keydown(pressed)).toEqual(passed)
  })
})

describe('Enter and delete', () => {
  it.each([
    ['Enter', 'insertLineBreak'],
    ['Enter', 'insertParagraph'],
    ['Backspace', 'deleteContentBackward'],
  ])('are sent once when %s comes with %s', (key, inputType) => {
    const input = typing()
    const steps = [input.keydown(press(key)), input.beforeinput(typed(inputType))]
    expect(steps.flatMap(({ send }) => send)).toHaveLength(1)
  })

  it('swallows the line break of an Enter already sent', () => {
    const input = typing()
    input.keydown(press('Enter'))
    expect(input.beforeinput(typed('insertLineBreak'))).toEqual(swallowed)
  })

  it('sends a later line break that no key sent', () => {
    const input = typing()
    input.keydown(press('Enter'))
    input.beforeinput(typed('insertText', 'a'))
    expect(input.beforeinput(typed('insertLineBreak'))).toEqual(sent({ key: { code: 'enter' } }))
  })

  it('sends a line break after a key that was not Enter', () => {
    const input = typing()
    input.keydown(press('Backspace'))
    expect(input.beforeinput(typed('insertLineBreak'))).toEqual(sent({ key: { code: 'enter' } }))
  })

  it('sends each of two Enters pressed in a row', () => {
    const input = typing()
    const steps = [input.keydown(press('Enter')), input.keydown(press('Enter'))]
    expect(steps.flatMap(({ send }) => send)).toEqual([{ key: { code: 'enter' } }, { key: { code: 'enter' } }])
  })
})

describe('composition', () => {
  const composing = () => {
    const input = typing()
    input.compositionstart()
    return input
  }

  it('sends nothing while it lasts', () => {
    const input = composing()
    const steps = [
      input.keydown(press('Enter')),
      input.beforeinput(typed('insertCompositionText', 'にほ')),
      input.beforeinput(typed('insertLineBreak')),
    ]
    expect(steps).toEqual([passed, passed, passed])
  })

  it('sends the final text once when it ends and restores the field', () => {
    expect(composing().compositionend('日本')).toEqual({ send: [{ text: '日本' }], prevent: false, restore: true })
  })

  it('sends nothing for a cancelled composition but restores the field', () => {
    expect(composing().compositionend('')).toEqual({ send: [], prevent: false, restore: true })
  })

  it('takes keys again once it ends', () => {
    const input = composing()
    input.compositionend('日本')
    expect(input.keydown(press('Enter'))).toEqual(sent({ key: { code: 'enter' } }))
  })
})

describe('sticky ctrl', () => {
  it('applies to a typed character', () => {
    expect(armed().typing.beforeinput(typed('insertText', 'c'))).toEqual(sent({ key: { code: 'c', mods: ['ctrl'] } }))
  })

  it('applies only to the first character of a prediction', () => {
    const step = armed().typing.beforeinput(typed('insertText', 'cat '))
    expect(step).toEqual(sent({ key: { code: 'c', mods: ['ctrl'] } }, { text: 'at ' }))
  })

  it('applies to a key of a hardware keyboard', () => {
    expect(armed().typing.keydown(press('ArrowLeft'))).toEqual(sent({ key: { code: 'left', mods: ['ctrl'] } }))
  })

  it('applies to the final text of a composition', () => {
    const { typing: input } = armed()
    input.compositionstart()
    expect(input.compositionend('d').send).toEqual([{ key: { code: 'd', mods: ['ctrl'] } }])
  })

  it('lets go after the key it applied to', () => {
    const { ctrl, typing: input } = armed()
    input.beforeinput(typed('insertText', 'c'))
    expect([ctrl.armed, input.beforeinput(typed('insertText', 'c'))]).toEqual([false, sent({ text: 'c' })])
  })

  it('stays armed through keys that send nothing', () => {
    const { ctrl, typing: input } = armed()
    input.keydown(press('Control', { ctrlKey: true }))
    input.beforeinput(typed('deleteWordBackward'))
    expect(ctrl.armed).toBe(true)
  })
})
