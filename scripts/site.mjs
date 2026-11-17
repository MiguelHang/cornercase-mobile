import { posix } from 'node:path'
import { fromMarkdown } from 'mdast-util-from-markdown'
import { gfmFromMarkdown } from 'mdast-util-gfm'
import { gfm } from 'micromark-extension-gfm'

export const PAGES = { 'site/index.md': '', 'PRIVACY.md': 'privacy', 'SUPPORT.md': 'support' }

const REPOSITORY = /^[\w.-]+\/[\w.-]+$/
const ESCAPES = { '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' }

export function escape(text) {
  return text.replace(/[&<>"]/g, (character) => ESCAPES[character])
}

function pageOf(source) {
  if (!Object.hasOwn(PAGES, source)) throw new Error(`${source}: not a page of the site`)
  return PAGES[source]
}

function pageLink(from, to) {
  const path = posix.relative(`/${from}`, `/${to}`)
  return path === '' ? './' : `${path}/`
}

function repositoryBase(repository) {
  if (!REPOSITORY.test(repository ?? '')) throw new Error(`not a GitHub repository as owner/name: ${repository}`)
  return `https://github.com/${repository}/blob/main/`
}

function secure(url, source) {
  if (new URL(url).protocol !== 'https:') throw new Error(`${source}: a link that is not https: ${url}`)
  return url
}

export function resolveLink(url, { source, repository }) {
  const from = pageOf(source)
  if (URL.canParse(url)) return secure(url, source)
  const base = repositoryBase(repository)
  const target = new URL(url, base + source)
  const file = Object.keys(PAGES).find((path) => target.origin + target.pathname === base + path)
  return file === undefined ? target.href : pageLink(from, PAGES[file]) + target.hash
}

function unsupported(node, context, what) {
  const line = node.position?.start.line ?? '?'
  return new Error(`${context.source}:${line}: the site has no HTML for the markdown ${what}`)
}

function element(tag, content, attributes = '') {
  return `<${tag}${attributes}>${content}</${tag}>`
}

function inner(node, context, separator = '') {
  return node.children.map((child) => html(child, context)).join(separator)
}

function listItem(node, context, loose) {
  if (typeof node.checked === 'boolean') throw unsupported(node, context, 'task list item')
  const content = node.children.map((child) =>
    !loose && child.type === 'paragraph' ? inner(child, context) : html(child, context),
  )
  return element('li', content.join(loose ? '\n' : ''))
}

function list(node, context) {
  const loose = node.spread === true || node.children.some((child) => child.spread === true)
  const items = node.children.map((child) => listItem(child, context, loose)).join('\n')
  const start = node.ordered && node.start !== 1 && node.start != null ? ` start="${node.start}"` : ''
  return element(node.ordered ? 'ol' : 'ul', `\n${items}\n`, start)
}

const HANDLERS = {
  root: (node, context) => inner(node, context, '\n'),
  heading: (node, context) => element(`h${node.depth}`, inner(node, context)),
  paragraph: (node, context) => element('p', inner(node, context)),
  text: (node) => escape(node.value),
  emphasis: (node, context) => element('em', inner(node, context)),
  strong: (node, context) => element('strong', inner(node, context)),
  inlineCode: (node) => element('code', escape(node.value)),
  code: (node) => element('pre', element('code', escape(node.value))),
  break: () => '<br>',
  thematicBreak: () => '<hr>',
  list,
  link: (node, context) => element('a', inner(node, context), ` href="${escape(resolveLink(node.url, context))}"`),
}

function html(node, context) {
  if (!Object.hasOwn(HANDLERS, node.type)) throw unsupported(node, context, `node "${node.type}"`)
  return HANDLERS[node.type](node, context)
}

export function render(markdown, context) {
  const tree = fromMarkdown(markdown, { extensions: [gfm()], mdastExtensions: [gfmFromMarkdown()] })
  return html(tree, context)
}
