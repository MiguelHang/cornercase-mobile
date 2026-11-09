import { createRawSnippet } from 'svelte'
import { render } from 'svelte/server'
import { describe, expect, it } from 'vitest'
import Button from './Button.svelte'
import Emblem from './Emblem.svelte'
import Field from './Field.svelte'
import Glass from './Glass.svelte'
import Icon from './Icon.svelte'
import Spinner from './Spinner.svelte'
import buttonSource from './Button.svelte?raw'
import emblemSource from './Emblem.svelte?raw'
import fieldSource from './Field.svelte?raw'
import glassSource from './Glass.svelte?raw'
import spinnerSource from './Spinner.svelte?raw'
import baseSource from '../theme/base.css?raw'

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
  type Props = { tone?: 'primary' | 'plain' | 'danger'; type?: 'button' | 'submit'; resting?: boolean; busy?: boolean }
  const button = (props: Props, label = 'Add machine') =>
    render(Button, { props: { ...props, children: html(label) } }).body

  it('is disabled for every reader while resting', () => {
    expect(button({ resting: true })).toMatch(
      /<button class="button primary svelte-[\w-]+ resting" type="button" disabled="" aria-disabled="true">(<!--[^>]*-->)* Add machine/,
    )
  })

  it('is an enabled primary button otherwise', () => {
    const body = button({})
    expect(body).toMatch(/<button class="button primary svelte-[\w-]+" type="button">(<!--[^>]*-->)* Add machine/)
    expect(body).not.toMatch(/disabled|aria-busy|spinner/)
  })

  it('takes the plain tone and the submit type', () => {
    expect(button({ tone: 'plain' }, 'Cancel')).toMatch(/<button class="button plain svelte-[\w-]+" type="button">/)
    expect(button({ type: 'submit' }, 'Add')).toMatch(/<button class="button primary svelte-[\w-]+" type="submit">/)
  })

  it('takes the danger tone', () => {
    expect(button({ tone: 'danger' }, 'Remove')).toMatch(/<button class="button danger svelte-[\w-]+" type="button">/)
  })

  it('shows the spinner before its label and stops answering while busy', () => {
    const body = button({ busy: true, type: 'submit' }, 'Adding…')
    expect(body).toMatch(
      /<button class="button primary svelte-[\w-]+ busy" type="submit" disabled="" aria-busy="true">(<!--[^>]*-->)*<span class="spinner [^>]*>.*<\/span>(<!--[^>]*-->)* Adding…/,
    )
    expect(body).not.toMatch(/aria-disabled|resting/)
    expect(buttonSource).toMatch(/\.busy \{\s*pointer-events: none;\s*\}/)
  })

  it.each([
    'gap: 6px;',
    'border-radius: var(--radius-control);',
    '.primary { border: 1px solid rgb(43 123 255 / 0.7); background: linear-gradient(180deg, var(--brand-light), var(--brand)); box-shadow: 0 0 10px rgb(0 95 255 / 0.28); color: var(--on-brand); font-weight: 500; }',
    '.plain { border: 1px solid var(--border); background: var(--hover); color: var(--soft); }',
    '.danger { border: 1px solid color-mix(in srgb, var(--failed) 35%, transparent); background: color-mix(in srgb, var(--failed) 10%, transparent); color: var(--failed); }',
  ])('holds %s', (text) => {
    expect(flat(buttonSource)).toContain(text)
  })

  it('is big enough to touch and shows a press only when it can be pressed', () => {
    expect(buttonSource).toContain('min-height: 44px;')
    expect(flat(buttonSource)).toContain('.primary:not(.resting):active { background: var(--brand-hover); }')
    expect(flat(buttonSource)).toContain(
      '.plain:not(.resting):active { border-color: var(--edge); background: var(--selected); }',
    )
    expect(flat(buttonSource)).toContain(
      '.danger:not(.resting):active { border-color: color-mix(in srgb, var(--failed) 70%, transparent); background: color-mix(in srgb, var(--failed) 18%, transparent); }',
    )
    expect(buttonSource).toMatch(/\.resting \{\s*opacity: 0\.4;\s*pointer-events: none;\s*\}/)
  })

  it('passes its click on and listens to touches so that WebKit on iOS shows the press', () => {
    expect(buttonSource).toContain('{onclick}')
    expect(buttonSource).toContain('ontouchstart={() => {}}')
  })
})

