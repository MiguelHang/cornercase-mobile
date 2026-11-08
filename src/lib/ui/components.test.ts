import { createRawSnippet } from 'svelte'
import { render } from 'svelte/server'
import { describe, expect, it } from 'vitest'
import Button from './Button.svelte'
import Emblem from './Emblem.svelte'
import Glass from './Glass.svelte'
import buttonSource from './Button.svelte?raw'
import emblemSource from './Emblem.svelte?raw'
import glassSource from './Glass.svelte?raw'

const html = (markup: string) => createRawSnippet(() => ({ render: () => markup }))

const flat = (text: string) => text.replace(/\s+/g, ' ')

describe('Glass', () => {
  it('draws its content under the top highlight', () => {
    const body = render(Glass, { props: { children: html('<p>inside</p>') } }).body
    expect(body).toMatch(/<div class="glass svelte-[\w-]+"><span class="highlight svelte-[\w-]+"><\/span> <p>inside<\/p>/)
  })

  it.each([
    'border: 1px solid var(--border);',
    'border-radius: var(--radius-card);',
    'background: linear-gradient(180deg, #1c1f24e0, #101215e8);',
    'box-shadow: 0 24px 60px rgb(0 0 0 / 0.5), 0 4px 12px rgb(0 0 0 / 0.3);',
    'left: var(--radius-card); right: var(--radius-card); height: 1px;',
    'background: linear-gradient(90deg, rgb(236 238 241 / 0.18), rgb(236 238 241 / 0.02));',
  ])('holds %s', (text) => {
    expect(flat(glassSource)).toContain(text)
  })

  it('blurs what is behind it with and without the prefix', () => {
    expect(glassSource).toContain('-webkit-backdrop-filter: blur(24px);')
    expect(glassSource).toContain('\n    backdrop-filter: blur(24px);')
  })
})

describe('Button', () => {
  const button = (props: { resting?: boolean }) =>
    render(Button, { props: { ...props, children: html('Add machine') } }).body

  it('is disabled for every reader while resting', () => {
    expect(button({ resting: true })).toMatch(
      /<button class="button svelte-[\w-]+ resting" type="button" disabled="" aria-disabled="true">Add machine/,
    )
  })

  it('is enabled otherwise', () => {
    const body = button({})
    expect(body).toMatch(/<button class="button svelte-[\w-]+" type="button">Add machine/)
    expect(body).not.toMatch(/disabled/)
  })

  it.each([
    'border: 1px solid rgb(43 123 255 / 0.7);',
    'border-radius: var(--radius-control);',
    'background: linear-gradient(180deg, var(--brand-light), var(--brand));',
    'box-shadow: 0 0 10px rgb(0 95 255 / 0.28);',
    'color: var(--on-brand);',
    'font-weight: 500;',
  ])('holds %s', (text) => {
    expect(flat(buttonSource)).toContain(text)
  })

  it('is big enough to touch and shows a press only when it can be pressed', () => {
    expect(buttonSource).toContain('min-height: 44px;')
    expect(buttonSource).toContain('.button:not(.resting):active {\n    background: var(--brand-hover);')
    expect(buttonSource).toMatch(/\.resting \{\s*opacity: 0\.4;\s*pointer-events: none;\s*\}/)
  })

  it('listens to touches so that WebKit on iOS shows the press', () => {
    expect(buttonSource).toContain('ontouchstart={() => {}}')
  })
})

describe('Emblem', () => {
  const body = render(Emblem, {
    props: { icon: 'server', title: 'No machines yet', lines: ['First line.', 'Second line.'] },
  }).body

  it('draws the icon in its disc', () => {
    expect(body).toMatch(/<div class="disc svelte-[\w-]+"><svg [^>]*width="24" height="24"/)
    expect(emblemSource).toMatch(/\.disc \{[^}]*width: 52px;\s*height: 52px;/)
  })

  it('writes the title and each line', () => {
    expect(body).toMatch(/<h2 class="title svelte-[\w-]+">No machines yet<\/h2>/)
    const lines = [...body.matchAll(/<p class="line svelte-[\w-]+">([^<]*)<\/p>/g)].map((match) => match[1])
    expect(lines).toEqual(['First line.', 'Second line.'])
  })

  it.each([
    'border: 1px solid rgb(43 123 255 / 0.35);',
    'background: rgb(0 95 255 / 0.16);',
    'color: var(--accent);',
    'gap: 10px;',
    'padding: 40px var(--inset-card) var(--inset-card);',
    'max-width: 470px;',
    'color: var(--muted);',
  ])('holds %s', (text) => {
    expect(flat(emblemSource)).toContain(text)
  })
})
