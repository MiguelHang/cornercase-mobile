import type {
  Lists_Serialize as Lists,
  PlaceInfo_Serialize as PlaceInfo,
  ProjectChoice,
  ReadIssue_Serialize as ReadIssue,
  SourceInfo_Serialize as SourceInfo,
  Sources_Serialize as Sources,
  commands,
} from './bindings'
import {
  CAP,
  capped as reachedCap,
  failed,
  gathered,
  limited,
  placesIn,
  standing,
  toList,
  type Found,
  type Swept,
  type Wanted,
} from './issueSweep'
import { CHOICES_UNANSWERED, ask, type Reply } from './startForm.svelte'

export type IssueCalls = Pick<typeof commands, 'startProjects' | 'issueSources' | 'issueList'>

export type ReadCalls = Pick<typeof commands, 'issueRead'>

export type Trackers = { kind: 'loading' } | { kind: 'failed'; reason: string } | { kind: 'ready'; sources: SourceInfo[] }

export type Filters = { closed: boolean; mine: boolean; source: string | null }

export type Failure = { source: string; name: string; error: string }

export type WantedIssue = { project: number; source: string; key: string }

export type IssueReading = { kind: 'reading' } | { kind: 'read'; read: ReadIssue } | { kind: 'failed'; reason: string }

type Read = { projects: ProjectChoice[]; answers: Sources[] }

type Round = { asked: number; wanted: Wanted[]; filters: Filters; slots: (Lists | null)[] }

type Refused = { ok: false; error: string }

export { CAP }

export const ANY_FILTERS: Filters = { closed: false, mine: false, source: null }

const termsOf = (query: string) => query.toLowerCase().split(/\s+/).filter((term) => term !== '')

const matches = (issue: Found, terms: string[]) => {
  const words = [issue.key, issue.title, ...issue.labels].map((word) => word.toLowerCase())
  return terms.every((term) => words.some((word) => word.includes(term)))
}

const sameFilters = (one: Filters, other: Filters) =>
  one.closed === other.closed && one.mine === other.mine && one.source === other.source

const sameWanted = (one: Wanted, other: Wanted) => one.project === other.project && one.source === other.source

const refusedIn = <T>(replies: Reply<T>[]) => replies.find((reply): reply is Refused => !reply.ok)

const filled = (round: Round): Swept[] =>
  round.wanted.flatMap((wanted, at) => {
    const lists = round.slots[at]
    return lists === null ? [] : [{ wanted, lists }]
  })

class Sight {
  #connected: boolean | null = null

  changed(connected: boolean) {
    const back = connected && this.#connected === false
    this.#connected = connected
    return back
  }
}

export class IssuesState {
  readonly machine: number
  trackers = $state.raw<Trackers>({ kind: 'loading' })
  filters = $state.raw<Filters>(ANY_FILTERS)
  query = $state('')
  swept = $state.raw<Swept[] | null>(null)
  places = $state.raw<PlaceInfo[]>([])
  loading = $state(false)
  error = $state<string | null>(null)
  #projects = $state.raw<ProjectChoice[]>([])
  #answers: Sources[] = []
  #calls: IssueCalls
  #left = false
  #sourcesAsked = 0
  #listsAsked = 0
  #sourcesFailed = false
  #sight = new Sight()

