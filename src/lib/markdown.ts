import { fromMarkdown } from 'mdast-util-from-markdown'
import { gfmFromMarkdown } from 'mdast-util-gfm'
import { gfm } from 'micromark-extension-gfm'

type Source = ReturnType<typeof fromMarkdown>['children'][number]

type Of<T extends Source['type']> = Extract<Source, { type: T }>

type Handlers = { [T in Source['type']]?: (node: Of<T>) => Node[] }

export type Align = 'left' | 'right' | 'center' | null

export type Cell = { type: 'cell'; children: Node[] }

export type Row = { type: 'row'; children: Cell[] }

export type Item = { type: 'item'; checked: boolean | null; children: Node[] }

export type Node =
  | { type: 'text' | 'inlineCode' | 'code'; value: string }
  | { type: 'break' | 'thematicBreak' }
  | { type: 'paragraph' | 'emphasis' | 'strong' | 'delete' | 'blockquote'; children: Node[] }
  | { type: 'heading'; depth: number; children: Node[] }
  | { type: 'list'; ordered: boolean; start: number | null; children: Item[] }
  | { type: 'table'; align: Align[]; children: Row[] }

const text = (value: string): Node[] => (value === '' ? [] : [{ type: 'text', value }])

const textOf = (node: Source): string => {
  if ('value' in node) {
    return node.value
  }
  if ('alt' in node) {
    return node.alt ?? ''
  }
  return 'children' in node ? node.children.map(textOf).join('') : ''
}

const joined = (nodes: Node[], node: Node): Node[] => {
  const last = nodes.at(-1)
  if (last?.type === 'text' && node.type === 'text') {
    return [...nodes.slice(0, -1), { type: 'text', value: last.value + node.value }]
  }
  return [...nodes, node]
}

const handled = (handlers: Handlers, node: Source, otherwise: (node: Source) => Node[]): Node[] => {
  const handle = handlers[node.type] as ((node: Source) => Node[]) | undefined
  return handle ? handle(node) : otherwise(node)
}

const inline = (node: Source): Node[] => handled(INLINE, node, (unknown) => text(textOf(unknown)))

const inlines = (children: Source[]): Node[] => children.flatMap(inline).reduce(joined, [])

const asParagraph = (value: string): Node[] => (value === '' ? [] : [{ type: 'paragraph', children: text(value) }])

const block = (node: Source): Node[] => handled(BLOCK, node, (unknown) => asParagraph(textOf(unknown)))

const blocks = (children: Source[]): Node[] => children.flatMap(block)

const formatted = (node: Of<'emphasis' | 'strong' | 'delete'>): Node[] => [
  { type: node.type, children: inlines(node.children) },
]

const INLINE: Handlers = {
  text: (node) => text(node.value),
  html: (node) => text(node.value),
  inlineCode: (node) => [{ type: 'inlineCode', value: node.value }],
  break: () => [{ type: 'break' }],
  emphasis: formatted,
  strong: formatted,
  delete: formatted,
  link: (node) => inlines(node.children),
  linkReference: (node) => inlines(node.children),
}

const item = (node: Of<'listItem'>): Item => ({ type: 'item', checked: node.checked ?? null, children: blocks(node.children) })

const row = (node: Of<'tableRow'>): Row => ({
  type: 'row',
  children: node.children.map((cell) => ({ type: 'cell', children: inlines(cell.children) })),
})

const BLOCK: Handlers = {
  paragraph: (node) => [{ type: 'paragraph', children: inlines(node.children) }],
  heading: (node) => [{ type: 'heading', depth: node.depth, children: inlines(node.children) }],
  code: (node) => [{ type: 'code', value: node.value }],
  list: (node) => [
    { type: 'list', ordered: node.ordered ?? false, start: node.start ?? null, children: node.children.map(item) },
  ],
  blockquote: (node) => [{ type: 'blockquote', children: blocks(node.children) }],
  table: (node) => [{ type: 'table', align: node.align ?? [], children: node.children.map(row) }],
  thematicBreak: () => [{ type: 'thematicBreak' }],
  html: (node) => asParagraph(node.value),
}

export const parse = (markdown: string): Node[] =>
  blocks(fromMarkdown(markdown, { extensions: [gfm()], mdastExtensions: [gfmFromMarkdown()] }).children)
