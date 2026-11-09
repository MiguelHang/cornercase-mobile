import { render } from 'svelte/server'
import { describe, expect, it } from 'vitest'
import { ShownCtrl } from './ctrl.svelte'
import Typist from './Typist.svelte'
import typistSource from './Typist.svelte?raw'

const typist = (disabled = false) =>
  render(Typist, { props: { ctrl: new ShownCtrl(), send: () => {}, disabled } }).body

const flat = (text: string) => text.replace(/\s+/g, ' ')

describe('Typist', () => {
  it('is a plain field that holds the sentinel', () => {
    expect(typist()).toMatch(
      /<textarea class="typist svelte-[\w-]+" aria-label="Terminal input" rows="1" autocapitalize="off" autocorrect="off" autocomplete="off" spellcheck="false"> <\/textarea>/,
    )
  })

  it('is disabled on a closed tab', () => {
    expect(typist(true)).toMatch(/<textarea [^>]*disabled=""/)
  })

  it('sends nothing on a closed tab', () => {
    expect(typistSource).toContain('if (!disabled && step.send.length > 0) send(step.send)')
  })

  it.each([
    'onkeydown={(event) => run(event, typing.keydown(event))}',
    'onbeforeinput={(event) => run(event, typing.beforeinput(event))}',
    'oncompositionstart={() => typing.compositionstart()}',
    'oncompositionend={(event) => run(event, typing.compositionend(event.data))}',
  ])('leaves its events to Typing: %s', (text) => {
    expect(typistSource).toContain(text)
  })

  it('puts the sentinel back with the caret after it', () => {
    expect(flat(typistSource)).toContain(
      'field.value = SENTINEL field.setSelectionRange(SENTINEL.length, SENTINEL.length)',
    )
  })

  it.each(['font-size: 16px;', 'opacity: 0;', 'caret-color: transparent;', 'user-select: text;'])(
    'holds %s',
    (text) => {
      expect(flat(typistSource)).toContain(text)
    },
  )
})
