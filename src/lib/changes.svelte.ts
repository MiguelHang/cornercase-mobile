import type {
  ChangedFile_Serialize as ChangedFile,
  ChangesChanged_Serialize as ChangesChanged,
  commands,
  Mode,
  Patch_Serialize as Patch,
  Summary_Serialize as Summary,
  Toast,
} from './bindings'
import { deliver, messageOf } from './toasts'

export type Calls = {
  follow: typeof commands.changesFollow
  unfollow: typeof commands.changesUnfollow
  read: typeof commands.changesFile
  mark: typeof commands.changesViewed
}

export type Reading =
  | { kind: 'loading' }
  | { kind: 'patch'; patch: Patch }
  | { kind: 'failed'; reason: string }
  | { kind: 'gone' }

export type OpenDiff = { path: string; reading: Reading }

type Mark = { viewed: boolean; id: number; settled: boolean }

export const CLOSED = 'This workspace was closed on the Mac'

export const goneMessage = (path: string) => `${path} no longer has changes`

export const readPatch = async (call: () => ReturnType<Calls['read']>): Promise<Reading> => {
  try {
    const answer = await call()
    return answer.status === 'ok' ? { kind: 'patch', patch: answer.data } : { kind: 'failed', reason: answer.error.message }
  } catch (error) {
    return { kind: 'failed', reason: messageOf(error) }
  }
}

const unsettled = (marks: Record<string, Mark>) =>
  Object.fromEntries(Object.entries(marks).filter(([, mark]) => !mark.settled))

export class ChangesState {
  readonly machine: number
  readonly workspace: number
  mode = $state<Mode>('uncommitted')
  summary = $state.raw<Summary | null>(null)
  diff = $state.raw<OpenDiff | null>(null)
  gone = $state(false)
  #marks = $state.raw<Record<string, Mark>>({})
  #calls: Calls
  #tell: (toast: Toast) => void
  #left = false
  #turn: Promise<void> = Promise.resolve()
  #nextMark = 0
  #generation = 0
  #readFor: string | null = null

  constructor(machine: number, workspace: number, calls: Calls, tell: (toast: Toast) => void) {
    this.machine = machine
    this.workspace = workspace
    this.#calls = calls
    this.#tell = tell
  }

  get listed() {
    return this.summary?.workspaces.find((changed) => changed.workspace === this.workspace) ?? null
  }

  get files(): ChangedFile[] {
    return (this.listed?.files ?? []).map((file) => ({ ...file, viewed: this.viewed(file.path) }))
  }

  viewed(path: string) {
    const shown = this.diff?.path === path && this.diff.reading.kind === 'patch' ? this.diff.reading.patch : undefined
    return this.#marks[path]?.viewed ?? this.#row(path)?.viewed ?? shown?.viewed ?? false
  }

  digestOf(path: string) {
    const diff = this.diff
    if (diff?.path === path && diff.reading.kind === 'patch') {
      return diff.reading.patch.digest
    }
    return this.#row(path)?.digest ?? null
  }

  start() {
    return this.#queue(this.mode)
  }

  async show(mode: Mode) {
    if (this.#left || this.gone || mode === this.mode) {
      return
    }
    this.mode = mode
    this.summary = null
    this.#marks = {}
    this.closeDiff()
    await this.#queue(mode)
  }

  stop() {
    this.#left = true
    this.#generation += 1
    void this.#unfollow()
  }

  see(changed: ChangesChanged) {
    if (this.#left || this.gone || !this.#mine(changed)) {
      return
    }
    if (changed.closed) {
      this.#close()
    } else if (changed.summary !== null) {
      this.#adopt(changed.summary)
    }
  }

  async mark(path: string, viewed: boolean) {
    if (this.#left || this.gone) {
      return
    }
    const digest = viewed ? this.digestOf(path) : null
    const id = ++this.#nextMark
    this.#marks = { ...this.#marks, [path]: { viewed, id, settled: false } }
    const file = { workspace: this.workspace, path, viewed, digest, mode: this.mode }
    const failed = await deliver(() => this.#calls.mark(file))
    this.#settle(path, id, failed)
  }

  open(path: string) {
    if (this.#left || this.gone) {
      return Promise.resolve()
    }
    this.diff = { path, reading: { kind: 'loading' } }
    return this.#read(path)
  }

  closeDiff() {
    this.diff = null
    this.#generation += 1
  }

  #row(path: string) {
    return this.listed?.files.find((file) => file.path === path)
  }

  #mine(changed: ChangesChanged) {
    return changed.machine === this.machine && changed.workspace === this.workspace && changed.mode === this.mode
  }

  #queue(mode: Mode) {
    this.#turn = this.#turn.then(() => this.#follow(mode))
    return this.#turn
  }

  async #follow(mode: Mode) {
    if (this.#left || mode !== this.mode) {
      return
    }
    try {
      const followed = await this.#calls.follow(this.workspace, mode)
      if (this.#left) {
        await this.#unfollow()
      } else if (followed.status === 'error') {
        this.#failed(followed.error.message)
      }
    } catch (error) {
      this.#failed(messageOf(error))
    }
  }

  #unfollow() {
    return this.#calls.unfollow(this.workspace).catch((error: unknown) => this.#failed(messageOf(error)))
  }

  #close() {
    this.gone = true
    this.closeDiff()
    this.#tell({ level: 'info', message: CLOSED })
  }

  #adopt(summary: Summary) {
    this.summary = summary
    this.#marks = unsettled(this.#marks)
    const diff = this.diff
    if (diff === null || diff.reading.kind === 'gone' || summary.state !== 'ready') {
      return
    }
    const row = this.#row(diff.path)
    if (row === undefined) {
      this.#generation += 1
      this.diff = { path: diff.path, reading: { kind: 'gone' } }
      this.#tell({ level: 'info', message: goneMessage(diff.path) })
    } else if (row.digest !== this.#readFor) {
      void this.#read(diff.path)
    }
  }

  async #read(path: string) {
    const number = ++this.#generation
    this.#readFor = this.#row(path)?.digest ?? null
    const reading = await readPatch(() => this.#calls.read(this.workspace, path, this.mode))
    if (number === this.#generation && this.diff?.path === path) {
      this.diff = { path, reading }
    }
  }

  #settle(path: string, id: number, failed: string | null) {
    if (failed !== null) {
      this.#failed(failed)
    }
    const mark = this.#marks[path]
    if (mark?.id !== id) {
      return
    }
    if (failed === null) {
      this.#marks = { ...this.#marks, [path]: { ...mark, settled: true } }
    } else {
      this.#marks = Object.fromEntries(Object.entries(this.#marks).filter(([kept]) => kept !== path))
    }
  }

  #failed(message: string) {
    this.#tell({ level: 'error', message })
  }
}
