import { describe, expect, it } from 'vitest'
import type { Hunk_Serialize as Hunk, Line_Serialize as Line } from './bindings'
import * as diff from './diff'

const line = (kind: string, old: number | null, now: number | null, text: string, more: Partial<Line> = {}): Line => ({
  kind: kind as Line['kind'],
  old,
  new: now,
  text,
  raw: null,
  emphasis: [],
  ...more,
})

const hunk = (lines: Line[], more: Partial<Hunk> = {}): Hunk => ({
  old_start: 9,
  new_start: 11,
  context: 'fn main() {',
  lines,
  ...more,
})

const strong = (text: string, emphasis: [number, number][]) =>
  diff
    .spans(text, emphasis)
    .filter((span) => span.strong)
    .map((span) => span.text)

describe('rows', () => {
  it('opens each hunk with its header row', () => {
    const [header] = diff.rows({ hunks: [hunk([])] })
    expect(header).toEqual({
      kind: 'hunk',
      old: '',
      new: '',
      sign: '',
      spans: [{ text: '@@ -9 +11 @@ fn main() {', strong: false }],
    })
  })

  it.each([
    ['added', 'added', '+'],
    ['removed', 'removed', '-'],
    ['context', 'context', ' '],
    ['other', 'context', ' '],
    ['something new', 'context', ' '],
  ])('gives a %s line the %s class and the sign %o', (kind, shown, sign) => {
    const [, row] = diff.rows({ hunks: [hunk([line(kind, 3, 4, 'say(1);')])] })
    expect({ kind: row.kind, sign: row.sign }).toEqual({ kind: shown, sign })
  })

  it('keeps the line numbers and leaves empty the side a line lacks', () => {
    const lines = [line('context', 1, 1, 'a'), line('removed', 2, null, 'b'), line('added', null, 2, 'c')]
    const numbers = diff.rows({ hunks: [hunk(lines)] }).map((row) => [row.old, row.new])
    expect(numbers).toEqual([
      ['', ''],
      ['1', '1'],
      ['2', ''],
      ['', '2'],
    ])
  })

  it('follows the hunks in order', () => {
    const hunks = [hunk([line('added', null, 1, 'a')]), hunk([line('removed', 5, null, 'b')], { old_start: 5 })]
    const kinds = diff.rows({ hunks }).map((row) => row.kind)
    expect(kinds).toEqual(['hunk', 'added', 'hunk', 'removed'])
  })

  it('draws an empty patch as no rows', () => {
    expect(diff.rows({ hunks: [] })).toEqual([])
  })

  it('splits the text of a line by its emphasis', () => {
    const [, row] = diff.rows({ hunks: [hunk([line('removed', 2, null, '    say(1);', { emphasis: [[8, 9]] })])] })
    expect(row.spans).toEqual([
      { text: '    say(', strong: false },
      { text: '1', strong: true },
      { text: ');', strong: false },
    ])
  })
})

describe('header', () => {
  it('leaves out the context when there is none', () => {
    expect(diff.header({ old_start: 0, new_start: 1, context: '' })).toBe('@@ -0 +1 @@')
  })
})

describe('spans', () => {
  it.each([
    ['an emoji before', '🦀 let x = 1;', [[6, 7]], ['x']],
    ['accents before', 'café niño = 2', [[5, 9]], ['niño']],
    ['a decomposed accent', 'café = 3', [[4, 5]], ['́']],
    ['an emoji inside', 'say("🦀🦀")', [[5, 7]], ['🦀🦀']],
  ])('counts the emphasis in characters with %s', (_, text, emphasis, shown) => {
    expect(strong(text, emphasis as [number, number][])).toEqual(shown)
  })

  it('joins touching and overlapping ranges in any order', () => {
    expect(
      strong('abcdefgh', [
        [4, 6],
        [1, 3],
        [2, 4],
      ]),
    ).toEqual(['bcdef'])
  })

  it('ignores what falls outside the text', () => {
    expect(
      strong('abc', [
        [2, 10],
        [7, 9],
      ]),
    ).toEqual(['c'])
  })

  it('keeps the whole text when there is no emphasis', () => {
    expect(diff.spans('let x', [])).toEqual([{ text: 'let x', strong: false }])
  })

  it('gives an empty line no spans', () => {
    expect(diff.spans('', [[0, 1]])).toEqual([])
  })
})

describe('statusLetter', () => {
  it.each([
    ['modified', 'M'],
    ['added', 'A'],
    ['deleted', 'D'],
    ['renamed', 'R'],
    ['untracked', 'U'],
    ['other', '?'],
    ['copied', '?'],
  ])('labels %s as %s', (status, letter) => {
    expect(diff.statusLetter(status)).toBe(letter)
  })
})

describe('shownPath', () => {
  it.each([
    ['a renamed file', 'src/old.rs', 'src/old.rs → src/new.rs'],
    ['a file with no old path', null, 'src/new.rs'],
    ['an old path equal to the new', 'src/new.rs', 'src/new.rs'],
  ])('shows %s', (_, old, shown) => {
    expect(diff.shownPath({ path: 'src/new.rs', old_path: old })).toBe(shown)
  })
})
