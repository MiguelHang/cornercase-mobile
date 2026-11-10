import { describe, expect, it } from 'vitest'
import type {
  IssueInfo_Serialize as IssueInfo,
  ListedInfo_Serialize as ListedInfo,
  PlaceInfo_Serialize as PlaceInfo,
  SourceInfo_Serialize as SourceInfo,
  Sources_Serialize as Sources,
} from './bindings'
import { CAP, LIMIT, capped, failed, gathered, limited, placesIn, standing, toList, type Swept, type Wanted } from './issueSweep'

const SHOP = 4
const NOTES = 8
const BLOG = 9

const tracker = (source: string, more: Partial<SourceInfo> = {}): SourceInfo => ({
  source,
  name: source.toUpperCase(),
  repository: source === 'github' || source === 'gitlab',
  state: 'connected',
  missing: [],
  reason: null,
  ...more,
})

const unrepo = tracker('github', { state: 'missing', missing: ['repository'] })

const answer = (project: number, ...trackers: SourceInfo[]): Sources => ({ project, sources: trackers, places: [] })

const place = (project: number, workspace: number | null, label: string): PlaceInfo => ({
  project,
  workspace,
  label,
  worktree: workspace === null,
})

const issue = (source: string, key: string, updated: string, url = `https://example.com/${source}/${key}`): IssueInfo => ({
  source,
  key,
  title: `Issue ${key}`,
  url,
  state: 'open',
  closed: false,
  labels: [],
  author: 'grace',
  updated_at: updated,
  branch: `issue-${key}`,
})

const listed = (source: string, count: number, error: string | null = null): ListedInfo => ({
  source,
  count,
  cached: false,
  error,
})

const wanted = (project: number, source: string): Wanted => ({
  project,
  source,
  repository: source === 'github' || source === 'gitlab',
})

const swept = (want: Wanted, issues: IssueInfo[], sources = [listed(want.source, issues.length)]): Swept => ({
  wanted: want,
  lists: { project: want.project, issues, sources },
})

const keysOf = (found: { key: string; project: number }[]) => found.map(({ key, project }) => `${key}@${project}`)

describe('what to list', () => {
  it('lists a tracker of the account once, with the first project that has it connected', () => {
    const answers = [answer(SHOP, tracker('shortcut')), answer(NOTES, tracker('shortcut'))]
    expect(toList(answers, null)).toEqual([wanted(SHOP, 'shortcut')])
  })

  it('lists GitHub and GitLab in each project that has them connected', () => {
    const answers = [answer(SHOP, tracker('github')), answer(NOTES, tracker('gitlab')), answer(BLOG, tracker('github'))]
    expect(toList(answers, null)).toEqual([wanted(SHOP, 'github'), wanted(NOTES, 'gitlab'), wanted(BLOG, 'github')])
  })

  it('leaves out a project that is not a repository', () => {
    const answers = [answer(SHOP, unrepo, tracker('linear')), answer(NOTES, tracker('github'), tracker('linear'))]
    expect(toList(answers, null)).toEqual([wanted(SHOP, 'linear'), wanted(NOTES, 'github')])
  })

  it('skips the trackers that are not connected', () => {
    const failing = tracker('jira', { state: 'failing', reason: '401' })
    const answers = [answer(SHOP, failing, tracker('asana', { state: 'missing', missing: ['token'] }))]
    expect(toList(answers, null)).toEqual([])
  })

  it('lists only the chosen tracker', () => {
    const answers = [answer(SHOP, tracker('github'), tracker('shortcut')), answer(NOTES, tracker('github'))]
    expect(toList(answers, 'github')).toEqual([wanted(SHOP, 'github'), wanted(NOTES, 'github')])
  })

  it('lists nothing without projects', () => {
    expect(toList([], null)).toEqual([])
  })
})

