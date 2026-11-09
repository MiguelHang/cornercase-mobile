import { render } from 'svelte/server'
import { describe, expect, it } from 'vitest'
import Terminal from './Terminal.svelte'
import terminalSource from './Terminal.svelte?raw'

const TAB = { tab: 3, pane: 30, name: 'fix login', agent: 'claude', where: 'app · main' }

const body = render(Terminal, {
  props: { machine: 1, boot: 'b1', tab: TAB, onback: () => {}, onfailed: () => {} },
}).body.replaceAll('<!---->', '')

const source = terminalSource.replace(/\s+/g, ' ')

describe('the terminal screen', () => {
  it('names the tab with its agent and place, under a button back', () => {
    expect(body).toMatch(/<button class="back svelte-[\w-]+" type="button" aria-label="Back"><svg/)
    expect(body).toMatch(/<span class="line name svelte-[\w-]+">fix login<\/span>/)
    expect(body).toMatch(/<span class="line where svelte-[\w-]+">claude · app · main<\/span>/)
  })

  it('draws the pane in the space left under its bar', () => {
    expect(body).toMatch(/<div class="view svelte-[\w-]+"><div class="terminal svelte-[\w-]+"/)
    expect(source).toContain('.view { flex: 1; min-height: 0; }')
  })

  it('says nothing about a closed tab before it closes', () => {
    expect(body).not.toContain('class="closed')
    expect(source).toContain(
      '{#if shown.current.closed} <p class="closed" role="status">This tab was closed on the Mac</p> {/if}',
    )
  })

  it('takes only the changes of its machine and tab', () => {
    expect(source).toContain('if (shows(event.payload, machine, tab.tab)) { shown.apply(event.payload) }')
  })

  it('leaves the order of opening and closing to an Opening that waits for it to listen', () => {
    expect(source).toContain('const close = () => commands.terminalClose(tab.tab)')
    expect(source).toContain('new Opening(unlisten, { open, close, failed: onfailed })')
    expect(source).toContain('opening?.open(wanted)')
    expect(source).toContain('opening?.leave()')
  })
})
