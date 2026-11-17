import { render } from 'svelte/server'
import { describe, expect, it } from 'vitest'
import fixture from '../../../crates/core/tests/fixtures/control/changes/v1/patch.json'
import type { Patch_Serialize as Patch } from '../../lib/bindings'
import { ChangesState, CLOSED, goneMessage, type Reading } from '../../lib/changes.svelte'
import Diff from './Diff.svelte'
import diffSource from './Diff.svelte?raw'
import lineSource from './DiffLine.svelte?raw'

const PATH = 'src/a.rs'

const never = () => new Promise<never>(() => {})

const PATCH = {
  ...fixture,
  staged: null,
  hunks: fixture.hunks.map((hunk) => ({
    ...hunk,
    lines: hunk.lines.map((line) => ({ raw: null, emphasis: [], ...line })),
  })),
} as Patch

const reading = (patch: Partial<Patch> = {}): Reading => ({ kind: 'patch', patch: { ...PATCH, ...patch } })

const stateWith = (shown: Reading | null, path = PATH) => {
  const state = new ChangesState(1, 2, { follow: never, unfollow: never, read: never, mark: never }, () => {})
  state.diff = shown === null ? null : { path, reading: shown }
  return state
}

const draw = (changes: ChangesState) =>
  render(Diff, { props: { path: PATH, changes, backTo: 'login', onback: () => {}, onfile: () => {} } }).body

const flat = (text: string) => text.replace(/\s+/g, ' ')

const bare = (html: string) => html.replace(/<!--[^]*?-->/g, '')

const note = (body: string) => bare(body).match(/<p class="note svelte-[\w-]+">(.*?)<\/p>/)?.[1] ?? null

const kinds = (body: string) => [...body.matchAll(/<div class="code-line (\w+) svelte-[\w-]+"/g)].map((m) => m[1])

const texts = (body: string) =>
  [...bare(body).matchAll(/<span class="code-text">(.*?)<\/span><\/div>/g)].map((m) => m[1].replace(/<[^>]+>/g, ''))

const numbers = (body: string) =>
  [...body.matchAll(/<span class="code-number">(\d*)<\/span><span class="code-number">(\d*)<\/span>/g)].map(
    (m) => [m[1], m[2]],
  )

const WHOLE = /<button class="tool svelte-[\w-]+" type="button" aria-label="Whole file">(?:<!--[^>]*-->| )*<svg [^>]*>(?:(?!<\/svg>)[^])*?<path d="M6 22a2 2 0 0 1-2-2V4/

const viewedButton = (body: string) => body.match(/aria-label="Viewed" aria-pressed="(\w+)"/)?.[1] ?? null

