import { render } from 'svelte/server'
import { describe, expect, it } from 'vitest'
import type { Chip } from '../../lib/attach.svelte'
import type { Chat } from '../../lib/chat.svelte'
import { CHIPS, ROW, chatWith, flat } from './agentFakes'
import Composer from './Composer.svelte'
import composerSource from './Composer.svelte?raw'

const draw = (chat: Chat) =>
  render(Composer, { props: { chat, agent: 'claude', onfailed: () => {} } }).body.replace(/<!--[^>]*-->/g, '')

const button = (body: string, name: string) =>
  body.match(new RegExp(`<button class="([^"]*)" type="button" aria-label="${name}"([^>]*)>(.*?)</button>`))

const send = (body: string) => button(body, 'Send')

const attachButton = (body: string) => button(body, 'Attach image')

const keysButton = (body: string) => button(body, 'Keys')

const KEY_ROW = /<div class="row svelte-[\w-]+" role="group" aria-label="Keys">/

const keyLabels = (body: string) =>
  [...(body.match(/role="group" aria-label="Keys">(.*?)<\/div>/)?.[1] ?? '').matchAll(/aria-label="([^"]+)"/g)].map(
    (found) => found[1],
  )

const PLUS = '<path d="M5 12h14"></path><path d="M12 5v14"></path>'

const ARROW_UP = '<path d="m5 12 7-7 7 7"></path><path d="M12 19V5"></path>'

const done = (draft = '') => chatWith({ ...ROW, status: 'done' }, draft)

const withChips = (draft = '', chips: Chip[] = CHIPS) => {
  const chat = done(draft)
  chat.attachments.chips = chips
  return chat
}

const opened = (chat: Chat) => {
  chat.toggleKeys()
  return chat
}

describe('Composer, its capsule', () => {
  it('holds Attach image, Keys, the field and Send, in this order', () => {
    const body = draw(done())
    const capsule = body.search(/<div class="capsule svelte-[\w-]+">/)
    const order = [
      capsule,
      attachButton(body)?.index ?? -1,
      keysButton(body)?.index ?? -1,
      body.indexOf('<textarea'),
      send(body)?.index ?? -1,
    ]
    expect(capsule).toBeGreaterThan(-1)
    expect(order).toEqual([...order].sort((a, b) => a - b))
  })

  it('rounds the capsule with the edge and the field colour, ringed while its field has the focus', () => {
    const source = flat(composerSource)
    expect(source).toContain(
      '.capsule { display: flex; flex: none; align-items: flex-end; border: 1px solid var(--edge); border-radius: var(--radius-sheet); background: var(--field); }',
    )
    expect(source).toContain(
      '.capsule:focus-within { border-color: var(--accent); box-shadow: 0 0 0 3px color-mix(in srgb, var(--brand) 32%, transparent); }',
    )
  })

  it('keeps its buttons at the foot as the field grows, each 44 by 44 px', () => {
    expect(flat(composerSource)).toContain(
      '.icon, .send { display: flex; flex: none; align-items: center; justify-content: center; width: 44px; height: 44px;',
    )
    expect(flat(composerSource)).toContain('.field { flex: 1; min-width: 0; }')
  })

  it('asks for the answer in a bare text area of the agent, the capsule drawing its frame', () => {
    const body = draw(done('go on'))
    expect(body).toMatch(/<div class="grow svelte-[\w-]+" data-value="go on"><textarea class="area svelte-[\w-]+" rows="1" aria-label="Answer" placeholder="Answer claude"/)
    expect(body).toContain('go on</textarea>')
    expect(flat(composerSource)).toContain('<TextArea bare label="Answer" placeholder="Answer {agent}" bind:value={chat.draft} />')
  })
})

