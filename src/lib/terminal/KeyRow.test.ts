import { render } from 'svelte/server'
import { describe, expect, it } from 'vitest'
import { ShownCtrl } from './ctrl.svelte'
import { ROW } from './keys'
import KeyRow from './KeyRow.svelte'
import keyRowSource from './KeyRow.svelte?raw'

const row = ({ armed = false, disabled = false } = {}) => {
  const ctrl = new ShownCtrl()
  if (armed) ctrl.toggle()
  return render(KeyRow, { props: { ctrl, send: () => {}, onfailed: () => {}, disabled } }).body.replaceAll(
    /<!--[^>]*-->/g,
    '',
  )
}

const flat = (text: string) => text.replace(/\s+/g, ' ')

describe('KeyRow', () => {
  it('lays every key of the row in a toolbar', () => {
    const body = row()
    expect(body).toMatch(/<div class="keys svelte-[\w-]+" role="toolbar" aria-label="Terminal keys">/)
    const labels = [...body.matchAll(/<button([^>]*)>(.*?)<\/button>/gs)].map(
      ([, attributes, content]) => /aria-label="([^"]*)"/.exec(attributes)?.[1] ?? content,
    )
    expect(labels).toEqual(ROW.map((key) => key.label))
  })

  it('draws Paste as an icon named for screen readers', () => {
    expect(row()).toMatch(/<button class="key svelte-[\w-]+" type="button" aria-label="Paste"><svg [^>]*width="20"/)
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

  it('acts as the finger lifts, which lets Paste read the clipboard', () => {
    expect(keyRowSource).toContain('onpointerup={() => pressed(key)}')
    expect(keyRowSource).not.toContain('onpointerdown')
  })

  it('acts on a click without a pointer, as from a hardware keyboard or VoiceOver', () => {
    expect(keyRowSource).toContain('onclick={(event) => byKeyboard(event) && pressed(key)}')
  })

  it('cancels the click of a touch, so the field keeps the focus and the keyboard stays open', () => {
    expect(keyRowSource).toContain('ontouchend={(event) => event.preventDefault()}')
  })

  it.each(['.key { display: flex; flex: 1 1 0; align-items: center; justify-content: center; min-width: 0; min-height: 44px;', '.armed { border-color: var(--accent);'])(
    'holds %s',
    (text) => {
      expect(flat(keyRowSource)).toContain(text)
    },
  )

  it('gives :active to touches', () => {
    expect(keyRowSource).toContain('ontouchstart={() => {}}')
  })
})
