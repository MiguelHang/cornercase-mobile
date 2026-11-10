import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import type {
  CommandError,
  Entry_Serialize as Entry,
  FileShown_Serialize as FileShown,
  Folder_Serialize as Folder,
  Found_Serialize as Found,
  Listing_Serialize as Listing,
  Read_Serialize as Read,
  Searched_Serialize as Searched,
  Toast,
} from './bindings'
import { CLOSED } from './changes.svelte'
import { childOf, FilesState, highlight, SEARCH_DELAY, type Calls, type Row } from './files.svelte'

const MACHINE = 1
const WORKSPACE = 3

const entry = (name: string, more: Partial<Entry> = {}): Entry => ({
  name,
  kind: 'file',
  link: false,
  hidden: false,
  bytes: 8,
  modified: null,
  status: null,
  ...more,
})

const folderEntry = (name: string) => entry(name, { kind: 'folder', bytes: null })

const folder = (path: string, entries: Entry[], more: Partial<Folder> = {}): Folder => ({
  path,
  dir: `/work/shop/${path}`,
  entries,
  more: false,
  error: null,
  ...more,
})

const TREE: Record<string, Folder> = {
  '': folder('', [folderEntry('src'), folderEntry('docs'), entry('notes.txt')]),
  src: folder('src', [folderEntry('cli'), entry('main.rs')]),
  'src/cli': folder('src/cli', [entry('args.rs')]),
  docs: folder('docs', [entry('a.md')]),
}

type Listed = Awaited<ReturnType<Calls['list']>>

const done = (data: Listing): Promise<Listed> => Promise.resolve({ status: 'ok', data })

type Refused = { status: 'error'; error: CommandError }

const refusal = (message: string, code: CommandError['code'] = 'failed'): Promise<Refused> =>
  Promise.resolve({ status: 'error', error: { code, message } })

const later = <T>() => {
  let answer: (value: T) => void = () => {}
  const promise = new Promise<T>((resolve) => (answer = resolve))
  return { promise, answer }
}

const fromTree = (tree: Record<string, Folder>) => (_: number, paths: string[]) =>
  done({ folders: paths.map((path) => tree[path] ?? folder(path, [], { error: 'no such folder' })) })

const read = (path: string, more: Partial<Read> = {}): Read => ({
  path,
  file: `/work/shop/${path}`,
  kind: 'text',
  bytes: 12,
  modified: null,
  language: 'rust',
  text: 'fn main() {}',
  image: null,
  ...more,
})

const found = (query: string, more: Partial<Found> = {}): Found => ({
  query,
  mode: 'names',
  names: [{ path: 'src/main.rs', indices: [4, 5] }],
  files: [],
  matches: 1,
  capped: false,
  ...more,
})

type Shown = Awaited<ReturnType<Calls['read']>>
type Search = Awaited<ReturnType<Calls['search']>>

const searched = (data: Searched): Promise<Search> => Promise.resolve({ status: 'ok', data })

const shownRead = (path: string): FileShown => ({ shown: 'read', read: read(path) })

const readsAll: Calls['read'] = (_, path) => Promise.resolve({ status: 'ok', data: shownRead(path) })

const findsAll: Calls['search'] = (_, query) => searched({ searched: 'found', found: found(query) })

const fakes = (list: Calls['list'], others: Partial<Calls> = {}) => ({
  list: vi.fn<Calls['list']>(list),
  read: vi.fn<Calls['read']>(others.read ?? readsAll),
  search: vi.fn<Calls['search']>(others.search ?? findsAll),
})

const browsing = async (list: Calls['list'] = fromTree(TREE), others: Partial<Calls> = {}) => {
  const calls = fakes(list, others)
  const tell = vi.fn<(toast: Toast) => void>()
  const files = new FilesState(MACHINE, WORKSPACE, calls, tell)
  await files.start()
  return { files, calls, tell }
}

const shown = (row: Row) => {
  const indent = '  '.repeat(row.depth)
  if (row.kind === 'entry') {
    const mark = row.entry.kind === 'folder' ? (row.open ? 'v ' : '> ') : ''
    return `${indent}${mark}${row.entry.name}${row.loading ? ' …' : ''}`
  }
  return row.kind === 'more' ? `${indent}(more in ${row.folder})` : `${indent}! ${row.error}`
}

