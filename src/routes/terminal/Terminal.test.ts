import { render } from 'svelte/server'
import { describe, expect, it, vi } from 'vitest'
import type { AgentRow_Serialize as AgentRow, Size_Deserialize } from '../../lib/bindings'
import { Keyboard, whileClosed } from '../../lib/terminal/keyboard.svelte'
import { Opening } from '../../lib/terminal/opening'
import { empty, type PaneState } from '../../lib/terminal/pane.svelte'
import { Sending, type Sent } from '../../lib/terminal/sending'
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

describe('typing and the keys on the terminal screen', () => {
  it('mounts the hidden field and the row of keys on one Ctrl, both off on a closed tab', () => {
    expect(body).toMatch(/<section class="screen svelte-[\w-]+"><textarea class="typist svelte-[\w-]+"/)
    expect(body).toMatch(/<div class="keys svelte-[\w-]+" role="toolbar" aria-label="Terminal keys">/)
    expect(source).toContain('const ctrl = new ShownCtrl()')
    expect(source).toContain('<Typist bind:this={typist} {ctrl} {send} disabled={shown.current.closed} />')
    expect(source).toContain('<KeyRow {ctrl} {send} {onfailed} disabled={shown.current.closed} />')
  })

  it('focuses the field on a tap, and sends swipes and scrolls to the shown pane', () => {
    expect(source).toContain('ontap={() => typist?.focus()}')
    expect(source).toContain('onsend={send} onscroll={(rowsUp) => sending.scroll(rowsUp)}')
    expect(source).toContain('input: commands.terminalInput, scroll: commands.terminalScroll,')
  })

  it('follows the keyboard while it watches, and asks for a size only with it closed', () => {
    expect(source).toContain('const unwatch = keyboard.watch() return () => { unwatch()')
    expect(source).toContain('style:height={keyboard.open ? `${keyboard.height}px` : undefined}')
    expect(source).toContain('style:top={keyboard.open ? `${keyboard.offset}px` : undefined}')
    expect(source).toContain('const fitted = whileClosed(keyboard, (size: Size_Deserialize) => (wanted = size))')
    expect(source).toContain('onfit={fitted}')
    expect(source).toContain('.keyboard { padding-bottom: 0; }')
  })
})

const FULL = 900

const settled = () => new Promise((resolve) => setTimeout(resolve, 0))

describe('a keyboard opening and closing', () => {
  const viewport = { height: FULL, offsetTop: 0, resized: () => {} }
  const screen = {
    innerHeight: FULL,
    visualViewport: {
      get height() {
        return viewport.height
      },
      offsetTop: 0,
      addEventListener: (_: string, listener: () => void) => (viewport.resized = listener),
      removeEventListener: () => {},
    },
    addEventListener: () => {},
    removeEventListener: () => {},
  }

  const resize = (height: number) => {
    viewport.height = height
    viewport.resized()
  }

  it('does not open the pane again with the size of the smaller screen', async () => {
    const keyboard = new Keyboard(screen)
    keyboard.watch()
    const open = vi.fn(() => Promise.resolve<Sent>({ status: 'ok' }))
    const opening = new Opening(Promise.resolve(), { open, close: () => Promise.resolve(), failed: () => {} })
    const fitted = whileClosed(keyboard, (size: Size_Deserialize) => opening.open(size))
    fitted({ cols: 40, rows: 50 })
    resize(FULL - 340)
    fitted({ cols: 40, rows: 31 })
    resize(FULL)
    fitted({ cols: 40, rows: 50 })
    await settled()
    expect(open.mock.calls).toEqual([[{ cols: 40, rows: 50 }]])
  })
})

describe('sending from the terminal screen', () => {
  const OK: Sent = { status: 'ok' }

  const sends = (answer: () => Promise<Sent> = () => Promise.resolve(OK)) => ({
    input: vi.fn(answer),
    scroll: vi.fn(answer),
    failed: vi.fn(),
  })

  const showing = (shown: Partial<PaneState>) => ({ current: { ...empty, ...shown } })

  it('sends what is typed to the pane the screen shows', () => {
    const calls = sends()
    new Sending(showing({ pane: 30 }), calls).input([{ text: 'ls' }])
    expect(calls.input.mock.calls).toEqual([[30, [{ text: 'ls' }]]])
  })

  it('sends a swipe through the history to the pane the screen shows', () => {
    const calls = sends()
    new Sending(showing({ pane: 30 }), calls).scroll(-3)
    expect(calls.scroll.mock.calls).toEqual([[30, -3]])
  })

  it('follows the pane the screen shows as it changes', () => {
    const calls = sends()
    const shown = showing({ pane: 30 })
    const sending = new Sending(shown, calls)
    shown.current = { ...shown.current, pane: 31 }
    sending.input([{ text: 'a' }])
    expect(calls.input.mock.calls).toEqual([[31, [{ text: 'a' }]]])
  })

  it.each([
    ['a closed tab', { pane: 30, closed: true }],
    ['a pane not shown yet', { pane: null }],
  ])('sends nothing to %s', (_, shown) => {
    const calls = sends()
    const sending = new Sending(showing(shown), calls)
    sending.input([{ text: 'ls' }])
    sending.scroll(2)
    expect([calls.input, calls.scroll].map((call) => call.mock.calls.length)).toEqual([0, 0])
  })

  it('sends nothing when there is nothing to send', () => {
    const calls = sends()
    const sending = new Sending(showing({ pane: 30 }), calls)
    sending.input([])
    sending.scroll(0)
    expect([calls.input, calls.scroll].map((call) => call.mock.calls.length)).toEqual([0, 0])
  })

  it.each([
    ['a refused input', (sending: Sending) => sending.input([{ text: 'ls' }])],
    ['a refused scroll', (sending: Sending) => sending.scroll(1)],
  ])('turns %s into a toast', async (_, send) => {
    const error = { code: 'conflict', message: 'the tab is gone' }
    const calls = sends(() => Promise.resolve({ status: 'error', error }))
    send(new Sending(showing({ pane: 30 }), calls))
    await settled()
    expect(calls.failed.mock.calls).toEqual([[error]])
  })

  it('turns a call that fails into a toast', async () => {
    const error = new Error('the app is gone')
    const calls = sends(() => Promise.reject(error))
    new Sending(showing({ pane: 30 }), calls).input([{ text: 'ls' }])
    await settled()
    expect(calls.failed.mock.calls).toEqual([[error]])
  })

  it('says nothing when the pane takes it', async () => {
    const calls = sends()
    new Sending(showing({ pane: 30 }), calls).input([{ text: 'ls' }])
    await settled()
    expect(calls.failed).not.toHaveBeenCalled()
  })
})
