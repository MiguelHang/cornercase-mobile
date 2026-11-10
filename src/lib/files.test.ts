import { describe, expect, it, vi } from 'vitest'
import type {
  CommandError,
  Entry_Serialize as Entry,
  Folder_Serialize as Folder,
  Listing_Serialize as Listing,
  Toast,
} from './bindings'
import { CLOSED } from './changes.svelte'
import { childOf, FilesState, type Calls, type Row } from './files.svelte'

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

const refusal = (message: string, code: CommandError['code'] = 'failed'): Promise<Listed> =>
  Promise.resolve({ status: 'error', error: { code, message } })

const later = <T>() => {
  let answer: (value: T) => void = () => {}
  const promise = new Promise<T>((resolve) => (answer = resolve))
  return { promise, answer }
}

const fromTree = (tree: Record<string, Folder>) => (_: number, paths: string[]) =>
  done({ folders: paths.map((path) => tree[path] ?? folder(path, [], { error: 'no such folder' })) })

const browsing = async (list: Calls['list'] = fromTree(TREE)) => {
  const calls = { list: vi.fn<Calls['list']>(list) }
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
    expect([tree(files), tell.mock.calls]).toEqual([['v src', '  ! permission denied', 'v docs', '  a.md', 'notes.txt'], []])
  })

  it('says when a folder holds more entries than it listed', async () => {
    const { files } = await browsing(fromTree({ ...TREE, docs: folder('docs', [entry('a.md')], { more: true }) }))
    await files.toggle('docs')
    expect(tree(files)).toEqual(['> src', 'v docs', '  a.md', '  (more in docs)', 'notes.txt'])
  })

  it('keeps the hidden, linked and other entries with what the server said of them', async () => {
    const odd = [entry('.env', { hidden: true }), entry('link', { link: true, status: 'modified' }), entry('fifo', { kind: 'other' })]
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
    expect(tree(files)).toEqual(['v src', '  v cli', '    args.rs', '  main.rs', 'v docs', '  a.md', '  b.md', 'notes.txt'])
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
    const calls = { list: vi.fn<Calls['list']>(() => answer.promise) }
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
