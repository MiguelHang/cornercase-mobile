import { describe, expect, it, vi } from 'vitest'
import type {
  CommandError,
  IssueInfo_Serialize as IssueInfo,
  ListedInfo_Serialize as ListedInfo,
  ListFilter_Deserialize as ListFilter,
  Lists_Serialize as Lists,
  ReadIssue_Serialize as ReadIssue,
  SourceInfo_Serialize as SourceInfo,
  Sources_Serialize as Sources,
} from './bindings'
import { ANY_FILTERS, CAP, IssueState, IssuesState, type IssueCalls, type ReadCalls } from './issues.svelte'
import { CHOICES_UNANSWERED } from './startForm.svelte'

const MACHINE = 1
const PROJECT = 7

type Answer<T> = { status: 'ok'; data: T } | { status: 'error'; error: CommandError }

const answered = <T>(data: T): Promise<Answer<T>> => Promise.resolve({ status: 'ok', data })

const refusal = <T>(message: string, code: CommandError['code'] = 'failed'): Promise<Answer<T>> =>
  Promise.resolve({ status: 'error', error: { code, message } })

const tracker = (source: string, more: Partial<SourceInfo> = {}): SourceInfo => ({
  source,
  name: source.toUpperCase(),
  repository: true,
  state: 'connected',
  missing: [],
  reason: null,
  ...more,
})

const issue = (key: string, title: string, labels: string[] = []): IssueInfo => ({
  source: 'github',
  key,
  title,
  state: 'open',
  closed: false,
  labels,
  author: 'grace',
  updated_at: '2026-11-09T09:20:24Z',
  branch: `issue-${key.slice(1)}`,
})

const listed = (source: string, count: number, error: string | null = null): ListedInfo => ({
  source,
  count,
  cached: false,
  error,
})

const ACME_41 = issue('#41', 'Checkout forgets the coupon', ['bug', 'checkout'])
const ACME_38 = issue('#38', 'Add a dark theme', ['design'])
const ENG_42 = { ...issue('ENG-42', 'Speed up the search'), source: 'shortcut' }

const lists = (issues: IssueInfo[], sources: ListedInfo[] = [listed('github', issues.length)]): Lists => ({
  project: PROJECT,
  issues,
  sources,
})

const sources = (...trackers: SourceInfo[]): Sources => ({ project: PROJECT, sources: trackers })

type Lister = (project: number, filter: ListFilter) => Promise<Answer<Lists>>

const live =
  (fresh: Lists, kept: Lists = lists([])): Lister =>
  (_project, filter) =>
    answered(filter.cached === true ? kept : fresh)

const fakes = (trackers: Sources = sources(tracker('github')), list: Lister = live(lists([ACME_41, ACME_38]))) => ({
  issueSources: vi.fn<IssueCalls['issueSources']>(() => answered(trackers)),
  issueList: vi.fn<IssueCalls['issueList']>(list),
})

const opened = async (calls = fakes()) => {
  const state = new IssuesState(MACHINE, PROJECT, calls)
  await state.load()
  return { state, calls }
}

const filtersSent = (calls: ReturnType<typeof fakes>) => calls.issueList.mock.calls.map(([, filter]) => filter)

const held = <T>() => {
  const waiting: ((answer: Answer<T>) => void)[] = []
  const call = () => new Promise<Answer<T>>((resolve) => waiting.push(resolve))
  const answer = (index: number, data: T) => waiting[index]?.({ status: 'ok', data })
  return { call, answer, waiting }
}

const settled = () => new Promise((resolve) => setTimeout(resolve, 0))

