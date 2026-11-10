import { render } from 'svelte/server'
import { describe, expect, it } from 'vitest'
import type { Chat } from '../../lib/chat.svelte'
import { MACHINE, PANE, asked, fakeAsker, fresh, row, settle } from '../../lib/chatFakes'
import type { Question as Asked } from '../../lib/question.svelte'
import Agent from './Agent.svelte'
import agentSource from './Agent.svelte?raw'
import composerSource from './Composer.svelte?raw'
import Question from './Question.svelte'
import questionSource from './Question.svelte?raw'

const MENU = ['Which colour?', '1. Red', '2. Green', '3. Blue']

const flat = (text: string) => text.replace(/\s+/g, ' ')

const questionOf = (lines: string[]): Asked => {
  const { rows, options, colours } = asked(lines)
  return { rows, options, colours }
}

const drawn = (question: Asked, agent = 'claude') =>
  render(Question, { props: { question, agent, onoption: () => {} } }).body

const asking = async (lines: string[] | null) => {
  const told = fakeAsker()
  const chat = fresh({ asker: told.asker })
  chat.see(MACHINE, [row(PANE, 'waiting')])
  chat.watchQuestion()
  await settle()
  if (lines !== null) told.tell(asked(lines))
  return chat
}

const screen = (chat: Chat) =>
  render(Agent, {
    props: {
      chat,
      connection: { state: 'connected', route: 'ws://mac:7000' },
      agents: null,
      onfailed: () => {},
      ontoast: () => {},
      backTo: 'Agents',
      onback: () => {},
      onopen: () => {},
      onchanges: () => {},
      onfiles: () => {},
      onterminal: () => {},
    },
  }).body

const options = (body: string) =>
  [...body.matchAll(/<button [^>]*aria-label="(Option [^"]+)"[^>]*>(?:<!--[^>]*-->|\s)*<span class="label[^"]*">([^<]*)<\/span>/g)].map(
    (found) => [found[1], found[2]],
  )

describe('Agent, its question', () => {
  it('shows the question of its agent in the composer, instead of the note', async () => {
    const body = screen(await asking(MENU))
    expect(body).toContain('aria-label="codex asks"')
    expect(body).not.toContain('is asking you')
    expect(body.indexOf('aria-label="codex asks"')).toBeLessThan(body.indexOf('aria-label="Keys"'))
  })

  it('keeps the note while the question has no rows yet', async () => {
    const body = screen(await asking(null))
    expect(body).toContain('codex is asking you something on its screen; answer with the keys.')
    expect(body).not.toContain('asks"')
  })

  it('puts the options between the question and the keys', async () => {
    const body = screen(await asking(MENU))
    const options = body.indexOf('aria-label="Options"')
    expect(body.indexOf('aria-label="codex asks"')).toBeLessThan(options)
    expect(options).toBeLessThan(body.indexOf('aria-label="Keys"'))
  })

  it('follows the question while the page shows', () => {
    expect(flat(agentSource)).toContain(
      '$effect(() => { const asked = chat return untrack(() => whileShown(document, () => asked.watchQuestion())) })',
    )
  })

  it('presses the option tapped and hands the reason of a failure over, to be told', () => {
    expect(flat(composerSource)).toContain(
      'const choose = async (number: number) => { const failed = await chat.pressOption(number) if (failed !== null) { onfailed(failed) } }',
    )
    expect(flat(composerSource)).toContain('onoption={(number) => void choose(number)}')
    expect(flat(questionSource)).toContain('onclick={() => onoption(option.number)}')
  })
})

describe('Question', () => {
  it('draws every row of the screen, in the colours of the pane', () => {
    const body = drawn(questionOf(MENU))
    expect([...body.matchAll(/<span class="run[^"]*"[^>]*>([^<]*)<\/span>/g)].map((found) => found[1])).toEqual(MENU)
    expect(body).toMatch(/<div class="screen svelte-[\w-]+" role="group" aria-label="claude asks" style="background: rgb\(26 27 38\); color: rgb\(192 202 245\); font-size: 13px;/)
  })

  it('makes the rows as wide as the longest, so each keeps its line', () => {
    expect(drawn(questionOf(['Which colour?', 'Red']))).toContain('style="width: 7.8em;"')
  })

  it('names every option with its number and label, on a button of its own', () => {
    expect(options(drawn(questionOf(MENU)))).toEqual([
      ['Option 1, Red', '1. Red'],
      ['Option 2, Green', '2. Green'],
      ['Option 3, Blue', '3. Blue'],
    ])
  })

  it('offers no buttons when the screen holds no options', () => {
    const body = drawn(questionOf(['Go on? (y/n)']))
    expect([options(body), body.includes('aria-label="Options"')]).toEqual([[], false])
  })

  it('scrolls the screen within its own box, never the page sideways, and keeps its end in sight', () => {
    expect(flat(questionSource)).toContain(
      '.screen { position: relative; flex: 0 1 auto; min-height: calc(3 * var(--line)); max-height: 40dvh; overflow: auto;',
    )
    expect(flat(questionSource)).toContain('.rows, .probe { font-family: var(--font-mono); white-space: pre; }')
    expect(flat(questionSource)).toContain('if (question.rows.length > 0) anchor(box)')
  })

  it('stretches each option across the composer and cuts a long label with an ellipsis', () => {
    expect(flat(questionSource)).toContain('.options { display: grid; grid-template-columns: minmax(0, 1fr); gap: var(--gap); }')
    expect(flat(questionSource)).toContain(
      '.label { flex: 1; min-width: 0; overflow: hidden; text-align: left; text-overflow: ellipsis; }',
    )
  })
})
