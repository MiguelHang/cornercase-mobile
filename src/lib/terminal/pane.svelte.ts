import type {
  Colours_Serialize,
  Cursor_Serialize,
  Modes_Serialize,
  Run_Serialize,
  Size_Serialize,
  TerminalChanged_Serialize,
} from '../bindings'

export type Row = Run_Serialize[]

export type PaneState = {
  pane: number | null
  size: Size_Serialize
  cursor: Cursor_Serialize | null
  colours: Colours_Serialize
  modes: Modes_Serialize
  rows: Row[]
  closed: boolean
}

export const empty: PaneState = {
  pane: null,
  size: { cols: 0, rows: 0 },
  cursor: null,
  colours: { fg: null, bg: null, cursor: null, palette: {} },
  modes: {
    mouse: 'none',
    mouse_encoding: 'default',
    alternate_screen: false,
    alternate_scroll: false,
    bracketed_paste: false,
    application_cursor: false,
    focus_events: false,
    kitty_keyboard: 0,
  },
  rows: [],
  closed: false,
}

const blank = (count: number): Row[] => Array.from({ length: Math.max(0, count) }, () => [])

const kept = (state: PaneState, changed: TerminalChanged_Serialize): Row[] => {
  const count = changed.size.rows
  if (changed.replaced || changed.pane !== state.pane) {
    return blank(count)
  }
  return [...state.rows.slice(0, count), ...blank(count - state.rows.length)]
}

const drawn = (state: PaneState, changed: TerminalChanged_Serialize): Row[] => {
  const rows = kept(state, changed)
  for (const [key, runs] of Object.entries(changed.rows)) {
    const row = Number(key)
    if (row < rows.length) {
      rows[row] = runs
    }
  }
  return rows
}

const steady = (before: Colours_Serialize, after: Colours_Serialize) =>
  JSON.stringify(before) === JSON.stringify(after) ? before : after

export const apply = (state: PaneState, changed: TerminalChanged_Serialize): PaneState => {
  if (changed.pane === null) {
    return { ...state, closed: changed.closed }
  }
  const { pane, size, cursor, colours, modes, closed } = changed
  return { pane, size, cursor, colours: steady(state.colours, colours), modes, rows: drawn(state, changed), closed }
}

export const shows = (changed: TerminalChanged_Serialize, machine: number, tab: number) =>
  changed.machine === machine && changed.tab === tab

export class Pane {
  #state: PaneState = $state.raw(empty)

  get current() {
    return this.#state
  }

  apply(changed: TerminalChanged_Serialize) {
    this.#state = apply(this.#state, changed)
  }
}
