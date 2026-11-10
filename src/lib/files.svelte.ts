import type {
  CommandError,
  commands,
  Entry_Serialize as Entry,
  FileShown_Serialize as FileShown,
  Folder_Serialize as Folder,
  Found_Serialize as Found,
  Listing_Serialize as Listing,
  Toast,
} from './bindings'
import { CLOSED } from './changes.svelte'
import { messageOf } from './toasts'
import type { IconName } from './ui/icons/shapes'

export type Calls = {
  list: typeof commands.filesList
  read: typeof commands.filesRead
  search: typeof commands.filesSearch
  stopSearch: typeof commands.filesSearchStop
}

export type Listed = { entries: Entry[]; more: boolean; error: string | null }

export type Shown = { listed: Listed | null; loading: boolean }

export type Row =
  | { kind: 'entry'; path: string; depth: number; entry: Entry; open: boolean; loading: boolean }
  | { kind: 'more'; folder: string; depth: number }
  | { kind: 'error'; folder: string; depth: number; error: string }

export type Results = { found: Found | null; loading: boolean; error: string | null }

export type Reading = { kind: 'loading' } | { kind: 'shown'; shown: FileShown } | { kind: 'failed'; reason: string }

export type OpenFile = { path: string; reading: Reading }

export type Piece = { text: string; hit: boolean }

type Answer<T> = { status: 'ok'; data: T } | { status: 'error'; error: CommandError }

export const ROOT = ''

export const SEARCH_DELAY = 200

export const highlight = (path: string, indices: readonly number[]): Piece[] =>
  Array.from(path).reduce<Piece[]>((pieces, char, index) => {
    const hit = indices.includes(index)
    const last = pieces.at(-1)
    return last?.hit === hit
      ? [...pieces.slice(0, -1), { text: last.text + char, hit }]
      : [...pieces, { text: char, hit }]
  }, [])

const PICTURES = /\.(png|jpe?g|gif|webp|bmp)$/i

export const iconOf = (kind: Entry['kind'], name: string, open: boolean): IconName => {
  if (kind === 'folder') {
    return open ? 'folder-open' : 'folder'
  }
  return PICTURES.test(name) ? 'image' : 'file'
}

export const childOf = (folder: string, name: string) => (folder === ROOT ? name : `${folder}/${name}`)

const inside = (path: string, folder: string) => path === folder || path.startsWith(`${folder}/`)

const closedBy = (workspace: number, message: string) =>
  message.startsWith(`there is no workspace ${workspace};`) ||
  message === `workspace ${workspace} is closing` ||
  message === `workspace ${workspace} is being removed`

const listedOf = (folder: Folder): Listed => ({ entries: folder.entries, more: folder.more, error: folder.error })

export class FilesState {
  readonly machine: number
  readonly workspace: number
  gone = $state(false)
  query = $state('')
  results = $state.raw<Results | null>(null)
  file = $state.raw<OpenFile | null>(null)
  #folders = $state.raw<Record<string, Shown>>({})
  #open = $state.raw<readonly string[]>([])
  #calls: Calls
  #tell: (toast: Toast) => void
  #left = false
  #generation = 0
  #asked: Record<string, number> = {}
  #searches = 0
  #timer: ReturnType<typeof setTimeout> | undefined
  #reads = 0

  constructor(machine: number, workspace: number, calls: Calls, tell: (toast: Toast) => void) {
    this.machine = machine
    this.workspace = workspace
    this.#calls = calls
    this.#tell = tell
  }

  get rows(): Row[] {
    return this.#rowsOf(ROOT, 0)
  }

  folder(path: string): Shown | null {
    return this.#folders[path] ?? null
  }

  isOpen(path: string) {
    return this.#open.includes(path)
  }

  start() {
    return this.#list([ROOT])
  }

  refresh() {
    return this.#list([ROOT, ...this.#open])
  }

  toggle(path: string) {
    if (this.#open.includes(path)) {
      this.#open = this.#open.filter((open) => !inside(open, path))
      return Promise.resolve()
    }
    if (!this.#usable()) {
      return Promise.resolve()
    }
    this.#open = [...this.#open, path]
    return this.#list([path])
  }

