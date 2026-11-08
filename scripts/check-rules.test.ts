import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { dirname, join } from 'node:path'
import { afterEach, describe, expect, it } from 'vitest'
import { check, lineCount, rustComments } from './check-rules.mjs'

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

describe('Rust comments', () => {
  it.each([
    ['a line comment', 'fn a() {}\n// why\n', 2, 'line comment'],
    ['a doc comment', '/// what\nfn a() {}\n', 1, 'line comment'],
    ['an inner doc comment', '//! crate\n', 1, 'line comment'],
    ['a comment after code', 'fn a() {} // why\n', 1, 'line comment'],
    ['a block comment at the line it starts', 'fn a() {}\n/* one\ntwo */\nfn b() {}\n', 2, 'block comment'],
    ['an unclosed block comment', 'fn a() {}\n/* one\n', 2, 'block comment'],
    ['a comment after a lifetime', "impl<'a> Trait for Type {} // why\n", 1, 'line comment'],
    ['a comment after a static lifetime', "const NAME: &'static str = NAME; // why\n", 1, 'line comment'],
    ['a comment after an identifier that starts like a literal', 'let br = bar; // why\n', 1, 'line comment'],
  ])('finds %s', (_, text, line, reason) => {
    expect(rustComments(text)).toEqual([{ line, reason }])
  })

  it('finds a nested block comment once', () => {
    expect(rustComments('/* outer /* inner */ " */\nlet url = "https://x";\n')).toEqual([
      { line: 1, reason: 'block comment' },
    ])
  })

  it('finds every comment, each at its line', () => {
    expect(rustComments('// one\nfn a() {}\n// two\n')).toEqual([
      { line: 1, reason: 'line comment' },
      { line: 3, reason: 'line comment' },
    ])
  })

  it.each([
    ['a string', 'let url = "https://x";\n'],
    ['a string with an escaped quote', 'let text = "say \\" // no";\n'],
    ['a string with an escaped backslash', 'let path = "C:\\\\"; let url = "https://x";\n'],
    ['a multiline string', 'let text = "one\n// two";\n'],
    ['a raw string ending in a backslash', 'let path = r"C:\\"; let url = "https://x";\n'],
    ['a raw string with a quote', 'let text = r#"say " // no"#;\n'],
    ['a raw string with a quote and a hash', 'let text = r##"say "# // no"##;\n'],
    ['a byte string', 'let bytes = b"https://x";\n'],
    ['a raw byte string', 'let bytes = br#"say " // no"#;\n'],
    ['a C string', 'let name = c"https://x";\n'],
    ['a slash char', "let slashes = ['/', '/']; let half = 4 / 2;\n"],
    ['a byte slash char', "let slash = b'/'; let half = 4 / 2;\n"],
    ['a quote char', 'let quote = \'"\'; let url = "https://x";\n'],
    ['an escaped quote char', "let quote = '\\''; let url = \"https://x\";\n"],
    ['an escaped double quote char', "let quote = '\\\"'; let url = \"https://x\";\n"],
    ['an escaped char', "let tab = '\\t'; let byte = '\\x2f'; let code = '\\u{2f}';\n"],
    ['a lifetime next to a char', "fn a<'a>(text: &'a str) -> char { '\"' }\nlet url = \"https://x\";\n"],
    ['a division', 'let half = 4 / 2;\n'],
    ['an unclosed string to the end', 'let text = "open // no\n'],
    ['an unclosed string ending in a backslash', 'let text = "open // no\\'],
    ['an unclosed raw string to the end', 'let text = r#"open " // no\n'],
  ])('skips %s', (_, text) => {
    expect(rustComments(text)).toEqual([])
  })

  it('fails a Rust file with a comment at its line', () => {
    const failures = check(tree({ 'crates/core/src/lib.rs': 'fn a() {}\n// why\n' }))
    expect(failures).toEqual([expect.stringMatching(/^crates\/core\/src\/lib\.rs:2: line comment\. /)])
  })

  it('lets a listed commented Rust file pass', () => {
    const files = { 'crates/core/src/lib.rs': '// why\n' }
    expect(check(tree(files), { commented: ['crates/core/src/lib.rs'] })).toEqual([])
  })
})
