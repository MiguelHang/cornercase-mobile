import { render } from 'svelte/server'
import { describe, expect, it } from 'vitest'
import StatusDot from './StatusDot.svelte'
import dotSource from './StatusDot.svelte?raw'

const draw = (status?: string | null, size?: number) => render(StatusDot, { props: { status, size } }).body

const shape = (html: string) => {
  if (html.includes('<mask')) return 'disc'
  if (html.includes('class="orbit')) return 'orbit'
  if (html.includes('r="4.5"')) return 'pending'
  return 'ring'
}

const flat = (text: string) => text.replace(/\s+/g, ' ')

describe('StatusDot', () => {
  it.each([
    ['idle', 'ring', 'var(--faint)', 'Idle'],
    ['working', 'orbit', 'var(--working)', 'Working'],
    ['shell', 'orbit', 'var(--working)', 'Shell'],
    ['waiting', 'disc', 'var(--waiting)', 'Waiting'],
    ['failed', 'disc', 'var(--failed)', 'Failed'],
    ['done', 'disc', 'var(--done)', 'Done'],
    ['pending', 'pending', 'var(--brand)', 'Pending'],
  ])('draws %s as a %s in %s, named %s', (status, form, colour, name) => {
    const html = draw(status)
    expect(shape(html)).toBe(form)
    expect(html).toContain(`style="color: ${colour};"`)
    expect(html).toMatch(new RegExp(`<svg class="dot svelte-[\\w-]+" role="img" aria-label="${name}"`))
  })

  it.each([null, undefined, 'compacting'])('draws %s as idle', (status) => {
    const html = draw(status)
    expect(shape(html)).toBe('ring')
    expect(html).toContain('aria-label="Idle"')
  })

  it.each([
    ['waiting', '<path d="M12 7v5.6"'],
    ['failed', '<path d="M9 9l6 6M15 9l-6 6"'],
    ['done', '<path d="M8 12.4l2.8 2.8L16.2 9.6"'],
  ])('cuts the mark of %s out of its disc', (status, mark) => {
    const html = draw(status)
    expect(html).toContain(mark)
    const id = html.match(/<mask id="([\w-]+)">/)?.[1]
    expect(id).toMatch(/^cut-/)
    expect(html).toContain(`<circle cx="12" cy="12" r="9.5" mask="url(#${id})"></circle>`)
  })

  it.each(['idle', 'waiting', 'failed', 'done', 'pending', null])('mounts no orbit for %s', (status) => {
    expect(draw(status)).not.toContain('orbit')
  })

  it('orbits a fading tail behind the dot of a faint ring', () => {
    const html = draw('working')
    expect(html).toContain('stroke-width="2" stroke-opacity="0.3"')
    expect(html.match(/fill-opacity="[\d.]+"/g)).toEqual(['fill-opacity="0.25"', 'fill-opacity="0.45"', 'fill-opacity="0.7"'])
  })

  it('draws at the size asked for, fourteen pixels otherwise', () => {
    expect(draw('idle', 20)).toContain('width="20" height="20" viewBox="0 0 24 24"')
    expect(draw('idle')).toContain('width="14" height="14" viewBox="0 0 24 24"')
  })

  it('turns the orbit once every 1.2 s around the centre', () => {
    expect(flat(dotSource)).toContain('.orbit { transform-origin: 12px 12px; animation: orbit 1200ms linear infinite; }')
    expect(flat(dotSource)).toContain('@keyframes orbit { to { transform: rotate(360deg); } }')
  })

  it('stops the orbit with reduced motion', () => {
    expect(flat(dotSource)).toContain('@media (prefers-reduced-motion: reduce) { .orbit { animation: none; } }')
  })
})
