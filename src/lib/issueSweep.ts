import type {
  IssueInfo_Serialize as IssueInfo,
  Lists_Serialize as Lists,
  SourceInfo_Serialize as SourceInfo,
  Sources_Serialize as Sources,
} from './bindings'

export type Wanted = { project: number; source: string; repository: boolean }

export type Swept = { wanted: Wanted; lists: Lists }

export type Found = IssueInfo & { project: number; repository: boolean }

export type Failed = { source: string; error: string }

export const CAP = 100

export const LIMIT = 4

const connectedIn = (answer: Sources) => answer.sources.filter((tracker) => tracker.state === 'connected')

const firstBy = <T>(items: T[], keyOf: (item: T) => string) => {
  const seen = new Set<string>()
  return items.filter((item) => {
    const key = keyOf(item)
    const fresh = !seen.has(key)
    seen.add(key)
    return fresh
  })
}

const everyWanted = (answers: Sources[]): Wanted[] =>
  answers.flatMap((answer) =>
    connectedIn(answer).map((tracker) => ({
      project: answer.project,
      source: tracker.source,
      repository: tracker.repository,
    })),
  )

const oneKey = (wanted: Wanted) =>
  wanted.repository ? `${wanted.source}\n${wanted.project}` : wanted.source

export const toList = (answers: Sources[], source: string | null): Wanted[] =>
  firstBy(everyWanted(answers), oneKey).filter((wanted) => source === null || wanted.source === source)

export const tracked = (answers: Sources[]): SourceInfo[] =>
  firstBy(answers.flatMap(connectedIn), (tracker) => tracker.source)

const inRepository = (tracker: SourceInfo) => !tracker.missing.includes('repository')

const closer = (one: SourceInfo, other: SourceInfo) =>
  inRepository(one) === inRepository(other) ? one.missing.length < other.missing.length : inRepository(one)

const nearest = (trackers: SourceInfo[], source: string) =>
  trackers
    .filter((tracker) => tracker.source === source)
    .reduce((best, tracker) => (closer(tracker, best) ? tracker : best))

export const connect = (answers: Sources[]): SourceInfo[] | null => {
  const every = answers.flatMap((answer) => answer.sources)
  if (every.some((tracker) => tracker.state === 'connected')) {
    return null
  }
  return firstBy(every, (tracker) => tracker.source).map((tracker) => nearest(every, tracker.source))
}

const sameIssue = (found: Found) =>
  found.url !== '' ? found.url : `${found.source}\n${found.key}\n${found.project}`

const newestFirst = (one: Found, other: Found) =>
  one.updated_at === other.updated_at ? 0 : one.updated_at > other.updated_at ? -1 : 1

export const gathered = (swept: Swept[]): Found[] => {
  const found = swept.flatMap(({ wanted, lists }) =>
    lists.issues.map((issue) => ({ ...issue, project: wanted.project, repository: wanted.repository })),
  )
  return firstBy(found, sameIssue).sort(newestFirst)
}

const listedIn = (swept: Swept[]) => swept.flatMap(({ lists }) => lists.sources)

export const failed = (swept: Swept[]): Failed[] =>
  firstBy(
    listedIn(swept).flatMap(({ source, error }) => (error === null ? [] : [{ source, error }])),
    (failure) => failure.source,
  )

export const capped = (swept: Swept[]) =>
  listedIn(swept).some((listed) => listed.error === null && listed.count >= CAP)

export const limited = async <T>(tasks: (() => Promise<T>)[], limit = LIMIT): Promise<T[]> => {
  const results: T[] = []
  let next = 0
  const work = async () => {
    while (next < tasks.length) {
      const at = next++
      results[at] = await tasks[at]()
    }
  }
  await Promise.all(Array.from({ length: Math.min(limit, tasks.length) }, work))
  return results
}
