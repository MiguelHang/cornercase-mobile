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
const LEXERS = {}
const SCRIPT = 'scripts/check-rules.mjs'
const GUIDE = 'The rules and why they exist are in design/code-layout.md.'

export function lineCount(text) {
  if (text === '') return 0
  const breaks = text.split('\n').length - 1
  return text.endsWith('\n') ? breaks : breaks + 1
}

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