describe('Field', () => {
  type Props = { label: string; value?: string; error?: string | null; url?: boolean; disabled?: boolean }
  const field = (props: Props) => render(Field, { props }).body
  const input = (body: string) => body.match(/<input [^>]*>/)?.[0] ?? ''

  it('labels its input and keeps an empty slot for the error', () => {
    const body = field({ label: 'Name', value: 'studio' })
    const id = body.match(/<label class="label svelte-[\w-]+" for="([\w-]+)">Name<\/label>/)?.[1]
    expect(input(body)).toContain(`id="${id}"`)
    expect(input(body)).toContain('value="studio"')
    expect(input(body)).not.toMatch(/aria-invalid|aria-describedby|inputmode|disabled/)
    expect(body).toMatch(/<p class="error svelte-[\w-]+" id="[\w-]+"><\/p>/)
  })

  it('points its input at the error it shows', () => {
    const body = field({ label: 'Bridge URL', error: 'Start it with ws://' })
    const id = body.match(/<p class="error svelte-[\w-]+" id="([\w-]+)">Start it with ws:\/\/<\/p>/)?.[1]
    expect(input(body)).toContain(`aria-invalid="true" aria-describedby="${id}"`)
  })

  it('passes input attributes on', () => {
    expect(input(render(Field, { props: { label: 'Name', placeholder: 'studio', disabled: true } }).body)).toMatch(
      /placeholder="studio" disabled=""/,
    )
  })

  it('stops iOS from correcting a URL', () => {
    expect(input(field({ label: 'Bridge URL', url: true }))).toContain(
      'autocapitalize="off" autocorrect="off" spellcheck="false" inputmode="url"',
    )
  })

  it.each([
    'min-height: 44px;',
    'border: 1px solid var(--edge);',
    'border-radius: var(--radius-control);',
    'background: var(--field);',
    'font-size: 16px;',
    '-webkit-user-select: text; user-select: text;',
    '.input:focus { border-color: var(--accent); box-shadow: 0 0 0 3px color-mix(in srgb, var(--brand) 32%, transparent); }',
    'color: var(--failed);',
  ])('holds %s', (text) => {
    expect(flat(fieldSource)).toContain(text)
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

describe('Spinner', () => {
  it('draws the loader circle at 12 px, hidden from readers', () => {
    const body = render(Spinner).body
    expect(body).toMatch(/<span class="spinner svelte-[\w-]+" aria-hidden="true"><svg [^>]*width="12" height="12"/)
    expect(body).toContain('<path d="M21 12a9 9 0 1 1-6.219-8.56"></path>')
  })

  it.each([
    'animation: turn 900ms linear infinite;',
    '@keyframes turn { to { transform: rotate(360deg); } }',
    '@media (prefers-reduced-motion: reduce) { .spinner { animation: none; } }',
  ])('holds %s', (text) => {
    expect(flat(spinnerSource)).toContain(text)
  })
})

describe('Icon', () => {
  it.each([
    ['scan-qr-code', 'M17 12v4a1 1 0 0 1-1 1h-4', '<rect x="7" y="7" width="5" height="5" rx="1"></rect>'],
    ['chevron-left', 'm15 18-6-6 6-6', ''],
    ['chevron-up', 'm18 15-6-6-6 6', ''],
    ['chevron-down', 'm6 9 6 6 6-6', ''],
    ['trash-2', 'M3 6h18', '<line x1="14" y1="11" x2="14" y2="17"></line>'],
  ] as const)('draws %s', (name, path, other) => {
    const body = render(Icon, { props: { name } }).body
    expect(body).toContain(`<path d="${path}"></path>`)
    expect(body).toContain(other)
  })
})

describe('scanning', () => {
  it('lets the camera show through the page and drops the top glow', () => {
    expect(flat(baseSource)).toContain('html.scanning, html.scanning body { background: transparent; }')
    expect(flat(baseSource)).toContain('html.scanning body::before { display: none; }')
  })

  it('hides the app while scanning, so only the layer over the camera shows', () => {
    expect(flat(baseSource)).toContain('html.scanning #app { visibility: hidden; }')
  })
})
