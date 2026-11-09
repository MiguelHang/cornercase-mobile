import { render } from 'svelte/server'
import { describe, expect, it } from 'vitest'
import type { AgentRow } from '../../lib/bindings'
import Agent from './Agent.svelte'

const ROW: AgentRow = {
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

const draw = (row: AgentRow | undefined, below = 0) => render(Agent, { props: { row, below, onback: () => {} } }).body

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

  it.each([ROW, undefined])('offers going back, with or without its agent', (row) => {
    expect(draw(row)).toMatch(/<button class="button plain svelte-[\w-]+" type="button"[^>]*>(?:<!--[^>]*-->| )*Back/)
  })

  it('sits below the connection bar', () => {
    expect(draw(ROW, 52)).toContain('style="--below: 52px;"')
  })
})
