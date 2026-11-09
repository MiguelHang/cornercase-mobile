import { describe, expect, it } from 'vitest'
import { parse, type Node } from './markdown'

const text = (value: string): Node => ({ type: 'text', value })

const paragraph = (...children: Node[]): Node => ({ type: 'paragraph', children })

const kinds = (nodes: unknown[]): string[] =>
  nodes.flatMap((node) => {
    const { type, children } = node as { type: string; children?: unknown[] }
    return [type, ...kinds(children ?? [])]
  })

describe('parse', () => {
  it.each<[string, string, Node[]]>([
    ['a paragraph', 'Hello there', [paragraph(text('Hello there'))]],
    ['a heading', '## Plan', [{ type: 'heading', depth: 2, children: [text('Plan')] }]],
    ['emphasis', '*soft*', [paragraph({ type: 'emphasis', children: [text('soft')] })]],
    ['strong', '**loud**', [paragraph({ type: 'strong', children: [text('loud')] })]],
    ['a deletion', '~~gone~~', [paragraph({ type: 'delete', children: [text('gone')] })]],
    ['inline code', 'run `pnpm test`', [paragraph(text('run '), { type: 'inlineCode', value: 'pnpm test' })]],
    ['a code block', '```rust\nfn main() {}\n```', [{ type: 'code', value: 'fn main() {}' }]],
    ['a hard break', 'one\\\ntwo', [paragraph(text('one'), { type: 'break' }, text('two'))]],
    ['a thematic break', '---', [{ type: 'thematicBreak' }]],
    ['a quote', '> said', [{ type: 'blockquote', children: [paragraph(text('said'))] }]],
  ])('keeps %s', (_, markdown, tree) => {
    expect(parse(markdown)).toEqual(tree)
  })

  it.each<[string, string, Node[]]>([
    [
      'a bulleted list',
      '- one\n- two',
      [
        {
          type: 'list',
          ordered: false,
          start: null,
          children: [
            { type: 'item', checked: null, children: [paragraph(text('one'))] },
            { type: 'item', checked: null, children: [paragraph(text('two'))] },
          ],
        },
      ],
    ],
    [
      'a numbered list from its start',
      '3. three',
      [
        {
          type: 'list',
          ordered: true,
          start: 3,
          children: [{ type: 'item', checked: null, children: [paragraph(text('three'))] }],
        },
      ],
    ],
    [
      'a task list',
      '- [x] done\n- [ ] next',
      [
        {
          type: 'list',
          ordered: false,
          start: null,
          children: [
            { type: 'item', checked: true, children: [paragraph(text('done'))] },
            { type: 'item', checked: false, children: [paragraph(text('next'))] },
          ],
        },
      ],
    ],
  ])('keeps %s', (_, markdown, tree) => {
    expect(parse(markdown)).toEqual(tree)
  })

  it('keeps a table with its alignment', () => {
    expect(parse('| file | lines |\n| :--- | ---: |\n| a.rs | `12` |')).toEqual([
      {
        type: 'table',
        align: ['left', 'right'],
        children: [
          {
            type: 'row',
            children: [
              { type: 'cell', children: [text('file')] },
              { type: 'cell', children: [text('lines')] },
            ],
          },
          {
            type: 'row',
            children: [
              { type: 'cell', children: [text('a.rs')] },
              { type: 'cell', children: [{ type: 'inlineCode', value: '12' }] },
            ],
          },
        ],
      },
    ])
  })

  it('keeps nested formatting inside a list in a quote', () => {
    expect(kinds(parse('> - **a *b***'))).toEqual([
      'blockquote',
      'list',
      'item',
      'paragraph',
      'strong',
      'text',
      'emphasis',
      'text',
    ])
  })

  it('gives nothing for an empty message', () => {
    expect(parse('')).toEqual([])
  })
})

describe('parse, with what could run or load', () => {
  it.each<[string, string, Node[]]>([
    ['a script block', '<script>alert(1)</script>', [paragraph(text('<script>alert(1)</script>'))]],
    ['raw html', '<div onclick="x()">hi</div>', [paragraph(text('<div onclick="x()">hi</div>'))]],
    ['an image tag', '<img src=x onerror=alert(1)>', [paragraph(text('<img src=x onerror=alert(1)>'))]],
    ['inline html', 'a <b>bold</b> word', [paragraph(text('a <b>bold</b> word'))]],
    ['a javascript link', '[x](javascript:alert(1))', [paragraph(text('x'))]],
    ['a link', 'see [the docs](https://example.com)', [paragraph(text('see the docs'))]],
    ['a bare url', 'at https://example.com now', [paragraph(text('at https://example.com now'))]],
    ['an autolink', '<https://example.com>', [paragraph(text('https://example.com'))]],
    ['a reference link', '[docs][d]\n\n[d]: https://example.com', [paragraph(text('docs'))]],
    ['an image', '![a cat](https://example.com/cat.png)', [paragraph(text('a cat'))]],
    ['an image without alt', 'before ![](https://example.com/x.png) after', [paragraph(text('before  after'))]],
    ['a reference image', '![a cat][c]\n\n[c]: https://example.com/cat.png', [paragraph(text('a cat'))]],
  ])('turns %s into text', (_, markdown, tree) => {
    expect(parse(markdown)).toEqual(tree)
  })

  it('keeps what it does not draw as its text', () => {
    expect(parse('a note[^1]\n\n[^1]: the footnote')).toEqual([paragraph(text('a note')), paragraph(text('the footnote'))])
  })

  it('never gives a link or an image', () => {
    const markdown = [
      '[x](javascript:alert(1)) ![y](https://example.com/y.png) <https://example.com> www.example.com',
      '> - [z](https://example.com) ![w][w]',
      '| [a](https://a.example) | ![b](https://b.example/b.png) |\n| --- | --- |',
      '[w]: https://example.com/w.png',
    ].join('\n\n')
    expect(kinds(parse(markdown)).filter((kind) => /link|image/i.test(kind))).toEqual([])
  })
})
