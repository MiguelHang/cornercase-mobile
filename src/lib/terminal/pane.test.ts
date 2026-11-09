import { describe, expect, it } from 'vitest'
import type {
  Colours_Serialize,
  Cursor_Serialize,
  Modes_Serialize,
  Style_Serialize,
  TerminalChanged_Serialize,
} from '../bindings'
import { apply, empty, Pane, shows, type PaneState } from './pane.svelte'

const COLOURS: Colours_Serialize = { fg: [220, 220, 220], bg: [16, 16, 16], cursor: null, palette: { '1': [255, 0, 0] } }

const CURSOR: Cursor_Serialize = { x: 2, y: 1, shape: 'bar', blinking: true }

const MOUSE: Modes_Serialize = { ...empty.modes, mouse: 'press_release', mouse_encoding: 'sgr' }

const STYLE: Style_Serialize = {
  fg: null,
  bg: null,
  underline_style: null,
  underline_colour: null,
  bold: false,
  dim: false,
  italic: false,
  underline: false,
  overline: false,
  blink: false,
  inverse: false,
  invisible: false,
  strikethrough: false,
}

const runs = (text: string) => [{ text, width: text.length, style: STYLE }]

const changed = (rows: Record<number, string>, partial: Partial<TerminalChanged_Serialize> = {}) => {
  const event: TerminalChanged_Serialize = {
    machine: 7,
    tab: 3,
    pane: 1,
    size: { cols: 80, rows: 3 },
    cursor: CURSOR,
    colours: COLOURS,
    modes: empty.modes,
    replaced: false,
    rows: Object.fromEntries(Object.entries(rows).map(([row, text]) => [row, runs(text)])),
    closed: false,
    ...partial,
  }
  return event
}

const applied = (...events: TerminalChanged_Serialize[]) => events.reduce(apply, empty)

const texts = (state: PaneState) => state.rows.map((row) => row.map((run) => run.text).join(''))

describe('apply', () => {
  it('draws the rows it gets by their number and leaves the others empty', () => {
    expect(texts(applied(changed({ 0: 'a', 2: 'c' }, { replaced: true })))).toEqual(['a', '', 'c'])
  })

  it('changes only the rows a later event brings', () => {
    const state = applied(changed({ 0: 'a', 1: 'b', 2: 'c' }, { replaced: true }), changed({ 1: 'x' }))
    expect(texts(state)).toEqual(['a', 'x', 'c'])
  })

  it('keeps the rows a later event did not change as they were', () => {
    const first = applied(changed({ 0: 'a', 1: 'b' }, { replaced: true }))
    expect(apply(first, changed({ 1: 'x' })).rows[0]).toBe(first.rows[0])
  })

  it('ignores a row past the size of the pane', () => {
    expect(texts(applied(changed({ 0: 'a', 3: 'd', 40: 'e' }, { replaced: true })))).toEqual(['a', '', ''])
  })

  it('empties every row before drawing an event that replaced them', () => {
    const state = applied(changed({ 0: 'a', 1: 'b', 2: 'c' }), changed({ 1: 'x' }, { replaced: true }))
    expect(texts(state)).toEqual(['', 'x', ''])
  })

  it('starts again from empty rows when another pane shows', () => {
    const state = applied(changed({ 0: 'a', 1: 'b' }), changed({ 2: 'z' }, { pane: 2 }))
    expect([state.pane, texts(state)]).toEqual([2, ['', '', 'z']])
  })

  it.each<[string, number, string[]]>([
    ['cuts the rows when the pane gets shorter', 2, ['a', 'b']],
    ['adds empty rows when the pane gets taller', 4, ['a', 'b', 'c', '']],
  ])('%s', (_, rows, expected) => {
    const first = changed({ 0: 'a', 1: 'b', 2: 'c' })
    expect(texts(applied(first, changed({}, { size: { cols: 80, rows } })))).toEqual(expected)
  })

  it('keeps the size, cursor and colours of the latest event', () => {
    const size = { cols: 51, rows: 5 }
    const state = applied(changed({}), changed({}, { size, cursor: null, colours: empty.colours }))
    expect([state.size, state.cursor, state.colours]).toEqual([size, null, empty.colours])
  })

  it('keeps the modes of the latest event', () => {
    const alternate = { ...empty.modes, alternate_screen: true, alternate_scroll: true }
    expect(applied(changed({}, { modes: MOUSE }), changed({}, { modes: alternate })).modes).toEqual(alternate)
  })

  it('keeps the modes it had when an event only says the tab closed', () => {
    const state = applied(changed({}, { modes: MOUSE }), changed({}, { pane: null, closed: true }))
    expect(state.modes).toEqual(MOUSE)
  })

  it('keeps the same colours while an event brings equal ones', () => {
    const first = applied(changed({}))
    expect(apply(first, changed({}, { colours: { ...COLOURS, palette: { '1': [255, 0, 0] } } })).colours).toBe(
      first.colours,
    )
  })

  it('keeps the screen it showed when an event only says the tab closed', () => {
    const shown = applied(changed({ 0: 'a' }))
    const closed = apply(shown, changed({}, { pane: null, size: { cols: 0, rows: 0 }, cursor: null, closed: true }))
    expect(closed).toEqual({ ...shown, closed: true })
  })

  it('closes with the rows that came with it', () => {
    const state = applied(changed({ 0: 'a' }, { closed: true }))
    expect([state.closed, texts(state)]).toEqual([true, ['a', '', '']])
  })
})

describe('shows', () => {
  it.each<[string, number, number, boolean]>([
    ['takes an event of its machine and tab', 7, 3, true],
    ['leaves out an event of another tab', 7, 4, false],
    ['leaves out an event of another machine', 8, 3, false],
  ])('%s', (_, machine, tab, expected) => {
    expect(shows(changed({ 0: 'a' }), machine, tab)).toBe(expected)
  })
})

describe('Pane', () => {
  it('starts empty', () => {
    expect(new Pane().current).toEqual(empty)
  })

  it('starts with the modes of a plain shell until an event brings others', () => {
    expect(new Pane().current.modes).toEqual({
      mouse: 'none',
      mouse_encoding: 'default',
      alternate_screen: false,
      alternate_scroll: false,
      bracketed_paste: false,
      application_cursor: false,
      focus_events: false,
      kitty_keyboard: 0,
    })
  })

  it('applies each event to what it shows', () => {
    const pane = new Pane()
    pane.apply(changed({ 0: 'a' }))
    pane.apply(changed({ 1: 'b' }))
    expect(texts(pane.current)).toEqual(['a', 'b', ''])
  })
})
