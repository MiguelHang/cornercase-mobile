import { render } from 'svelte/server'
import { describe, expect, it } from 'vitest'
import type { AgentRow_Serialize as AgentRow, ConnectionView_Serialize } from '../../lib/bindings'
import Agent from './Agent.svelte'
import agentSource from './Agent.svelte?raw'

const CONNECTED: ConnectionView_Serialize = { state: 'connected', route: 'ws://mac:7000' }

const ROW: AgentRow = {
  boot: 'b1',
  agent: 'claude',
  title: 'Fix the login flow',
  status: 'waiting',
  since: '2026-11-09T11:40:00.000Z',
  pane: 12,
  tab: 4,
  tab_name: 'agent',
  workspace: 9,
  workspace_name: 'login',
  workspace_count: 2,
  project: 5,
  project_name: 'web',
}

const draw = (row: AgentRow | null | undefined, below = 0, connection: ConnectionView_Serialize = CONNECTED) =>
  render(Agent, { props: { row, connection, below, onback: () => {}, onterminal: () => {} } }).body

const TERMINAL = /<button class="button plain svelte-[\w-]+" type="button"[^>]*>(?:<!--[^>]*-->| )*Terminal/

describe('Agent', () => {
  it('shows the status, title and place of its agent', () => {
    const body = draw(ROW)
    expect(body).toContain('aria-label="Waiting"')
    expect(body).toMatch(/<h1 class="title svelte-[\w-]+">Fix the login flow<\/h1>/)
    expect(body).toMatch(/<p class="place svelte-[\w-]+">web › login<\/p>/)
    expect(body).not.toContain('no longer running')
  })

  it('names the agent when it has no title', () => {
    expect(draw({ ...ROW, title: null })).toMatch(/<h1 class="title svelte-[\w-]+">claude<\/h1>/)
  })

  it('says so once its agent stops running', () => {
    const body = draw(undefined)
    expect(body).toMatch(/<p class="gone svelte-[\w-]+">This agent is no longer running\.<\/p>/)
    expect(body).not.toContain('<h1')
    expect(body).not.toContain('role="img"')
  })

  it('pulses the mark while it connects, before the first list', () => {
    const body = draw(null, 0, { state: 'connecting' })
    expect(body).toMatch(/<span class="cell cursor svelte-[\w-]+ pulsing"><\/span>/)
    expect(body).not.toContain('no longer running')
    expect(body).not.toContain('<h1')
  })

  it('keeps the mark still without a list once it stops connecting', () => {
    const body = draw(null, 0, { state: 'unauthorized' })
    expect(body).toMatch(/<span class="cell cursor svelte-[\w-]+"><\/span>/)
    expect(body).not.toContain('no longer running')
  })

  it.each([ROW, null, undefined])('offers going back, with or without its agent', (row) => {
    expect(draw(row)).toMatch(/<button class="button plain svelte-[\w-]+" type="button"[^>]*>(?:<!--[^>]*-->| )*Back/)
  })

  it('offers opening the terminal of its agent', () => {
    expect(draw(ROW)).toMatch(TERMINAL)
    expect(agentSource).toContain('onclick={() => onterminal(opened)}')
  })

  it.each([null, undefined])('offers no terminal without its agent', (row) => {
    expect(draw(row)).not.toMatch(TERMINAL)
  })

  it('sits below the connection bar', () => {
    expect(draw(ROW, 52)).toContain('style="--below: 52px;"')
  })
})