  find(query: string) {
    this.query = query
    this.#searches += 1
    clearTimeout(this.#timer)
    const asked = query.trim()
    if (asked === '' || !this.#usable()) {
      this.results = null
      return
    }
    this.results = { found: this.results?.found ?? null, loading: true, error: null }
    const number = this.#searches
    this.#timer = setTimeout(() => void this.#search(asked, number), SEARCH_DELAY)
  }

  open(path: string) {
    if (!this.#usable()) {
      return Promise.resolve()
    }
    this.file = { path, reading: { kind: 'loading' } }
    return this.#read(path)
  }

  closeFile() {
    this.file = null
    this.#reads += 1
  }

  stop() {
    this.#left = true
    clearTimeout(this.#timer)
    this.#searches += 1
    this.#reads += 1
    this.#calls.stopSearch(this.workspace).catch(() => undefined)
  }

  #usable() {
    return !this.#left && !this.gone
  }

  #rowsOf(folder: string, depth: number): Row[] {
    const listed = this.#folders[folder]?.listed
    if (listed === null || listed === undefined) {
      return []
    }
    if (listed.error !== null) {
      return [{ kind: 'error', folder, depth, error: listed.error }]
    }
    const rows = listed.entries.flatMap((entry) => this.#rowsBelow(folder, depth, entry))
    return listed.more ? [...rows, { kind: 'more', folder, depth }] : rows
  }

  #rowsBelow(folder: string, depth: number, entry: Entry): Row[] {
    const path = childOf(folder, entry.name)
    const open = entry.kind === 'folder' && this.#open.includes(path)
    const loading = this.#folders[path]?.loading ?? false
    const row: Row = { kind: 'entry', path, depth, entry, open, loading }
    return open ? [row, ...this.#rowsOf(path, depth + 1)] : [row]
  }

  async #list(paths: string[]) {
    if (!this.#usable()) {
      return
    }
    const number = ++this.#generation
    paths.forEach((path) => (this.#asked[path] = number))
    this.#update(paths, (shown) => ({ listed: shown?.listed ?? null, loading: true }))
    const answer = await this.#ask(() => this.#calls.list(this.workspace, paths))
    if (!this.#usable()) {
      return
    }
    const mine = paths.filter((path) => this.#asked[path] === number)
    if (answer.status === 'ok') {
      this.#adopt(paths, mine, answer.data)
    } else {
      this.#failed(mine, answer.error.message)
    }
  }

  async #search(query: string, number: number) {
    const answer = await this.#ask(() => this.#calls.search(this.workspace, query))
    if (number !== this.#searches || !this.#usable()) {
      return
    }
    const found = this.results?.found ?? null
    if (answer.status === 'error') {
      this.results = { found, loading: false, error: this.#shownError(answer.error.message) }
      return
    }
    const searched = answer.data
    this.results = { found: searched.searched === 'found' ? searched.found : found, loading: false, error: null }
  }

  async #read(path: string) {
    const number = ++this.#reads
    const answer = await this.#ask(() => this.#calls.read(this.workspace, path))
    if (number !== this.#reads || !this.#usable()) {
      return
    }
    if (answer.status === 'ok') {
      this.file = { path, reading: { kind: 'shown', shown: answer.data } }
    } else {
      this.file = { path, reading: { kind: 'failed', reason: this.#unread(answer.error) } }
    }
  }

  #unread(error: CommandError) {
    return error.code === 'too_large_here' ? error.message : this.#shownError(error.message)
  }

  async #ask<T>(call: () => Promise<Answer<T>>): Promise<Answer<T>> {
    try {
      return await call()
    } catch (error) {
      return { status: 'error', error: { code: 'failed', message: messageOf(error) } }
    }
  }

  #adopt(paths: string[], mine: string[], listing: Listing) {
    this.#update(mine, (shown, path) => {
      const folder = listing.folders.find((listed) => listed.path === path) ?? listing.folders.at(paths.indexOf(path))
      return { listed: folder === undefined ? (shown?.listed ?? null) : listedOf(folder), loading: false }
    })
  }

  #failed(mine: string[], message: string) {
    if (this.#refused(message)) {
      return
    }
    const failed: Listed = { entries: [], more: false, error: message }
    this.#update(mine, (shown) => ({ listed: shown?.listed ?? failed, loading: false }))
  }

  #shownError(message: string) {
    return this.#refused(message) ? CLOSED : message
  }

  #refused(message: string) {
    if (closedBy(this.workspace, message)) {
      this.gone = true
      this.#tell({ level: 'info', message: CLOSED })
      return true
    }
    this.#tell({ level: 'error', message })
    return false
  }

  #update(paths: string[], next: (shown: Shown | undefined, path: string) => Shown) {
    const updated = Object.fromEntries(paths.map((path) => [path, next(this.#folders[path], path)]))
    this.#folders = { ...this.#folders, ...updated }
  }
}
