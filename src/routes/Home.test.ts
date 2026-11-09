import { render } from 'svelte/server'
import { describe, expect, it } from 'vitest'
import type { ConnectionView_Serialize, Snapshot_Serialize } from '../lib/bindings'
import { screen } from '../lib/connection'
import Home from './Home.svelte'
import homeSource from './Home.svelte?raw'

const SESSION: Snapshot_Serialize = { boot: 'boot', revision: 1, version: '0.1.0', groups: [], projects: [] }

const draw = (connection: ConnectionView_Serialize, session: Snapshot_Serialize | null = SESSION, below = 0) =>
  render(Home, { props: { screen: screen({ machine: 1, connection }, session), below, onopen: () => {} } }).body

const pane = (id: number, agent: string | null) => ({
  id,
  agent,
  status: null,
  program: null,
  title: null,
  path: null,
  parent: null,
  active: false,
  dialog: false,
  at_prompt: null,
})

const tab = (id: number, name: string, agent: string | null) => ({
  id,
  name,
  custom_name: null,
  status: null,
  active: false,
  panes: [pane(id * 10, agent)],
})

const WITH_AGENTS: Snapshot_Serialize = {
  ...SESSION,
  projects: [
    {
      id: 1,
      name: 'app',
      custom_name: null,
      path: null,
      group: null,
      workspaces: [
        {
          id: 2,
          name: 'main',
          custom_name: null,
          path: null,
          branch: null,
          worktree: false,
          tabs: [tab(3, 'fix login', 'claude'), tab(4, 'shell', null), tab(5, 'docs', 'codex')],
        },
      ],
    },
  ],
}

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

  it('lists the tabs where an agent runs, each a button with its agent and place', () => {
    const body = draw(CONNECTED, WITH_AGENTS).replaceAll('<!---->', '')
    const buttons = [...body.matchAll(/<button class="tab svelte-[\w-]+" type="button">(.*?)<\/button>/g)]
    const texts = buttons.map(([, inside]) => inside.replace(/<[^>]+>/g, ' ').replace(/\s+/g, ' ').trim())
    expect(texts).toEqual(['fix login claude · app · main', 'docs codex · app · main'])
  })

  it('leaves the list out while no tab runs an agent', () => {
    expect(draw(CONNECTED)).not.toContain('class="tabs')
  })

  it('opens the tab that is pressed', () => {
    expect(homeSource).toContain('onclick={() => onopen(tab)}')
  })

  it('starts below the connection bar', () => {
    expect(draw(CONNECTED, SESSION, 44)).toContain('<section class="home svelte-')
    expect(draw(CONNECTED, SESSION, 44)).toContain('style="--below: 44px;"')
    expect(homeSource.replace(/\s+/g, ' ')).toContain('padding: calc(var(--below) + 40px) var(--inset-wide) 0;')
  })
})
