import { render } from 'svelte/server'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import type { AgentRow_Serialize as Agent, ConnectionView_Serialize } from '../../lib/bindings'
import Board from './Board.svelte'
import boardSource from './Board.svelte?raw'
import rowSource from './AgentRow.svelte?raw'

const NOW = Date.parse('2026-11-09T12:00:00.000Z')

const CONNECTED: ConnectionView_Serialize = { state: 'connected', route: 'ws://mac:7000' }

const BASE: Agent = {
  boot: 'b1',
  pane: 0,
  project: 1,
  workspace: 2,
  tab: 3,
  tab_name: 'tab',
  agent: 'codex',
  title: null,
  status: null,
  since: null,
  project_name: 'shop',
  workspace_name: 'main',
  workspace_count: 1,
}

const at = (pane: number, status: string | null, more: Partial<Agent> = {}): Agent => ({ ...BASE, pane, status, ...more })

const draw = (agents: Agent[] | null, connection: ConnectionView_Serialize = CONNECTED, below = 0) =>
  render(Board, { props: { agents, connection, below, onopen: () => {} } }).body

const flat = (text: string) => text.replace(/\s+/g, ' ')

const sections = (body: string) => [...body.matchAll(/<h2 class="section svelte-[\w-]+">(\w+)<\/h2>/g)].map((m) => m[1])

describe('Board', () => {
  beforeEach(() => {
    vi.useFakeTimers()
    vi.setSystemTime(NOW)
  })

  afterEach(() => {
    vi.useRealTimers()
  })

  it('says so when no agent runs', () => {
    expect(draw([])).toMatch(/<p class="empty svelte-[\w-]+">No agents running\.<\/p>/)
    expect(draw([])).not.toContain('<ul')
  })

  it('pulses the mark while it connects, before the first list', () => {
    const body = draw(null, { state: 'connecting' })
    expect(body).toMatch(/<span class="cell cursor svelte-[\w-]+ pulsing"><\/span>/)
    expect(body).not.toContain('No agents running.')
  })

  it('keeps the mark still without a list once it stops connecting', () => {
    expect(draw(null, { state: 'unauthorized' })).not.toContain('pulsing')
  })

  it('heads the sections in their order and leaves the empty ones out', () => {
    expect(sections(draw([at(1, 'working'), at(2, null), at(3, 'failed')]))).toEqual(['Waiting', 'Working', 'Idle'])
    expect(sections(draw([at(1, 'pending'), at(2, 'shell'), at(3, 'waiting'), at(4, 'idle')]))).toEqual([
      'Waiting',
      'Done',
      'Working',
      'Idle',
    ])
  })

  it('puts each row under its section', () => {
    const body = draw([at(1, 'working', { title: 'Build' }), at(2, 'waiting', { title: 'Ask' })])
    expect(body.indexOf('>Waiting<')).toBeLessThan(body.indexOf('>Ask<'))
    expect(body.indexOf('>Ask<')).toBeLessThan(body.indexOf('>Working<'))
    expect(body.indexOf('>Working<')).toBeLessThan(body.indexOf('>Build<'))
  })

  it('draws a row with its status, title, place and time in the state', () => {
    const since = new Date(NOW - 5 * 60_000).toISOString()
    const body = draw([at(7, 'waiting', { title: 'Fix login', workspace_count: 3, workspace_name: 'login', since })])
    expect(body).toContain('aria-label="Waiting"')
    expect(body).toMatch(/<span class="title svelte-[\w-]+">Fix login<\/span>/)
    expect(body).toMatch(/<span class="place svelte-[\w-]+">shop › login<\/span>/)
    expect(body).toMatch(/<span class="time svelte-[\w-]+">5 min<\/span>/)
  })

  it('names the agent and only the project when the row has no title and one workspace', () => {
    const body = draw([at(1, 'done')])
    expect(body).toMatch(/<span class="title svelte-[\w-]+">codex<\/span>/)
    expect(body).toMatch(/<span class="place svelte-[\w-]+">shop<\/span>/)
  })

  it('shows no time when it does not know since when', () => {
    expect(draw([at(1, 'working')])).not.toContain('class="time')
  })

  it('makes each row a button of at least 44 px that reacts to a touch', () => {
    expect(draw([at(1, 'idle')])).toMatch(/<button class="row svelte-[\w-]+" type="button">/)
    expect(flat(rowSource)).toContain('min-height: 44px;')
    expect(flat(rowSource)).toContain(
      '<button class="row" type="button" onclick={() => onopen(row.pane)} ontouchstart={() => {}}>',
    )
  })

  it('keys every item by its section or pane and moves them with flip, still with reduced motion', () => {
    const source = flat(boardSource)
    expect(source).toContain('{#each items(agents) as item (item.key)} <li class="item" animate:flip={moving}>')
    expect(source).toContain('const moving = { duration: prefersReducedMotion() ? 0 : ENTER_MS }')
  })

  it('refreshes the time with one timer at least once a minute, cleared when it unmounts', () => {
    const source = flat(boardSource)
    expect(source).toContain('const TICK_MS = 30_000')
    expect(source).toContain(
      '$effect(() => { const timer = setInterval(() => (now = Date.now()), TICK_MS) return () => clearInterval(timer) })',
    )
    expect(source.match(/setInterval/g)).toHaveLength(1)
    expect(rowSource).not.toMatch(/setInterval|setTimeout/)
  })

  it.each([null, [], [at(1, 'waiting')]])('heads the screen with an h1 only a screen reader reads, for %o', (agents) => {
    expect(draw(agents)).toContain('<h1 class="spoken">Agents</h1>')
  })

  it('starts below the connection bar', () => {
    expect(draw([], CONNECTED, 44)).toMatch(/<section class="board svelte-[\w-]+" style="--below: 44px;">/)
    expect(flat(boardSource)).toContain('padding: calc(var(--below) + var(--inset)) var(--inset-wide) var(--inset-wide);')
  })
})
