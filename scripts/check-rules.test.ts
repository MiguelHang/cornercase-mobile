import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { dirname, join } from 'node:path'
import { afterEach, describe, expect, it } from 'vitest'
import { check, lineCount } from './check-rules.mjs'

const roots: string[] = []

function tree(files: Record<string, string>) {
  const root = mkdtempSync(join(tmpdir(), 'check-rules-'))
  roots.push(root)
  for (const [path, text] of Object.entries(files)) {
    mkdirSync(dirname(join(root, path)), { recursive: true })
    writeFileSync(join(root, path), text)
  }
  return root
}

function lines(count: number) {
  return 'x\n'.repeat(count)
}

afterEach(() => {
  for (const root of roots.splice(0)) rmSync(root, { recursive: true, force: true })
})

describe('lineCount', () => {
  it.each([
    ['', 0],
    ['a', 1],
    ['a\n', 1],
    ['a\nb', 2],
    ['\n', 1],
    ['a\n\nb\n', 3],
  ])('counts %j as %i lines, like Rust lines()', (text, count) => {
    expect(lineCount(text)).toBe(count)
  })
})

describe('file size', () => {
  it('passes a file of 1,000 lines with a final newline', () => {
    expect(check(tree({ 'src/a.ts': lines(1000) }))).toEqual([])
  })

  it('passes a file of 1,000 lines without a final newline', () => {
    expect(check(tree({ 'src/a.ts': lines(999) + 'x' }))).toEqual([])
  })

  it('fails a file of 1,001 lines at its first line over', () => {
    const failures = check(tree({ 'src/a.ts': lines(1001) }))
    expect(failures).toHaveLength(1)
    expect(failures[0]).toMatch(/^src\/a\.ts:1001: 1001 lines, over 1000\./)
  })
})

describe('what is walked', () => {
  it.each([
    'crates/core/src/lib.rs',
    'bridge/src/main.rs',
    'src-tauri/src/lib.rs',
    'src-tauri/build.rs',
    'src/App.svelte',
    'src/lib/store.svelte.ts',
    'scripts/tool.mjs',
    'scripts/tool.js',
    'vite.config.ts',
    'eslint.config.js',
    'other.config.mjs',
  ])('checks %s', (path) => {
    expect(check(tree({ [path]: lines(1001) }))).toHaveLength(1)
  })

  it.each([
    'src-tauri/gen/apple/big.rs',
    'crates/core/src/gen/big.rs',
    'crates/core/target/debug/big.rs',
    'src/node_modules/pkg/index.js',
    'src/dist/index.js',
    'src/lib/bindings.ts',
    'crates/core/fixtures/status.json',
    'src-tauri/other.rs',
    'docs/tool.ts',
    'src/lib/notes.md',
    'nested/vite.config.ts',
  ])('ignores %s', (path) => {
    expect(check(tree({ [path]: lines(1001) }))).toEqual([])
  })
})

describe('exceptions', () => {
  it('lets a listed long file pass', () => {
    expect(check(tree({ 'src/a.ts': lines(1001) }), { long: ['src/a.ts'] })).toEqual([])
  })

  it.each([
    ['long', 'LONG_FILES'],
    ['commented', 'COMMENTED_FILES'],
  ])('fails a %s entry whose file is gone', (option, list) => {
    const failures = check(tree({ 'src/a.ts': 'x\n' }), { [option]: ['src/gone.ts'] })
    expect(failures).toEqual([expect.stringMatching(new RegExp(`^src/gone\\.ts:1: is gone: remove it from ${list}`))])
  })

  it.each([
    ['long', 'LONG_FILES'],
    ['commented', 'COMMENTED_FILES'],
  ])('fails a %s entry whose file follows the rule', (option, list) => {
    const failures = check(tree({ 'src/a.ts': 'x\n' }), { [option]: ['src/a.ts'] })
    expect(failures).toEqual([
      expect.stringMatching(new RegExp(`^src/a\\.ts:1: follows the rule now: remove it from ${list}`)),
    ])
  })
})