const tree = (files: FilesState) => files.rows.map(shown)

describe('FilesState, its tree', () => {
  it('lists the root of its workspace when it starts', async () => {
    const { files, calls } = await browsing()
    expect([calls.list.mock.calls, tree(files)]).toEqual([[[WORKSPACE, ['']]], ['> src', '> docs', 'notes.txt']])
  })

  it('names a child by its folder, the root adding nothing', () => {
    expect([childOf('', 'src'), childOf('src', 'cli')]).toEqual(['src', 'src/cli'])
  })

  it('opens a folder under its row, a level deeper, with a spinner while it lists', async () => {
    const answer = later<Listed>()
    const list = vi.fn<Calls['list']>(fromTree(TREE))
    const { files, calls } = await browsing(list)
    list.mockReturnValueOnce(answer.promise)
    const opening = files.toggle('src')
    expect([tree(files), files.folder('src')?.loading]).toEqual([['v src …', '> docs', 'notes.txt'], true])
    answer.answer({ status: 'ok', data: { folders: [TREE.src] } })
    await opening
    expect([tree(files), calls.list.mock.calls.at(-1)]).toEqual([
      ['v src', '  > cli', '  main.rs', '> docs', 'notes.txt'],
      [WORKSPACE, ['src']],
    ])
  })

  it('opens folders inside open folders', async () => {
    const { files } = await browsing()
    await files.toggle('src')
    await files.toggle('src/cli')
    expect(tree(files)).toEqual(['v src', '  v cli', '    args.rs', '  main.rs', '> docs', 'notes.txt'])
  })

  it('closes an open folder and the folders open inside it, without asking', async () => {
    const { files, calls } = await browsing()
    await files.toggle('src')
    await files.toggle('src/cli')
    await files.toggle('docs')
    await files.toggle('src')
    expect([tree(files), files.isOpen('src/cli'), calls.list.mock.calls.length]).toEqual([
      ['> src', 'v docs', '  a.md', 'notes.txt'],
      false,
      4,
    ])
  })

  it('lists a folder again each time it opens, showing what it listed before meanwhile', async () => {
    const answer = later<Listed>()
    const list = vi.fn<Calls['list']>(fromTree(TREE))
    const { files } = await browsing(list)
    await files.toggle('src')
    await files.toggle('src')
    list.mockReturnValueOnce(answer.promise)
    const opening = files.toggle('src')
    expect(tree(files)).toEqual(['v src …', '  > cli', '  main.rs', '> docs', 'notes.txt'])
    answer.answer({ status: 'ok', data: { folders: [folder('src', [entry('lib.rs')])] } })
    await opening
    expect(tree(files)).toEqual(['v src', '  lib.rs', '> docs', 'notes.txt'])
  })

  it('shows the error of a folder in place of its entries, leaving the others alone', async () => {
    const broken = { ...TREE, src: folder('src', [], { error: 'permission denied' }) }
    const { files, tell } = await browsing(fromTree(broken))
    await files.toggle('src')
    await files.toggle('docs')
    expect([tree(files), tell.mock.calls]).toEqual([
      ['v src', '  ! permission denied', 'v docs', '  a.md', 'notes.txt'],
      [],
    ])
  })

  it('says when a folder holds more entries than it listed', async () => {
    const { files } = await browsing(fromTree({ ...TREE, docs: folder('docs', [entry('a.md')], { more: true }) }))
    await files.toggle('docs')
    expect(tree(files)).toEqual(['> src', 'v docs', '  a.md', '  (more in docs)', 'notes.txt'])
  })

  it('keeps the hidden, linked and other entries with what the server said of them', async () => {
    const odd = [
      entry('.env', { hidden: true }),
      entry('link', { link: true, status: 'modified' }),
      entry('fifo', { kind: 'other' }),
    ]
    const { files } = await browsing(fromTree({ '': folder('', odd) }))
    expect(files.rows.map((row) => row.kind === 'entry' && row.entry)).toEqual(odd)
  })

  it('takes only the newest listing of a folder opened twice', async () => {
    const first = later<Listed>()
    const list = vi.fn<Calls['list']>(fromTree(TREE))
    const { files } = await browsing(list)
    list.mockReturnValueOnce(first.promise)
    const opening = files.toggle('src')
    await files.toggle('src')
    await files.toggle('src')
    first.answer({ status: 'ok', data: { folders: [folder('src', [entry('old.rs')])] } })
    await opening
    expect(tree(files)).toEqual(['v src', '  > cli', '  main.rs', '> docs', 'notes.txt'])
  })
})

