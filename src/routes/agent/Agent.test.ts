import { render } from 'svelte/server'
import { describe, expect, it } from 'vitest'
import type { AgentRow_Serialize as AgentRow, ConnectionView_Serialize } from '../../lib/bindings'
import type { AttachImage } from '../../lib/attach.svelte'
import { subagents } from '../../lib/board'
import { Chat, timers, type Clock, type Outgoing, type PressKey, type ReadLast, type SendText } from '../../lib/chat.svelte'
import { fakeClock, message, ok } from '../../lib/chatFakes'
import Agent from './Agent.svelte'
import agentSource from './Agent.svelte?raw'
import chatHeaderSource from './ChatHeader.svelte?raw'
import AgentLine from './AgentLine.svelte'
import agentLineSource from './AgentLine.svelte?raw'
import Children from './Children.svelte'
import childrenSource from './Children.svelte?raw'
import KeyRow from './KeyRow.svelte'
import keyRowSource from './KeyRow.svelte?raw'
import Sent from './Sent.svelte'
import sentSource from './Sent.svelte?raw'
import subagentsSource from './Subagents.svelte?raw'
import Thread from './Thread.svelte'
import threadSource from './Thread.svelte?raw'

const CONNECTED: ConnectionView_Serialize = { state: 'connected', route: 'ws://mac:7000' }

const ROW: AgentRow = {
  boot: 'b1',
  agent: 'claude',
  title: 'Fix the login flow',
  status: 'waiting',
  since: '2026-11-09T11:40:00.000Z',
  parent: null,
  depth: 0,
  role: null,
  rank_status: 'waiting',
  pane: 12,
  tab: 4,
  tab_name: 'agent',
  workspace: 9,
  workspace_name: 'login',
  workspace_count: 2,
  project: 5,
  project_name: 'web',
}

const unread: ReadLast = () => new Promise(() => {})

const tool = (name: string, path: string) =>
  new RegExp(`<button class="tool svelte-[\\w-]+" type="button" aria-label="${name}">(?:<!--[^>]*-->| )*<svg [^>]*>(?:(?!</svg>)[^])*?<path d="${path}"></path>`)

const BACK =
  /<button class="back svelte-[\w-]+" type="button" aria-label="Back to Agents">(?:<!--[^>]*-->| )*<svg [^>]*>(?:(?!<\/svg>)[^])*?<path d="m15 18-6-6 6-6"><\/path>/

const TERMINAL = tool('Terminal', 'm7 11 2-2-2-2')

const CHANGES = tool('Changes', 'M13 6h3a2 2 0 0 1 2 2v7')

const unsent: SendText = () => new Promise(() => {})

const unpressed: PressKey = () => new Promise(() => {})

const unattached: AttachImage = () => new Promise(() => {})

const still = { impactFeedback: () => Promise.resolve({ status: 'ok' as const, data: null }) }

const flat = (text: string) => text.replace(/\s+/g, ' ')

const chatWith = (row: AgentRow | null, draft = '') => {
  const chat = new Chat(1, ROW.pane, unread, unsent, unpressed, unattached, still, timers)
  if (row !== null) {
    chat.see(1, [row])
  }
  chat.draft = draft
  return chat
}

const draw = (
  chat: Chat,
  below = 0,
  connection: ConnectionView_Serialize = CONNECTED,
  starting: string | null = null,
  agents: AgentRow[] | null = null,
) =>
  render(Agent, {
    props: {
      chat,
      agents,
      connection,
      backTo: 'Agents',
      below,
      starting,
      onback: () => {},
      onopen: () => {},
      onfailed: () => {},
      onterminal: () => {},
      onchanges: () => {},
    },
  }).body

const STARTING = /<p class="starting svelte-[\w-]+"><span class="spinner[^]*?<\/span>(?:<!---->)?Starting claude…<\/p>/

const send = (body: string) => body.match(/<button class="([^"]*)" type="button"([^>]*)>(?:<!--[^>]*-->|\s|<span class="spinner[^>]*>.*?<\/span>)*Send/)

const minutesAgo = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString()

describe('Agent, its header', () => {
  it('shows the dot, the status in words, the agent, its place and its tab', () => {
    const body = draw(chatWith(ROW))
    expect(body).toContain('aria-label="Waiting"')
    expect(body).toMatch(/<h1 class="title svelte-[\w-]+">Fix the login flow<\/h1>/)
    expect(body).toMatch(
      /<p class="place svelte-[\w-]+"><span class="status svelte-[\w-]+">Waiting<\/span> · web › login › agent<\/p>/,
    )
    expect(body).not.toContain('no longer running')
  })

  it('names the agent when it has no title, and only the project when it has one workspace', () => {
    const body = draw(chatWith({ ...ROW, title: null, status: 'done', workspace_count: 1, tab_name: 'review' }))
    expect(body).toMatch(/<h1 class="title svelte-[\w-]+">claude<\/h1>/)
    expect(body).toMatch(/<span class="status svelte-[\w-]+">Done<\/span> · web › review<\/p>/)
  })

  it('sits below the connection bar', () => {
    expect(draw(chatWith(ROW), 52)).toContain('style="--below: 52px;"')
  })

  it("names a flow's member by its role, as on the board, and keeps its place", () => {
    const coder = { ...ROW, parent: 11, depth: 1, role: 'coder', title: 'Write the form', status: 'working' }
    const team = [{ ...ROW, pane: 11, role: 'orchestrator' }, coder]
    const body = draw(chatWith(coder), 0, CONNECTED, null, team)
    expect(body).toMatch(/<h1 class="title svelte-[\w-]+">coder<\/h1>/)
    expect(body).toMatch(/<span class="called svelte-[\w-]+">coder<\/span>/)
    expect(body).toMatch(/<span class="status svelte-[\w-]+">Working<\/span> · web › login › agent<\/p>/)
    expect(draw(chatWith(coder))).toMatch(/<h1 class="title svelte-[\w-]+">Write the form<\/h1>/)
  })
})

const LEAD: AgentRow = { ...ROW, status: 'working', role: 'orchestrator', rank_status: 'waiting' }

const CODER: AgentRow = { ...ROW, pane: 13, parent: 12, depth: 1, role: 'coder', title: 'Write the form', status: 'working' }

const DIG: AgentRow = { ...ROW, pane: 14, parent: 13, depth: 2, role: null, title: 'Dig', status: 'waiting' }