describe('IssuesState trackers', () => {
  it('reads the project trackers and lists the issues of the connected ones', async () => {
    const { state, calls } = await opened()
    expect(calls.issueSources).toHaveBeenCalledWith(PROJECT)
    expect(state.connected.map((source) => source.source)).toEqual(['github'])
    expect(state.issues).toEqual([ACME_41, ACME_38])
    expect(state.loading).toBe(false)
  })

  it('asks to connect a tracker on the Mac and lists nothing when none is connected', async () => {
    const missing = tracker('github', { state: 'missing', missing: ['repository'] })
    const failing = tracker('shortcut', { state: 'failing', reason: 'Shortcut rejected the token' })
    const { state, calls } = await opened(fakes(sources(missing, failing)))
    expect(state.connect).toBe(true)
    expect(calls.issueList).not.toHaveBeenCalled()
    expect(state.trackers).toEqual({ kind: 'ready', sources: [missing, failing] })
  })

  it('is not asking to connect while the trackers are read', () => {
    const state = new IssuesState(MACHINE, PROJECT, fakes())
    expect(state.connect).toBe(false)
    expect(state.trackers).toEqual({ kind: 'loading' })
  })

  it.each<[string, () => Promise<Answer<Sources>>, string]>([
    ['the reason cornercase gives', () => refusal('not a project'), 'not a project'],
    ['that cornercase did not answer', () => refusal('lost', 'unanswered'), CHOICES_UNANSWERED],
    ['what the call threw', () => Promise.reject(new Error('bridge down')), 'bridge down'],
  ])('says %s when the trackers cannot be read', async (_, answer, reason) => {
    const calls = fakes()
    calls.issueSources.mockImplementation(answer)
    const { state } = await opened(calls)
    expect(state.trackers).toEqual({ kind: 'failed', reason })
    expect(calls.issueList).not.toHaveBeenCalled()
  })

  it('reads the trackers again on Try again after they failed', async () => {
    const calls = fakes()
    calls.issueSources.mockImplementationOnce(() => refusal('offline'))
    const { state } = await opened(calls)
    await state.retry()
    expect(calls.issueSources).toHaveBeenCalledTimes(2)
    expect(state.issues).toEqual([ACME_41, ACME_38])
  })

  it('offers to choose a tracker only with more than one connected', async () => {
    const one = await opened(fakes(sources(tracker('github'), tracker('jira', { state: 'missing' }))))
    const two = await opened(fakes(sources(tracker('github'), tracker('shortcut'))))
    expect([one.state.choosable, two.state.choosable]).toEqual([false, true])
  })
})

describe('IssuesState filters', () => {
  it('lists open issues of every connected tracker at first', async () => {
    const { calls } = await opened()
    expect(filtersSent(calls)).toEqual([
      { ...ANY_FILTERS, cached: true },
      { ...ANY_FILTERS, cached: false },
    ])
  })

  it.each<[string, Partial<typeof ANY_FILTERS>]>([
    ['closed ones too', { closed: true }],
    ['mine', { mine: true }],
    ['one tracker', { source: 'shortcut' }],
    ['mine, closed ones too, in one tracker', { closed: true, mine: true, source: 'shortcut' }],
  ])('lists %s again', async (_, change) => {
    const { state, calls } = await opened(fakes(sources(tracker('github'), tracker('shortcut'))))
    calls.issueList.mockClear()
    await state.filter(change)
    expect(state.filters).toEqual({ ...ANY_FILTERS, ...change })
    expect(filtersSent(calls)).toEqual([
      { ...ANY_FILTERS, ...change, cached: true },
      { ...ANY_FILTERS, ...change, cached: false },
    ])
  })

  it('asks nothing when the filters stay the same', async () => {
    const { state, calls } = await opened()
    calls.issueList.mockClear()
    await state.filter({ closed: false })
    expect(calls.issueList).not.toHaveBeenCalled()
  })

  it('drops the issues of the other filters while the new ones are read', async () => {
    const pending = held<Lists>()
    const calls = fakes()
    const { state } = await opened(calls)
    calls.issueList.mockImplementation(pending.call)
    void state.filter({ mine: true })
    expect(state.lists).toBeNull()
    expect(state.loading).toBe(true)
  })

  it('forgets a chosen tracker that is no longer connected', async () => {
    const calls = fakes(sources(tracker('github'), tracker('shortcut')))
    const { state } = await opened(calls)
    await state.filter({ source: 'shortcut' })
    calls.issueSources.mockImplementation(() =>
      answered(sources(tracker('github'), tracker('shortcut', { state: 'failing' }))),
    )
    calls.issueList.mockClear()
    await state.refresh()
    expect(state.filters.source).toBeNull()
    expect(filtersSent(calls).at(-1)).toEqual({ ...ANY_FILTERS, cached: false })
  })
})