describe('Diff', () => {
  it('draws the hunk header, then each line with a class of its kind', () => {
    const body = draw(stateWith(reading()))
    expect(kinds(body)).toEqual(['hunk', 'context', 'removed', 'added', 'context'])
    expect(texts(body)).toEqual(['@@ -1 +1 @@ fn main() {', 'fn main() {', '    say(1);', '    say(2);', '    done();'])
  })

  it('numbers each line on its old and its new side', () => {
    expect(numbers(draw(stateWith(reading())))).toEqual([
      ['', ''],
      ['1', '1'],
      ['2', ''],
      ['', '2'],
      ['3', '3'],
    ])
  })

  it('signs the added and removed lines', () => {
    const signs = [...draw(stateWith(reading())).matchAll(/<span class="sign svelte-[\w-]+">(.?)<\/span>/g)]
    expect(signs.map((m) => m[1])).toEqual(['', ' ', '-', '+', ' '])
  })

  it('tints the changed characters of a line strong', () => {
    const strong = [...bare(draw(stateWith(reading()))).matchAll(/<span class="svelte-[\w-]+ strong">([^<]*)<\/span>/g)]
    expect(strong.map((m) => m[1])).toEqual(['1', '2'])
  })

  it('sizes the number columns for the longest number', () => {
    const wide = { ...PATCH.hunks[0], old_start: 998, lines: [{ ...PATCH.hunks[0].lines[0], old: 1000, new: 998 }] }
    expect(draw(stateWith(reading()))).toContain('style="--digits: 1ch;"')
    expect(draw(stateWith(reading({ hunks: [wide] })))).toContain('style="--digits: 4ch;"')
  })

  it("goes back to the list from a bar titled with the file's path, a rename with both", () => {
    const body = draw(stateWith(reading()))
    expect(body).toMatch(/<div class="bar ios svelte-[\w-]+ fixed topmost"[^]*?aria-label="Back to login"/)
    expect(body).toMatch(/<div class="center svelte-[\w-]+">(<!--[^>]*-->| )*<h1 class="path svelte-[\w-]+"><span dir="ltr">src\/a.rs<\/span>/)
    expect(body.match(/<h1/g)).toHaveLength(1)
    expect(body).not.toContain('class="top')
    expect(draw(stateWith(reading({ old_path: 'src/b.rs' })))).toContain('<span dir="ltr">src/b.rs → src/a.rs</span>')
  })

  it.each([
    ['binary', 'Binary file, not shown'],
    ['large', 'Too large to show'],
  ] as const)('shows a notice instead of the diff for a %s file', (fold, said) => {
    const body = draw(stateWith(reading({ fold, hunks: [] })))
    expect(note(body)).toBe(said)
    expect(body).not.toContain('class="line')
    expect(viewedButton(body)).toBe('false')
  })

  it('still draws the lines of a lockfile', () => {
    expect(kinds(draw(stateWith(reading({ fold: 'lockfile' }))))).toHaveLength(5)
  })

  it('says so for a change with no lines, such as a bare rename', () => {
    expect(note(draw(stateWith(reading({ hunks: [] }))))).toBe('No lines changed')
  })

  it('spins while the patch is on its way, also before the state opens it', () => {
    expect(note(draw(stateWith({ kind: 'loading' })))).toMatch(/Reading the diff…$/)
    expect(note(draw(stateWith(null)))).toMatch(/Reading the diff…$/)
    expect(note(draw(stateWith(reading(), 'src/other.rs')))).toMatch(/Reading the diff…$/)
  })

  it('shows why the patch could not be read', () => {
    expect(note(draw(stateWith({ kind: 'failed', reason: 'no such file' })))).toBe('no such file')
  })

  it('says the file has no changes any more, without the viewed button', () => {
    const body = draw(stateWith({ kind: 'gone' }))
    expect(note(body)).toBe(`${goneMessage(PATH)}.`)
    expect(viewedButton(body)).toBeNull()
  })

  it('leaves only the notice and the way back once the workspace is closed on the machine', () => {
    const state = stateWith(null)
    state.gone = true
    const body = draw(state)
    expect(note(body)).toBe(`${CLOSED}.`)
    expect(viewedButton(body)).toBeNull()
    expect(body).toContain('aria-label="Back to login"')
  })

  it('cuts a long path at its start and puts the viewed button at the right of the bar', () => {
    expect(flat(diffSource)).toContain('.path { margin: 0; overflow: hidden; direction: rtl;')
    expect(draw(stateWith(reading()))).toMatch(/<div class="end svelte-[\w-]+">(?:(?!<\/div>)[^])*<button class="check/)
  })

  it.each(['modified', 'added', 'renamed', 'untracked', 'other'] as const)(
    'offers the whole file of a %s change before the viewed button',
    (status) => {
      const body = draw(stateWith(reading({ status })))
      expect(body).toMatch(WHOLE)
      expect(body.search(WHOLE)).toBeLessThan(body.indexOf('aria-label="Viewed"'))
    },
  )

  it('offers no whole file for a deleted change, nor before the patch, once the change is gone or the workspace closed', () => {
    const closed = stateWith(reading())
    closed.gone = true
    expect(draw(stateWith(reading({ status: 'deleted' })))).not.toMatch(WHOLE)
    expect(draw(stateWith({ kind: 'loading' }))).not.toMatch(WHOLE)
    expect(draw(stateWith({ kind: 'gone' }))).not.toMatch(WHOLE)
    expect(draw(closed)).not.toMatch(WHOLE)
  })

  it("opens the patch's own path, the new one of a rename", () => {
    expect(flat(diffSource)).toContain(
      "const whole = $derived(open && patch !== null && patch.status !== 'deleted' ? patch.path : null)",
    )
    expect(flat(diffSource)).toContain('<ToolButton label="Whole file" icon="file-text" size={22} onclick={() => onfile(whole)} />')
  })

  it("presses the viewed button by the state's mark of the file", () => {
    expect(viewedButton(draw(stateWith(reading())))).toBe('false')
    expect(viewedButton(draw(stateWith(reading({ viewed: true }))))).toBe('true')
  })

  it('flips the viewed mark of the file through the state', () => {
    expect(flat(diffSource)).toContain('<Viewed {viewed} onflip={() => void changes.mark(path, !viewed)} />')
  })

  it('draws its lines in the code box, which scrolls them sideways with the numbers held at the left', () => {
    expect(draw(stateWith(reading()))).toMatch(
      /<div class="box svelte-[\w-]+"><div class="lines svelte-[\w-]+" style="--digits: 1ch;"><!--\[--><div class="code-line hunk /,
    )
  })

  it('tints the lines and their changed characters with the theme', () => {
    const line = flat(lineSource)
    expect(line).toContain('.added { --tint: var(--diff-added);')
    expect(line).toContain('.removed { --tint: var(--diff-removed);')
    expect(line).toContain('.added .strong { background: var(--diff-added-strong); }')
    expect(line).toContain('.removed .strong { background: var(--diff-removed-strong); }')
  })
})