const REVIEWER: AgentRow = { ...ROW, pane: 15, parent: 12, depth: 1, role: 'reviewer', title: 'Review', status: 'working' }

const TEAM = [LEAD, CODER, DIG, REVIEWER]

const COUNT = /<span class="count svelte-[\w-]+">([^<]*)<\/span>/

const unfolded = (agents: AgentRow[]) =>
  render(Children, { props: { children: subagents(agents, LEAD.pane)?.children ?? [], agents, onopen: () => {} } }).body

const CHEVRON_RIGHT = /<span class="arrow svelte-[\w-]+"><svg [^>]*width="18" height="18"[^]*?<path d="m9 18 6-6-6-6"><\/path>/

const names = (body: string) => [...body.matchAll(/<span class="name svelte-[\w-]+">([^<]*)<\/span>/g)].map(([, name]) => name)

describe('Agent, its subagents', () => {
  it('counts the subagents of every level on a folded line under the header', () => {
    const body = draw(chatWith(LEAD), 0, CONNECTED, null, TEAM)
    expect(body.match(COUNT)?.[1]).toBe('3 subagents · 1 waiting, 2 working')
    expect(body).toMatch(/<\/header>(?:<!--[^>]*-->|\s)*<div class="subagents[^]*?<div class="thread/)
    expect(body).toContain('aria-expanded="false"')
    expect(body).toMatch(CHEVRON_RIGHT)
    expect(body).not.toContain('class="children')
  })

  it.each<[string, AgentRow[] | null]>([
    ['an agent without children', [LEAD, DIG]],
    ['no list of agents', null],
  ])('has no line for %s', (_, agents) => {
    const body = draw(chatWith(LEAD), 0, CONNECTED, null, agents)
    expect(body).toContain('</header>')
    expect(body).not.toContain('class="subagents')
  })

  it('unfolds into its direct children, named by their role, with the attention and count below each', () => {
    const body = unfolded(TEAM)
    expect(body).toMatch(/<ul class="children svelte-[\w-]+" aria-label="Subagents">/)
    expect(names(body)).toEqual(['coder', 'reviewer'])
    expect(body).toMatch(/<span class="where svelte-[\w-]+">Write the form<\/span>/)
    expect(body).toMatch(
      /role="img" aria-label="1 subagent hidden, waiting"><span class="marks svelte-[\w-]+" aria-hidden="true">[^]*?aria-label="Waiting" width="10"[^]*?<span class="hidden svelte-[\w-]+">\+1<\/span>/,
    )
    expect(body).not.toContain('>Dig<')
  })

  it("opens a child's chat from its row and folds the list from the line", () => {
    expect(flat(childrenSource)).toContain(
      '<button class="child" type="button" onclick={() => onopen(child.row.pane)} ontouchstart={() => {}}>',
    )
    expect(flat(childrenSource)).toContain('min-height: 44px;')
    expect(flat(subagentsSource)).toContain('let open = $state(false)')
    expect(flat(subagentsSource)).toContain('aria-expanded={open} onclick={() => (open = !open)}')
    expect(flat(subagentsSource)).toContain("<Icon name={open ? 'chevron-down' : 'chevron-right'} size={18} />")
    expect(flat(subagentsSource)).toContain('{#if open} <Children children={summary.children} {agents} {onopen} /> {/if}')
    expect(flat(subagentsSource)).toContain('.arrow { display: flex; flex: none; align-items: center; justify-content: center; min-width: 44px;')
  })

  it('starts folded again for each agent opened, and opens its children like the board', () => {
    expect(flat(agentSource)).toContain('{#key row.pane} <Subagents {row} agents={agents ?? []} {onopen} /> {/key} <Thread {chat} {now} />')
  })

  it('follows the rows as they change', () => {
    const chat = chatWith(LEAD)
    const changed = [LEAD, CODER, { ...DIG, status: 'done' }]
    chat.see(1, changed)
    expect(draw(chat, 0, CONNECTED, null, changed).match(COUNT)?.[1]).toBe('2 subagents · 1 done, 1 working')
    expect(names(unfolded(changed))).toEqual(['coder'])
  })
})

const BAR = /<div class="bar ios svelte-[\w-]+ fixed" style="--below: 0px;">(.*?)<\/div><!----> <header class="head svelte-[\w-]+">/

const SHORT = /<div class="short svelte-[\w-]+" aria-hidden="true">(.*?)<\/div>/

const bar = (row: AgentRow) => draw(chatWith(row)).match(BAR)?.[1] ?? ''

