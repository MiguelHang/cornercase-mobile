import { copyFileSync, existsSync, mkdirSync, readFileSync, realpathSync, rmSync, writeFileSync } from 'node:fs'
import { join, posix } from 'node:path'
import { fileURLToPath, pathToFileURL } from 'node:url'
import { parseArgs } from 'node:util'
import { fromMarkdown } from 'mdast-util-from-markdown'
import { gfmFromMarkdown } from 'mdast-util-gfm'
import { gfm } from 'micromark-extension-gfm'

const PAGES = { 'site/index.md': '', 'PRIVACY.md': 'privacy', 'SUPPORT.md': 'support' }
const ASSETS = { 'src/lib/theme/tokens.css': 'tokens.css', 'site/site.css': 'site.css' }
const TEMPLATE = 'site/page.html'
const SITE = 'Cornercase remote'
const NO_REPOSITORY = 'site.mjs needs the GitHub repository: set GITHUB_REPOSITORY or pass --repository owner/name.'

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

function checkRepository(repository) {
  if (!REPOSITORY.test(repository ?? '')) throw new Error(`not a GitHub repository as owner/name: ${repository}`)
}

function repositoryBase(repository) {
  checkRepository(repository)
  return `https://github.com/${repository}/blob/main/`
}

function secure(url, source) {
  if (new URL(url).protocol !== 'https:') throw new Error(`${source}: a link that is not https: ${url}`)
  return url
}

export function resolveLink(url, { source, repository }) {
  const from = pageOf(source)
  if (URL.canParse(url)) return secure(url, source)
  if (/^\/(?!\/)/.test(url)) throw new Error(`${source}: a link from the root, which GitHub reads otherwise: ${url}`)
  const base = repositoryBase(repository)
  const target = new URL(url, base + source)
  const file = Object.keys(PAGES).find((path) => target.origin + target.pathname === base + path)
  if (file === undefined) return target.href
  if (target.hash !== '') throw new Error(`${source}: the site has no anchors: ${url}`)
  return pageLink(from, PAGES[file])
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

function link(node, context) {
  if (node.title) throw unsupported(node, context, 'link title')
  return element('a', inner(node, context), ` href="${escape(resolveLink(node.url, context))}"`)
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
  link,
}

function html(node, context) {
  if (!Object.hasOwn(HANDLERS, node.type)) throw unsupported(node, context, `node "${node.type}"`)
  return HANDLERS[node.type](node, context)
}

function parse(markdown) {
  return fromMarkdown(markdown, { extensions: [gfm()], mdastExtensions: [gfmFromMarkdown()] })
}

export function render(markdown, context) {
  return html(parse(markdown), context)
}

export function plainText(node) {
  return node.value ?? (node.children ?? []).map(plainText).join('')
}

function pageTitle(tree, source) {
  const heading = tree.children.find((node) => node.type === 'heading' && node.depth === 1)
  if (heading === undefined) throw new Error(`${source}: the page has no # heading to name it`)
  return [...new Set([plainText(heading), SITE])].join(' · ')
}

export function fill(template, values) {
  return template.replace(/\{\{(\w+)\}\}/g, (_, key) => {
    if (!Object.hasOwn(values, key)) throw new Error(`${TEMPLATE}: no value for {{${key}}}`)
    return values[key]
  })
}

function page(template, markdown, context) {
  const tree = parse(markdown)
  const title = escape(pageTitle(tree, context.source))
  return fill(template, { root: pageLink(PAGES[context.source], ''), title, content: html(tree, context) })
}

export function build({ root, out, repository }) {
  checkRepository(repository)
  const template = readFileSync(join(root, TEMPLATE), 'utf8')
  const pages = Object.entries(PAGES).map(([source, folder]) => {
    const markdown = readFileSync(join(root, source), 'utf8')
    return { folder, text: page(template, markdown, { source, repository }) }
  })
  rmSync(out, { recursive: true, force: true })
  for (const { folder, text } of pages) {
    mkdirSync(join(out, folder), { recursive: true })
    writeFileSync(join(out, folder, 'index.html'), text)
  }
  for (const [source, name] of Object.entries(ASSETS)) copyFileSync(join(root, source), join(out, name))
}

function main() {
  const root = fileURLToPath(new URL('..', import.meta.url))
  try {
    const { values } = parseArgs({ options: { repository: { type: 'string' } } })
    const repository = values.repository || process.env.GITHUB_REPOSITORY
    if (!repository) throw new Error(NO_REPOSITORY)
    build({ root, out: join(root, '_site'), repository })
  } catch (error) {
    console.error(error.message)
    process.exitCode = 1
  }
}

const started = process.argv[1] && existsSync(process.argv[1]) && realpathSync(process.argv[1])
if (started && import.meta.url === pathToFileURL(started).href) main()