describe('each tracker', () => {
  it('is shown once, in the order they first come, connected if it is in some project', () => {
    const github = tracker('github')
    const shortcut = tracker('shortcut')
    const answers = [answer(SHOP, unrepo, shortcut), answer(NOTES, github, shortcut), answer(BLOG, github)]
    expect(standing(answers)).toEqual([github, shortcut])
  })

  it('is shown as it is in a repository when no project has it connected', () => {
    const signedOut = tracker('github', { state: 'missing', missing: ['gh'] })
    const token = tracker('shortcut', { state: 'missing', missing: ['token'] })
    expect(standing([answer(SHOP, unrepo, token), answer(NOTES, signedOut, token)])).toEqual([signedOut, token])
  })

  it('is shown where it misses least', () => {
    const both = tracker('gitlab', { state: 'missing', missing: ['host', 'token'] })
    const token = tracker('gitlab', { state: 'missing', missing: ['token'] })
    const nowhere = tracker('gitlab', { state: 'missing', missing: ['repository'] })
    expect(standing([answer(SHOP, both), answer(NOTES, token), answer(BLOG, nowhere)])).toEqual([token])
  })

  it('is shown as in the first project when two miss as much', () => {
    const first = tracker('jira', { state: 'failing', reason: '401' })
    const second = tracker('jira', { state: 'failing', reason: '500' })
    expect(standing([answer(SHOP, first), answer(NOTES, second)])).toEqual([first])
  })

  it('keeps a tracker failing everywhere next to a connected one', () => {
    const failing = tracker('github', { state: 'failing', reason: 'gh is signed out' })
    const shortcut = tracker('shortcut')
    expect(standing([answer(SHOP, failing, shortcut), answer(NOTES, failing)])).toEqual([failing, shortcut])
  })

  it('is none without projects', () => {
    expect(standing([])).toEqual([])
  })
})

describe('the places', () => {
  it('are each place once, in the order they first come', () => {
    const worktree = place(SHOP, null, 'shop · new worktree')
    const main = place(NOTES, 3, 'notes › main')
    const answers = [
      { ...answer(SHOP), places: [worktree, main] },
      { ...answer(NOTES), places: [worktree, main, { ...main, label: 'shown again' }] },
      { ...answer(BLOG), places: [place(NOTES, 5, 'notes › draft')] },
    ]
    expect(placesIn(answers)).toEqual([worktree, main, place(NOTES, 5, 'notes › draft')])
  })
})

describe('the gathered issues', () => {
  it('are the newest first, each with the project it was listed with', () => {
    const story = issue('shortcut', 'sc-7', '2026-11-08T10:00:00Z')
    const bug = issue('github', '#41', '2026-11-09T10:00:00Z')
    const theme = issue('github', '#38', '2026-11-07T10:00:00Z')
    const found = gathered([swept(wanted(SHOP, 'shortcut'), [story]), swept(wanted(NOTES, 'github'), [bug, theme])])
    expect(found).toEqual([
      { ...bug, project: NOTES, repository: true },
      { ...story, project: SHOP, repository: false },
      { ...theme, project: NOTES, repository: true },
    ])
  })

  it('show an issue of a repository opened as two projects once, with the first project', () => {
    const bug = issue('github', '#41', '2026-11-09T10:00:00Z', 'https://github.com/acme/shop/issues/41')
    const found = gathered([swept(wanted(SHOP, 'github'), [bug]), swept(wanted(NOTES, 'github'), [bug])])
    expect(keysOf(found)).toEqual([`#41@${SHOP}`])
  })

  it('keep the same number in two repositories apart', () => {
    const shop = issue('github', '#12', '2026-11-09T10:00:00Z', 'https://github.com/acme/shop/issues/12')
    const notes = issue('github', '#12', '2026-11-08T10:00:00Z', 'https://github.com/acme/notes/issues/12')
    const found = gathered([swept(wanted(SHOP, 'github'), [shop]), swept(wanted(NOTES, 'github'), [notes])])
    expect(keysOf(found)).toEqual([`#12@${SHOP}`, `#12@${NOTES}`])
  })

  it('tell issues without a url apart by tracker, key and project', () => {
    const bug = issue('github', '#41', '2026-11-09T10:00:00Z', '')
    const twice = gathered([swept(wanted(SHOP, 'github'), [bug, bug]), swept(wanted(NOTES, 'github'), [bug])])
    const story = { ...bug, source: 'gitlab' }
    const other = gathered([swept(wanted(SHOP, 'github'), [bug]), swept(wanted(SHOP, 'gitlab'), [story])])
    expect([keysOf(twice), other.map(({ source }) => source)]).toEqual([
      [`#41@${SHOP}`, `#41@${NOTES}`],
      ['github', 'gitlab'],
    ])
  })

  it('keep the order they came in when updated at the same time', () => {
    const one = issue('linear', 'ENG-1', '2026-11-09T10:00:00Z')
    const two = issue('linear', 'ENG-2', '2026-11-09T10:00:00Z')
    expect(keysOf(gathered([swept(wanted(SHOP, 'linear'), [one, two])]))).toEqual([`ENG-1@${SHOP}`, `ENG-2@${SHOP}`])
  })

  it.each<[string, [string, string], [string, string]]>([
    ["Jira's offset without a colon", ['github', '2026-11-09T09:45:00Z'], ['jira', '2026-11-09T10:30:00.000+0100']],
    ['milliseconds next to whole seconds', ['gitlab', '2026-11-09T10:00:00.123Z'], ['github', '2026-11-09T10:00:00Z']],
    ['an offset with a colon', ['linear', '2026-11-09T05:30:00-05:00'], ['github', '2026-11-09T10:15:00Z']],
  ])('are ordered by the instant they were updated, with %s', (_, [newSource, newer], [oldSource, older]) => {
    const old = swept(wanted(SHOP, oldSource), [issue(oldSource, 'OLD', older)])
    const found = gathered([old, swept(wanted(NOTES, newSource), [issue(newSource, 'NEW', newer)])])
    expect(keysOf(found)).toEqual([`NEW@${NOTES}`, `OLD@${SHOP}`])
  })

  it('put an issue whose time cannot be read last', () => {
    const unread = issue('plane', 'WEB-2', 'yesterday')
    const dated = issue('github', '#41', '2026-11-09T10:00:00Z')
    const found = gathered([swept(wanted(SHOP, 'plane'), [unread]), swept(wanted(NOTES, 'github'), [dated])])
    expect(keysOf(found)).toEqual([`#41@${NOTES}`, `WEB-2@${SHOP}`])
  })

  it('are none when nothing was listed', () => {
    expect(gathered([])).toEqual([])
  })
})

