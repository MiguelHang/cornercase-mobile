import { describe, expect, it } from 'vitest'
import tokens from './tokens.css?raw'

const sources = import.meta.glob<string>(['/src/**/*.{svelte,css,ts}', '!/src/**/*.test.ts'], {
  query: '?raw',
  import: 'default',
  eager: true,
})

const names = (text: string, pattern: RegExp) => new Set([...text.matchAll(pattern)].map((match) => match[1]))

const defined = (text: string) =>
  new Set([...names(text, /(--[\w-]+)\s*:/g), ...names(text, /style:(--[\w-]+)/g)])

const undefinedVars = (text: string) => {
  const own = defined(text)
  const shared = defined(tokens)
  return [...names(text, /var\(\s*(--[\w-]+)/g)].filter((name) => !own.has(name) && !shared.has(name))
}

describe('tokens', () => {
  it('reads the text of the stylesheets', () => {
    expect(tokens).toContain('--brand: #005fff;')
    expect(sources['/src/lib/theme/base.css']).toContain('var(--glass)')
  })

  it.each(Object.entries(sources))('%s uses only defined custom properties', (_, text) => {
    expect(undefinedVars(text)).toEqual([])
  })

  it.each([
    ['a misspelt token', 'a { color: var(--brnad) }', ['--brnad']],
    ['a local property used before its definition', 'a { width: var(--unit); --unit: 4px }', []],
    ['a property set with style:', '<div style:--unit={unit}></div><style>div { width: var(--unit) }</style>', []],
    ['a shared token', 'a { color: var(--brand) }', []],
  ])('finds %s', (_, text, missing) => {
    expect(undefinedVars(text)).toEqual(missing)
  })

  it('holds no Tokyo Night colour', () => {
    expect(tokens.toLowerCase()).not.toContain('#1a1b26')
  })
})
