import type {
  IssueInfo_Serialize as IssueInfo,
  Lists_Serialize as Lists,
  SourceInfo_Serialize as SourceInfo,
  commands,
} from './bindings'
import { CHOICES_UNANSWERED, ask, type Reply } from './startForm.svelte'

export type IssueCalls = Pick<typeof commands, 'issueSources' | 'issueList'>

export type Trackers = { kind: 'loading' } | { kind: 'failed'; reason: string } | { kind: 'ready'; sources: SourceInfo[] }

export type Filters = { closed: boolean; mine: boolean; source: string | null }

export type Failure = { source: string; name: string; error: string }

export const CAP = 100

export const ANY_FILTERS: Filters = { closed: false, mine: false, source: null }

const termsOf = (query: string) => query.toLowerCase().split(/\s+/).filter((term) => term !== '')

const matches = (issue: IssueInfo, terms: string[]) => {
  const words = [issue.key, issue.title, ...issue.labels].map((word) => word.toLowerCase())
  return terms.every((term) => words.some((word) => word.includes(term)))
}

const sameFilters = (one: Filters, other: Filters) =>
  one.closed === other.closed && one.mine === other.mine && one.source === other.source

export class IssuesState {
  readonly machine: number
  readonly project: number
  trackers = $state.raw<Trackers>({ kind: 'loading' })
  filters = $state.raw<Filters>(ANY_FILTERS)
  query = $state('')
  lists = $state.raw<Lists | null>(null)
  loading = $state(false)
  error = $state<string | null>(null)
  #calls: IssueCalls
  #left = false
  #sourcesAsked = 0
  #listsAsked = 0
  #boot: string | null = null
  #connected: boolean | null = null

  constructor(machine: number, project: number, calls: IssueCalls) {
    this.machine = machine
    this.project = project
    this.#calls = calls
  }

  get connected(): SourceInfo[] {
    return this.trackers.kind === 'ready' ? this.trackers.sources.filter((source) => source.state === 'connected') : []
  }

  get connect() {
    return this.trackers.kind === 'ready' && this.connected.length === 0
  }

  get choosable() {
    return this.connected.length > 1
  }

  get issues(): IssueInfo[] {
    const terms = termsOf(this.query)
    return (this.lists?.issues ?? []).filter((issue) => matches(issue, terms))
  }

  get failures(): Failure[] {
    return (this.lists?.sources ?? []).flatMap(({ source, error }) =>
      error === null ? [] : [{ source, name: this.nameOf(source), error }],
    )
  }

  get capped() {
    return (this.lists?.sources ?? []).some((listed) => listed.error === null && listed.count >= CAP)
  }

  nameOf(source: string) {
    const known = this.trackers.kind === 'ready' ? this.trackers.sources : []
    return known.find((tracker) => tracker.source === source)?.name ?? source
  }

  load() {
    if (this.#left) {
      return Promise.resolve()
    }
    this.trackers = { kind: 'loading' }
    return this.refresh()
  }

  async refresh() {
    if (this.#left) {
      return
    }
    const asked = ++this.#sourcesAsked
    const reply = await ask(() => this.#calls.issueSources(this.project), CHOICES_UNANSWERED)
    if (asked !== this.#sourcesAsked || this.#left) {
      return
    }
    if (!reply.ok) {
      this.#unread(reply.error)
      return
    }
    this.trackers = { kind: 'ready', sources: reply.data.sources }
    this.#keepSource()
    await this.list()
  }

  retry() {
    return this.trackers.kind === 'ready' ? this.list() : this.load()
  }

  filter(change: Partial<Filters>) {
    const next = { ...this.filters, ...change }
    if (this.#left || sameFilters(next, this.filters)) {
      return Promise.resolve()
    }
    this.filters = next
    this.lists = null
    return this.list()
  }

  async see(connected: boolean, boot: string | null) {
    const back = connected && this.#connected === false
    const restarted = boot !== null && this.#boot !== null && boot !== this.#boot
    this.#connected = connected
    this.#boot = boot ?? this.#boot
    if (back || restarted) {
      await this.refresh()
    }
  }

  stop() {
    this.#left = true
    this.#sourcesAsked += 1
    this.#listsAsked += 1
  }

  async list() {
    if (this.#left || this.connect) {
      return
    }
    const asked = ++this.#listsAsked
    this.loading = true
    this.error = null
    await this.#read(asked, true)
    await this.#read(asked, false)
  }

  async #read(asked: number, cached: boolean) {
    if (asked !== this.#listsAsked) {
      return
    }
    const filter = { ...this.filters, cached }
    const reply = await ask(() => this.#calls.issueList(this.project, filter), CHOICES_UNANSWERED)
    if (asked === this.#listsAsked && !this.#left) {
      this.#adopt(reply, cached)
    }
  }

  #adopt(reply: Reply<Lists>, cached: boolean) {
    const lists = reply.ok && reply.data.project === this.project ? reply.data : null
    if (cached) {
      this.lists = lists !== null && lists.issues.length > 0 ? lists : this.lists
      return
    }
    this.loading = false
    this.lists = lists ?? this.lists
    this.error = reply.ok ? null : reply.error
  }

  #unread(reason: string) {
    if (this.trackers.kind === 'ready') {
      this.error = reason
    } else {
      this.trackers = { kind: 'failed', reason }
    }
  }

  #keepSource() {
    const source = this.filters.source
    if (source !== null && !this.connected.some((tracker) => tracker.source === source)) {
      this.filters = { ...this.filters, source: null }
      this.lists = null
    }
  }
}