describe('FilesState, coming back to it', () => {
  it('lists the root and every open folder again in one call', async () => {
    const { files, calls } = await browsing()
    await files.toggle('src')
    await files.toggle('src/cli')
    await files.toggle('docs')
    const fresh = { ...TREE, docs: folder('docs', [entry('a.md'), entry('b.md')]) }
    calls.list.mockImplementation(fromTree(fresh))
    await files.refresh()
    expect(calls.list.mock.calls.at(-1)).toEqual([WORKSPACE, ['', 'src', 'src/cli', 'docs']])
    expect(tree(files)).toEqual([
      'v src',
      '  v cli',
      '    args.rs',
      '  main.rs',
      'v docs',
      '  a.md',
      '  b.md',
      'notes.txt',
    ])
  })

  it('pairs each folder with the path asked, whatever path the server echoes', async () => {
    const { files } = await browsing((_, paths) =>
      done({ folders: paths.map((path) => ({ ...TREE[path], path: path === '' ? '.' : `./${path}` })) }),
    )
    await files.toggle('docs')
    expect(tree(files)).toEqual(['> src', 'v docs', '  a.md', 'notes.txt'])
  })

  it('keeps what it listed when the listing again fails, and tells why', async () => {
    const { files, calls, tell } = await browsing()
    await files.toggle('docs')
    calls.list.mockImplementation(() => refusal('no machine is connected', 'not_connected'))
    await files.refresh()
    expect([tree(files), tell.mock.calls]).toEqual([
      ['> src', 'v docs', '  a.md', 'notes.txt'],
      [[{ level: 'error', message: 'no machine is connected' }]],
    ])
  })
})

describe('FilesState, failing', () => {
  it.each<[string, Calls['list'], string]>([
    ['refused', () => refusal('a line of output passed 16 MiB'), 'a line of output passed 16 MiB'],
    ['rejected', () => Promise.reject(new Error('IPC is gone')), 'IPC is gone'],
  ])('shows why a listing never listed was %s, and tells it', async (_, list, message) => {
    const { files, tell } = await browsing(list)
    expect([files.folder(''), tree(files), tell.mock.calls]).toEqual([
      { listed: { entries: [], more: false, error: message }, loading: false },
      [`! ${message}`],
      [[{ level: 'error', message }]],
    ])
  })

  it.each([
    ['gone', `there is no workspace ${WORKSPACE}; \`cornercase status\` lists them`],
    ['closing', `workspace ${WORKSPACE} is closing`],
    ['being removed', `workspace ${WORKSPACE} is being removed`],
  ])('says the workspace was closed on the Mac when it is %s, and asks nothing more', async (_, message) => {
    const { files, calls, tell } = await browsing(() => refusal(message))
    await files.toggle('src')
    await files.refresh()
    expect([files.gone, calls.list.mock.calls.length, tell.mock.calls]).toEqual([
      true,
      1,
      [[{ level: 'info', message: CLOSED }]],
    ])
  })

  it('takes another workspace that is gone as any other error', async () => {
    const message = `there is no workspace 30; \`cornercase status\` lists them`
    const { files, tell } = await browsing(() => refusal(message))
    expect([files.gone, tell.mock.calls]).toEqual([false, [[{ level: 'error', message }]]])
  })

  it('drops what answers once left, telling nothing', async () => {
    const answer = later<Listed>()
    const calls = fakes(() => answer.promise)
    const tell = vi.fn<(toast: Toast) => void>()
    const files = new FilesState(MACHINE, WORKSPACE, calls, tell)
    const starting = files.start()
    files.stop()
    answer.answer({ status: 'error', error: { code: 'failed', message: 'boom' } })
    await starting
    await files.toggle('src')
    expect([files.folder('')?.listed, tell.mock.calls, calls.list.mock.calls.length]).toEqual([null, [], 1])
  })
})

