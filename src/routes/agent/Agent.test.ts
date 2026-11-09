import { render } from 'svelte/server'
import { describe, expect, it } from 'vitest'
import type { AgentRow_Serialize as AgentRow, ConnectionView_Serialize } from '../../lib/bindings'
import { Chat, type ReadLast, type SendText } from '../../lib/chat.svelte'
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

const unread: ReadLast = () => new Promise(() => {})

const TERMINAL = /<button class="button plain svelte-[\w-]+" type="button"[^>]*>(?:<!--[^>]*-->| )*Terminal/

const unsent: SendText = () => new Promise(() => {})

const flat = (text: string) => text.replace(/\s+/g, ' ')

const chatOf = (row: AgentRow | null, draft = '') => {
  const chat = new Chat(1, ROW.pane, unread, unsent)
  if (row !== null) {
    chat.see(1, [row])
  }
  chat.draft = draft
  return chat
}

const draw = (chat: Chat, below = 0, connection: ConnectionView_Serialize = CONNECTED) =>
  render(Agent, { props: { chat, connection, below, onback: () => {}, onfailed: () => {}, onterminal: () => {} } }).body

const send = (body: string) => body.match(/<button class="([^"]*)" type="button"([^>]*)>(?:<!--[^>]*-->|\s|<span[^>]*>.*?<\/span>)*Send/)

const minutesAgo = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString()

describe('Agent, its header', () => {
  it('shows the dot, the status in words, the agent, its place and its tab', () => {
    const body = draw(chatOf(ROW))
    expect(body).toContain('aria-label="Waiting"')
    expect(body).toMatch(/<h1 class="title svelte-[\w-]+">Fix the login flow<\/h1>/)
    expect(body).toMatch(
      /<p class="place svelte-[\w-]+"><span class="status svelte-[\w-]+">Waiting<\/span> · web › login › agent<\/p>/,
    )
    expect(body).not.toContain('no longer running')
  })

  it('names the agent when it has no title, and only the project when it has one workspace', () => {
    const body = draw(chatOf({ ...ROW, title: null, status: 'done', workspace_count: 1, tab_name: 'review' }))
    expect(body).toMatch(/<h1 class="title svelte-[\w-]+">claude<\/h1>/)
    expect(body).toMatch(/<span class="status svelte-[\w-]+">Done<\/span> · web › review<\/p>/)
  })

  it('sits below the connection bar', () => {
    expect(draw(chatOf(ROW), 52)).toContain('style="--below: 52px;"')
  })
})

describe('Agent, its message', () => {
  it('draws the message as markdown, with the time it was written', () => {
    const chat = chatOf(ROW)
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
    const chat = chatOf(ROW)
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
    const chat = chatOf(ROW)
    chat.reading = reading
    const body = draw(chat)
    expect(body).toMatch(new RegExp(`<p class="reason svelte-[\\w-]+">${text}</p>`))
    expect(body).not.toContain('class="markdown')
  })

  it('mutes the reason and the note', () => {
    expect(flat(agentSource)).toContain('.reason, .note { margin: 0; color: var(--muted); }')
  })
})

describe('Agent, its field', () => {
  it('asks for the answer in a text area, with a Send button ready for typed text', () => {
    const body = draw(chatOf({ ...ROW, status: 'done' }, 'go on'))
    expect(body).toMatch(/<textarea class="area svelte-[\w-]+" rows="1" aria-label="Answer" placeholder="Answer claude"/)
    expect(body).toContain('go on</textarea>')
    expect(send(body)?.[1]).toMatch(/^button primary svelte-[\w-]+$/)
    expect(send(body)?.[2]).not.toContain('disabled')
    expect(body).not.toContain('is asking you')
  })

  it('cannot send blank text', () => {
    const body = draw(chatOf({ ...ROW, status: 'done' }, '  '))
    expect(send(body)?.[1]).toContain(' resting')
    expect(send(body)?.[2]).toContain('disabled=""')
  })

  it('says the agent is asking on its screen, keeps the text and cannot send', () => {
    const body = draw(chatOf(ROW, 'yes, go on'))
    expect(body).toMatch(
      /<p class="note svelte-[\w-]+">claude is asking you something on its screen; answer with the keys\.<\/p>/,
    )
    expect(body).toContain('yes, go on</textarea>')
    expect(send(body)?.[2]).toContain('disabled=""')
  })

  it('shows the send under way, with the field held', () => {
    const chat = chatOf({ ...ROW, status: 'done' }, 'go on')
    void chat.send()
    const body = draw(chat)
    expect(send(body)?.[1]).toMatch(/^button primary svelte-[\w-]+ busy$/)
    expect(send(body)?.[2]).toContain('aria-busy="true"')
    expect(body).toMatch(/<textarea[^>]* disabled=""/)
  })

  it('hands the reason of a failed send over, to be told', () => {
    expect(flat(agentSource)).toContain('const failed = await chat.send() if (failed !== null) { onfailed(failed) }')
  })

  it('stays at the bottom of the screen while the message scrolls', () => {
    expect(flat(agentSource)).toContain('.composer { position: sticky; bottom: 0;')
  })
})

describe('Agent, without its row', () => {
  it('says so once its agent stops running, offers the board and has no field', () => {
    const chat = chatOf(ROW, 'go on')
    chat.see(1, [{ ...ROW, pane: 30 }])
    const body = draw(chat)
    expect(body).toMatch(/<p class="gone svelte-[\w-]+">This agent is no longer running\.<\/p>/)
    expect(body).toMatch(/<button class="button plain svelte-[\w-]+" type="button"[^>]*>(?:<!--[^>]*-->| )*Back to the board/)
    expect(body).not.toContain('<textarea')
    expect(body).not.toContain('<h1')
    expect(body).not.toContain('role="img"')
  })

  it('pulses the mark while it connects, before the first list', () => {
    const body = draw(chatOf(null), 0, { state: 'connecting' })
    expect(body).toMatch(/<span class="cell cursor svelte-[\w-]+ pulsing"><\/span>/)
    expect(body).not.toContain('no longer running')
    expect(body).not.toContain('<textarea')
  })

  it('keeps the mark still without a list once it stops connecting', () => {
    const body = draw(chatOf(null), 0, { state: 'unauthorized' })
    expect(body).toMatch(/<span class="cell cursor svelte-[\w-]+"><\/span>/)
    expect(body).not.toContain('no longer running')
  })

  it.each([ROW, null])('offers going back before it is gone', (row) => {
    expect(draw(chatOf(row))).toMatch(/<button class="button plain svelte-[\w-]+" type="button"[^>]*>(?:<!--[^>]*-->| )*Back<!---->/)
  })

  it('offers opening the terminal of its agent, beside going back', () => {
    expect(draw(chatOf(ROW))).toMatch(TERMINAL)
    expect(flat(agentSource)).toContain(
      '<Button tone="plain" onclick={onback}>Back</Button> {#if row !== null} {@const opened = row} <Button tone="plain" onclick={() => onterminal(opened)}>Terminal</Button> {/if} </div>',
    )
  })

  it('offers no terminal without its agent, nor once it is gone', () => {
    const gone = chatOf(ROW)
    gone.see(1, [{ ...ROW, pane: 30 }])
    expect(draw(chatOf(null))).not.toMatch(TERMINAL)
    expect(draw(gone)).not.toMatch(TERMINAL)
  })
})
