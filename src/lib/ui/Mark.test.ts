import { render } from 'svelte/server'
import { describe, expect, it } from 'vitest'
import Icon from './Icon.svelte'
import Mark from './Mark.svelte'
import Wordmark from './Wordmark.svelte'
import markSource from './Mark.svelte?raw'

const cells = (html: string) => [...html.matchAll(/<span class="cell ([\w ]+?) svelte-[\w-]+"><\/span>/g)].map((m) => m[1])

describe('Mark', () => {
  it('draws a box of twelve units with its three cells', () => {
    const html = render(Mark, { props: { unit: 4 } }).body
    expect(html).toContain('--unit: 4px; width: 48px; height: 48px;')
    expect(cells(html)).toEqual(['corner top', 'corner side', 'cursor'])
  })

  it('scales the box with the unit', () => {
    expect(render(Mark, { props: { unit: 5 } }).body).toContain('--unit: 5px; width: 60px; height: 60px;')
  })

  it('places the cells on the twelve-unit grid', () => {
    const rule = (name: string) => markSource.match(new RegExp(`\\.${name} \\{([^}]*)\\}`))?.[1].replace(/\s+/g, ' ')
    expect(rule('top')).toBe(
      ' left: calc(3 * var(--unit)); top: 0; width: calc(9 * var(--unit)); height: calc(3 * var(--unit)); ',
    )
    expect(rule('side')).toBe(
      ' left: 0; top: calc(3 * var(--unit)); width: calc(3 * var(--unit)); height: calc(9 * var(--unit)); ',
    )
    expect(rule('cursor')).toContain('left: calc(6 * var(--unit)); top: calc(6 * var(--unit));')
    expect(rule('cursor')).toContain('width: calc(2 * var(--unit)); height: calc(5 * var(--unit));')
    expect(rule('corner')).toContain('box-shadow: 0 0 calc(3 * var(--unit)) rgb(0 95 255 / 0.55);')
  })

  it('stays still unless it pulses', () => {
    expect(render(Mark, { props: {} }).body).not.toContain('pulsing')
  })

  it('pulses only its cursor', () => {
    const html = render(Mark, { props: { pulsing: true } }).body
    expect(html).toMatch(/<span class="cell cursor svelte-[\w-]+ pulsing"><\/span>/)
    expect(html.match(/pulsing/g)).toHaveLength(1)
  })

  it('leaves the pulse to the shared one in the base styles', () => {
    expect(markSource).not.toMatch(/animation|@keyframes/)
  })
})

describe('Wordmark', () => {
  it('writes the name at the size asked for', () => {
    expect(render(Wordmark, { props: { size: 20 } }).body).toMatch(/style="font-size: 20px;">cornercase<\/span>/)
  })
})

describe('Icon', () => {
  const html = render(Icon, { props: { name: 'server', size: 28 } }).body

  it('draws at the size asked for, in the current colour', () => {
    expect(html).toContain('width="28" height="28" viewBox="0 0 24 24" fill="none" stroke="currentColor"')
    expect(html).toContain('stroke-width="1.75"')
  })

  it('draws the rects and lines of the server icon', () => {
    expect(html).toContain('<rect x="2" y="2" width="20" height="8" rx="2"></rect>')
    expect(html).toContain('<rect x="2" y="14" width="20" height="8" rx="2"></rect>')
    expect(html).toContain('<line x1="6" y1="6" x2="6.01" y2="6"></line>')
    expect(html).toContain('<line x1="6" y1="18" x2="6.01" y2="18"></line>')
  })

  it('draws the path of the loader circle', () => {
    expect(render(Icon, { props: { name: 'loader-circle' } }).body).toContain(
      '<path d="M21 12a9 9 0 1 1-6.219-8.56"></path>',
    )
  })
})
