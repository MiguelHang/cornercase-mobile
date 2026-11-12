import { render } from 'svelte/server'
import { describe, expect, it } from 'vitest'
import type { Card_Serialize as Card } from '../../lib/bindings'
import type { Chat } from '../../lib/chat.svelte'
import { MACHINE, PANE, asked, fakeAsker, fresh, row, settle } from '../../lib/chatFakes'
import type { Question as Asked } from '../../lib/question.svelte'
import Agent from './Agent.svelte'
import agentSource from './Agent.svelte?raw'
import composerSource from './Composer.svelte?raw'
import Question from './Question.svelte'
import questionSource from './Question.svelte?raw'
import cardSource from './QuestionCard.svelte?raw'
import tabsSource from './QuestionTabs.svelte?raw'

const MENU = ['Which colour?', '1. Red', '2. Green', '3. Blue']

const flat = (text: string) => text.replace(/\s+/g, ' ')

const questionOf = (lines: string[]): Asked => {
  const { rows, options, colours, card } = asked(lines)
  return { rows, options, colours, card }
}

const drawn = (question: Asked, agent = 'claude') =>
  render(Question, { props: { question, agent, onoption: () => {}, ontab: () => {} } }).body

const PERMISSION: Card = {
  tabs: [],
  open: null,
  blocks: [
    { kind: 'text', text: 'Bash command' },
    { kind: 'code', text: 'date > later.txt' },
    { kind: 'text', text: 'Do you want to proceed?' },
  ],
  options: [
    { number: 1, label: 'Yes', detail: null, selected: true },
    { number: 2, label: 'Yes, and always allow access to', detail: '/Users/dev/web-shop from this project', selected: false },
    { number: 3, label: 'No', detail: null, selected: false },
  ],
  help: ['Esc to cancel · Tab to amend'],
}

const tabbed = (open: number | null, labels = ['Colour', 'Size', 'Submit']): Card => ({
  ...PERMISSION,
  tabs: labels.map((label) => ({ label, answered: label === 'Submit' })),
  open,
})

const carded = (card: Card) => drawn({ ...questionOf(MENU), card })

