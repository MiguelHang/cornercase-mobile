import { existsSync, readdirSync, readFileSync, statSync } from 'node:fs'
import { extname, join } from 'node:path'
import { fileURLToPath, pathToFileURL } from 'node:url'

export const MAX_LINES = 1000

const ROOTS = ['crates', 'bridge', 'src-tauri/src', 'src-tauri/build.rs', 'src', 'scripts']
const ROOT_CONFIG = /^[^/]+\.config\.(ts|js|mjs)$/
const EXTENSIONS = new Set(['.rs', '.ts', '.js', '.mjs', '.svelte'])
const GENERATED = { folders: new Set(['gen', 'target', 'node_modules', 'dist']), files: new Set(['src/lib/bindings.ts']) }
const LONG_FILES = []
const COMMENTED_FILES = []
const SCRIPT = 'scripts/check-rules.mjs'
const GUIDE = 'The rules and why they exist are in design/code-layout.md.'
const RUST_RAW_STRING = /r(#*)"/y
const RUST_CHAR = /'(?:\\.|[^\\'])'/y
const TS_SPACE = /\s+/y
const TS_WORD = /[\p{ID_Continue}$]+/uy
const TS_REGEX_KEYWORDS = new Set(
  'await case delete do else in instanceof new of return throw typeof void yield'.split(' '),
)

export function lineCount(text) {
  if (text === '') return 0
  const breaks = text.split('\n').length - 1
  return text.endsWith('\n') ? breaks : breaks + 1
}

function matchAt(pattern, text, cursor) {
  pattern.lastIndex = cursor
  return pattern.exec(text)
}

function lineAt(text, offset) {
  return text.slice(0, offset).split('\n').length
}

function lineEnd(text, cursor) {
  const end = text.indexOf('\n', cursor)
  return end === -1 ? text.length : end
}

function quotedEnd(text, quote) {
  let cursor = quote + 1
  while (cursor < text.length && text[cursor] !== text[quote]) cursor += text[cursor] === '\\' ? 2 : 1
  return cursor + 1
}

function rawStringEnd(text, body, hashes) {
  const ending = `"${hashes}`
  const close = text.indexOf(ending, body)
  return close === -1 ? text.length : close + ending.length
}

function nestedCommentEnd(text, start) {
  let depth = 0
  let cursor = start
  do {
    if (text.startsWith('/*', cursor)) {
      depth += 1
      cursor += 2
    } else if (text.startsWith('*/', cursor)) {
      depth -= 1
      cursor += 2
    } else {
      cursor += 1
    }
  } while (depth > 0 && cursor < text.length)
  return cursor
}

function rustLiteralEnd(text, cursor) {
  const raw = matchAt(RUST_RAW_STRING, text, cursor)
  if (raw) return rawStringEnd(text, RUST_RAW_STRING.lastIndex, raw[1])
  if (text[cursor] === '"') return quotedEnd(text, cursor)
  return matchAt(RUST_CHAR, text, cursor) ? RUST_CHAR.lastIndex : cursor + 1
}

function commentStep(text, cursor, blockEnd) {
  if (text.startsWith('//', cursor)) return { end: lineEnd(text, cursor), reason: 'line comment' }
  if (text.startsWith('/*', cursor)) return { end: blockEnd(text, cursor), reason: 'block comment' }
  return null
}

function rustStep(text, cursor) {
  return commentStep(text, cursor, nestedCommentEnd) ?? { end: rustLiteralEnd(text, cursor) }
}

function comments(text, step, state) {
  const found = []
  let cursor = 0
  while (cursor < text.length) {
    const { end, reason } = step(text, cursor, state)
    if (reason) found.push({ line: lineAt(text, cursor), reason })
    cursor = end
  }
  return found
}

export function rustComments(text) {
  return comments(text, rustStep)
}

function blockCommentEnd(text, cursor) {
  const close = text.indexOf('*/', cursor + 2)
  return close === -1 ? text.length : close + 2
}

function templateEnd(text, start, state) {
  let cursor = start
  while (cursor < text.length && text[cursor] !== '`') {
    if (text.startsWith('${', cursor)) {
      state.braces.push('template')
      state.regex = true
      return cursor + 2
    }
    cursor += text[cursor] === '\\' ? 2 : 1
  }
  state.regex = false
  return cursor + 1
}

function regexEnd(text, start) {
  let cursor = start + 1
  let inClass = false
  while (cursor < text.length && text[cursor] !== '\n' && (inClass || text[cursor] !== '/')) {
    if (text[cursor] === '\\') cursor += 1
    else if (text[cursor] === '[') inClass = true
    else if (text[cursor] === ']') inClass = false
    cursor += 1
  }
  return cursor + 1
}