describe('Agent, its sticky header', () => {
  it('keeps Back at the start of the bar and Terminal and Changes at its end, with the large header after it to scroll away', () => {
    const shown = bar(ROW)
    const end = shown.slice(shown.indexOf('<div class="end'))
    expect(shown.slice(0, shown.indexOf('<div class="center'))).toMatch(BACK)
    expect(end).toMatch(TERMINAL)
    expect(end).toMatch(CHANGES)
    expect(shown).not.toContain('<h1')
  })

  it('starts large at the top of the page, below the connection bar', () => {
    expect(draw(chatWith(ROW), 52)).toMatch(/<div class="bar ios svelte-[\w-]+ fixed" style="--below: 52px;"><div class="start/)
  })

  it('carries a compact line between them, with the dot, the title and the status, hidden from VoiceOver', () => {
    const short = bar(ROW).match(SHORT)?.[1] ?? ''
    expect(short).toContain('aria-label="Waiting"')
    expect(short).toMatch(/<span class="called svelte-[\w-]+">Fix the login flow<\/span> <span class="state svelte-[\w-]+">Waiting<\/span>$/)
    const shown = bar(ROW)
    expect(shown.indexOf('class="short')).toBeGreaterThan(shown.indexOf('<div class="center'))
    expect(shown.indexOf('class="short')).toBeLessThan(shown.indexOf('<div class="end'))
  })

  it('names the agent in the compact line when it has no title', () => {
    const short = bar({ ...ROW, title: null, status: 'working' }).match(SHORT)?.[1] ?? ''
    expect(short).toMatch(/<span class="called svelte-[\w-]+">claude<\/span> <span class="state svelte-[\w-]+">Working<\/span>$/)
  })

  it('turns compact past the large header, measured, following the page with a passive listener', () => {
    const source = flat(chatHeaderSource)
    expect(source).toContain('const compact = $derived(compactHeader(scrolled, tall))')
    expect(source).toContain('<NavBar {backTo} {onback} {below} edge={compact}> {#snippet center()} <div class="short" class:compact aria-hidden="true">')
    expect(source).toContain('<header class="head" bind:clientHeight={tall}>')
    expect(source).toContain("const seen = () => (scrolled = window.scrollY) seen() window.addEventListener('scroll', seen, { passive: true })")
    expect(source).toContain("return () => window.removeEventListener('scroll', seen)")
  })

  it('sticks under the connection bar in the shared bar, with its edge only while compact', () => {
    expect(bar(ROW)).not.toContain('edge')
    expect(flat(chatHeaderSource)).not.toContain('.bar {')
  })

  it('shows the compact line only while compact, as tall as its 44 px buttons', () => {
    const source = flat(chatHeaderSource)
    expect(source).toContain('min-width: 0; min-height: 44px; visibility: hidden; opacity: 0;')
    expect(source).toContain('.short.compact { max-width: 100%; visibility: visible; opacity: 1; transition-delay: 0s; }')
    expect(bar(ROW)).not.toMatch(/class="short[^"]* compact"/)
  })

  it('leaves the compact line the room its icon buttons of 44 by 44 px do not take, centred and cut at the end', () => {
    const source = flat(chatHeaderSource)
    expect(source.match(/<button class="tool" type="button" aria-label="\w+" onclick=\{.*?\} ontouchstart=\{\(\) => \{\}\}/g)).toHaveLength(2)
    expect(source).toContain('{#snippet right()} <button class="tool" type="button" aria-label="Terminal"')
    expect(source).toContain('.tool { display: flex; flex: none; align-items: center; justify-content: center; width: 44px; height: 44px;')
    expect(source).toContain('.short { display: inline-flex; align-items: center; max-width: 0; overflow: hidden; vertical-align: middle;')
    expect(source).not.toContain('<Button')
  })

  it('leaves the back button the whole room while the compact line is hidden, taking it back once faded out', () => {
    const source = flat(chatHeaderSource)
    expect(source).toContain('.short { display: inline-flex; align-items: center; max-width: 0;')
    expect(source).toContain('transition: opacity 150ms ease-out, visibility 150ms, max-width 0s 150ms; }')
  })

  it('changes without animation under Reduce Motion', () => {
    expect(flat(chatHeaderSource)).toContain('@media (prefers-reduced-motion: reduce) { .short { transition: none; } }')
  })
})

describe('Agent, its message', () => {
  it('draws the message as markdown, with the time it was written', () => {
    const chat = chatWith(ROW)
    chat.reading = {
      kind: 'message',
      message: { pane: ROW.pane, agent: 'claude', text: 'Tests **pass**.', written: minutesAgo(5), turn_over: true },
    }
    const body = draw(chat)
    expect(body).toContain('<strong')
    expect(body).toContain('pass')
    expect(body).toMatch(/<p class="written svelte-[\w-]+">5 min<\/p>/)
  })

  it('leaves out the time when the message has none', () => {
    const chat = chatWith(ROW)
    chat.reading = {
      kind: 'message',
      message: { pane: ROW.pane, agent: 'claude', text: 'Ready.', written: null, turn_over: null },
    }
    expect(draw(chat)).not.toContain('class="written')
  })

  it.each<[string, Chat['reading'], string]>([
    ['the reason it cannot be read', { kind: 'unreadable', reason: 'no message yet' }, 'no message yet'],
    ['that it is reading', { kind: 'loading' }, 'Reading its last message…'],
  ])('says %s in its place, muted', (_, reading, text) => {
    const chat = chatWith(ROW)
    chat.reading = reading
    const body = draw(chat)
    expect(body).toMatch(new RegExp(`<p class="reason svelte-[\\w-]+">${text}</p>`))
    expect(body).not.toContain('class="markdown')
  })

  it('mutes the reason and the note', () => {
    expect(flat(threadSource)).toContain('.reason { margin: 0; color: var(--muted); }')
    expect(flat(agentSource)).toContain('.note { margin: 0; color: var(--muted); }')
  })

  it('reads it again while the screen shows, and stops when it goes', () => {
    expect(flat(agentSource)).toContain(
      '$effect(() => { const followed = chat return untrack(() => whileShown(document, () => followed.follow())) })',
    )
  })
})

describe('Agent, its field', () => {
  it('asks for the answer in a text area, with a Send button ready for typed text', () => {
    const body = draw(chatWith({ ...ROW, status: 'done' }, 'go on'))
    expect(body).toMatch(/<textarea class="area svelte-[\w-]+" rows="1" aria-label="Answer" placeholder="Answer claude"/)
    expect(body).toContain('go on</textarea>')
    expect(send(body)?.[1]).toMatch(/^button primary svelte-[\w-]+$/)
    expect(send(body)?.[2]).not.toContain('disabled')
    expect(body).not.toContain('is asking you')
  })

  it('cannot send blank text', () => {
    const body = draw(chatWith({ ...ROW, status: 'done' }, '  '))
    expect(send(body)?.[1]).toContain(' resting')
    expect(send(body)?.[2]).toContain('disabled=""')
  })

  it('says the agent is asking on its screen, keeps the text and cannot send', () => {
    const body = draw(chatWith(ROW, 'yes, go on'))
    expect(body).toMatch(
      /<p class="note svelte-[\w-]+">claude is asking you something on its screen; answer with the keys\.<\/p>/,
    )
    expect(body).toContain('yes, go on</textarea>')
    expect(send(body)?.[2]).toContain('disabled=""')
  })

  it('shows the send under way, with the field emptied and open', () => {
    const chat = chatWith({ ...ROW, status: 'done' }, 'go on')
    void chat.send()
    const body = draw(chat)
    expect(send(body)?.[1]).toMatch(/^button primary svelte-[\w-]+ busy$/)
    expect(send(body)?.[2]).toContain('aria-busy="true"')
    expect(body).not.toMatch(/<textarea[^>]* disabled=""/)
    expect(body).toContain('></textarea>')
  })

  it('leaves a failed send to its bubble, with no toast', () => {
    expect(flat(agentSource)).toContain('onclick={() => void chat.send()}>Send</Button>')
    expect(flat(agentSource)).not.toContain('await chat.send()')
  })

  it('stays at the bottom of the screen while the message scrolls', () => {
    expect(flat(agentSource)).toContain('.composer { position: sticky; bottom: 0;')
  })
})

describe('Agent, without its row', () => {
  it('says so once its agent stops running, offers going back to the board in the bar and has no field', () => {
    const chat = chatWith(ROW, 'go on')
    chat.see(1, [{ ...ROW, pane: 30 }])
    const body = draw(chat)
    expect(body).toMatch(/<p class="gone svelte-[\w-]+">This agent is no longer running\.<\/p>/)
    expect(body).toMatch(BACK)
    expect(body.search(BACK)).toBeLessThan(body.indexOf('class="gone'))
    expect(body).not.toContain('Back to the board')
    expect(body).not.toContain('<textarea')
    expect(body).not.toContain('<h1')
    expect(body).not.toContain('role="img"')
  })

  it('says the agent it started is starting until its row comes, with the bar and no field', () => {
    const chat = chatWith(null)
    chat.see(1, [{ ...ROW, pane: 30 }], true)
    const body = draw(chat, 0, CONNECTED, 'claude')
    expect(body).toMatch(STARTING)
    expect(body).toMatch(BACK)
    expect(body).not.toContain('no longer running')
    expect(body).not.toContain('<textarea')
    expect(body).not.toContain('class="cell cursor')
  })

  it('shows the agent it started once its row comes', () => {
    const body = draw(chatWith(ROW), 0, CONNECTED, 'claude')
    expect(body).not.toMatch(STARTING)
    expect(body).toMatch(/<h1 class="title svelte-[\w-]+">Fix the login flow<\/h1>/)
  })

  it('says it stopped running when its row goes, though it was started from here', () => {
    const chat = chatWith(null)
    chat.see(1, [ROW], true)
    chat.see(1, [{ ...ROW, pane: 30 }], true)
    const body = draw(chat, 0, CONNECTED, 'claude')
    expect(body).toContain('This agent is no longer running.')
    expect(body).not.toMatch(STARTING)
  })

  it('pulses the mark while it connects, before the first list', () => {
    const body = draw(chatWith(null), 0, { state: 'connecting' })
    expect(body).toMatch(/<span class="cell cursor svelte-[\w-]+ pulsing"><\/span>/)
    expect(body).not.toContain('no longer running')
    expect(body).not.toContain('<textarea')
  })

  it('keeps the mark still without a list once it stops connecting', () => {
    const body = draw(chatWith(null), 0, { state: 'unauthorized' })
    expect(body).toMatch(/<span class="cell cursor svelte-[\w-]+"><\/span>/)
    expect(body).not.toContain('no longer running')
  })

  it.each([
    ['with its row', () => chatWith(ROW)],
    ['without its row', () => chatWith(null)],
    [
      'once it is gone',
      () => {
        const chat = chatWith(ROW)
        chat.see(1, [{ ...ROW, pane: 30 }])
        return chat
      },
    ],
  ])('offers going back to the Agents in one bar %s, with no word button', (_, made) => {
    const body = draw(made())
    expect(body.match(/<div class="bar /g)).toHaveLength(1)
    expect(body).toMatch(BACK)
    expect(body).not.toMatch(/>(?:<!--[^>]*-->| )*Back(?: to the board)?<!---->/)
  })

  it('draws the bar of the shared NavBar in every state, with no text Button to go back', () => {
    const source = flat(agentSource)
    expect(source.match(/<NavBar \{backTo\} \{onback\} \{below\} \/>/g)).toHaveLength(2)
    expect(source).not.toContain('<Button tone="plain" onclick={onback}>')
  })

  it('offers opening the terminal and the changes of its agent, beside going back', () => {
    expect(draw(chatWith(ROW))).toMatch(TERMINAL)
    expect(flat(agentSource)).toContain('{:else} <ChatHeader {row} agents={agents ?? []} {backTo} {below} {onback} {onterminal} {onchanges} /> {#key row.pane}')
    const source = flat(chatHeaderSource)
    expect(source).toContain('aria-label="Terminal" onclick={() => onterminal(row)}')
    expect(source).toContain('aria-label="Changes" onclick={() => onchanges(row.workspace, row.workspace_name)}')
  })

  it("offers the changes of its agent's workspace, beside its terminal", () => {
    const body = draw(chatWith(ROW))
    expect(body).toMatch(CHANGES)
    expect(body.search(CHANGES)).toBeGreaterThan(body.search(TERMINAL))
  })

  it.each([
    ['terminal', TERMINAL],
    ['changes', CHANGES],
  ])('offers no %s without its agent, nor once it is gone', (_, button) => {
    const gone = chatWith(ROW)
    gone.see(1, [{ ...ROW, pane: 30 }])
    expect(draw(chatWith(null))).not.toMatch(button)
    expect(draw(gone)).not.toMatch(button)
  })
})

const ATTACHED = [
  { name: 'IMG_0042.jpg', size: '47 KiB' },
  { name: 'pasted.png', size: '900 B' },
]

const attachButton = (body: string) =>
  body.match(/<button class="([^"]*)" type="button"([^>]*) aria-label="Attach image"[^>]*>(.*?)<\/button>/)

const withChips = (draft = '') => {
  const chat = chatWith({ ...ROW, status: 'done' }, draft)
  chat.attachments.chips = ATTACHED
  return chat
}

describe('Agent, its attachments', () => {
  it('offers attaching an image between the field and Send, named for every reader', () => {
    const body = draw(chatWith({ ...ROW, status: 'done' }))
    const found = attachButton(body)
    expect(found?.[1]).toMatch(/^button plain svelte-[\w-]+ compact$/)
    expect(found?.[2]).not.toContain('disabled')
    expect(found?.[3]).toContain('<circle cx="9" cy="9" r="2"></circle>')
    expect(body.indexOf('<textarea')).toBeLessThan(body.indexOf('aria-label="Attach image"'))
    expect(body.indexOf('aria-label="Attach image"')).toBeLessThan(send(body)?.index ?? -1)
  })

  it('offers it while the agent is asking, as it pastes into whatever is there', () => {
    expect(attachButton(draw(chatWith(ROW)))?.[2]).not.toContain('disabled')
  })

  it('shows the attaching under way with a spinner in place of the icon, taking no second tap', () => {
    const chat = chatWith({ ...ROW, status: 'done' })
    void chat.attach()
    const found = attachButton(draw(chat))
    expect(found?.[1]).toMatch(/^button plain svelte-[\w-]+ compact busy$/)
    expect(found?.[2]).toContain('disabled=""')
    expect(found?.[2]).toContain('aria-busy="true"')
    expect(found?.[3]).toContain('class="spinner')
    expect(found?.[3]).not.toContain('<circle')
  })

  it('hands the reason of a failed attaching over, to be told', () => {
    expect(flat(agentSource)).toContain('const failed = await chat.attach() if (failed !== null) { onfailed(failed) }')
    expect(flat(agentSource)).toContain('busy={chat.attachments.attaching} onclick={attach}>')
  })

  it('shows each attached image as a chip with its name and size, between the keys and the field', () => {
    const body = draw(withChips())
    const chips = [...body.matchAll(/<li class="chip svelte-[\w-]+"><span class="name svelte-[\w-]+">([^<]*)<\/span><span class="size svelte-[\w-]+">([^<]*)<\/span><\/li>/g)]
    expect(chips.map((chip) => [chip[1], chip[2]])).toEqual([
      ['IMG_0042.jpg', '47 KiB'],
      ['pasted.png', '900 B'],
    ])
    expect(body).toMatch(/<ul class="chips svelte-[\w-]+" aria-label="Attached images">/)
    expect(body.indexOf('aria-label="Keys"')).toBeLessThan(body.indexOf('aria-label="Attached images"'))
    expect(body.indexOf('aria-label="Attached images"')).toBeLessThan(body.indexOf('<textarea'))
  })

  it('shows no chips before an image is attached', () => {
    expect(draw(chatWith({ ...ROW, status: 'done' }))).not.toContain('Attached images')
  })

  it('can send an empty field once an image is attached', () => {
    const body = draw(withChips())
    expect(send(body)?.[1]).toMatch(/^button primary svelte-[\w-]+$/)
    expect(send(body)?.[2]).not.toContain('disabled')
  })

  it('cuts a long name rather than its size', () => {
    expect(flat(agentSource)).toContain('.name { overflow: hidden; color: var(--soft); text-overflow: ellipsis; white-space: nowrap; }')
    expect(flat(agentSource)).toContain('.size { flex: none; color: var(--faint); }')
  })

  it('rests Send and Enter while an image is being attached, and leaves the other keys', () => {
    const chat = withChips('what is wrong here?')
    void chat.attach()
    const body = draw(chat)
    const button = (name: string) =>
      body.match(new RegExp(`<button class="([^"]*)" type="button"([^>]*) aria-label="${name}"`))
    expect(send(body)?.[1]).toMatch(/^button primary svelte-[\w-]+ resting$/)
    expect(send(body)?.[2]).toContain('disabled=""')
    expect(button('Enter')?.[1]).toMatch(/^button plain svelte-[\w-]+ compact resting$/)
    expect(button('Enter')?.[2]).toBe(' disabled="" aria-disabled="true"')
    expect(button('Escape')?.[2]).toBe('')
    expect(flat(keyRowSource)).toContain('resting={!chat.canPressKey(button.key)}')
  })

  it('offers no attaching without its agent, nor once it is gone', () => {
    const gone = withChips()
    gone.see(1, [{ ...ROW, pane: 30 }])
    expect(attachButton(draw(chatWith(null)))).toBeNull()
    expect(attachButton(draw(gone))).toBeNull()
    expect(draw(gone)).not.toContain('Attached images')
  })
})

const keys = (chat: Chat, initialDigits = false) =>
  render(KeyRow, { props: { chat, initialDigits, onfailed: () => {} } }).body

const group = (body: string, name: string) =>
  body.match(new RegExp(`<div class="row svelte-[\\w-]+" role="group" aria-label="${name}">(.*?)</div>`))?.[1] ?? ''

const labels = (markup: string) => [...markup.matchAll(/<button [^>]*aria-label="([^"]+)"/g)].map((found) => found[1])

const key = (body: string, name: string) =>
  body.match(new RegExp(`<button class="([^"]*)" type="button" aria-label="${name}"[^>]*>(.*?)</button>`))

describe('Agent, its keys', () => {
  it('shows every key in the composer, above the field, named for every reader', () => {
    const body = draw(chatWith({ ...ROW, status: 'done' }))
    expect(labels(group(body, 'Keys'))).toEqual(['Escape', 'Tab', 'Up', 'Down', 'Left', 'Right', 'Enter', 'Ctrl-C', 'Digits'])
    expect(body.indexOf('aria-label="Keys"')).toBeLessThan(body.indexOf('<textarea'))
    expect(flat(agentSource)).toContain('{/if} <KeyRow {chat} {onfailed} /> {#if chat.attachments.chips.length > 0}')
  })

  it('offers the keys while the agent is asking, when the field cannot send', () => {
    expect(labels(group(draw(chatWith(ROW)), 'Keys'))).toHaveLength(9)
  })

  it('has no keys without its agent, nor once it is gone', () => {
    const gone = chatWith(ROW)
    gone.see(1, [{ ...ROW, pane: 30 }])
    expect(draw(chatWith(null))).not.toContain('aria-label="Keys"')
    expect(draw(gone)).not.toContain('aria-label="Keys"')
  })

  it.each<[string, string | RegExp]>([
    ['Escape', 'Esc'],
    ['Tab', 'Tab'],
    ['Enter', 'Enter'],
    ['Ctrl-C', /<span class="svelte-[\w-]+ shown">Ctrl-C<\/span>/],
  ])('labels %s with its short name', (name, text) => {
    const found = key(keys(chatWith(ROW)), name)
    expect(found?.[1]).toMatch(/^button plain svelte-[\w-]+ compact$/)
    expect(found?.[2]).toMatch(text)
  })

  it.each(['Up', 'Down', 'Left', 'Right'])('draws %s as an arrow', (name) => {
    const drawn = key(keys(chatWith(ROW)), name)?.[2] ?? ''
    expect(drawn).toContain('<svg')
    expect(drawn).not.toMatch(/[↑↓←→]/)
  })

  it('presses the key of each button and hands the reason of a failure over, to be told', () => {
    expect(keyRowSource.match(/onclick=\{\(\) => void press\(button\.key\)\}/g)).toHaveLength(2)
    expect(flat(keyRowSource)).toContain(
      'const press = async (key: KeyName) => { const failed = await chat.press(key) if (failed !== null) { onfailed(failed) } }',
    )
  })

  it('keeps the digits folded until their button opens them', () => {
    const body = keys(chatWith(ROW))
    expect(key(body, 'Digits')?.[0]).toContain('aria-expanded="false"')
    expect(body).not.toContain('aria-label="1"')
  })

  it('opens the digits from 1 to 0 above the keys', () => {
    const body = keys(chatWith(ROW), true)
    expect(key(body, 'Digits')?.[0]).toContain('aria-expanded="true"')
    expect(labels(group(body, 'Digits'))).toEqual(['1', '2', '3', '4', '5', '6', '7', '8', '9', '0'])
    expect(body.indexOf('aria-label="Digits"')).toBeLessThan(body.indexOf('aria-label="Keys"'))
  })

  it('folds the digits only from their button, so they stay open after one is pressed', () => {
    expect(keyRowSource.match(/\bdigits = (?!\$state)/g)).toEqual(['digits = '])
    expect(keyRowSource).toContain('expanded={digits} onclick={() => (digits = !digits)}>0–9</Button>')
  })

  it('asks for a second tap in the danger tone once Ctrl-C is armed, and says it aloud', () => {
    const chat = chatWith(ROW)
    void chat.press('ctrl+c')
    const body = keys(chat)
    const found = key(body, 'Tap again to interrupt')
    expect(found?.[1]).toMatch(/^button danger svelte-[\w-]+ compact$/)
    expect(found?.[2]).toMatch(/<span class="svelte-[\w-]+ shown">Again\?<\/span>/)
    expect(found?.[2]).not.toContain('aria-live')
    expect(body).toContain('</button><!----></div> <span class="spoken" aria-live="polite">Tap again to interrupt</span></div>')
    expect(body).not.toContain('aria-label="Ctrl-C"')
  })

  it('keeps the live region outside the buttons, silent until Ctrl-C is armed', () => {
    expect(keys(chatWith(ROW))).toContain('</button><!----></div> <span class="spoken" aria-live="polite"></span></div>')
  })

  it('keeps the width of Ctrl-C when armed, so the row does not move under the second tap', () => {
    const drawn = (chat: Chat) => (key(keys(chat), chat.interrupting ? 'Tap again to interrupt' : 'Ctrl-C')?.[2] ?? '').replace(/ shown/, '')
    const chat = chatWith(ROW)
    const resting = drawn(chat)
    void chat.press('ctrl+c')
    expect(drawn(chat)).toEqual(resting)
    expect(resting).toMatch(/<span class="steady svelte-[\w-]+"><span class="svelte-[\w-]+">Ctrl-C<\/span><span class="svelte-[\w-]+">Again\?<\/span><\/span>/)
    expect(flat(keyRowSource)).toContain(
      '.steady { display: grid; } .steady > span { grid-area: 1 / 1; visibility: hidden; } .steady > .shown { visibility: visible; }',
    )
  })

  it('wraps the keys onto more lines rather than scroll sideways', () => {
    expect(flat(keyRowSource)).toContain('.row { display: flex; flex-wrap: wrap; gap: var(--gap); }')
  })
})

const CHIPS = [
  { name: 'IMG_0042.jpg', size: '47 KiB' },
  { name: 'pasted.png', size: '900 B' },
]

const outgoing = (state: Outgoing['state'], fields: Partial<Outgoing> = {}): Outgoing => ({
  text: 'what is wrong here?',
  chips: [],
  state,
  reason: state === 'failed' ? 'nothing was sent' : null,
  answered: null,
  ...fields,
})

const bubble = (shown: Outgoing | null, chat = chatWith({ ...ROW, status: 'done' })) => {
  chat.outgoing = shown
  return render(Sent, { props: { chat } }).body
}

const action = (body: string, name: string) =>
  body.match(new RegExp(`<button class="([^"]*)" type="button"([^>]*)>((?:(?!</button>)[^])*)${name}(?:<!---->|\\s)*</button>`))

describe('Sent', () => {
  it('shows the text in a bubble while it is sending, named as your message, with no actions', () => {
    const body = bubble(outgoing('sending'))
    expect(body).toMatch(/<article class="sent svelte-[\w-]+" aria-label="Your message, sending">/)
    expect(body).toMatch(/<p class="text svelte-[\w-]+">what is wrong here\?<\/p>/)
    expect(body).toMatch(/<p class="state svelte-[\w-]+"><span class="spinner[^]*?<\/span>(?:<!---->)?Sending…<\/p>/)
    expect(body).not.toContain('Retry')
    expect(body).not.toContain('Edit')
    expect(body).not.toContain('<ul')
  })

  it('says it was sent with a check, and offers nothing', () => {
    const body = bubble(outgoing('sent'))
    expect(body).toContain('aria-label="Your message, sent"')
    expect(body).toMatch(/<p class="state svelte-[\w-]+"><svg[^]*?<path d="M20 6 9 17l-5-5"><\/path>[^]*?<\/svg>(?:<!---->)?Sent<\/p>/)
    expect(body).not.toContain('class="spinner')
    expect(body).not.toContain('<button')
  })

  it('says why it was not sent and offers Retry and Edit, of 44 px, with their icons', () => {
    const body = bubble(outgoing('failed', { reason: 'it may have been sent, but no answer came back' }))
    expect(body).toContain('aria-label="Your message, not sent"')
    expect(body).toMatch(
      /<p class="state failed svelte-[\w-]+"><svg[^]*?<circle cx="12" cy="12" r="10"><\/circle>[^]*?<\/svg>(?:<!---->)?Not sent: it may have been sent, but no answer came back<\/p>/,
    )
    expect(action(body, 'Retry')?.[1]).toMatch(/^button plain svelte-[\w-]+$/)
    expect(action(body, 'Retry')?.[2]).toBe('')
    expect(action(body, 'Retry')?.[3]).toContain('<path d="M3 3v5h5"></path>')
    expect(action(body, 'Edit')?.[1]).toMatch(/^button plain svelte-[\w-]+$/)
    expect(action(body, 'Edit')?.[3]).toContain('<path d="m15 5 4 4"></path>')
    expect(body.indexOf('Not sent')).toBeLessThan(body.indexOf('Retry'))
    expect(body.indexOf('Retry')).toBeLessThan(body.indexOf('Edit'))
  })

  it('rests Retry while it cannot go, and leaves Edit', () => {
    const body = bubble(outgoing('failed'), chatWith(ROW))
    expect(action(body, 'Retry')?.[1]).toMatch(/^button plain svelte-[\w-]+ resting$/)
    expect(action(body, 'Retry')?.[2]).toBe(' disabled="" aria-disabled="true"')
    expect(action(body, 'Edit')?.[2]).toBe('')
  })

  it('shows the names of the images that went with it, without their sizes', () => {
    const body = bubble(outgoing('sent', { chips: CHIPS }))
    const names = [...body.matchAll(/<li class="chip svelte-[\w-]+">([^<]*)<\/li>/g)].map((found) => found[1])
    expect(names).toEqual(['IMG_0042.jpg', 'pasted.png'])
    expect(body).not.toContain('KiB')
    expect(body.indexOf('what is wrong here?')).toBeLessThan(body.indexOf('IMG_0042.jpg'))
  })

  it('shows only the chips when only an image went', () => {
    const body = bubble(outgoing('sending', { text: '', chips: CHIPS.slice(0, 1) }))
    expect(body).not.toContain('class="text')
    expect(body).toMatch(/<div class="bubble svelte-[\w-]+">(?:<!--[^>]*-->|\s)*<ul class="chips svelte-[\w-]+">/)
    expect(body).toContain('IMG_0042.jpg')
  })

  it('draws nothing while nothing was sent', () => {
    expect(bubble(null)).not.toContain('<article')
  })

  it('retries and edits through the chat, keeping the text as it was written', () => {
    expect(flat(sentSource)).toContain('resting={!chat.canRetry} onclick={() => void chat.retry()}>')
    expect(flat(sentSource)).toContain('onclick={() => chat.edit()}>')
    expect(flat(sentSource)).toContain('.text { margin: 0; overflow-wrap: anywhere; white-space: pre-wrap; }')
  })

  it('sits on the right', () => {
    expect(flat(sentSource)).toContain('align-items: flex-end;')
    expect(flat(sentSource)).toContain('align-self: flex-end;')
  })
})

const SINCE = Date.parse(ROW.since ?? '')

const LATER = SINCE + 3 * 60_000

const clockAt = (millis: number) => {
  const { clock, tick } = fakeClock()
  tick(millis)
  return { clock, tick }
}

const lineChat = (status: string | null, since: string | null = ROW.since, clock: Clock = clockAt(LATER).clock) => {
  const chat = new Chat(1, ROW.pane, unread, unsent, unpressed, unattached, still, clock)
  chat.see(1, [{ ...ROW, status, since }])
  return chat
}

const line = (chat: Chat, now = LATER) => render(AgentLine, { props: { chat, now } }).body

const region = (body: string) => body.match(/<p class="said svelte-[\w-]+" aria-live="polite">([^]*?)<\/p>/)?.[1]

const phrase = (text: string) => `<span class="phrase">${text}</span>`

describe('AgentLine', () => {
  it('says a working agent works, with its orbit and the time since it started outside the region', () => {
    const body = line(lineChat('working'))
    expect(body).toMatch(/<div class="line working svelte-[\w-]+">/)
    expect(body).toContain('aria-label="Working"')
    expect(body).toMatch(/<g class="orbit svelte-[\w-]+">/)
    expect(region(body)).toContain(phrase('claude is working…'))
    expect(region(body)).not.toContain('3 min')
    expect(body).toMatch(/<\/p> (?:<!--[^>]*-->)*<span class="time svelte-[\w-]+">· 3 min<\/span>/)
  })

  it('says a shell agent waits for a background command, timed and orbiting too', () => {
    const body = line(lineChat('shell'))
    expect(body).toMatch(/<g class="orbit svelte-[\w-]+">/)
    expect(region(body)).toContain(phrase('claude is waiting for a background command…'))
    expect(body).toMatch(/<span class="time svelte-[\w-]+">· 3 min<\/span>/)
  })

  it('shows no time when the start is not known', () => {
    const body = line(lineChat('working', null))
    expect(region(body)).toContain(phrase('claude is working…'))
    expect(body).not.toContain('class="time')
  })

  it('says a waiting agent waits for your answer, untimed', () => {
    const body = line(lineChat('waiting'))
    expect(body).toMatch(/<div class="line waiting svelte-[\w-]+">/)
    expect(region(body)).toContain(phrase('claude is waiting for your answer'))
    expect(body).not.toContain('class="time')
  })

  it('says a failed agent stopped, in the failure tone', () => {
    const body = line(lineChat('failed'))
    expect(body).toMatch(/<div class="line failed svelte-[\w-]+">/)
    expect(body).toContain('aria-label="Failed"')
    expect(region(body)).toContain(phrase('claude stopped with an error'))
    expect(flat(agentLineSource)).toContain('.failed { color: var(--failed); }')
  })

  it('mutes a working agent with no news for two minutes, and keeps the minutes out of the region', () => {
    const body = line(lineChat('working', ROW.since, clockAt(SINCE).clock), SINCE + 2 * 60_000)
    expect(body).toMatch(/<div class="line muted svelte-[\w-]+">/)
    expect(region(body)).toContain(phrase('claude is working… · no news'))
    expect(region(body)).not.toContain('min')
    expect(body).toMatch(/<span class="time svelte-[\w-]+">for 2 min · Ctrl-C interrupts<\/span>/)
    expect(flat(agentLineSource)).toContain('.muted, .muted .time { color: var(--muted); }')
  })

  it('keeps working one moment before two minutes without news', () => {
    expect(line(lineChat('working', ROW.since, clockAt(SINCE).clock), SINCE + 2 * 60_000 - 1)).toMatch(/<div class="line working svelte-[\w-]+">/)
  })

  it.each(['done', 'idle', 'pending', null])('draws nothing for a %o agent but keeps the region, hidden', (status) => {
    const body = line(lineChat(status))
    expect(body).toMatch(/<div class="line {2}svelte-[\w-]+ spoken">/)
    expect(body).not.toContain('<svg')
    expect(body).not.toContain('class="phrase"')
    expect(region(body)).toBe('<span class="spoken"></span> <!--[-1--><!--]-->')
  })

  it('keeps the region while the agent is not seen yet', () => {
    const chat = new Chat(1, ROW.pane, unread, unsent, unpressed, unattached, still, timers)
    expect(region(line(chat))).toBe('<span class="spoken"></span> <!--[-1--><!--]-->')
  })

  it.each([
    [outgoing('sending'), 'Sending'],
    [outgoing('sent'), 'Sent'],
    [outgoing('failed', { reason: 'the agent is gone' }), 'Not sent: the agent is gone'],
  ])('says what happened to your message in the same region', (shown, news) => {
    const chat = lineChat('working')
    chat.outgoing = shown
    expect(region(line(chat))).toContain(`<span class="spoken">${news}</span> <!--[0-->${phrase('claude is working…')}`)
  })
})

describe('Agent, its live line', () => {
  it('draws the line at the end of the thread, above the composer', () => {
    expect(flat(agentSource)).toContain('<Thread {chat} {now} /> <div class="composer">')
    expect(flat(threadSource)).toContain('{/each} <AgentLine {chat} {now} /> </div>')
  })

  it('tells the time by the chat clock', () => {
    const body = draw(lineChat('working', ROW.since, clockAt(SINCE + 7 * 60_000).clock))
    expect(body).toMatch(/<span class="time svelte-[\w-]+">· 7 min<\/span>/)
  })

  it('ticks with the chat clock every 15 seconds', () => {
    expect(agentSource).toContain('const TICK_MS = 15_000')
    expect(flat(agentSource)).toContain('let now = $state(untrack(() => chat.now()))')
    expect(flat(agentSource)).toContain(
      '$effect(() => { const told = chat const timer = setInterval(() => (now = told.now()), TICK_MS) return () => clearInterval(timer) })',
    )
  })
})

const BEFORE = message('Shall I run the migration?')

const threadChat = (shown: Chat['reading'], sent: Outgoing | null = outgoing('sent', { answered: BEFORE })) => {
  const chat = lineChat('working')
  chat.reading = shown
  chat.outgoing = sent
  return chat
}

const thread = (chat: Chat) => render(Thread, { props: { chat, now: LATER } }).body

const at = (body: string, mark: string) => {
  expect(body).toContain(mark)
  return body.indexOf(mark)
}

const order = (body: string) => [
  at(body, '<article class="message'),
  at(body, '<article class="sent'),
  at(body, '<p class="said'),
]

const count = (body: string, mark: string) => body.split(mark).length - 1

describe('Thread', () => {
  it('shows the message you answered, then your bubble, then the line, before the answer comes', () => {
    const body = thread(threadChat({ kind: 'message', message: BEFORE }))
    const [agent, you, line] = order(body)
    expect(agent).toBeLessThan(you)
    expect(you).toBeLessThan(line)
    expect(body).toContain('Shall I run the migration?')
  })

  it.each([
    ['its text', { ...BEFORE, text: 'Migrated.' }],
    ['the time it was written', { ...BEFORE, written: '2026-11-09T11:43:00.000Z' }],
  ])('shows your bubble, then the answer, then the line, once %s changes', (_, answer) => {
    const body = thread(threadChat({ kind: 'message', message: answer }))
    const [agent, you, line] = order(body)
    expect(you).toBeLessThan(agent)
    expect(agent).toBeLessThan(line)
  })

  it('keeps a single message of the agent and a single bubble', () => {
    const body = thread(threadChat({ kind: 'message', message: message('Migrated.') }))
    expect(count(body, '<article class="message')).toBe(1)
    expect(count(body, '<article class="sent')).toBe(1)
    expect(body).not.toContain('Shall I run the migration?')
  })

  it('shows only the message and the line while nothing was sent', () => {
    const body = thread(threadChat({ kind: 'message', message: BEFORE }, null))
    expect(body).not.toContain('<article class="sent')
    expect(at(body, '<article class="message')).toBeLessThan(at(body, '<p class="said'))
  })

  it('keeps your bubble below the reading until a message comes', () => {
    const body = thread(threadChat({ kind: 'loading' }))
    expect(at(body, 'Reading its last message…')).toBeLessThan(at(body, '<article class="sent'))
  })

  it('marks a message whose turn is not over, faintly, after its text', () => {
    const body = thread(threadChat({ kind: 'message', message: { ...BEFORE, turn_over: false } }, null))
    expect(body).toMatch(/<p class="going svelte-[\w-]+">More to come…<\/p>/)
    expect(at(body, 'Shall I run the migration?')).toBeLessThan(at(body, 'More to come…'))
    expect(flat(threadSource)).toContain('.written, .going { color: var(--faint); font-size: 13px; }')
  })

  it.each([true, null])('leaves the mark out when the turn is %o', (turnOver) => {
    const body = thread(threadChat({ kind: 'message', message: { ...BEFORE, turn_over: turnOver } }, null))
    expect(body).not.toContain('More to come')
  })

  it('replaces a growing message in its place, without reading it again from the start', async () => {
    const said = ['Reading thread.ts', 'Reading thread.ts and chat.svelte.ts']
    const chat = new Chat(1, ROW.pane, () => Promise.resolve(ok({ ...message(said.shift() ?? ''), turn_over: false })), unsent, unpressed, unattached, still, timers)
    chat.see(1, [{ ...ROW, status: 'working' }])
    await chat.refresh()
    expect(thread(chat)).toContain('Reading thread.ts')
    await chat.refresh()
    const body = thread(chat)
    expect(body).toContain('Reading thread.ts and chat.svelte.ts')
    expect(body).not.toContain('Reading its last message…')
    expect(threadSource).not.toContain('{#key')
  })

  it('goes down to the end after a reading only when the page was there before it', () => {
    expect(flat(threadSource)).toContain(
      '$effect.pre(() => { const was = read read = chat.reading.kind === \'message\' void chat.outgoing if (was && atEnd(window.scrollY, window.innerHeight, document.documentElement.scrollHeight)) { void tick().then(toEnd) } })',
    )
    expect(flat(threadSource)).toContain('const toEnd = () => window.scrollTo({ top: document.documentElement.scrollHeight })')
  })

  it('stacks your bubble on the right of the column', () => {
    expect(flat(threadSource)).toContain('.thread { display: flex; flex: 1; flex-direction: column;')
  })
})
