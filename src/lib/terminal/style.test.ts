import { describe, expect, it } from 'vitest'
import type { Colour, Colours_Serialize, Style_Serialize, UnderlineKind_Serialize } from '../bindings'
import * as style from './style'

const plain: Style_Serialize = {
  fg: null,
  bg: null,
  bold: false,
  dim: false,
  italic: false,
  underline: false,
  underline_style: null,
  underline_colour: null,
  overline: false,
  blink: false,
  inverse: false,
  invisible: false,
  strikethrough: false,
}

const none: Colours_Serialize = { fg: null, bg: null, cursor: null, palette: {} }

const own: Colours_Serialize = { fg: [1, 2, 3], bg: [4, 5, 6], cursor: null, palette: { '1': [9, 9, 9] } }

const styled = (changes: Partial<Style_Serialize>) => ({ ...plain, ...changes })

describe('resolve', () => {
  it.each<[string, number, style.Rgb]>([
    ['the first ANSI colour', 0, [21, 22, 30]],
    ['a normal ANSI colour', 4, [122, 162, 247]],
    ['a bright ANSI colour as its normal one', 9, [247, 118, 142]],
    ['bright white as the foreground', 15, style.FOREGROUND],
    ['the first colour of the cube', 16, [0, 0, 0]],
    ['a colour inside the cube', 67, [95, 135, 175]],
    ['the last colour of the cube', 231, [255, 255, 255]],
    ['the first grey', 232, [8, 8, 8]],
    ['the last grey', 255, [238, 238, 238]],
  ])('reads %s from Tokyo Night', (_, index, expected) => {
    expect(style.resolve({ index }, none)).toEqual(expected)
  })

  it('reads an index the pane overrides from its palette', () => {
    expect(style.resolve({ index: 1 }, own)).toEqual([9, 9, 9])
  })

  it('reads an index the pane does not override from Tokyo Night', () => {
    expect(style.resolve({ index: 2 }, own)).toEqual([158, 206, 106])
  })

  it('takes an rgb colour as it is', () => {
    expect(style.resolve({ rgb: [7, 8, 9] }, own)).toEqual([7, 8, 9])
  })
})

describe('paneColours', () => {
  it('falls back to Tokyo Night, with the cursor in the foreground', () => {
    expect(style.paneColours(none)).toEqual({ fg: style.FOREGROUND, bg: style.BACKGROUND, cursor: style.FOREGROUND })
  })

  it('takes the colours the pane sends', () => {
    const colours: Colours_Serialize = { ...own, cursor: [7, 7, 7] }
    expect(style.paneColours(colours)).toEqual({ fg: [1, 2, 3], bg: [4, 5, 6], cursor: [7, 7, 7] })
  })
})

describe('runCss', () => {
  const red: Colour = { index: 1 }

  it('paints a plain run in the foreground of the pane and leaves its background to the pane', () => {
    expect(style.runCss(plain, own)).toEqual({ color: 'rgb(1 2 3)' })
  })

  it('paints the colours a run sets', () => {
    expect(style.runCss(styled({ fg: { rgb: [7, 8, 9] }, bg: red }), own)).toEqual({
      color: 'rgb(7 8 9)',
      background: 'rgb(9 9 9)',
    })
  })

  it('swaps the colours of an inverse run', () => {
    const css = style.runCss(styled({ fg: { rgb: [7, 8, 9] }, bg: red, inverse: true }), own)
    expect(css).toEqual({ color: 'rgb(9 9 9)', background: 'rgb(7 8 9)' })
  })

  it('swaps the colours of the pane for an inverse run without its own', () => {
    expect(style.runCss(styled({ inverse: true }), none)).toEqual({
      color: style.rgb(style.BACKGROUND),
      background: style.rgb(style.FOREGROUND),
    })
  })

  it('fades a dim run', () => {
    expect(style.runCss(styled({ dim: true }), own).color).toBe('rgb(1 2 3 / 0.6)')
  })

  it('fades a dim inverse run after swapping', () => {
    expect(style.runCss(styled({ dim: true, inverse: true }), own)).toEqual({
      color: 'rgb(4 5 6 / 0.6)',
      background: 'rgb(1 2 3)',
    })
  })

  it('hides the text of an invisible run and keeps its background', () => {
    const css = style.runCss(styled({ invisible: true, dim: true, bg: red }), own)
    expect(css).toEqual({ color: 'rgb(1 2 3 / 0)', background: 'rgb(9 9 9)' })
  })

  it('makes a run bold and italic', () => {
    const css = style.runCss(styled({ bold: true, italic: true }), own)
    expect(css).toMatchObject({ 'font-weight': 'bold', 'font-style': 'italic' })
  })

  it('draws a curly underline wavy, in its own colour', () => {
    const css = style.runCss(styled({ underline: true, underline_style: 'curly', underline_colour: red }), own)
    expect(css).toMatchObject({
      'text-decoration-line': 'underline',
      'text-decoration-style': 'wavy',
      'text-decoration-color': 'rgb(9 9 9)',
    })
  })

  it.each<[UnderlineKind_Serialize, string]>([
    ['double', 'double'],
    ['dotted', 'dotted'],
    ['dashed', 'dashed'],
  ])('draws a %s underline as CSS does', (kind, drawn) => {
    const css = style.runCss(styled({ underline: true, underline_style: kind }), own)
    expect(css['text-decoration-style']).toBe(drawn)
  })

  it('draws an underline of another kind as a plain one', () => {
    const css = style.runCss(styled({ underline: true, underline_style: 'other' }), own)
    expect(css).toEqual({ color: 'rgb(1 2 3)', 'text-decoration-line': 'underline' })
  })

  it('draws every line a run asks for at once', () => {
    const css = style.runCss(styled({ underline: true, overline: true, strikethrough: true }), own)
    expect(css['text-decoration-line']).toBe('underline overline line-through')
  })

  it('ignores the underline kind and colour of a run that is not underlined', () => {
    const css = style.runCss(styled({ strikethrough: true, underline_style: 'curly', underline_colour: red }), own)
    expect(css).toEqual({ color: 'rgb(1 2 3)', 'text-decoration-line': 'line-through' })
  })
})

describe('inline', () => {
  it('writes the declarations for a style attribute', () => {
    expect(style.inline({ color: 'rgb(1 2 3)', 'font-weight': 'bold' })).toBe('color: rgb(1 2 3); font-weight: bold')
  })
})
