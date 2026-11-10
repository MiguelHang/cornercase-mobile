import type {
  CommandError,
  commands,
  Entry_Serialize as Entry,
  Folder_Serialize as Folder,
  Listing_Serialize as Listing,
  Toast,
} from './bindings'
import { CLOSED } from './changes.svelte'
import { messageOf } from './toasts'

export type Calls = {
  list: typeof commands.filesList
}

export type Listed = { entries: Entry[]; more: boolean; error: string | null }

export type Shown = { listed: Listed | null; loading: boolean }

export type Row =
  | { kind: 'entry'; path: string; depth: number; entry: Entry; open: boolean; loading: boolean }
  | { kind: 'more'; folder: string; depth: number }
  | { kind: 'error'; folder: string; depth: number; error: string }

type Answer = { status: 'ok'; data: Listing } | { status: 'error'; error: CommandError }

export const ROOT = ''

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
  #folders = $state.raw<Record<string, Shown>>({})
  #open = $state.raw<readonly string[]>([])
  #calls: Calls
  #tell: (toast: Toast) => void
  #left = false
  #generation = 0
  #asked: Record<string, number> = {}

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

  stop() {
    this.#left = true
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
    const answer = await this.#call(paths)
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

  async #call(paths: string[]): Promise<Answer> {
    try {
      return await this.#calls.list(this.workspace, paths)
    } catch (error) {
      return { status: 'error', error: { code: 'failed', message: messageOf(error) } }
    }
  }

  #adopt(paths: string[], mine: string[], listing: Listing) {
    this.#update(mine, (shown, path) => {
      const folder = listing.folders.at(paths.indexOf(path))
      return { listed: folder === undefined ? (shown?.listed ?? null) : listedOf(folder), loading: false }
    })
  }

  #failed(mine: string[], message: string) {
    if (closedBy(this.workspace, message)) {
      this.gone = true
      this.#tell({ level: 'info', message: CLOSED })
      return
    }
    const failed: Listed = { entries: [], more: false, error: message }
    this.#update(mine, (shown) => ({ listed: shown?.listed ?? failed, loading: false }))
    this.#tell({ level: 'error', message })
  }

  #update(paths: string[], next: (shown: Shown | undefined, path: string) => Shown) {
    const updated = Object.fromEntries(paths.map((path) => [path, next(this.#folders[path], path)]))
    this.#folders = { ...this.#folders, ...updated }
  }
}
