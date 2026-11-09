import { render } from 'svelte/server'
import { describe, expect, it } from 'vitest'
import { ShownCtrl } from './ctrl.svelte'
import { ROW } from './keys'
import KeyRow from './KeyRow.svelte'
import keyRowSource from './KeyRow.svelte?raw'

const row = ({ armed = false, disabled = false } = {}) => {
  const ctrl = new ShownCtrl()
  if (armed) ctrl.toggle()
  return render(KeyRow, { props: { ctrl, send: () => {}, onfailed: () => {}, disabled } }).body
}

const flat = (text: string) => text.replace(/\s+/g, ' ')

describe('KeyRow', () => {
  it('lays every key of the row in a toolbar', () => {
    const body = row()
    expect(body).toMatch(/<div class="keys svelte-[\w-]+" role="toolbar" aria-label="Terminal keys">/)
    expect([...body.matchAll(/<button[^>]*>([^<]*)<\/button>/g)].map(([, label]) => label)).toEqual(
      ROW.map((key) => key.label),
    )
  })

  it.each([
    ['let go', false, /<button class="key svelte-[\w-]+" type="button" aria-pressed="false">Ctrl/],
    ['armed', true, /<button class="key svelte-[\w-]+ armed" type="button" aria-pressed="true">Ctrl/],
  ])('shows Ctrl %s', (_, armed, ctrl) => {
    expect(row({ armed })).toMatch(ctrl)
  })

  it('disables every key on a closed tab', () => {
    expect(row({ disabled: true }).match(/disabled=""/g)).toHaveLength(ROW.length)
    expect(row()).not.toContain('disabled')
  })

  it('acts on the press and keeps the focus in the field, so the keyboard stays open', () => {
    expect(keyRowSource).toContain('onpointerdown={(event) => pressed(event, key)}')
    expect(flat(keyRowSource)).toContain(
      'event.preventDefault() press(key, { ctrl, disabled, send, failed: onfailed, clipboard: navigator.clipboard })',
    )
  })

  it.each(['.key { flex: 1 1 0; min-width: 0; min-height: 44px;', '.armed { border-color: var(--accent);'])(
    'holds %s',
    (text) => {
      expect(flat(keyRowSource)).toContain(text)
    },
  )

  it('gives :active to touches', () => {
    expect(keyRowSource).toContain('ontouchstart={() => {}}')
  })
})
