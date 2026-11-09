import type {
  ChangedFile_Serialize as ChangedFile,
  Hunk_Serialize as Hunk,
  Line_Serialize as Line,
  Patch_Serialize as Patch,
} from './bindings'

export type RowKind = 'hunk' | 'added' | 'removed' | 'context'

export type Span = { text: string; strong: boolean }

export type Row = { kind: RowKind; old: string; new: string; sign: string; spans: Span[] }

const SIGNS: Record<RowKind, string> = { hunk: '', added: '+', removed: '-', context: ' ' }

const LETTERS: Partial<Record<string, string>> = {
  modified: 'M',
  added: 'A',
  deleted: 'D',
  renamed: 'R',
  untracked: 'U',
}

const kindOf = (kind: string): RowKind => (kind === 'added' || kind === 'removed' ? kind : 'context')

const number = (value: number | null) => (value === null ? '' : String(value))

const strongMask = (length: number, emphasis: [number, number][]) => {
  const mask = new Array<boolean>(length).fill(false)
  for (const [start, end] of emphasis) {
    mask.fill(true, Math.max(0, start), Math.min(length, end))
  }
  return mask
}

export const spans = (text: string, emphasis: [number, number][]): Span[] => {
  const chars = Array.from(text)
  const mask = strongMask(chars.length, emphasis)
  const runs: Span[] = []
  chars.forEach((char, at) => {
    const last = runs.at(-1)
    if (last?.strong === mask[at]) last.text += char
    else runs.push({ text: char, strong: mask[at] })
  })
  return runs
}

export const header = (hunk: Pick<Hunk, 'old_start' | 'new_start' | 'context'>) =>
  `@@ -${hunk.old_start} +${hunk.new_start} @@${hunk.context === '' ? '' : ` ${hunk.context}`}`

const lineRow = (line: Line): Row => {
  const kind = kindOf(line.kind)
  return { kind, old: number(line.old), new: number(line.new), sign: SIGNS[kind], spans: spans(line.text, line.emphasis) }
}

const hunkRows = (hunk: Hunk): Row[] => [
  { kind: 'hunk', old: '', new: '', sign: SIGNS.hunk, spans: [{ text: header(hunk), strong: false }] },
  ...hunk.lines.map(lineRow),
]

export const rows = (patch: Pick<Patch, 'hunks'>): Row[] => patch.hunks.flatMap(hunkRows)

export const statusLetter = (status: string) => LETTERS[status] ?? '?'

export const shownPath = (file: Pick<ChangedFile, 'path' | 'old_path'>) =>
  file.old_path === null || file.old_path === file.path ? file.path : `${file.old_path} → ${file.path}`