describe('FilesState, searching names', () => {
  beforeEach(() => {
    vi.useFakeTimers()
  })

  afterEach(() => {
    vi.useRealTimers()
  })

  const typed = async (files: FilesState, query: string) => {
    files.find(query)
    await vi.advanceTimersByTimeAsync(SEARCH_DELAY)
  }

  it('searches once typing stops for 200 ms, with the last query', async () => {
    const { files, calls } = await browsing()
    files.find('m')
    await vi.advanceTimersByTimeAsync(SEARCH_DELAY - 1)
    files.find('ma ')
    await vi.advanceTimersByTimeAsync(SEARCH_DELAY - 1)
    expect([calls.search.mock.calls, files.results, files.query]).toEqual([
      [],
      { found: null, loading: true, error: null },
      'ma ',
    ])
    await vi.advanceTimersByTimeAsync(1)
    expect([calls.search.mock.calls, files.results]).toEqual([
      [[WORKSPACE, 'ma']],
      { found: found('ma'), loading: false, error: null },
    ])
  })

  it('keeps the results it found while the next query waits', async () => {
    const { files } = await browsing()
    await typed(files, 'ma')
    files.find('main')
    expect(files.results).toEqual({ found: found('ma'), loading: true, error: null })
  })

  it.each([
    ['empty', ''],
    ['blank', '   '],
  ])('asks nothing for an %s query and goes back to the tree', async (_, blank) => {
    const { files, calls } = await browsing()
    await typed(files, 'ma')
    files.find('main')
    await typed(files, blank)
    await vi.advanceTimersByTimeAsync(SEARCH_DELAY * 5)
    expect([files.results, files.query, calls.search.mock.calls]).toEqual([null, blank, [[WORKSPACE, 'ma']]])
  })

  it('drops the answer of a query typed over, even when it comes last', async () => {
    const old = later<Search>()
    const search = vi.fn<Calls['search']>(findsAll)
    const { files } = await browsing(fromTree(TREE), { search })
    search.mockReturnValueOnce(old.promise)
    await typed(files, 'ma')
    await typed(files, 'main')
    old.answer({ status: 'ok', data: { searched: 'found', found: found('ma') } })
    await vi.advanceTimersByTimeAsync(0)
    expect(files.results).toEqual({ found: found('main'), loading: false, error: null })
  })

  it('keeps what it found when its search comes back cancelled', async () => {
    const search = vi.fn<Calls['search']>(findsAll)
    const { files, tell } = await browsing(fromTree(TREE), { search })
    await typed(files, 'ma')
    search.mockReturnValueOnce(searched({ searched: 'cancelled' }))
    await typed(files, 'main')
    expect([files.results, tell.mock.calls]).toEqual([{ found: found('ma'), loading: false, error: null }, []])
  })

  it.each<[string, Found]>([
    ['capped', found('s', { names: [{ path: 'a.rs', indices: [] }], capped: true })],
    ['empty', found('zzz', { names: [], matches: 0 })],
  ])('keeps a %s answer as it came', async (_, answer) => {
    const { files } = await browsing(fromTree(TREE), {
      search: () => searched({ searched: 'found', found: answer }),
    })
    await typed(files, answer.query)
    expect(files.results?.found).toEqual(answer)
  })

  it('shows why a search failed, and tells it', async () => {
    const { files, tell } = await browsing(fromTree(TREE), {
      search: () => Promise.reject(new Error('IPC is gone')),
    })
    await typed(files, 'ma')
    expect([files.results, tell.mock.calls]).toEqual([
      { found: null, loading: false, error: 'IPC is gone' },
      [[{ level: 'error', message: 'IPC is gone' }]],
    ])
  })

  it('says the workspace was closed on the Mac when a search finds it gone', async () => {
    const { files, tell } = await browsing(fromTree(TREE), {
      search: () => refusal(`workspace ${WORKSPACE} is closing`),
    })
    await typed(files, 'ma')
    expect([files.gone, files.results?.error, tell.mock.calls]).toEqual([
      true,
      CLOSED,
      [[{ level: 'info', message: CLOSED }]],
    ])
  })

  it('searches nothing once left', async () => {
    const { files, calls } = await browsing()
    files.find('ma')
    files.stop()
    await vi.advanceTimersByTimeAsync(SEARCH_DELAY)
    files.find('main')
    await vi.advanceTimersByTimeAsync(SEARCH_DELAY)
    expect([calls.search.mock.calls, files.results]).toEqual([[], null])
  })
})

