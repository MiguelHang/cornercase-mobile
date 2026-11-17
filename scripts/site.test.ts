import { describe, expect, it } from 'vitest'
import { escape, render, resolveLink } from './site.mjs'

const repository = 'owner/name'

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
