import { render } from 'svelte/server'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import type { AgentRow_Serialize as Agent, ConnectionView_Serialize } from '../../lib/bindings'
import { boardSearch } from '../../lib/boardSearch.svelte'
import { folds } from '../../lib/folds.svelte'
import hiddenSource from '../../lib/ui/Hidden.svelte?raw'
import Board from './Board.svelte'
import boardSource from './Board.svelte?raw'
import rowSource from './AgentRow.svelte?raw'
import searchSource from '../../lib/ui/SearchField.svelte?raw'

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
  parent: null,
  depth: 0,
  role: null,
  rank_status: null,
  project_name: 'shop',
  workspace_name: 'main',
  workspace_count: 1,
}

const at = (pane: number, status: string | null, more: Partial<Agent> = {}): Agent => ({
  ...BASE,
  pane,
  status,
  rank_status: status,
  ...more,
})

const draw = (agents: Agent[] | null, connection: ConnectionView_Serialize = CONNECTED, below = 0, machine: number | null = 1) =>
  render(Board, { props: { machine, agents, connection, below, onopen: () => {}, onstart: () => {} } }).body

const BRANCH = [
  at(1, 'working', { title: 'Lead', rank_status: 'waiting' }),
  at(2, 'waiting', { title: 'Ask', parent: 1, depth: 1 }),
  at(3, 'working', { title: 'Dig', parent: 2, depth: 2 }),
]

const titles = (body: string) => [...body.matchAll(/<span class="title svelte-[\w-]+">([^<]+)<\/span>/g)].map((m) => m[1])

const unfolded = (machine: number, ...panes: number[]) => {
  panes.forEach((pane) => folds.toggle(machine, 'b1', pane))
  return machine
}

const START = /<button class="button primary svelte-[\w-]+( resting)?" type="button"([^>]*)>(?:<!--[^>]*-->| )*Start</

const flat = (text: string) => text.replace(/\s+/g, ' ')

