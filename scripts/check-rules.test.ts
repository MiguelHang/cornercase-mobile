import { spawnSync } from 'node:child_process'
import { copyFileSync, mkdirSync, mkdtempSync, readFileSync, realpathSync, rmSync, symlinkSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { dirname, join } from 'node:path'
import { afterEach, describe, expect, it } from 'vitest'
import { check, lineCount, rustComments, svelteComments, tsComments } from './check-rules.mjs'

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
    ['a unicode char before a quote char', "let a = ['\\u{41}','\"']; let url = \"https://x\";\n"],
    ['a hex char before a quote char', "let a = ['\\x41','\"']; let url = \"https://x\";\n"],
    ['an astral char before a quote char', "let a = ['😀','\"']; let url = \"https://x\";\n"],
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

describe('TypeScript and JavaScript comments', () => {
  it.each([
    ['a line comment', 'const a = 1\n// why\n', 2, 'line comment'],
    ['a doc comment', '/** what */\nexport const a = 1\n', 1, 'block comment'],
    ['a block comment at the line it starts', 'const a = 1\n/* one\ntwo */\n', 2, 'block comment'],
    ['an unclosed block comment', 'const a = 1\n/* one\n', 2, 'block comment'],
    ['a comment after a division', 'const half = a / b // why\n', 1, 'line comment'],
    ['a comment after a division of a call', 'const half = size() / 2 // why\n', 1, 'line comment'],
    ['a comment after a division of an index', 'const half = sizes[0] / 2 // why\n', 1, 'line comment'],
    ['a comment after a regex', 'const slash = /\\//g // why\n', 1, 'line comment'],
    ['a comment after a string', "const url = 'https://x' // why\n", 1, 'line comment'],
    ['a comment after a template', 'const url = `https://${host}` // why\n', 1, 'line comment'],
    ['a comment after an object in a template', 'const a = `${ {a:1}.a }` // why\n', 1, 'line comment'],
    ['a comment after a shebang', '#!/usr/bin/env node\n// why\n', 2, 'line comment'],
    ['a comment after a private field', 'const half = this.#size / 2 // why\n', 1, 'line comment'],
    ['a comment after a non-ASCII name', 'const half = café / 2 // why\n', 1, 'line comment'],
    ['a comment after a regex after a block', "if (a) { b }\n/'/.test(s) // why\n", 2, 'line comment'],
    ['a comment after a regex at the start of a block', "if (a) { /'/.test(s) } // why\n", 1, 'line comment'],
    ['a comment after a regex after return', "function a() {\n  return /'/.test(s) // why\n}\n", 2, 'line comment'],
    ['a comment after a regex after typeof', "const a = typeof /'/ // why\n", 1, 'line comment'],
    ['a comment after a regex after a division', "const a = b / /'/.source.length // why\n", 1, 'line comment'],
    ['a comment after a regex with a slash in a class', "const a = /[/]'/.test(s) // why\n", 1, 'line comment'],
    ['a comment on the line after an unclosed regex', 'const a = /open\n// why\n', 2, 'line comment'],
    ['a comment after a division of a string', "const half = '4' / 2 // why\n", 1, 'line comment'],
    ['a comment after a division of a template', 'const half = `4` / 2 // why\n', 1, 'line comment'],
    ['a comment after a string in a template', "const a = `${'`'}` // why\n", 1, 'line comment'],
    ['a comment after a template regex', 'function a() { return `${a}${/`/.source}` } // why\n', 1, 'line comment'],
    ['a comment after a postfix increment and a division', 'const a = i++ / 2 // why\n', 1, 'line comment'],
    ['a comment after a postfix decrement and a division', 'const a = n-- / 2 // why\n', 1, 'line comment'],
    ['a comment after a hash bang past the start', 'const a = 1\n#! // why\n', 2, 'line comment'],
  ])('finds %s', (_, text, line, reason) => {
    expect(tsComments(text)).toEqual([{ line, reason }])
  })

  it('finds every comment, each at its line', () => {
    expect(tsComments('// one\nconst a = 1\n/* two */\n')).toEqual([
      { line: 1, reason: 'line comment' },
      { line: 3, reason: 'block comment' },
    ])
  })

  it.each([
    ['a single-quoted string', "const url = 'https://x'\n"],
    ['a double-quoted string', 'const url = "https://x"\n'],
    ['a string with an escaped quote', "const text = 'say \\' // no'\n"],
    ['a string with the other quote', 'const text = "it\'s // no"\n'],
    ['a template', 'const url = `https://x`\n'],
    ['a template with an escaped backtick', 'const text = `say \\` // no`\n'],
    ['a template with a brace in its text', 'const text = `{ // no }`\n'],
    ['a nested template', 'const text = `${ `${a}` } // no`\n'],
    ['an object inside a template', 'const text = `${ {a: 1}.a } // no`\n'],
    ['a regex with slashes', 'const doubled = /\\/\\//.test(s)\n'],
    ['a regex with a block opener', 'const opener = /\\/*/\n'],
    ['a regex with a slash in a class', 'const slash = /[/]/\n'],
    ['a regex with an escaped bracket in a class', 'const slash = /[\\]/]/\n'],
    ['a regex after return', 'function a() {\n  return /a/.test(s) || /b/.test(s)\n}\n'],
    ['a regex after typeof', 'const a = typeof /a/\n'],
    ['a regex at the start', '/a/.test(s)\n'],
    ['a regex in a call', "text.replace(/'/g, '//')\n"],
    ['a regex after an arrow', 'const test = () => /\\/\\//\n'],
    ['a regex in a template', 'const text = `${/`/.source}`\n'],
    ['a division', 'const half = a / b / c\n'],
    ['a shebang', '#!/usr/bin/env node\nconst a = 1\n'],
    ['a shebang with a double slash', '#!//usr/bin/node\nconst a = 1\n'],
    ['an unclosed string to the end', "const text = 'open // no\n"],
    ['an unclosed template to the end', 'const text = `open // no\n'],
    ['an unclosed regex at the end', 'const text = /open'],
  ])('skips %s', (_, text) => {
    expect(tsComments(text)).toEqual([])
  })

  it.each([
    ['src/main.ts', 'const a = 1\n/* why */\n'],
    ['scripts/build.mjs', 'const a = 1\n/* why */\n'],
    ['src/lib/a.js', 'const a = 1\n/* why */\n'],
  ])('fails %s with a comment at its line', (path, text) => {
    expect(check(tree({ [path]: text }))).toEqual([expect.stringMatching(new RegExp(`^${path}:2: block comment\\. `))])
  })

  it('lets a listed commented TypeScript file pass', () => {
    expect(check(tree({ 'src/main.ts': '// why\n' }), { commented: ['src/main.ts'] })).toEqual([])
  })
})

describe('Svelte comments', () => {
  it.each([
    ['a markup comment', '<main>\n  <!-- why -->\n</main>\n', 2, 'markup comment'],
    ['a svelte-ignore directive', '<!-- svelte-ignore a11y_autofocus -->\n<input />\n', 1, 'markup comment'],
    ['a markup comment at the line it starts', '<p>a</p>\n<!-- one\ntwo -->\n', 2, 'markup comment'],
    ['an unclosed markup comment', '<p>a</p>\n<!-- open\n', 2, 'markup comment'],
    ['a line comment in a script', '<script>\n  let a = 1\n  // why\n</script>\n', 3, 'line comment'],
    ['a block comment in a script', '<script>\n  /** why */\n  let a = 1\n</script>\n', 2, 'block comment'],
    ['a comment in a script below the markup', '<main></main>\n\n<script>\n  // why\n</script>\n', 4, 'line comment'],
    ['a comment on the line of its script tag', '<script>// why\n</script>\n', 1, 'line comment'],
    ['a comment in an unclosed script', '<script>\n  // why\n', 2, 'line comment'],
    ['a comment in a generic script', '<script generics="T extends A<B>">\n  // why\n</script>\n', 2, 'line comment'],
    ['a comment in a single-quoted generic script', "<script generics='T extends A<B>'>\n// why\n", 2, 'line comment'],
    ['a block comment in a style', '<style>\n  p { color: red; }\n  /* why */\n</style>\n', 3, 'block comment'],
    ['a markup comment after a script', "<script>\n  let a = '<!--'\n</script>\n<!-- why -->\n", 4, 'markup comment'],
  ])('finds %s', (_, text, line, reason) => {
    expect(svelteComments(text)).toEqual([{ line, reason }])
  })

  it('finds every comment, each at its line', () => {
    const text =
      '<script module>\n  // one\n</script>\n<script>\n  /* two */\n</script>\n' +
      '<!-- three -->\n<style>\n  /* four */\n</style>\n'
    expect(svelteComments(text)).toEqual([
      { line: 2, reason: 'line comment' },
      { line: 5, reason: 'block comment' },
      { line: 7, reason: 'markup comment' },
      { line: 9, reason: 'block comment' },
    ])
  })

  it.each([
    ['a URL in the markup text', '<p>See https://usecornercase.dev</p>\n'],
    ['a URL in an attribute', '<a href="https://usecornercase.dev">site</a>\n'],
    ['a URL in a style', '<style>\n  main { background: url(https://x.dev/a.png); }\n</style>\n'],
    ['a single-quoted string in a style', "<style>\n  p::after { content: '/* no */'; }\n</style>\n"],
    ['a double-quoted string in a style', '<style>\n  p::after { content: "/* no */"; }\n</style>\n'],
    ['a markup comment opener in a script string', "<script>\n  const a = '<!-- no -->'\n</script>\n"],
    ['an element named like a script', '<script-card>See https://usecornercase.dev</script-card>\n'],
    ['the real src/App.svelte', readFileSync(new URL('../src/App.svelte', import.meta.url), 'utf8')],
  ])('skips %s', (_, text) => {
    expect(svelteComments(text)).toEqual([])
  })

  it.each([
    ['src/App.svelte', '<main>\n  <!-- why -->\n</main>\n', 'markup comment'],
    ['src/lib/Card.svelte', '<script>\n  // why\n</script>\n', 'line comment'],
  ])('fails %s with a comment at its line', (path, text, reason) => {
    expect(check(tree({ [path]: text }))).toEqual([expect.stringMatching(new RegExp(`^${path}:2: ${reason}\\. `))])
  })
})

describe('the command', () => {
  function copied(files: Record<string, string>) {
    const root = realpathSync(tree(files))
    mkdirSync(join(root, 'scripts'), { recursive: true })
    copyFileSync(new URL('./check-rules.mjs', import.meta.url), join(root, 'scripts/check-rules.mjs'))
    return root
  }

  function run(files: Record<string, string>) {
    const root = copied(files)
    return spawnSync(process.execPath, [join(root, 'scripts/check-rules.mjs')], { cwd: root, encoding: 'utf8' })
  }

  it('exits 0 and prints nothing on a clean tree', () => {
    const result = run({ 'src/a.ts': 'const a = 1\n' })
    expect([result.status, result.stderr]).toEqual([0, ''])
  })

  it('exits 1 with path:line, the reason and the guide on a comment', () => {
    const result = run({ 'src/a.ts': 'const a = 1\n// why\n' })
    expect(result.status).toBe(1)
    expect(result.stderr).toMatch(/^src\/a\.ts:2: line comment\. /m)
    expect(result.stderr).toContain('design/code-layout.md')
  })

  it('exits 1 on a file over 1,000 lines', () => {
    const result = run({ 'src/a.ts': lines(1001) })
    expect(result.status).toBe(1)
    expect(result.stderr).toMatch(/^src\/a\.ts:1001: /m)
  })

  it('exits 1 when started through a symlink to the checkout', () => {
    const root = copied({ 'src/a.ts': 'const a = 1\n// why\n' })
    const link = join(realpathSync(tree({})), 'link')
    symlinkSync(root, link)
    const result = spawnSync(process.execPath, [join(link, 'scripts/check-rules.mjs')], { cwd: link, encoding: 'utf8' })
    expect(result.status).toBe(1)
    expect(result.stderr).toMatch(/^src\/a\.ts:2: line comment\. /m)
  })
})
