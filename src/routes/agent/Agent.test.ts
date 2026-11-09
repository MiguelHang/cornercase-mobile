import { render } from 'svelte/server'
import { describe, expect, it } from 'vitest'
import type { AgentRow_Serialize as AgentRow, ConnectionView_Serialize } from '../../lib/bindings'
import type { AttachImage } from '../../lib/attach.svelte'
import { Chat, timers, type Outgoing, type PressKey, type ReadLast, type SendText } from '../../lib/chat.svelte'
import Agent from './Agent.svelte'
import agentSource from './Agent.svelte?raw'
import KeyRow from './KeyRow.svelte'
import keyRowSource from './KeyRow.svelte?raw'
import Sent from './Sent.svelte'
import sentSource from './Sent.svelte?raw'

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

const draw = (chat: Chat, below = 0, connection: ConnectionView_Serialize = CONNECTED, starting: string | null = null) =>
  render(Agent, {
    props: { chat, connection, below, starting, onback: () => {}, onfailed: () => {}, onterminal: () => {} },
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
    expect(flat(agentSource)).toContain('.reason, .note { margin: 0; color: var(--muted); }')
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
  it('says so once its agent stops running, offers the board and has no field', () => {
    const chat = chatWith(ROW, 'go on')
    chat.see(1, [{ ...ROW, pane: 30 }])
    const body = draw(chat)
    expect(body).toMatch(/<p class="gone svelte-[\w-]+">This agent is no longer running\.<\/p>/)
    expect(body).toMatch(/<button class="button plain svelte-[\w-]+" type="button"[^>]*>(?:<!--[^>]*-->| )*Back to the board/)
    expect(body).not.toContain('<textarea')
    expect(body).not.toContain('<h1')
    expect(body).not.toContain('role="img"')
  })

  it('says the agent it started is starting until its row comes, with Back and no field', () => {
    const chat = chatWith(null)
    chat.see(1, [{ ...ROW, pane: 30 }], true)
    const body = draw(chat, 0, CONNECTED, 'claude')
    expect(body).toMatch(STARTING)
    expect(body).toMatch(/<button class="button plain svelte-[\w-]+" type="button"[^>]*>(?:<!--[^>]*-->| )*Back<!---->/)
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

  it.each([ROW, null])('offers going back before it is gone', (row) => {
    expect(draw(chatWith(row))).toMatch(/<button class="button plain svelte-[\w-]+" type="button"[^>]*>(?:<!--[^>]*-->| )*Back<!---->/)
  })

  it('offers opening the terminal of its agent, beside going back', () => {
    expect(draw(chatWith(ROW))).toMatch(TERMINAL)
    expect(flat(agentSource)).toContain(
      '<Button tone="plain" onclick={onback}>Back</Button> {#if row !== null} {@const opened = row} <Button tone="plain" onclick={() => onterminal(opened)}>Terminal</Button> {/if} </div>',
    )
  })

  it('offers no terminal without its agent, nor once it is gone', () => {
    const gone = chatWith(ROW)
    gone.see(1, [{ ...ROW, pane: 30 }])
    expect(draw(chatWith(null))).not.toMatch(TERMINAL)
    expect(draw(gone)).not.toMatch(TERMINAL)
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
