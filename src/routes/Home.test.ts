import { render } from 'svelte/server'
import { describe, expect, it } from 'vitest'
import type { ConnectionView_Serialize } from '../lib/bindings'
import { screen } from '../lib/connection'
import Home from './Home.svelte'
import homeSource from './Home.svelte?raw'

const SESSION = { boot: 'boot', revision: 1, version: '0.1.0', groups: [], projects: [] }

const draw = (connection: ConnectionView_Serialize, session: typeof SESSION | null = SESSION, below = 0) =>
  render(Home, { props: { screen: screen({ machine: 1, connection }, session), below } }).body

const CONNECTED: ConnectionView_Serialize = { state: 'connected', route: 'ws://mac:7000' }

describe('Home', () => {
  it('counts the panes of the session', () => {
    expect(draw(CONNECTED)).toMatch(/<p class="panes svelte-[\w-]+">0 panes<\/p>/)
  })

  it('leaves the panes out without a session', () => {
    expect(draw(CONNECTED, null)).not.toContain('class="panes')
  })

  it('pulses the mark while connecting', () => {
    expect(draw({ state: 'connecting' })).toMatch(/<span class="cell cursor svelte-[\w-]+ pulsing"><\/span>/)
  })

  it.each<[string, ConnectionView_Serialize]>([
    ['connected', CONNECTED],
    ['retrying', { state: 'retrying', in_ms: 2000, error: { code: 'connect', message: 'timed out' } }],
    ['unauthorized', { state: 'unauthorized' }],
    ['none', { state: 'none' }],
  ])('keeps the mark still while %s', (_, connection) => {
    expect(draw(connection)).not.toContain('pulsing')
  })

  it('starts below the connection bar', () => {
    expect(draw(CONNECTED, SESSION, 44)).toContain('<section class="home svelte-')
    expect(draw(CONNECTED, SESSION, 44)).toContain('style="--below: 44px;"')
    expect(homeSource.replace(/\s+/g, ' ')).toContain('padding: calc(var(--below) + 40px) var(--inset-wide) 0;')
  })
})
