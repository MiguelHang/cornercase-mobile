import { render } from 'svelte/server'
import { describe, expect, it } from 'vitest'
import Markdown from './Markdown.svelte'
import nodeSource from './MarkdownNode.svelte?raw'
import markdownSource from './Markdown.svelte?raw'

const draw = (text: string) =>
  render(Markdown, { props: { text } })
    .body.replace(/<!--[^>]*-->/g, '')
    .replace(/ class="([^"]*?) ?svelte-[\w-]+( [^"]*)?"/g, (_, before: string, after = '') => {
      const names = `${before}${after}`.trim()
      return names === '' ? '' : ` class="${names}"`
    })
    .replace(/>\s+</g, '><')

const flat = (text: string) => text.replace(/\s+/g, ' ')

describe('Markdown', () => {
  it.each<[string, string, string]>([
    ['a paragraph', 'Hello there', '<p class="block">Hello there</p>'],
    ['a heading below the screen\'s own', '# Plan', '<h2 class="block heading">Plan</h2>'],
    ['a heading one level below its depth', '## Plan', '<h3 class="block heading">Plan</h3>'],
    ['the deepest heading at the deepest level', '###### Plan', '<h6 class="block heading">Plan</h6>'],
    ['emphasis', '*soft*', '<p class="block"><em>soft</em></p>'],
    ['strong', '**loud**', '<p class="block"><strong>loud</strong></p>'],
    ['a deletion', '~~gone~~', '<p class="block"><del>gone</del></p>'],
    ['inline code', 'run `pnpm test`', '<p class="block">run <code class="inline">pnpm test</code></p>'],
    ['a hard break', 'one\\\ntwo', '<p class="block">one<br/>two</p>'],
    ['a thematic break', '---', '<hr class="rule"/>'],
    ['a quote', '> said', '<blockquote class="block quote"><p class="block">said</p></blockquote>'],
  ])('draws %s', (_, markdown, html) => {
    expect(draw(markdown)).toBe(`<div class="markdown">${html}</div>`)
  })

  it('draws a bulleted list', () => {
    expect(draw('- one\n- two')).toContain(
      '<ul class="block list"><li><p class="block">one</p></li><li><p class="block">two</p></li></ul>',
    )
  })

  it('draws a numbered list from its start', () => {
    expect(draw('3. three')).toContain('<ol class="block list" start="3"><li><p class="block">three</p></li></ol>')
  })

  it('draws a task list with its boxes ticked or not', () => {
    const html = draw('- [x] done\n- [ ] next')
    expect(html).toContain('<li class="task"><input class="check" type="checkbox" checked="" disabled=""/>')
    expect(html).toContain('<li class="task"><input class="check" type="checkbox" disabled=""/><p class="block">next</p>')
  })

  it('draws a code block in mono inside a container that scrolls', () => {
    expect(draw('```rust\nfn main() {}\n```')).toContain(
      '<div class="scroll"><pre class="code"><code>fn main() {}</code></pre></div>',
    )
  })

  it('draws a GFM table with its first row as the head, aligned, inside a container that scrolls', () => {
    expect(draw('| file | lines |\n| :--- | ---: |\n| a.rs | `12` |')).toContain(
      '<div class="block scroll"><table class="table">' +
        '<thead><tr><th class="cell" style="text-align: left;">file</th><th class="cell" style="text-align: right;">lines</th></tr></thead>' +
        '<tbody><tr><td class="cell" style="text-align: left;">a.rs</td><td class="cell" style="text-align: right;"><code class="inline">12</code></td></tr></tbody>' +
        '</table></div>',
    )
  })

  it('draws a table without a body as its head only', () => {
    expect(draw('| a |\n| - |')).toContain('<thead><tr><th class="cell">a</th></tr></thead></table>')
  })

  it('draws nothing for an empty message', () => {
    expect(draw('')).toBe('<div class="markdown"></div>')
  })
})

describe('Markdown, with what could run or load', () => {
  it.each<[string, string, string]>([
    ['a script block', '<script>alert(1)</script>', '&lt;script>alert(1)&lt;/script>'],
    ['an image tag', '<img src=x onerror=alert(1)>', '&lt;img src=x onerror=alert(1)>'],
    ['raw html', '<div onclick="x()">hi</div>', '&lt;div onclick="x()">hi&lt;/div>'],
    ['inline html', 'a <b>bold</b> word', 'a &lt;b>bold&lt;/b> word'],
    ['a javascript link', '[x](javascript:alert(1))', 'x'],
    ['an image', '![a cat](https://example.com/cat.png)', 'a cat'],
  ])('draws %s as escaped text', (_, markdown, text) => {
    expect(draw(markdown)).toBe(`<div class="markdown"><p class="block">${text}</p></div>`)
  })

  it('never draws a link, an image or a script', () => {
    const html = draw(
      [
        '<script>alert(1)</script>',
        '[x](javascript:alert(1)) ![y](https://example.com/y.png) <https://example.com> www.example.com',
        '| [a](https://a.example) | ![b](https://b.example/b.png) |\n| --- | --- |',
        '```html\n<img src=x onerror=alert(1)>\n```',
      ].join('\n\n'),
    )
    expect(html).not.toMatch(/<(a|img|script)[\s>]|href=/)
  })

  it('never writes raw markup', () => {
    expect(nodeSource + markdownSource).not.toContain('{@html')
  })
})

describe('Markdown styles', () => {
  it.each([
    'overflow-x: auto;',
    'font-family: var(--font-mono);',
    'white-space: pre;',
    'border-left: 3px solid var(--edge);',
  ])('hold %s', (text) => {
    expect(flat(nodeSource)).toContain(text)
  })

  it('lets the message be selected and copied', () => {
    expect(markdownSource).toContain('\n    user-select: text;')
  })
})
