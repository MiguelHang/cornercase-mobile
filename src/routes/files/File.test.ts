import { render } from 'svelte/server'
import { describe, expect, it } from 'vitest'
import picture from '../../../crates/core/tests/fixtures/control/files/v1/image.json'
import text from '../../../crates/core/tests/fixtures/control/files/v1/text.json'
import type { FileShown_Serialize as FileShown, Read_Serialize as Read } from '../../lib/bindings'
import { CLOSED } from '../../lib/changes.svelte'
import { FilesState, type Reading } from '../../lib/files.svelte'
import File from './File.svelte'
import fileSource from './File.svelte?raw'

const never = () => new Promise<never>(() => {})

const TEXT = text as Read

const PICTURE = picture as Read

const showing = (reading: Reading | null, path = TEXT.path) => {
  const files = new FilesState(1, 3, { list: never, read: never, search: never, stopSearch: never }, () => {})
  files.file = reading === null ? null : { path, reading }
  return files
}

const shown = (shown: FileShown): Reading => ({ kind: 'shown', shown })

const reads = (read: Partial<Read>, base = TEXT) => shown({ shown: 'read', read: { ...base, ...read } })

const draw = (files: FilesState, path = TEXT.path) => render(File, { props: { path, files, backTo: 'login', onback: () => {} } }).body

const noted = (body: string) =>
  body.replace(/<!--[^]*?-->/g, '').match(/<p class="note svelte-[\w-]+">(.*?)<\/p>/s)?.[1].trim() ?? null

const numbers = (body: string) => [...body.matchAll(/<span class="code-number">(\d+)<\/span>/g)].map((m) => m[1])

const texts = (body: string) =>
  [...body.matchAll(/<span class="code-text text svelte-[\w-]+">([^<]*)<\/span>/g)].map((m) => m[1])

describe('File', () => {
  it('heads the screen with the path cut at its start in the shared bar, going back to the files', () => {
    const body = draw(showing(reads({})))
    expect(body).toMatch(/<div class="bar ios svelte-[\w-]+ fixed topmost"[^]*?aria-label="Back to login"/)
    expect(body).toMatch(/<div class="center svelte-[\w-]+">(<!--[^>]*-->| )*<h1 class="path svelte-[\w-]+"><span dir="ltr">src\/main\.rs<\/span><\/h1>/)
    expect(body.match(/<h1/g)).toHaveLength(1)
    expect(fileSource).toMatch(/\.path \{[^}]*direction: rtl;[^}]*text-overflow: ellipsis;/)
  })

  it('spins while the file is read', () => {
    expect(draw(showing({ kind: 'loading' }))).toMatch(/<p class="note svelte-[\w-]+"><span class="spinner[^]*?Reading the file…<\/p>/)
    expect(noted(draw(showing(null)))).toContain('Reading the file…')
  })

  it('spins rather than show another file that is still open', () => {
    expect(noted(draw(showing(reads({}), 'README.md')))).toContain('Reading the file…')
  })

  it('draws a text file in the code box with a number for each line', () => {
    const body = draw(showing(reads({ text: 'fn main() {\n    run();\n}\n' })))
    expect(body).toContain('<div class="box')
    expect(body).toContain('style="--digits: 1ch;"')
    expect(numbers(body)).toEqual(['1', '2', '3'])
    expect(texts(body)).toEqual(['fn main() {', '    run();', '}'])
  })

  it('keeps a last line with no line break, splits Windows line breaks and widens the numbers', () => {
    const body = draw(showing(reads({ text: `${'x\r\n'.repeat(11)}end` })))
    expect(numbers(body).at(-1)).toBe('12')
    expect(texts(body).at(-1)).toBe('end')
    expect(texts(body)[0]).toBe('x')
    expect(body).toContain('style="--digits: 2ch;"')
  })

  it('reads the server text fixture as one line', () => {
    expect(texts(draw(showing(reads({}))))).toEqual(['fn main() {}'])
  })

  it('says a file is empty', () => {
    const body = draw(showing(reads({ text: '' })))
    expect(noted(body)).toBe('Empty file')
    expect(body).not.toContain('class="box')
  })

  it('draws a picture from its data at its own size, fitted to the width', () => {
    const body = draw(showing(reads({}, PICTURE), PICTURE.path), PICTURE.path)
    expect(body).toContain(`src="data:image/png;base64,${PICTURE.image?.data}"`)
    expect(body).toContain('width="4" height="3" alt="logo.png"')
    expect(body).not.toContain('class="box')
    expect(fileSource).toMatch(/\.picture \{[^}]*max-width: 100%;[^}]*height: auto;/)
  })

  it.each([
    ['PNG', 'image/png'],
    ['JPEG', 'image/jpeg'],
    ['GIF', 'image/gif'],
    ['WebP', 'image/webp'],
    ['BMP', 'image/bmp'],
  ])('gives a %s picture the mime type %s', (format, mime) => {
    const image = { format, width: null, height: null, data: 'AAAA' }
    const body = draw(showing(reads({ image }, PICTURE)))
    expect(body).toContain(`src="data:${mime};base64,AAAA"`)
    expect(body).toMatch(/<img class="picture svelte-[\w-]+" src="[^"]*" alt="src\/main\.rs"\/>/)
  })

  it.each([
    ['binary', {}, 'Binary file, not shown'],
    ['too-large', { bytes: 12 * 1024 * 1024 }, 'Too large to show (12 MiB)'],
    ['missing', {}, 'src/main.rs no longer exists'],
    ['folder', {}, 'src/main.rs is a folder'],
    ['other', {}, 'Not a file, not shown'],
  ] as const)('shows a notice instead of a %s file', (kind, more, notice) => {
    const body = draw(showing(reads({ kind, text: null, ...more })))
    expect(noted(body)).toBe(notice)
    expect(body).not.toContain('class="box')
  })

  it('says a file too large for the phone never crossed to it', () => {
    const body = draw(showing(shown({ shown: 'too_large_here', bytes: 3 * 1024 * 1024, lines: 25_000 })))
    expect(noted(body)).toBe('Too large to show on the phone (3.0 MiB, 25,000 lines)')
  })

  it('shows why the file could not be read', () => {
    expect(noted(draw(showing({ kind: 'failed', reason: 'IPC is gone' })))).toBe('IPC is gone')
  })

  it('leaves only the notice and Back once the workspace is closed on the machine', () => {
    const files = showing(reads({}))
    files.gone = true
    const body = draw(files)
    expect(noted(body)).toBe(`${CLOSED}.`)
    expect(body).not.toContain('class="box')
    expect(body).toContain('aria-label="Back to login"')
  })
})