describe('IssuesState cached then live', () => {
  it('shows the cached issues until the live ones come', async () => {
    const pending = held<Lists>()
    const calls = fakes(sources(tracker('github')), pending.call)
    const state = new IssuesState(MACHINE, PROJECT, calls)
    const loading = state.load()
    await settled()
    pending.answer(0, lists([ACME_38]))
    await settled()
    expect([state.issues, state.loading]).toEqual([[ACME_38], true])
    pending.answer(1, lists([ACME_41, ACME_38]))
    await loading
    expect([state.issues, state.loading]).toEqual([[ACME_41, ACME_38], false])
  })

  it('keeps what it shows when the cache is empty', async () => {
    const calls = fakes()
    const { state } = await opened(calls)
    const pending = held<Lists>()
    calls.issueList.mockImplementation(pending.call)
    const reading = state.refresh()
    await settled()
    pending.answer(0, lists([]))
    await settled()
    expect([state.issues, state.loading]).toEqual([[ACME_41, ACME_38], true])
    pending.answer(1, lists([ACME_41]))
    await reading
    expect(state.issues).toEqual([ACME_41])
  })

  it('takes an empty live list', async () => {
    const calls = fakes()
    const { state } = await opened(calls)
    calls.issueList.mockImplementation(live(lists([]), lists([ACME_41])))
    await state.refresh()
    expect(state.issues).toEqual([])
  })

  it('goes on to the live list when the cache cannot be read', async () => {
    const calls = fakes(sources(tracker('github')), (_project, filter) =>
      filter.cached === true ? refusal('no cache') : answered(lists([ACME_41])),
    )
    const { state } = await opened(calls)
    expect([state.issues, state.error]).toEqual([[ACME_41], null])
  })

  it('keeps what it shows and says why when the live list fails, and lists again on Try again', async () => {
    const calls = fakes()
    const { state } = await opened(calls)
    calls.issueList.mockImplementation((_project, filter) =>
      filter.cached === true ? answered(lists([])) : refusal('gh failed'),
    )
    await state.refresh()
    expect([state.issues, state.error, state.loading]).toEqual([[ACME_41, ACME_38], 'gh failed', false])
    calls.issueList.mockImplementation(live(lists([ACME_38])))
    await state.retry()
    expect([state.issues, state.error]).toEqual([[ACME_38], null])
  })
})

describe('IssuesState search', () => {
  it.each<[string, string, IssueInfo[]]>([
    ['nothing typed', '  ', [ACME_41, ACME_38, ENG_42]],
    ['a key', 'eng-42', [ENG_42]],
    ['a word of a title', 'COUPON', [ACME_41]],
    ['a label', 'design', [ACME_38]],
    ['several words, each somewhere', 'bug  forgets', [ACME_41]],
    ['a word nowhere', 'invoice', []],
  ])('filters on the phone by %s', async (_, query, shown) => {
    const all = lists([ACME_41, ACME_38, ENG_42])
    const { state, calls } = await opened(fakes(sources(tracker('github')), live(all)))
    calls.issueList.mockClear()
    state.query = query
    expect(state.issues).toEqual(shown)
    expect(calls.issueList).not.toHaveBeenCalled()
  })
})

describe('IssuesState partial lists', () => {
  it('lists what came and one failure per tracker that failed, by its name', async () => {
    const partial = lists([ACME_41], [listed('github', 1), listed('shortcut', 0, 'Shortcut rejected the token')])
    const { state } = await opened(fakes(sources(tracker('github'), tracker('shortcut')), live(partial)))
    expect(state.issues).toEqual([ACME_41])
    expect(state.failures).toEqual([{ source: 'shortcut', name: 'SHORTCUT', error: 'Shortcut rejected the token' }])
  })

  it('names a failing tracker it does not know by its id', async () => {
    const partial = lists([], [listed('plane', 0, 'Plane is down')])
    const { state } = await opened(fakes(sources(tracker('github')), live(partial)))
    expect(state.failures).toEqual([{ source: 'plane', name: 'plane', error: 'Plane is down' }])
  })

  it.each<[string, ListedInfo[], boolean]>([
    ['a tracker listed as many as cornercase lists', [listed('github', 1), listed('shortcut', CAP)], true],
    ['every tracker listed fewer', [listed('github', CAP - 1), listed('shortcut', 3)], false],
    ['only a failed tracker counts as many', [listed('shortcut', CAP, 'rate limited')], false],
  ])('asks to narrow the filters when %s', async (_, counts, capped) => {
    const { state } = await opened(fakes(sources(tracker('github')), live(lists([ACME_41], counts))))
    expect(state.capped).toBe(capped)
  })
})