  constructor(machine: number, calls: IssueCalls) {
    this.machine = machine
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

  get issues(): Found[] {
    const terms = termsOf(this.query)
    return gathered(this.swept ?? []).filter((issue) => matches(issue, terms))
  }

  get failures(): Failure[] {
    return failed(this.swept ?? []).map(({ source, error }) => ({ source, name: this.nameOf(source), error }))
  }

  get capped() {
    return reachedCap(this.swept ?? [])
  }

  nameOf(source: string) {
    const known = this.trackers.kind === 'ready' ? this.trackers.sources : []
    return known.find((tracker) => tracker.source === source)?.name ?? source
  }

  projectName(project: number) {
    return this.#projects.find((known) => known.id === project)?.name ?? `${project}`
  }

  placesOf(project: number | null) {
    return project === null ? this.places : this.places.filter((place) => place.project === project)
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
    const reply = await this.#readSources()
    if (asked !== this.#sourcesAsked || this.#left) {
      return
    }
    this.#sourcesFailed = !reply.ok
    if (!reply.ok) {
      this.#unread(reply.error)
      return
    }
    this.#projects = reply.data.projects
    this.#answers = reply.data.answers
    this.places = placesIn(reply.data.answers)
    this.trackers = { kind: 'ready', sources: standing(reply.data.answers) }
    this.#keepSource()
    await this.list()
  }

  retry() {
    if (this.trackers.kind !== 'ready') {
      return this.load()
    }
    return this.#sourcesFailed ? this.refresh() : this.list()
  }

  filter(change: Partial<Filters>) {
    const next = { ...this.filters, ...change }
    if (this.#left || sameFilters(next, this.filters)) {
      return Promise.resolve()
    }
    this.filters = next
    this.swept = null
    return this.list()
  }

  async see(connected: boolean) {
    if (this.#sight.changed(connected)) {
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
    const wanted = toList(this.#answers, this.filters.source)
    const shown = this.swept ?? []
    const slots = wanted.map((one) => shown.find((swept) => sameWanted(swept.wanted, one))?.lists ?? null)
    const round = { asked: ++this.#listsAsked, wanted, filters: this.filters, slots }
    this.loading = true
    this.error = null
    await this.#sweep(round, true)
    await this.#sweep(round, false)
    if (round.asked === this.#listsAsked) {
      this.loading = false
    }
  }

  async #readSources(): Promise<Reply<Read>> {
    const chosen = await ask(() => this.#calls.startProjects(), CHOICES_UNANSWERED)
    if (!chosen.ok) {
      return chosen
    }
    const { projects } = chosen.data
    const replies = await limited(
      projects.map(({ id }) => () => ask(() => this.#calls.issueSources(id), CHOICES_UNANSWERED)),
    )
    const answers = replies.flatMap((reply) => (reply.ok ? [reply.data] : []))
    return refusedIn(replies) ?? { ok: true, data: { projects, answers } }
  }

  #sweep(round: Round, cached: boolean) {
    return limited(round.wanted.map((_, at) => () => this.#read(round, at, cached)))
  }

  async #read(round: Round, at: number, cached: boolean) {
    if (round.asked !== this.#listsAsked) {
      return
    }
    const { project, source } = round.wanted[at]
    const filter = { ...round.filters, source, cached }
    const reply = await ask(() => this.#calls.issueList(project, filter), CHOICES_UNANSWERED)
    if (round.asked === this.#listsAsked && !this.#left) {
      this.#adopt(round, at, reply, cached)
    }
  }

  #adopt(round: Round, at: number, reply: Reply<Lists>, cached: boolean) {
    if (!reply.ok) {
      this.error = cached || this.error !== null ? this.error : reply.error
      return
    }
    const lists = reply.data
    if (lists.project !== round.wanted[at].project || (cached && lists.issues.length === 0)) {
      return
    }
    round.slots[at] = lists
    this.swept = filled(round)
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
      this.swept = null
    }
  }
}

export class IssueState {
  readonly machine: number
  readonly project: number
  readonly source: string
  readonly key: string
  reading = $state.raw<IssueReading>({ kind: 'reading' })
  #calls: ReadCalls
  #left = false
  #asked = 0
  #sight = new Sight()

  constructor(machine: number, wanted: WantedIssue, calls: ReadCalls) {
    this.machine = machine
    this.project = wanted.project
    this.source = wanted.source
    this.key = wanted.key
    this.#calls = calls
  }

  load() {
    if (this.#left) {
      return Promise.resolve()
    }
    this.reading = { kind: 'reading' }
    return this.#read()
  }

  retry() {
    return this.load()
  }

  async see(connected: boolean) {
    if (this.#sight.changed(connected)) {
      await this.#read()
    }
  }

  stop() {
    this.#left = true
  }

  async #read() {
    if (this.#left) {
      return
    }
    const asked = ++this.#asked
    const reply = await ask(() => this.#calls.issueRead(this.project, this.source, this.key), CHOICES_UNANSWERED)
    if (asked !== this.#asked || this.#left) {
      return
    }
    if (reply.ok) {
      this.reading = { kind: 'read', read: reply.data }
    } else if (this.reading.kind !== 'read') {
      this.reading = { kind: 'failed', reason: reply.error }
    }
  }
}