const FOLD = /<button class="fold svelte-[\w-]+" type="button" aria-label="([^"]+)" aria-expanded="(true|false)"><span class="chevron svelte-[\w-]+( open)?" style="--turn: (\d+)ms;"><svg [^>]*width="18" height="18"[^]*?<path d="([^"]+)"><\/path>/g

const disclosures = (body: string) => [...body.matchAll(FOLD)].map((m) => m.slice(1))

const CHEVRON_RIGHT = 'm9 18 6-6-6-6'

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
    expect(source).toContain(
      '{#each shown as item (item.key)} <li class="item" animate:flip={moving}>',
    )
    expect(source).toContain('const shown = $derived(agents === null ? [] : items(agents, opened, query))')
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

  it.each([[], null, [at(1, 'working')]])('offers starting new work once connected, for %o', (agents) => {
    const button = draw(agents).match(START)
    expect(button?.[1]).toBeUndefined()
    expect(button?.[2]).not.toContain('disabled')
    expect(flat(boardSource)).toContain(
      "<Button resting={connection.state !== 'connected'} onclick={onstart}>Start</Button>",
    )
  })

  it.each<ConnectionView_Serialize>([
    { state: 'connecting' },
    { state: 'none' },
    { state: 'unauthorized' },
  ])('holds Start while the connection is $state', (connection) => {
    const button = draw([], connection).match(START)
    expect(button?.[1]).toBe(' resting')
    expect(button?.[2]).toContain('disabled=""')
  })

  it('starts below the connection bar', () => {
    expect(draw([], CONNECTED, 44)).toMatch(/<section class="board svelte-[\w-]+" style="--below: 44px;">/)
    expect(flat(boardSource)).toContain('padding: calc(var(--below) + var(--inset)) var(--inset-wide) var(--inset-wide);')
  })

  it('folds a branch by default, with the attention and count of what it hides', () => {
    const body = draw(BRANCH)
    expect(titles(body)).toEqual(['Lead'])
    expect(body).toMatch(/<span class="hidden svelte-[\w-]+">\+2<\/span>/)
    expect(body).toMatch(/role="img" aria-label="2 subagents hidden, at least one waiting"><span class="marks svelte-[\w-]+" aria-hidden="true">/)
    expect(body).toMatch(/aria-label="Waiting" width="10" height="10"/)
    expect(disclosures(body)).toEqual([['Show 2 subagents', 'false', undefined, '180', CHEVRON_RIGHT]])
  })

  it('unfolds the branches the board keeps for its machine and boot', () => {
    const machine = unfolded(20, 1)
    expect(titles(draw(BRANCH, CONNECTED, 0, machine))).toEqual(['Lead', 'Ask'])
    expect(disclosures(draw(BRANCH, CONNECTED, 0, machine))).toEqual([
      ['Hide 2 subagents', 'true', ' open', '180', CHEVRON_RIGHT],
      ['Show 1 subagent', 'false', undefined, '180', CHEVRON_RIGHT],
    ])
    expect(titles(draw(BRANCH, CONNECTED, 0, unfolded(21, 1, 2)))).toEqual(['Lead', 'Ask', 'Dig'])
    expect(titles(draw(BRANCH, CONNECTED, 0, 22))).toEqual(['Lead'])
    expect(titles(draw(BRANCH.map((agent) => ({ ...agent, boot: 'b2' })), CONNECTED, 0, machine))).toEqual(['Lead'])
  })

  it('folds and unfolds from its own button, before the row and never inside it', () => {
    const body = draw(BRANCH)
    const fold = body.indexOf('<button class="fold')
    expect(body.indexOf('<div class="lead')).toBeLessThan(fold)
    expect(body.indexOf('</button>', fold)).toBeLessThan(body.indexOf('<button class="row'))
    expect(body.indexOf('<span class="folded')).toBeLessThan(body.indexOf('<button class="row'))
    expect(flat(rowSource)).toContain('onclick={() => onfold(row.pane)} ontouchstart={() => {}} >')
    expect(flat(rowSource)).toContain('.fold { display: flex; flex: none; align-items: center; justify-content: center; min-width: 44px;')
    expect(flat(boardSource)).toContain('<AgentRow {item} {agents} {now} {onopen} onfold={fold} />')
    expect(flat(boardSource)).toContain('if (machine !== null) { folds.toggle(machine, boot, pane) }')
  })

  it('never opens the agent from the disclosure', () => {
    const fold = flat(rowSource).match(/<button class="fold"[^]*?<\/button>/)?.[0]
    expect(fold).toContain('onclick={() => onfold(row.pane)}')
    expect(fold).not.toContain('onopen')
  })

  it('turns one chevron a quarter in the time of ENTER_MS', () => {
    const source = flat(rowSource)
    expect(source.match(/<Icon /g)).toHaveLength(1)
    expect(source).toContain('<Icon name="chevron-right" size={18} />')
    expect(source).not.toContain('chevron-down')
    expect(source).toContain('const turn = prefersReducedMotion() ? 0 : ENTER_MS')
    expect(source).toContain('.chevron { display: flex; transition: transform var(--turn); }')
    expect(source).toContain('.chevron.open { transform: rotate(90deg); }')
  })

  it('turns the chevron at once with reduced motion', () => {
    vi.stubGlobal('window', { matchMedia: (query: string) => ({ matches: query.includes('reduce') }) })
    try {
      expect(disclosures(draw(BRANCH)).map((m) => m[3])).toEqual(['0'])
    } finally {
      vi.unstubAllGlobals()
    }
  })

  it('opens each shown row, a child too, with its own pane', () => {
    const body = draw(BRANCH, CONNECTED, 0, unfolded(23, 1, 2))
    expect(body.match(/<button class="row svelte-[\w-]+"/g)).toHaveLength(3)
    expect(flat(rowSource)).toContain('const row = $derived(item.row)')
    expect(flat(rowSource)).toContain('onclick={() => onopen(row.pane)}')
  })

  it('indents a row by its depth, up to three levels', () => {
    const deep = [at(1, 'working'), at(2, 'working', { parent: 1, depth: 1 }), at(3, 'idle', { parent: 2, depth: 5 })]
    const body = draw(deep, CONNECTED, 0, unfolded(24, 1, 2))
    expect([...body.matchAll(/style="--depth: (\d);"/g)].map((m) => m[1])).toEqual(['0', '1', '3'])
    expect(flat(rowSource)).toContain('padding-left: calc(var(--depth) * var(--inset-wide));')
  })

  it('keeps the dots of siblings in a column, with the room of the disclosure on rows without children', () => {
    const tree = [at(1, 'working', { title: 'Lead' }), at(2, 'idle', { title: 'Kid', parent: 1, depth: 1 }), at(4, 'idle', { title: 'Alone' })]
    const body = draw(tree, CONNECTED, 0, unfolded(26, 1))
    expect(body.match(/<button class="fold svelte-[\w-]+"/g)).toHaveLength(1)
    expect(body.match(/<div class="lead svelte-[\w-]+">/g)).toHaveLength(3)
    expect(body).not.toContain('slot')
    expect(rowSource).not.toContain('slot')
    expect(flat(rowSource)).toContain('.lead { display: flex; flex: none; align-items: center; min-width: 44px; }')
  })

  it('leaves only the time on the right edge, and lets it shrink when the row runs out of room', () => {
    const row = flat(rowSource).match(/<button class="row"[^]*?<\/button>/)?.[0]
    expect(row).not.toMatch(/Hidden|Icon|fold/)
    expect(row).toMatch(/<span class="time">\{time\}<\/span> \{\/if\} <\/button>$/)
    expect(flat(rowSource)).toContain('.time { flex: 0 1 auto; min-width: 0; overflow: hidden;')
    expect(flat(hiddenSource)).toContain('.folded, .marks { display: flex; flex: none; align-items: center; }')
  })

  it('names the members of a flow by their role under its orchestrator', () => {
    const flow = [
      at(1, 'working', { title: 'Ship login', role: 'orchestrator' }),
      at(2, 'working', { title: 'Write the form', role: 'coder', parent: 1, depth: 1 }),
    ]
    const body = draw(flow, CONNECTED, 0, unfolded(25, 1))
    expect(titles(body)).toEqual(['Ship login', 'coder'])
    expect(body).toMatch(/<span class="place svelte-[\w-]+">shop · orchestrator<\/span>/)
    expect(body).toMatch(/<span class="place svelte-[\w-]+">Write the form<\/span>/)
  })

  it('keeps every branch folded without a machine', () => {
    expect(titles(draw(BRANCH, CONNECTED, 0, null))).toEqual(['Lead'])
  })
})

