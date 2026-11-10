import { render } from 'svelte/server'
import { describe, expect, it } from 'vitest'
import { Chat, timers, type Outgoing } from '../../lib/chat.svelte'
import { message, ok } from '../../lib/chatFakes'
import { LATER, ROW, SINCE, chatWith, clockAt, flat, lineChat, still, unasked, unattached, unmarked, unpressed, unread, unsent } from './agentFakes'
import AgentLine from './AgentLine.svelte'
import agentLineSource from './AgentLine.svelte?raw'
import Sent from './Sent.svelte'
import sentSource from './Sent.svelte?raw'
import Thread from './Thread.svelte'
import threadSource from './Thread.svelte?raw'

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
    const chat = new Chat(1, ROW.pane, unread, unsent, unpressed, unmarked, unattached, still, timers, unasked)
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
    const chat = new Chat(1, ROW.pane, () => Promise.resolve(ok({ ...message(said.shift() ?? ''), turn_over: false })), unsent, unpressed, unmarked, unattached, still, timers, unasked)
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