describe('highlight', () => {
  it.each<[string, string, number[], [string, boolean][]]>([
    ['nothing', 'src/main.rs', [], [['src/main.rs', false]]],
    [
      'runs of letters',
      'src/main.rs',
      [4, 5, 6, 9],
      [
        ['src/', false],
        ['mai', true],
        ['n.', false],
        ['r', true],
        ['s', false],
      ],
    ],
    ['every letter', 'ab', [0, 1], [['ab', true]]],
    [
      'emoji as one letter',
      '🦀/main.rs',
      [0, 2],
      [
        ['🦀', true],
        ['/', false],
        ['m', true],
        ['ain.rs', false],
      ],
    ],
    [
      'accents as one letter',
      'café/ñu.md',
      [3, 5, 6],
      [
        ['caf', false],
        ['é', true],
        ['/', false],
        ['ñu', true],
        ['.md', false],
      ],
    ],
  ])('marks %s', (_, path, indices, pieces) => {
    expect(highlight(path, indices)).toEqual(pieces.map(([text, hit]) => ({ text, hit })))
  })
})

describe('FilesState, reading a file', () => {
  it('reads a file with a spinner meanwhile', async () => {
    const answer = later<Shown>()
    const { files, calls } = await browsing(fromTree(TREE), {
      read: () => answer.promise,
    })
    const opening = files.open('src/main.rs')
    expect(files.file).toEqual({ path: 'src/main.rs', reading: { kind: 'loading' } })
    answer.answer({ status: 'ok', data: shownRead('src/main.rs') })
    await opening
    expect([files.file, calls.read.mock.calls]).toEqual([
      {
        path: 'src/main.rs',
        reading: { kind: 'shown', shown: shownRead('src/main.rs') },
      },
      [[WORKSPACE, 'src/main.rs']],
    ])
  })

  it('reads a file again each time it opens', async () => {
    const { files, calls } = await browsing()
    await files.open('src/main.rs')
    files.closeFile()
    await files.open('src/main.rs')
    await files.open('src/main.rs')
    expect(calls.read.mock.calls).toEqual([1, 2, 3].map(() => [WORKSPACE, 'src/main.rs']))
  })

  it('takes only the newest read', async () => {
    const old = later<Shown>()
    const read = vi.fn<Calls['read']>(readsAll)
    const { files } = await browsing(fromTree(TREE), { read })
    read.mockReturnValueOnce(old.promise)
    const first = files.open('a.md')
    await files.open('b.md')
    old.answer({ status: 'ok', data: shownRead('a.md') })
    await first
    expect(files.file).toEqual({ path: 'b.md', reading: { kind: 'shown', shown: shownRead('b.md') } })
  })

  it('keeps a text too large for the phone as the bridge said it', async () => {
    const shown: FileShown = { shown: 'too_large_here', bytes: 3_000_000, lines: 40_000 }
    const { files } = await browsing(fromTree(TREE), {
      read: () => Promise.resolve({ status: 'ok', data: shown }),
    })
    await files.open('dump.sql')
    expect(files.file?.reading).toEqual({ kind: 'shown', shown })
  })

  it('drops a read that answers once the file is closed', async () => {
    const answer = later<Shown>()
    const { files } = await browsing(fromTree(TREE), {
      read: () => answer.promise,
    })
    const opening = files.open('a.md')
    files.closeFile()
    answer.answer({ status: 'ok', data: shownRead('a.md') })
    await opening
    expect(files.file).toBeNull()
  })

  it.each<[string, Calls['read'], string, Toast]>([
    [
      'refused',
      () => refusal('no machine is connected'),
      'no machine is connected',
      { level: 'error', message: 'no machine is connected' },
    ],
    [
      'rejected',
      () => Promise.reject(new Error('IPC is gone')),
      'IPC is gone',
      { level: 'error', message: 'IPC is gone' },
    ],
    [
      'on a closed workspace',
      () => refusal(`workspace ${WORKSPACE} is being removed`),
      CLOSED,
      { level: 'info', message: CLOSED },
    ],
  ])('shows why a read was %s, and tells it', async (_, readFile, reason, toast) => {
    const { files, tell } = await browsing(fromTree(TREE), {
      read: readFile,
    })
    await files.open('a.md')
    expect([files.file, tell.mock.calls]).toEqual([{ path: 'a.md', reading: { kind: 'failed', reason } }, [[toast]]])
  })
})