describe('Board search', () => {
  const searching = (machine: number, query: string) => {
    boardSearch.write(machine, query)
    return machine
  }

  const field = (body: string) => body.match(/<input class="query svelte-[\w-]+" type="search"[^>]*>/)?.[0] ?? null

  const CANCEL = /<button class="cancel svelte-[\w-]+" type="button">Cancel<\/button>/

  afterEach(() => {
    boardSearch.write(0, '')
  })

  it('draws a search field named Search agents between Start and the list, with the attributes of the other fields', () => {
    const body = draw(BRANCH)
    expect(field(body)).toMatch(
      /^<input class="query svelte-[\w-]+" type="search" aria-label="Search agents" placeholder="Search agents" autocomplete="off" autocapitalize="off" autocorrect="off" spellcheck="false" enterkeyhint="search" value=""\/>$/,
    )
    expect(body.indexOf('>Start<')).toBeLessThan(body.indexOf('type="search"'))
    expect(body.indexOf('type="search"')).toBeLessThan(body.indexOf('<ul'))
    expect(flat(boardSource)).toContain('.find { margin-bottom: var(--inset); }')
    expect(boardSource).not.toContain('sticky')
  })

  it.each<[string, Agent[] | null]>([
    ['no agent runs', []],
    ['the list has not come', null],
  ])('draws no search field while %s, and keeps the query', (_, agents) => {
    const machine = searching(40, 'dig')
    expect(field(draw(agents, CONNECTED, 0, machine))).toBeNull()
    expect(boardSearch.query(machine)).toBe('dig')
  })

  it('shows what matches with its parents, unfolded and without a disclosure, and leaves the folds alone', () => {
    const machine = searching(41, 'dig')
    const body = draw(BRANCH, CONNECTED, 0, machine)
    expect(titles(body)).toEqual(['Lead', 'Ask', 'Dig'])
    expect(disclosures(body)).toEqual([])
    expect(body).not.toContain('class="folded')
    expect(field(body)).toContain('value="dig"')
    expect(folds.expanded(machine, 'b1').size).toBe(0)
  })

  it('says so when nothing matches, and keeps the field to fix the query', () => {
    const body = draw(BRANCH, CONNECTED, 0, searching(42, 'zebra'))
    expect(body).toMatch(/<p class="empty svelte-[\w-]+">No agent matches the search<\/p>/)
    expect(body).not.toContain('<ul')
    expect(field(body)).toContain('value="zebra"')
  })

  it('offers Cancel beside a field with text, and not beside an empty one', () => {
    expect(draw(BRANCH, CONNECTED, 0, searching(43, 'ask'))).toMatch(CANCEL)
    expect(draw(BRANCH, CONNECTED, 0, searching(44, ''))).not.toContain('>Cancel<')
  })

  it('empties the query and leaves the field on Cancel, then shows the board folded as before', () => {
    const search = flat(searchSource)
    expect(search).toContain('const cancelling = $derived(oncancel !== undefined && (focused || value !== \'\'))')
    expect(search).toContain("const cancel = () => { value = '' field?.blur() oncancel?.() }")
    expect(search).toContain('onmousedown={(event) => event.preventDefault()} onclick={cancel}')
    expect(search).toContain('onfocus={() => (focused = true)} onblur={() => (focused = false)}')
    expect(flat(boardSource)).toContain("oncancel={() => search('')}")
    const machine = unfolded(45, 2)
    searching(machine, 'dig')
    searching(machine, '')
    const body = draw(BRANCH, CONNECTED, 0, machine)
    expect(titles(body)).toEqual(['Lead'])
    expect(disclosures(body).map(([label]) => label)).toEqual(['Show 2 subagents'])
    expect(folds.expanded(machine, 'b1')).toEqual(new Set([2]))
  })

  it('leaves the field on Enter and keeps the query', () => {
    expect(flat(searchSource)).toContain(
      "const submit = (event: KeyboardEvent) => { if (oncancel !== undefined && event.key === 'Enter') { field?.blur() } }",
    )
    expect(flat(searchSource)).toContain('onkeydown={submit}')
  })

  it('writes each input as the query of its machine', () => {
    expect(flat(boardSource)).toContain('oninput={(event) => search(event.currentTarget.value)}')
    expect(flat(boardSource)).toContain(
      'const search = (text: string) => { if (machine !== null) { boardSearch.write(machine, text) } }',
    )
  })

  it('keeps the query when the board comes back on the same machine, and starts empty on another', () => {
    const machine = searching(46, 'ask')
    expect(titles(draw(BRANCH, CONNECTED, 0, machine))).toEqual(['Lead', 'Ask'])
    expect(titles(draw(BRANCH, CONNECTED, 0, machine))).toEqual(['Lead', 'Ask'])
    const other = draw(BRANCH, CONNECTED, 0, 47)
    expect(titles(other)).toEqual(['Lead'])
    expect(field(other)).toContain('value=""')
  })

  it('searches nothing without a machine', () => {
    searching(48, 'dig')
    expect(titles(draw(BRANCH, CONNECTED, 0, null))).toEqual(['Lead'])
  })
})
