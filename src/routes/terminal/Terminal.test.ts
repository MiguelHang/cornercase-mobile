import { render } from 'svelte/server'
import { describe, expect, it } from 'vitest'
import type { AgentRow_Serialize as AgentRow } from '../../lib/bindings'
import Terminal from './Terminal.svelte'
import terminalSource from './Terminal.svelte?raw'

const ROW: AgentRow = {
  boot: 'b1',
  agent: 'claude',
  title: 'Fix the login flow',
  status: 'working',
  since: null,
  pane: 30,
  tab: 3,
  tab_name: 'fix login',
  workspace: 2,
  workspace_name: 'main',
  workspace_count: 2,
  project: 1,
  project_name: 'app',
}

const body = render(Terminal, {
  props: { machine: 1, row: ROW, onback: () => {}, onfailed: () => {} },
}).body.replaceAll('<!---->', '')

const source = terminalSource.replace(/\s+/g, ' ')

describe('the terminal screen', () => {
  it('names its agent, with the tab and place, under a button back', () => {
    expect(body).toMatch(/<button class="back svelte-[\w-]+" type="button" aria-label="Back"><svg/)
    expect(body).toMatch(/<span class="line name svelte-[\w-]+">Fix the login flow<\/span>/)
    expect(body).toMatch(/<span class="line where svelte-[\w-]+">fix login · app › main<\/span>/)
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
    expect(source).toContain('if (shows(event.payload, machine, row.tab)) { shown.apply(event.payload) }')
  })

  it('opens the pane of its agent and remembers the zoom on the boot of its server', () => {
    expect(source).toContain('const open = (size: Size_Deserialize) => commands.terminalOpen(row.tab, row.pane, size)')
    expect(source).toContain('place={{ machine, boot: row.boot, tab: row.tab }}')
  })

  it('leaves the order of opening and closing to an Opening that waits for it to listen', () => {
    expect(source).toContain('const close = () => commands.terminalClose(row.tab)')
    expect(source).toContain('new Opening(unlisten, { open, close, failed: onfailed })')
    expect(source).toContain('opening?.open(wanted)')
    expect(source).toContain('opening?.leave()')
  })
})