describe('IssuesState stale answers', () => {
  it('drops a list that comes for filters it no longer shows', async () => {
    const pending = held<Lists>()
    const calls = fakes()
    const { state } = await opened(calls)
    calls.issueList.mockImplementation(pending.call)
    const first = state.filter({ mine: true })
    await settled()
    const second = state.filter({ mine: false, closed: true })
    await settled()
    pending.answer(0, lists([ENG_42]))
    await first
    expect(state.lists).toBeNull()
    pending.answer(1, lists([ACME_38]))
    await settled()
    pending.answer(2, lists([ACME_41]))
    await second
    expect(state.issues).toEqual([ACME_41])
  })

  it('drops a list that comes for another project', async () => {
    const calls = fakes(sources(tracker('github')), live({ ...lists([ENG_42]), project: PROJECT + 1 }))
    const { state } = await opened(calls)
    expect([state.lists, state.loading]).toEqual([null, false])
  })

  it('drops what comes once it is left', async () => {
    const pending = held<Lists>()
    const state = new IssuesState(MACHINE, PROJECT, fakes(sources(tracker('github')), pending.call))
    const loading = state.load()
    await settled()
    state.stop()
    pending.answer(0, lists([ACME_41]))
    await loading
    expect(state.lists).toBeNull()
    expect(pending.waiting).toHaveLength(1)
    await state.filter({ mine: true })
    expect(pending.waiting).toHaveLength(1)
  })
})

describe('IssuesState reading again', () => {
  const reads = (calls: ReturnType<typeof fakes>) => [
    calls.issueSources.mock.calls.length,
    calls.issueList.mock.calls.length,
  ]

  it('reads nothing more while it stays connected', async () => {
    const { state, calls } = await opened()
    await state.see(true)
    await state.see(true)
    expect(reads(calls)).toEqual([1, 2])
  })

  it('reads the trackers and the list again when the connection comes back', async () => {
    const { state, calls } = await opened()
    await state.see(true)
    await state.see(false)
    await state.see(true)
    expect(reads(calls)).toEqual([2, 4])
  })

  it('keeps the issues it shows while it reads them again', async () => {
    const pending = held<Lists>()
    const calls = fakes()
    const { state } = await opened(calls)
    calls.issueList.mockImplementation(pending.call)
    void state.see(false)
    void state.see(true)
    await settled()
    expect([state.issues, state.loading]).toEqual([[ACME_41, ACME_38], true])
  })

  it('shows a failed read again as a failure to list, keeping the issues', async () => {
    const calls = fakes()
    const { state } = await opened(calls)
    calls.issueSources.mockImplementation(() => refusal('offline'))
    await state.refresh()
    expect(state.trackers.kind).toBe('ready')
    expect([state.issues, state.error]).toEqual([[ACME_41, ACME_38], 'offline'])
  })

  it('reads the trackers again on Try again after reading them again failed', async () => {
    const calls = fakes()
    const { state } = await opened(calls)
    calls.issueSources.mockImplementation(() => refusal('offline'))
    await state.refresh()
    calls.issueSources.mockImplementation(() => answered(sources(tracker('github'), tracker('shortcut'))))
    await state.retry()
    expect(state.connected.map((known) => known.source)).toEqual(['github', 'shortcut'])
    expect(state.error).toBeNull()
  })
})

const WANTED = { project: PROJECT, source: 'github', key: '#41' }

const readIssue = (body: string): ReadIssue => ({
  issue: ACME_41,
  body,
  comments: [{ author: 'linus', created_at: '2026-11-09T10:00:00Z', body: 'Seen on Safari too.' }],
  prompt: `Work on #41: ${body}`,
})