const chips = (body: string) =>
  [...body.matchAll(/<(button|span) class="chip[^"]*"([^>]*)>(.*?)(?=<(?:button|span) class="chip|<\/div>)/g)].map((found) => [
    found[1],
    found[2].includes('aria-current="true"'),
    found[3].replace(/<[^>]*>/g, ''),
  ])

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
      onstart: () => {},
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
    expect(flat(cardSource)).toContain('onclick={() => onoption(option.number)}')
  })

  it('opens the tab tapped and hands the reason of a failure over, to be told', () => {
    expect(flat(composerSource)).toContain(
      'const openTab = async (index: number) => { const failed = await chat.openTab(index) if (failed !== null) { onfailed(failed) } }',
    )
    expect(flat(composerSource)).toContain('ontab={(index) => void openTab(index)}')
    expect(flat(tabsSource)).toContain('onclick={() => ontab(index)}')
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

describe('Question, as a card', () => {
  it('lays the card out instead of the rows of the screen', () => {
    const body = carded(PERMISSION)
    expect(body).toContain('aria-label="claude asks"')
    expect(body).not.toContain('class="rows')
    expect(body).not.toContain('class="screen')
  })

  it('writes the text as paragraphs, the code as a block of its own and the help below the options', () => {
    const body = carded(PERMISSION)
    const blocks = [...body.matchAll(/<(p|pre) class="(text|code|help)[^"]*">([^<]*)<\/\1>/g)].map((found) => [found[2], found[3]])
    expect(blocks).toEqual([
      ['text', 'Bash command'],
      ['code', 'date > later.txt'],
      ['text', 'Do you want to proceed?'],
      ['help', 'Esc to cancel · Tab to amend'],
    ])
    expect(body.indexOf('aria-label="Options"')).toBeLessThan(body.indexOf('class="help'))
  })

  it('puts every option on a button of its own, with its number, its label and its detail', () => {
    const body = carded(PERMISSION)
    const answers = [...body.matchAll(/aria-label="(Option [^"]+)"[^>]*>.*?<span class="label[^"]*">([^<]*)<\/span>(?:<!--[^>]*-->)*(?:<span class="detail[^"]*">([^<]*)<\/span>)?/g)]
    expect(answers.map((found) => [found[1], found[2], found[3] ?? null])).toEqual([
      ['Option 1, Yes', '1. Yes', null],
      ['Option 2, Yes, and always allow access to, /Users/dev/web-shop from this project', '2. Yes, and always allow access to', '/Users/dev/web-shop from this project'],
      ['Option 3, No', '3. No', null],
    ])
  })

  it.each<[string, number]>([
    ['the first', 0],
    ['the last', 2],
  ])('marks %s option chosen, for the eye and for VoiceOver', (_, chosen) => {
    const card = { ...PERMISSION, options: PERMISSION.options.map((option, at) => ({ ...option, selected: at === chosen })) }
    const buttons = [...carded(card).matchAll(/<button class="button plain[^"]*"[^>]*>(?:<!--[^>]*-->|\s)*<span class="mark[^"]*"[^>]*>([^<]*)<\/span>/g)]
    expect(buttons.map((found) => [found[0].includes('aria-current="true"'), found[0].includes(' current'), found[1]])).toEqual(
      PERMISSION.options.map((_, at) => (at === chosen ? [true, true, '❯'] : [false, false, ''])),
    )
  })

  it('draws no tabs when the question has none', () => {
    expect(carded(PERMISSION)).not.toContain('aria-label="Questions"')
  })

  it.each<[string, Card, (string | boolean)[][]]>([
    ['makes every tab but the open one a button', tabbed(0), [['span', true, 'Colour'], ['button', false, 'Size'], ['button', false, '✔\uFE0ESubmit']]],
    ['follows the open tab', tabbed(1), [['button', false, 'Colour'], ['span', true, 'Size'], ['button', false, '✔\uFE0ESubmit']]],
    ['offers no tab to tap when none is seen open', tabbed(null), [['span', false, 'Colour'], ['span', false, 'Size'], ['span', false, '✔\uFE0ESubmit']]],
    ['shows a lone question as a heading, not a button', tabbed(0, ['Colour']), [['span', true, 'Colour']]],
  ])('%s', (_, card, expected) => {
    expect(chips(carded(card))).toEqual(expected)
  })

  it('names the answered tabs for VoiceOver', () => {
    expect(carded(tabbed(0))).toContain('<span class="tick svelte-')
    expect(carded(tabbed(0))).toMatch(/<span class="tick svelte-[\w-]+" role="img" aria-label="answered">✔\uFE0E<\/span>/)
  })

  it.each([
    '.text { font-family: var(--font-ui); white-space: pre-line; }',
    'font-family: var(--font-mono); font-size: 13px; white-space: pre-wrap; }',
    '.text, .code, .help { margin: 0; overflow-wrap: anywhere; }',
    '.options { display: grid; grid-template-columns: minmax(0, 1fr); gap: var(--gap); }',
    'white-space: normal; overflow-wrap: anywhere; }',
    '.detail, .help { color: var(--muted); }',
  ])('wraps every line to the width of the phone: %s', (text) => {
    expect(flat(cardSource)).toContain(text)
  })

  it('puts the mark of the chosen option on the line of its label, not in the middle of a tall button', () => {
    expect(flat(cardSource)).toContain(
      '.mark { flex: none; align-self: flex-start; width: 1em; padding-top: var(--gap); color: var(--accent); }',
    )
    expect(flat(cardSource)).toContain('padding: var(--gap) 0;')
  })

  it('never keeps a line whole nor cuts it with an ellipsis', () => {
    expect(flat(cardSource)).not.toMatch(/white-space: (pre|nowrap);|text-overflow|overflow-x: (auto|scroll|visible)/)
    expect(flat(tabsSource)).not.toMatch(/white-space: (pre|nowrap);|text-overflow/)
  })

  it('yields to the composer and scrolls only up and down inside itself', () => {
    expect(flat(cardSource)).toContain(
      '.card { display: flex; flex: 0 1 auto; flex-direction: column; gap: var(--gap); min-width: 0; min-height: calc(44px + 2 * var(--gap) + 3lh); max-height: 40dvh; overflow-x: hidden; overflow-y: auto; overscroll-behavior: contain;',
    )
  })

  it('follows the chosen option when it changes, when the question changes and when the card changes height', () => {
    expect(flat(cardSource)).toContain('const asked = $derived(JSON.stringify([card.open, card.blocks]))')
    expect(flat(cardSource)).toContain('bind:clientHeight={tall} {@attach follow(chosen, asked)}')
    expect(flat(cardSource)).toContain(
      '(box) => { void tall const fresh = question !== shown moved = !fresh && (moved || number !== picked) shown = question picked = number',
    )
    expect(flat(cardSource)).toContain('const seen = under(box.getBoundingClientRect(), tabs?.getBoundingClientRect() ?? null)')
    expect(flat(cardSource)).toContain('box.scrollTop = scrolledTo(box.scrollTop, fresh, seen, rect(option), rect(lead))')
  })

  it('keeps the start of the question in sight until the choice moves within it', () => {
    expect(flat(cardSource)).toContain("const lead = moved ? null : box.querySelector(':scope > .text, :scope > .code')")
  })

  it('keeps the tabs in sight over the scrolled card, on the composer\'s own background', () => {
    expect(flat(tabsSource)).toContain('.tabs { position: sticky; top: 0; z-index: 1;')
    expect(flat(tabsSource)).toContain('background: var(--glass); }')
  })

  it('ends the tabs in a band of their own with a line, so a row cut under them reads as cut', () => {
    expect(flat(tabsSource)).toContain(
      'margin-bottom: calc(-1 * var(--gap)); padding-bottom: var(--gap); border-bottom: 1px solid var(--border);',
    )
  })

  it('gives every tab a touch target of 44 px', () => {
    expect(flat(tabsSource)).toContain('min-height: 44px;')
  })

  it('wraps the tabs onto more lines rather than hiding any', () => {
    expect(flat(tabsSource)).toContain('display: flex; flex-wrap: wrap; gap: 6px;')
    expect(flat(tabsSource)).toContain('.open { border-color: var(--accent); background: var(--accent-soft); color: var(--text); }')
  })
})