describe('the failures', () => {
  it('show a tracker that failed in every project once, with its first error', () => {
    const jira = (project: number, error: string) => swept(wanted(project, 'jira'), [], [listed('jira', 0, error)])
    expect(failed([jira(SHOP, '401'), jira(NOTES, '500')])).toEqual([{ source: 'jira', error: '401' }])
  })

  it('leave out the trackers that answered', () => {
    const both = swept(wanted(SHOP, 'github'), [], [listed('github', 3), listed('shortcut', 0, 'timed out')])
    expect(failed([both])).toEqual([{ source: 'shortcut', error: 'timed out' }])
  })
})

describe('the cap', () => {
  it('is reached when a tracker that answered listed as many as the cap', () => {
    const full = swept(wanted(NOTES, 'github'), [], [listed('github', CAP)])
    expect(capped([swept(wanted(SHOP, 'github'), []), full])).toBe(true)
  })

  it('is not reached below it or by a tracker that failed', () => {
    const under = swept(wanted(SHOP, 'github'), [], [listed('github', CAP - 1)])
    const broken = swept(wanted(NOTES, 'jira'), [], [listed('jira', CAP, 'timed out')])
    expect(capped([under, broken])).toBe(false)
  })
})

describe('the limiter', () => {
  const gate = () => {
    let open = () => {}
    const opened = new Promise<void>((resolve) => {
      open = resolve
    })
    return { opened, open }
  }

  it('runs at most four at a time and starts the next as one ends', async () => {
    const gates = Array.from({ length: 6 }, gate)
    const started: number[] = []
    const tasks = gates.map((one, at) => async () => {
      started.push(at)
      await one.opened
      return at * 10
    })
    const all = limited(tasks)
    const atFirst = [...started]
    gates[2].open()
    await gates[2].opened
    await Promise.resolve()
    const afterOne = [...started]
    gates.forEach((one) => one.open())
    expect([LIMIT, atFirst, afterOne, await all]).toEqual([4, [0, 1, 2, 3], [0, 1, 2, 3, 4], [0, 10, 20, 30, 40, 50]])
  })

  it('takes another limit', async () => {
    const started: number[] = []
    const tasks = [0, 1, 2].map((at) => async () => {
      started.push(at)
      await new Promise<void>(() => {})
      return at
    })
    void limited(tasks, 1)
    await Promise.resolve()
    expect(started).toEqual([0])
  })

  it('answers nothing for no tasks', async () => {
    expect(await limited([])).toEqual([])
  })
})