const FIRST = readIssue('The coupon disappears at the last step.')
const SECOND = readIssue('The coupon disappears, steps attached.')

const reader = (answer: ReadCalls['issueRead'] = () => answered(FIRST)) => ({
  issueRead: vi.fn<ReadCalls['issueRead']>(answer),
})

const shown = async (calls = reader()) => {
  const state = new IssueState(MACHINE, WANTED, calls)
  await state.load()
  return { state, calls }
}

describe('IssueState reading', () => {
  it('reads the issue it was asked for', async () => {
    const { state, calls } = await shown()
    expect(calls.issueRead).toHaveBeenCalledWith(PROJECT, 'github', '#41')
    expect(state.reading).toEqual({ kind: 'read', read: FIRST })
  })

  it('says it is reading until cornercase answers', () => {
    const pending = held<ReadIssue>()
    const state = new IssueState(MACHINE, WANTED, reader(pending.call))
    void state.load()
    expect(state.reading).toEqual({ kind: 'reading' })
  })

  it.each<[string, () => Promise<Answer<ReadIssue>>, string]>([
    ['the reason cornercase gives', () => refusal('there is no issue #41'), 'there is no issue #41'],
    ['that cornercase did not answer', () => refusal('lost', 'unanswered'), CHOICES_UNANSWERED],
    ['what the call threw', () => Promise.reject(new Error('bridge down')), 'bridge down'],
  ])('says %s when the issue cannot be read', async (_, answer, reason) => {
    const { state } = await shown(reader(answer))
    expect(state.reading).toEqual({ kind: 'failed', reason })
  })

  it('reads the issue again on Try again', async () => {
    const calls = reader(() => refusal('offline'))
    const { state } = await shown(calls)
    calls.issueRead.mockImplementation(() => answered(FIRST))
    const again = state.retry()
    expect(state.reading).toEqual({ kind: 'reading' })
    await again
    expect(state.reading).toEqual({ kind: 'read', read: FIRST })
  })
})

describe('IssueState stale answers', () => {
  it('drops an answer that arrives after a newer read', async () => {
    const pending = held<ReadIssue>()
    const state = new IssueState(MACHINE, WANTED, reader(pending.call))
    void state.load()
    void state.retry()
    pending.answer(1, SECOND)
    pending.answer(0, FIRST)
    await settled()
    expect(state.reading).toEqual({ kind: 'read', read: SECOND })
  })

  it('drops the answer once the screen left the issue', async () => {
    const pending = held<ReadIssue>()
    const state = new IssueState(MACHINE, WANTED, reader(pending.call))
    void state.load()
    state.stop()
    pending.answer(0, FIRST)
    await settled()
    expect(state.reading).toEqual({ kind: 'reading' })
  })

  it('asks nothing once the screen left the issue', async () => {
    const calls = reader()
    const state = new IssueState(MACHINE, WANTED, calls)
    state.stop()
    await state.load()
    await state.see(true)
    expect(calls.issueRead).not.toHaveBeenCalled()
  })
})

describe('IssueState reading again', () => {
  it.each<[string, boolean[], number]>([
    ['the connection comes back', [true, false, true], 2],
    ['it stays connected', [true, true], 1],
  ])('reads as many times as it should when %s', async (_, seen, times) => {
    const { state, calls } = await shown()
    for (const connected of seen) {
      await state.see(connected)
    }
    expect(calls.issueRead).toHaveBeenCalledTimes(times)
  })

  it('keeps the issue it shows while it reads it again', async () => {
    const pending = held<ReadIssue>()
    const calls = reader()
    const { state } = await shown(calls)
    calls.issueRead.mockImplementation(pending.call)
    await state.see(false)
    void state.see(true)
    expect(state.reading).toEqual({ kind: 'read', read: FIRST })
    pending.answer(0, SECOND)
    await settled()
    expect(state.reading).toEqual({ kind: 'read', read: SECOND })
  })

  it('keeps the issue it shows when reading it again fails', async () => {
    const calls = reader()
    const { state } = await shown(calls)
    calls.issueRead.mockImplementation(() => refusal('offline'))
    await state.see(false)
    await state.see(true)
    expect(state.reading).toEqual({ kind: 'read', read: FIRST })
  })
})