function stringEnd(text, cursor, state) {
  state.regex = false
  return quotedEnd(text, cursor)
}

function templateStart(text, cursor, state) {
  return templateEnd(text, cursor + 1, state)
}

function openBraceEnd(text, cursor, state) {
  state.braces.push('brace')
  state.regex = true
  return cursor + 1
}

function closeBraceEnd(text, cursor, state) {
  if (state.braces.pop() === 'template') return templateEnd(text, cursor + 1, state)
  state.regex = true
  return cursor + 1
}

function slashEnd(text, cursor, state) {
  const regex = state.regex
  state.regex = !regex
  return regex ? regexEnd(text, cursor) : cursor + 1
}

function otherEnd(text, cursor, state) {
  if (matchAt(TS_SPACE, text, cursor)) return TS_SPACE.lastIndex
  const word = matchAt(TS_WORD, text, cursor)
  if (word) {
    state.regex = TS_REGEX_KEYWORDS.has(word[0])
    return TS_WORD.lastIndex
  }
  state.regex = !')]'.includes(text[cursor])
  return cursor + 1
}

function hashEnd(text, cursor, state) {
  return cursor === 0 && text[1] === '!' ? lineEnd(text, cursor) : otherEnd(text, cursor, state)
}

const TS_TOKENS = new Map([
  ["'", stringEnd],
  ['"', stringEnd],
  ['`', templateStart],
  ['{', openBraceEnd],
  ['}', closeBraceEnd],
  ['/', slashEnd],
  ['#', hashEnd],
])

function tsStep(text, cursor, state) {
  const token = TS_TOKENS.get(text[cursor]) ?? otherEnd
  return commentStep(text, cursor, blockCommentEnd) ?? { end: token(text, cursor, state) }
}

export function tsComments(text) {
  return comments(text, tsStep, { regex: true, braces: [] })
}

const LEXERS = { '.rs': rustComments, '.ts': tsComments, '.js': tsComments, '.mjs': tsComments }

function walk(root, path) {
  const full = join(root, path)
  if (!existsSync(full)) return []
  if (!statSync(full).isDirectory()) return [path]
  return readdirSync(full)
    .filter((name) => !GENERATED.folders.has(name))
    .flatMap((name) => walk(root, `${path}/${name}`))
}

function isChecked(path) {
  return EXTENSIONS.has(extname(path)) && !GENERATED.files.has(path)
}

function sourcePaths(root) {
  const configs = readdirSync(root).filter((name) => ROOT_CONFIG.test(name))
  return [...ROOTS.flatMap((path) => walk(root, path)), ...configs].filter(isChecked).sort()
}

function staleExceptions(sources, exceptions, list, breaks) {
  return exceptions.flatMap((path) => {
    const source = sources.find((candidate) => candidate.path === path)
    if (!source) return [`${path}:1: is gone: remove it from ${list} in ${SCRIPT}, or rename it if the file moved.`]
    return breaks(source) ? [] : [`${path}:1: follows the rule now: remove it from ${list} in ${SCRIPT}.`]
  })
}

function sizeFailures(sources, exceptions) {
  const breaks = (source) => lineCount(source.text) > MAX_LINES
  const found = sources
    .filter((source) => breaks(source) && !exceptions.includes(source.path))
    .map(
      (source) =>
        `${source.path}:${MAX_LINES + 1}: ${lineCount(source.text)} lines, over ${MAX_LINES}. Split it by concern; ` +
        'do not add it to LONG_FILES: that list only shrinks.',
    )
  return [...found, ...staleExceptions(sources, exceptions, 'LONG_FILES', breaks)]
}

function commentsOf(source) {
  const lexer = LEXERS[extname(source.path)]
  return lexer ? lexer(source.text) : []
}

function commentFailures(sources, exceptions) {
  const found = sources
    .filter((source) => !exceptions.includes(source.path))
    .flatMap((source) =>
      commentsOf(source).map(
        (comment) =>
          `${source.path}:${comment.line}: ${comment.reason}. Say it with names or move the why to design/; ` +
          'do not add it to COMMENTED_FILES: that list only shrinks.',
      ),
    )
  const breaks = (source) => commentsOf(source).length > 0
  return [...found, ...staleExceptions(sources, exceptions, 'COMMENTED_FILES', breaks)]
}

export function check(root, { long = LONG_FILES, commented = COMMENTED_FILES } = {}) {
  const sources = sourcePaths(root).map((path) => ({ path, text: readFileSync(join(root, path), 'utf8') }))
  return [...sizeFailures(sources, long), ...commentFailures(sources, commented)]
}

function main() {
  const failures = check(fileURLToPath(new URL('..', import.meta.url)))
  if (failures.length === 0) return
  console.error([...failures, GUIDE].join('\n'))
  process.exitCode = 1
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) main()
