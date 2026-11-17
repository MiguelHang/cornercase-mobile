import { spawnSync } from 'node:child_process'
import { existsSync, mkdirSync, mkdtempSync, readdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { dirname, join } from 'node:path'
import { fromMarkdown } from 'mdast-util-from-markdown'
import { afterEach, describe, expect, it } from 'vitest'
import { build, escape, fill, plainText, render, resolveLink } from './site.mjs'

const repository = 'owner/name'
const root = join(import.meta.dirname, '..')
const folders: string[] = []

afterEach(() => {
  for (const folder of folders.splice(0)) rmSync(folder, { recursive: true, force: true })
})

function temporary() {
  const folder = mkdtempSync(join(tmpdir(), 'site-'))
  folders.push(folder)
  return folder
}

function write(folder: string, files: Record<string, string>) {
  for (const [path, text] of Object.entries(files)) {
    mkdirSync(dirname(join(folder, path)), { recursive: true })
    writeFileSync(join(folder, path), text)
  }
  return folder
}

function html(markdown: string, source = 'SUPPORT.md') {
  return render(markdown, { source, repository })
}

describe('escape', () => {
  it('escapes &, <, > and "', () => {
    expect(escape('a & b < c > d "e"')).toBe('a &amp; b &lt; c &gt; d &quot;e&quot;')
  })

  it('escapes & before the others, so nothing is escaped twice', () => {
    expect(escape('&lt;')).toBe('&amp;lt;')
  })
})

describe('resolveLink', () => {
  it.each([
    ['../../issues', 'SUPPORT.md', 'https://github.com/owner/name/issues'],
    ['../../issues/new', 'SUPPORT.md', 'https://github.com/owner/name/issues/new'],
    ['../../issues', 'PRIVACY.md', 'https://github.com/owner/name/issues'],
    ['CHANGELOG.md', 'SUPPORT.md', 'https://github.com/owner/name/blob/main/CHANGELOG.md'],
    ['../../../releases/latest', 'site/index.md', 'https://github.com/owner/name/releases/latest'],
  ])('resolves %s from %s against the repository', (url, source, href) => {
    expect(resolveLink(url, { source, repository })).toBe(href)
  })

  it.each([
    ['PRIVACY.md', 'SUPPORT.md', '../privacy/'],
    ['SUPPORT.md', 'PRIVACY.md', '../support/'],
    ['PRIVACY.md#contact', 'SUPPORT.md', '../privacy/#contact'],
    ['./PRIVACY.md', 'SUPPORT.md', '../privacy/'],
    ['SUPPORT.md', 'SUPPORT.md', './'],
    ['../PRIVACY.md', 'site/index.md', 'privacy/'],
    ['../SUPPORT.md', 'site/index.md', 'support/'],
    ['site/index.md', 'PRIVACY.md', '../'],
  ])('sends %s from %s to its page of the site', (url, source, href) => {
    expect(resolveLink(url, { source, repository })).toBe(href)
  })

  it('keeps an https link as it is', () => {
    expect(resolveLink('https://tailscale.com', { source: 'SUPPORT.md', repository })).toBe('https://tailscale.com')
  })

  it.each(['http://example.com', 'javascript:alert(1)', 'mailto:someone@example.com'])(
    'refuses %s, which is not https',
    (url) => {
      expect(() => resolveLink(url, { source: 'SUPPORT.md', repository })).toThrow(/SUPPORT\.md: .*not https/)
    },
  )

  it('refuses a source that is not a page of the site', () => {
    expect(() => resolveLink('PRIVACY.md', { source: 'README.md', repository })).toThrow(/README\.md: not a page/)
  })

  it.each([undefined, '', 'name', 'owner/name/extra', 'owner name/x'])(
    'refuses %j as the repository of a relative link',
    (bad) => {
      expect(() => resolveLink('../../issues', { source: 'SUPPORT.md', repository: bad })).toThrow(
        /not a GitHub repository/,
      )
    },
  )
})

describe('render', () => {
  it.each([
    ['# Title', '<h1>Title</h1>'],
    ['### Small', '<h3>Small</h3>'],
    ['A paragraph.', '<p>A paragraph.</p>'],
    ['Some *soft* words', '<p>Some <em>soft</em> words</p>'],
    ['Some **bold** words', '<p>Some <strong>bold</strong> words</p>'],
    ['Run `a < b`', '<p>Run <code>a &lt; b</code></p>'],
    ['```\nx && y\n```', '<pre><code>x &amp;&amp; y</code></pre>'],
    ['one  \ntwo', '<p>one<br>two</p>'],
    ['---', '<hr>'],
    ['[here](PRIVACY.md)', '<p><a href="../privacy/">here</a></p>'],
    [
      '[**new**](../../issues/new)',
      '<p><a href="https://github.com/owner/name/issues/new"><strong>new</strong></a></p>',
    ],
  ])('renders %j', (markdown, expected) => {
    expect(html(markdown)).toBe(expected)
  })

  it('separates the blocks with a newline', () => {
    expect(html('# A\n\nB')).toBe('<h1>A</h1>\n<p>B</p>')
  })

  it('renders a tight list without paragraphs', () => {
    expect(html('- one\n- **two**')).toBe('<ul>\n<li>one</li>\n<li><strong>two</strong></li>\n</ul>')
  })

  it('keeps the paragraphs of a loose list', () => {
    expect(html('- one\n\n- two')).toBe('<ul>\n<li><p>one</p></li>\n<li><p>two</p></li>\n</ul>')
  })

  it('renders an ordered list, with its start when it is not 1', () => {
    expect(html('1. a\n2. b')).toBe('<ol>\n<li>a</li>\n<li>b</li>\n</ol>')
    expect(html('3. c')).toBe('<ol start="3">\n<li>c</li>\n</ol>')
  })

  it('renders a nested list inside its item', () => {
    expect(html('- a\n  - b')).toBe('<ul>\n<li>a<ul>\n<li>b</li>\n</ul></li>\n</ul>')
  })

  it('escapes <, & and " in text', () => {
    expect(html('a < b & "c"')).toBe('<p>a &lt; b &amp; &quot;c&quot;</p>')
  })

  it('escapes <, & and " in a link', () => {
    expect(html('[x](https://example.com/?a=1&b="<")')).toBe(
      '<p><a href="https://example.com/?a=1&amp;b=&quot;&lt;&quot;">x</a></p>',
    )
  })

  it.each([
    ['<b>raw</b>', 'html'],
    ['Inline <i>raw</i> html', 'html'],
    ['![a picture](a.png)', 'image'],
    ['> a quote', 'blockquote'],
    ['| a | b |\n| - | - |\n| 1 | 2 |', 'table'],
    ['~~gone~~', 'delete'],
    ['[a][b]\n\n[b]: https://example.com', 'linkReference'],
  ])('fails on %j, a node it has no HTML for', (markdown, type) => {
    expect(() => html(markdown)).toThrow(`SUPPORT.md:1: the site has no HTML for the markdown node "${type}"`)
  })

  it('fails on a task list item, instead of losing its box', () => {
    expect(() => html('\n- [x] done', 'PRIVACY.md')).toThrow(
      'PRIVACY.md:2: the site has no HTML for the markdown task list item',
    )
  })
})

describe('plainText', () => {
  it('joins the text of a node and of everything inside it', () => {
    const tree = fromMarkdown('A **bold** [link](x) and `code`.')
    expect(plainText(tree)).toBe('A bold link and code.')
  })
})

describe('fill', () => {
  it('puts each value in its placeholder, every time it appears', () => {
    expect(fill('{{a}}-{{b}}-{{a}}', { a: '1', b: '2' })).toBe('1-2-1')
  })

  it('takes a value as it is, with no special meaning for $', () => {
    expect(fill('<p>{{content}}</p>', { content: "$& $' $1" })).toBe("<p>$& $' $1</p>")
  })

  it('does not fill the placeholders that a value brings in', () => {
    expect(fill('{{a}}{{b}}', { a: '{{b}}', b: 'x' })).toBe('{{b}}x')
  })

  it('fails on a placeholder it has no value for', () => {
    expect(() => fill('{{title}} {{missing}}', { title: 'x' })).toThrow('site/page.html: no value for {{missing}}')
  })
})

const PAGES = [
  ['site/index.md', 'index.html'],
  ['PRIVACY.md', 'privacy/index.html'],
  ['SUPPORT.md', 'support/index.html'],
]

function blocks(node: { type: string; children?: unknown[] }): { type: string; children?: unknown[] }[] {
  const inside = (node.children ?? []).flatMap((child) => blocks(child as typeof node))
  return node.type === 'heading' || node.type === 'paragraph' ? [node, ...inside] : inside
}

function textOf(html: string) {
  return html.replace(/<[^>]*>/g, '')
}

function attributes(html: string, tag: string) {
  return [...html.matchAll(new RegExp(`<${tag}\\s[^>]*href="([^"]*)"`, 'g'))].map((match) => match[1])
}

describe('build with the repository', () => {
  function built() {
    const out = join(temporary(), '_site')
    build({ root, out, repository })
    return { out, read: (path: string) => readFileSync(join(out, path), 'utf8') }
  }

  it('writes the three pages and the two style sheets', () => {
    const { out } = built()
    for (const path of ['index.html', 'privacy/index.html', 'support/index.html', 'tokens.css', 'site.css']) {
      expect(existsSync(join(out, path)), path).toBe(true)
    }
  })

  it('copies the tokens of the app as they are', () => {
    expect(built().read('tokens.css')).toBe(readFileSync(join(root, 'src/lib/theme/tokens.css'), 'utf8'))
  })

  it.each(PAGES)('puts every heading and paragraph of %s in %s', (source, path) => {
    const page = textOf(built().read(path))
    for (const block of blocks(fromMarkdown(readFileSync(join(root, source), 'utf8')))) {
      expect(page).toContain(escape(plainText(block)))
    }
  })

  it.each(PAGES)('gives %s no script and nothing from another origin (%s)', (_, path) => {
    const page = built().read(path)
    expect(page).not.toMatch(/<script/i)
    expect(page).not.toMatch(/\ssrc=/i)
    for (const href of attributes(page, 'link')) expect(href).not.toMatch(/^([a-z]+:|\/\/)/i)
    for (const href of attributes(page, 'a')) {
      if (/^([a-z]+:|\/\/)/i.test(href)) expect(href).toMatch(/^https:\/\/github\.com\/owner\/name\//)
    }
  })

  it.each(PAGES)('gives %s its content security policy and no referrer (%s)', (_, path) => {
    const page = built().read(path)
    expect(page).toContain(`<meta http-equiv="Content-Security-Policy" content="default-src 'none'; style-src 'self'">`)
    expect(page).toContain('<meta name="referrer" content="no-referrer">')
  })

  it.each([
    ['index.html', './', 'Cornercase remote'],
    ['privacy/index.html', '../', 'Privacy policy · Cornercase remote'],
    ['support/index.html', '../', 'Support · Cornercase remote'],
  ])('links %s to the other pages through %s, with the title %s', (path, up, title) => {
    const page = built().read(path)
    expect(attributes(page, 'a')).toEqual(expect.arrayContaining([up, `${up}privacy/`, `${up}support/`]))
    expect(attributes(page, 'link')).toEqual([`${up}tokens.css`, `${up}site.css`])
    expect(page).toContain(`<title>${title}</title>`)
  })

  it('names no owner of the repository in the sources of the site', () => {
    const sources = [
      ...readdirSync(join(root, 'site')).map((name) => `site/${name}`),
      'scripts/site.mjs',
      'SUPPORT.md',
      'PRIVACY.md',
      '.github/workflows/pages.yml',
    ].filter((path) => existsSync(join(root, path)))
    expect(sources).toContain('site/page.html')
    const owner = new RegExp(['miguel', 'hang'].join(''), 'i')
    for (const path of sources) expect(readFileSync(join(root, path), 'utf8'), path).not.toMatch(owner)
  })
})

describe('build with a site of its own', () => {
  const site = {
    'site/page.html': '<title>{{title}}</title>{{root}}|{{content}}',
    'site/index.md': '# Home',
    'site/site.css': 'a {}',
    'src/lib/theme/tokens.css': ':root {}',
    'PRIVACY.md': '# Privacy',
    'SUPPORT.md': '# Support\n\nSee [privacy](PRIVACY.md).',
  }

  function out(files: Record<string, string> = {}) {
    const folder = write(temporary(), { ...site, ...files })
    return { folder, out: write(join(folder, '_site'), { 'stale.html': 'old' }) }
  }

  it('fills the template with the title, the way up and the content of each page', () => {
    const { folder, out: target } = out()
    build({ root: folder, out: target, repository })
    expect(readFileSync(join(target, 'index.html'), 'utf8')).toBe(
      '<title>Home · Cornercase remote</title>./|<h1>Home</h1>',
    )
    expect(readFileSync(join(target, 'support/index.html'), 'utf8')).toBe(
      '<title>Support · Cornercase remote</title>../|<h1>Support</h1>\n<p>See <a href="../privacy/">privacy</a>.</p>',
    )
  })

  it('escapes the title', () => {
    const { folder, out: target } = out({ 'PRIVACY.md': '# A & B' })
    build({ root: folder, out: target, repository })
    expect(readFileSync(join(target, 'privacy/index.html'), 'utf8')).toContain('<title>A &amp; B · Cornercase remote')
  })

  it('removes what an earlier build left', () => {
    const { folder, out: target } = out()
    build({ root: folder, out: target, repository })
    expect(existsSync(join(target, 'stale.html'))).toBe(false)
  })

  it.each([
    ['PRIVACY.md', 'No heading.', 'PRIVACY.md: the page has no # heading to name it'],
    ['SUPPORT.md', '# Support\n\n> a quote', 'SUPPORT.md:3: the site has no HTML for the markdown node "blockquote"'],
  ])('fails on %s with %j and leaves the earlier build alone', (path, text, message) => {
    const { folder, out: target } = out({ [path]: text })
    expect(() => build({ root: folder, out: target, repository })).toThrow(message)
    expect(existsSync(join(target, 'stale.html'))).toBe(true)
  })

  it.each([undefined, 'name'])('fails without a repository as owner/name (%j), even with no link to resolve', (bad) => {
    const { folder, out: target } = out()
    expect(() => build({ root: folder, out: target, repository: bad })).toThrow(/not a GitHub repository/)
    expect(existsSync(join(target, 'stale.html'))).toBe(true)
  })
})

describe('the script', () => {
  function run(args: string[]) {
    const env = { ...process.env, GITHUB_REPOSITORY: '' }
    return spawnSync(process.execPath, ['scripts/site.mjs', ...args], { cwd: root, env, encoding: 'utf8' })
  }

  it('fails with a clear message when it has no repository', () => {
    const result = run([])
    expect(result.status).toBe(1)
    expect(result.stderr).toContain('set GITHUB_REPOSITORY or pass --repository owner/name')
  })

  it('fails with a clear message when the repository is not owner/name', () => {
    const result = run(['--repository', 'name'])
    expect(result.status).toBe(1)
    expect(result.stderr.trim()).toBe('not a GitHub repository as owner/name: name')
  })
})
