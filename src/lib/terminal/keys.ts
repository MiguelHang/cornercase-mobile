import type { InputEvent_Deserialize, Modifier } from '../bindings'

export type InputEvent = InputEvent_Deserialize

export type RowKey =
  | { kind: 'key'; label: string; event: InputEvent }
  | { kind: 'ctrl'; label: string }
  | { kind: 'paste'; label: string }

export type Pressed = Pick<KeyboardEvent, 'key' | 'ctrlKey' | 'altKey' | 'shiftKey' | 'metaKey'>

const key = (code: string, mods: Modifier[] = []): InputEvent => ({ key: mods.length > 0 ? { code, mods } : { code } })

const text = (typed: string): InputEvent => ({ text: typed })

export const ROW: RowKey[] = [
  { kind: 'key', label: 'Esc', event: key('esc') },
  { kind: 'key', label: 'Tab', event: key('tab') },
  { kind: 'ctrl', label: 'Ctrl' },
  { kind: 'key', label: '↑', event: key('up') },
  { kind: 'key', label: '↓', event: key('down') },
  { kind: 'key', label: '←', event: key('left') },
  { kind: 'key', label: '→', event: key('right') },
  { kind: 'key', label: '|', event: text('|') },
  { kind: 'key', label: '~', event: text('~') },
  { kind: 'key', label: '/', event: text('/') },
  { kind: 'key', label: '-', event: text('-') },
  { kind: 'paste', label: 'Paste' },
]

export const pasted = (clipboard: string): InputEvent | null => (clipboard === '' ? null : { paste: clipboard })

const NAMED = new Map([
  ['ArrowUp', 'up'],
  ['ArrowDown', 'down'],
  ['ArrowLeft', 'left'],
  ['ArrowRight', 'right'],
  ['Escape', 'esc'],
  ['Tab', 'tab'],
  ['Enter', 'enter'],
  ['Backspace', 'backspace'],
])

const held = ({ ctrlKey, altKey, shiftKey }: Pressed): Modifier[] =>
  (
    [
      [ctrlKey, 'ctrl'],
      [altKey, 'alt'],
      [shiftKey, 'shift'],
    ] as const
  )
    .filter(([down]) => down)
    .map(([, modifier]) => modifier)

const single = (typed: string) => [...typed].length === 1

export const physical = (pressed: Pressed): InputEvent | null => {
  if (pressed.metaKey) return null
  const code = NAMED.get(pressed.key) ?? (pressed.ctrlKey && single(pressed.key) ? pressed.key : undefined)
  return code === undefined ? null : key(code, held(pressed))
}

const withCtrl = (event: InputEvent): InputEvent[] => {
  if ('key' in event && event.key !== undefined) {
    const mods = event.key.mods ?? []
    return [{ key: { ...event.key, mods: mods.includes('ctrl') ? mods : ['ctrl', ...mods] } }]
  }
  if ('text' in event && event.text !== undefined && event.text !== '') {
    const [first, ...rest] = event.text
    return rest.length > 0 ? [key(first, ['ctrl']), text(rest.join(''))] : [key(first, ['ctrl'])]
  }
  return [event]
}

export class StickyCtrl {
  #armed = false

  get armed() {
    return this.#armed
  }

  toggle() {
    this.#armed = !this.#armed
    return this.#armed
  }

  apply(events: InputEvent[]): InputEvent[] {
    if (!this.#armed || events.length === 0) return events
    this.#armed = false
    const [first, ...rest] = events
    return [...withCtrl(first), ...rest]
  }
}