describe('Composer, its Send', () => {
  it('is lit in the brand colour with an arrow for typed text', () => {
    const found = send(draw(done('go on')))
    expect(found?.[1]).toMatch(/^send svelte-[\w-]+ lit$/)
    expect(found?.[2]).not.toContain('disabled')
    expect(found?.[3]).toContain(ARROW_UP)
  })

  it('is dimmed and disabled for blank text', () => {
    const found = send(draw(done('  ')))
    expect(found?.[1]).toMatch(/^send svelte-[\w-]+$/)
    expect(found?.[2]).toContain('disabled=""')
    expect(found?.[2]).toContain('aria-disabled="true"')
    expect(found?.[3]).toContain(ARROW_UP)
  })

  it('draws a circle of 32 px, in the brand colour only while lit', () => {
    const source = flat(composerSource)
    expect(source).toContain(
      '.disc { display: flex; align-items: center; justify-content: center; width: 32px; height: 32px; border-radius: 50%; background: var(--hover); color: var(--faint); }',
    )
    expect(source).toContain('.lit .disc { background: var(--brand); color: var(--on-brand); }')
  })

  it('says the agent is asking on its screen, keeps the text and does nothing', () => {
    const body = draw(chatWith(ROW, 'yes, go on'))
    expect(body).toMatch(
      /<p class="note svelte-[\w-]+">claude is asking you something on its screen; answer with the keys\.<\/p>/,
    )
    expect(body).toContain('yes, go on</textarea>')
    expect(send(body)?.[2]).toContain('disabled=""')
    expect(flat(composerSource)).toContain('.note { margin: 0; color: var(--muted); }')
  })

  it('shows the send under way with a spinner in place of the arrow, the field emptied and open', () => {
    const chat = done('go on')
    void chat.send()
    const body = draw(chat)
    const found = send(body)
    expect(found?.[2]).toContain('aria-busy="true"')
    expect(found?.[2]).toContain('disabled=""')
    expect(found?.[2]).not.toContain('aria-disabled')
    expect(found?.[3]).toContain('class="spinner')
    expect(found?.[3]).not.toContain(ARROW_UP)
    expect(body).not.toMatch(/<textarea[^>]* disabled=""/)
    expect(body).toContain('></textarea>')
  })

  it('leaves a failed send to its bubble, with no toast', () => {
    expect(flat(composerSource)).toContain('onclick={() => void chat.send()}')
    expect(flat(composerSource)).not.toContain('await chat.send()')
  })

  it('can send an empty field once an image is attached', () => {
    const found = send(draw(withChips()))
    expect(found?.[1]).toMatch(/^send svelte-[\w-]+ lit$/)
    expect(found?.[2]).not.toContain('disabled')
  })

  it('rests while an image is being attached, and leaves every key, Enter too', () => {
    const chat = opened(withChips('what is wrong here?'))
    void chat.attach()
    const body = draw(chat)
    expect(send(body)?.[2]).toContain('disabled=""')
    expect(button(body, 'Enter')?.[1]).toMatch(/^button plain svelte-[\w-]+ compact$/)
    expect(button(body, 'Enter')?.[2]).toBe('')
    expect(button(body, 'Escape')?.[2]).toBe('')
  })
})

