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

describe('Glass', () => {
  it('draws its content under the top highlight', () => {
    const body = render(Glass, { props: { children: html('<p>inside</p>') } }).body
    expect(body).toMatch(/<div class="glass svelte-[\w-]+"><span class="highlight svelte-[\w-]+"><\/span> <p>inside<\/p>/)
  })

  it('takes the card radius unless given another', () => {
    expect(glassSource).toContain('--corner: var(--radius-card);')
    const body = render(Glass, { props: { radius: 'var(--radius-sheet)', children: html('<p>inside</p>') } }).body
    expect(body).toContain('style="--corner: var(--radius-sheet);"')
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
    expect(button({ resting: true })).toMatch(/<button class="button svelte-[\w-]+ resting" type="button" disabled="" aria-disabled="true">Add machine/)
  })

  it('is enabled otherwise', () => {
    const body = button({})
    expect(body).toMatch(/<button class="button svelte-[\w-]+" type="button">Add machine/)
    expect(body).not.toMatch(/disabled/)
  })

  it('is big enough to touch and shows a press only when it can be pressed', () => {
    expect(buttonSource).toContain('min-height: 44px;')
    expect(buttonSource).toContain('.button:not(.resting):active {\n    background: var(--brand-hover);')
    expect(buttonSource).toMatch(/\.resting \{\s*opacity: 0\.4;\s*pointer-events: none;\s*\}/)
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
})