describe('Composer, its keys', () => {
  it('start folded behind the Keys icon', () => {
    const body = draw(done())
    expect(keysButton(body)?.[1]).toMatch(/^icon svelte-[\w-]+$/)
    expect(keysButton(body)?.[2]).toContain('aria-expanded="false"')
    expect(keysButton(body)?.[3]).toContain('<svg')
    expect(body).not.toMatch(KEY_ROW)
  })

  it('open from the icon above the capsule, with the digits still folded inside', () => {
    const body = draw(opened(done()))
    expect(keysButton(body)?.[2]).toContain('aria-expanded="true"')
    expect(keyLabels(body)).toEqual(['Escape', 'Tab', 'Up', 'Down', 'Left', 'Right', 'Enter', 'Ctrl-C', 'Digits'])
    expect(body).not.toContain('aria-label="1"')
    expect(body.search(KEY_ROW)).toBeLessThan(body.search(/<div class="capsule/))
  })

  it('fold again from the icon', () => {
    const chat = opened(opened(done()))
    const body = draw(chat)
    expect(keysButton(body)?.[2]).toContain('aria-expanded="false"')
    expect(body).not.toMatch(KEY_ROW)
    expect(flat(composerSource)).toContain('aria-expanded={chat.keysOpen} onclick={() => chat.toggleKeys()}')
  })

  it('open on their own when the agent starts waiting, below the note', () => {
    const chat = done()
    chat.see(1, [ROW])
    const body = draw(chat)
    expect(keysButton(body)?.[2]).toContain('aria-expanded="true"')
    expect(keyLabels(body)).toHaveLength(9)
    expect(body.indexOf('class="note')).toBeLessThan(body.search(KEY_ROW))
  })
})

describe('Composer, its attachments', () => {
  it('offers attaching an image first in the capsule, as a plus named for every reader', () => {
    const found = attachButton(draw(done()))
    expect(found?.[1]).toMatch(/^icon svelte-[\w-]+$/)
    expect(found?.[2]).not.toContain('disabled')
    expect(found?.[3]).toContain(PLUS)
  })

  it('offers it while the agent is asking, as it pastes into whatever is there', () => {
    expect(attachButton(draw(chatWith(ROW)))?.[2]).not.toContain('disabled')
  })

  it('shows the attaching under way with a spinner in place of the plus, taking no second tap', () => {
    const chat = done()
    void chat.attach()
    const found = attachButton(draw(chat))
    expect(found?.[2]).toContain('disabled=""')
    expect(found?.[2]).toContain('aria-busy="true"')
    expect(found?.[3]).toContain('class="spinner')
    expect(found?.[3]).not.toContain(PLUS)
  })

  it('takes no more images once four are kept', () => {
    const four = [...CHIPS, { ...CHIPS[0], id: 3 }, { ...CHIPS[1], id: 4 }]
    expect(attachButton(draw(withChips('', four)))?.[2]).toContain('disabled=""')
    expect(attachButton(draw(withChips('', CHIPS)))?.[2]).not.toContain('disabled')
  })

  it('hands the reason of a failed attaching over, to be told', () => {
    expect(flat(composerSource)).toContain('const failed = await chat.attach() if (failed !== null) { onfailed(failed) }')
    expect(flat(composerSource)).toContain('onclick={attach}')
  })

  it('shows each attached image as a chip with its name and size, between the keys and the capsule', () => {
    const body = draw(opened(withChips()))
    const chips = [...body.matchAll(/<li class="chip svelte-[\w-]+"><span class="name svelte-[\w-]+">([^<]*)<\/span><span class="size svelte-[\w-]+">([^<]*)<\/span><\/li>/g)]
    expect(chips.map((chip) => [chip[1], chip[2]])).toEqual([
      ['IMG_0042.jpg', '47 KiB'],
      ['pasted.png', '900 B'],
    ])
    expect(body).toMatch(/<ul class="chips svelte-[\w-]+" aria-label="Attached images">/)
    expect(body.search(KEY_ROW)).toBeLessThan(body.indexOf('aria-label="Attached images"'))
    expect(body.indexOf('aria-label="Attached images"')).toBeLessThan(body.search(/<div class="capsule/))
  })

  it('shows no chips before an image is attached', () => {
    expect(draw(done())).not.toContain('Attached images')
  })

  it('cuts a long name rather than its size', () => {
    expect(flat(composerSource)).toContain('.name { overflow: hidden; color: var(--soft); text-overflow: ellipsis; white-space: nowrap; }')
    expect(flat(composerSource)).toContain('.size { flex: none; color: var(--faint); }')
  })
})

describe('Composer, its place', () => {
  it('stays at the bottom of the screen while the message scrolls', () => {
    expect(flat(composerSource)).toContain('.composer { position: sticky; bottom: 0;')
  })

  it('never grows over the bar and the title, scrolling within itself instead', () => {
    expect(flat(composerSource)).toContain(
      'max-height: calc(100dvh - env(safe-area-inset-top) - var(--below) - var(--over-composer)); overflow-y: auto;',
    )
    expect(flat(composerSource)).toContain('--over-composer: 132px;')
    expect(render(Composer, { props: { chat: done(), agent: 'claude', below: 52, onfailed: () => {} } }).body).toMatch(
      /<div class="composer svelte-[\w-]+" style="--below: 52px;">/,
    )
  })
})
