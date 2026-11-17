import { describe, expect, it, vi } from 'vitest'
import type {
  ChangedFile_Serialize as ChangedFile,
  ChangesChanged_Serialize as ChangesChanged,
  CommandError,
  Patch_Serialize as Patch,
  Summary_Serialize as Summary,
  Toast,
} from './bindings'
import { ChangesState, CLOSED, goneMessage, type Calls } from './changes.svelte'

const MACHINE = 1
const WORKSPACE = 2

const file = (path: string, digest: string, more: Partial<ChangedFile> = {}): ChangedFile => ({
  path,
  old_path: null,
  status: 'modified',
  added: 3,
  removed: 1,
  fold: 'open',
  viewed: false,
  staged: null,
  digest,
  ...more,
})

const summary = (files: ChangedFile[], more: Partial<Summary> = {}): Summary => ({
  boot: 'b1',
  stream: 1,
  revision: 1,
  scope: 'workspace',
  mode: 'uncommitted',
  state: 'ready',
  error: null,
  workspaces: [{ project: 4, workspace: WORKSPACE, base: 'main', added: 3, removed: 1, files }],
  ...more,
})

const told = (listed: Summary | null, more: Partial<ChangesChanged> = {}): ChangesChanged => ({
  machine: MACHINE,
  workspace: WORKSPACE,
  mode: 'uncommitted',
  summary: listed,
  closed: false,
  ...more,
})

const patch = (path: string, digest: string): Patch => ({
  ...file(path, digest),
  workspace: WORKSPACE,
  mode: 'uncommitted',
  hunks: [],
})

const done = <T>(data: T) => Promise.resolve({ status: 'ok' as const, data })

const refusal = (message: string, code: CommandError['code'] = 'failed') =>
  Promise.resolve({ status: 'error' as const, error: { code, message } })

const later = <T>() => {
  let answer: (value: T) => void = () => {}
  const promise = new Promise<T>((resolve) => (answer = resolve))
  return { promise, answer }
}

type Read = Awaited<ReturnType<Calls['read']>>
type Followed = Awaited<ReturnType<Calls['follow']>>
type Marked = Awaited<ReturnType<Calls['mark']>>

const settle = () => new Promise((resolve) => setTimeout(resolve))

const fakes = (more: Partial<Calls> = {}) => ({
  follow: vi.fn<Calls['follow']>(more.follow ?? (() => done(null))),
  unfollow: vi.fn<Calls['unfollow']>(more.unfollow ?? (() => Promise.resolve())),
  read: vi.fn<Calls['read']>(more.read ?? ((_, path) => done(patch(path, `patch of ${path}`)))),
  mark: vi.fn<Calls['mark']>(more.mark ?? (() => done(null))),
})

const following = async (more: Partial<Calls> = {}, files = [file('src/a.rs', 'a1'), file('src/b.rs', 'b1')]) => {
  const calls = fakes(more)
  const tell = vi.fn<(toast: Toast) => void>()
  const changes = new ChangesState(MACHINE, WORKSPACE, calls, tell)
  await changes.start()
  changes.see(told(summary(files)))
  return { changes, calls, tell }
}

const paths = (changes: ChangesState) => changes.files.map((shown) => `${shown.path} ${shown.viewed}`)

describe('ChangesState, its list', () => {
  it('follows its workspace in the mode shown, and stops when left', async () => {
    const { changes, calls } = await following()
    changes.stop()
    expect([calls.follow.mock.calls, calls.unfollow.mock.calls]).toEqual([[[WORKSPACE, 'uncommitted']], [[WORKSPACE]]])
  })

  it('takes each summary of its workspace in place of the last', async () => {
    const { changes } = await following()
    changes.see(told(summary([file('src/c.rs', 'c1', { viewed: true })])))
    expect([paths(changes), changes.listed?.base]).toEqual([['src/c.rs true'], 'main'])
  })

  it.each<[string, Partial<ChangesChanged>]>([
    ['another machine', { machine: 9 }],
    ['another workspace', { workspace: 9 }],
    ['another mode', { mode: 'commits' }],
  ])('ignores the summary of %s', async (_, more) => {
    const { changes } = await following()
    changes.see(told(summary([]), more))
    expect(paths(changes)).toEqual(['src/a.rs false', 'src/b.rs false'])
  })

  it('follows again in the new mode, forgetting the list of the old one', async () => {
    const { changes, calls } = await following()
    await changes.show('commits')
    changes.see(told(summary([file('src/old.rs', 'o1')])))
    expect([changes.mode, changes.summary, calls.follow.mock.calls]).toEqual([
      'commits',
      null,
      [
        [WORKSPACE, 'uncommitted'],
        [WORKSPACE, 'commits'],
      ],
    ])
    changes.see(told(summary([file('src/new.rs', 'n1')]), { mode: 'commits' }))
    expect(paths(changes)).toEqual(['src/new.rs false'])
  })

  it('does not follow again for the mode already shown', async () => {
    const { changes, calls } = await following()
    await changes.show('uncommitted')
    expect(calls.follow).toHaveBeenCalledTimes(1)
  })

  it('ignores what comes once left', async () => {
    const { changes } = await following()
    changes.stop()
    changes.see(told(summary([])))
    expect(changes.files).toHaveLength(2)
  })

  it('never follows once left, as when the screen goes before its listener is in place', async () => {
    const calls = fakes()
    const changes = new ChangesState(MACHINE, WORKSPACE, calls, vi.fn())
    changes.stop()
    await changes.start()
    expect([calls.follow.mock.calls, calls.unfollow.mock.calls]).toEqual([[], [[WORKSPACE]]])
  })

  it('unfollows again when a follow answers after the screen left', async () => {
    const followed = later<Followed>()
    const calls = fakes({ follow: () => followed.promise })
    const changes = new ChangesState(MACHINE, WORKSPACE, calls, vi.fn())
    const starting = changes.start()
    await settle()
    changes.stop()
    followed.answer({ status: 'ok', data: null })
    await starting
    expect([calls.follow.mock.calls, calls.unfollow.mock.calls]).toEqual([
      [[WORKSPACE, 'uncommitted']],
      [[WORKSPACE], [WORKSPACE]],
    ])
  })

  it('follows one mode after the other, skipping those switched away from before their turn', async () => {
    const commits = later<Followed>()
    const { changes, calls } = await following({ follow: (_, mode) => (mode === 'commits' ? commits.promise : done(null)) })
    const switches = [changes.show('commits')]
    await settle()
    switches.push(changes.show('all'), changes.show('uncommitted'))
    await settle()
    const sent = calls.follow.mock.calls.map(([, mode]) => mode)
    commits.answer({ status: 'ok', data: null })
    await Promise.all(switches)
    expect([sent, calls.follow.mock.calls.map(([, mode]) => mode)]).toEqual([
      ['uncommitted', 'commits'],
      ['uncommitted', 'commits', 'uncommitted'],
    ])
  })

  it.each<[string, Partial<Calls>, string]>([
    ['a refused follow', { follow: () => refusal('no machine is connected', 'not_connected') }, 'no machine is connected'],
    ['a rejected follow', { follow: () => Promise.reject(new Error('IPC is gone')) }, 'IPC is gone'],
    ['a rejected unfollow', { unfollow: () => Promise.reject(new Error('no window')) }, 'no window'],
  ])('tells of %s', async (_, more, message) => {
    const { changes, tell } = await following(more)
    changes.stop()
    await settle()
    expect(tell.mock.calls).toEqual([[{ level: 'error', message }]])
  })

  it('says the workspace was closed on the machine, and takes nothing more', async () => {
    const { changes, calls, tell } = await following()
    await changes.open('src/a.rs')
    changes.see(told(null, { closed: true }))
    changes.see(told(summary([])))
    await changes.mark('src/a.rs', true)
    expect([changes.gone, changes.diff, changes.files.length]).toEqual([true, null, 2])
    expect([tell.mock.calls, calls.mark.mock.calls]).toEqual([[[{ level: 'info', message: CLOSED }]], []])
  })
})

describe('ChangesState, its viewed marks', () => {
  it('shows a mark at once, before its answer comes', async () => {
    const answer = later<Marked>()
    const { changes } = await following({ mark: () => answer.promise })
    const marking = changes.mark('src/a.rs', true)
    expect(paths(changes)).toEqual(['src/a.rs true', 'src/b.rs false'])
    answer.answer({ status: 'ok', data: null })
    await marking
    expect(changes.viewed('src/a.rs')).toBe(true)
  })

  it('marks with the digest of the version seen, and clears with the mode and no digest', async () => {
    const { changes, calls } = await following()
    await changes.mark('src/b.rs', true)
    await changes.open('src/a.rs')
    await changes.mark('src/a.rs', true)
    await changes.mark('src/a.rs', false)
    const sent = { workspace: WORKSPACE, mode: 'uncommitted' }
    expect(calls.mark.mock.calls).toEqual([
      [{ ...sent, path: 'src/b.rs', viewed: true, digest: 'b1' }],
      [{ ...sent, path: 'src/a.rs', viewed: true, digest: 'patch of src/a.rs' }],
      [{ ...sent, path: 'src/a.rs', viewed: false, digest: null }],
    ])
  })

  it.each<[string, Partial<Calls>, string]>([
    ['refused', { mark: () => refusal('no file src/a.rs') }, 'no file src/a.rs'],
    ['skipped', { mark: () => refusal('nothing was sent', 'not_sent') }, 'nothing was sent'],
    ['rejected', { mark: () => Promise.reject(new Error('IPC is gone')) }, 'IPC is gone'],
  ])('goes back and tells why when the mark is %s', async (_, more, message) => {
    const { changes, tell } = await following(more, [file('src/a.rs', 'a1', { viewed: true })])
    await changes.mark('src/a.rs', false)
    expect([paths(changes), tell.mock.calls]).toEqual([['src/a.rs true'], [[{ level: 'error', message }]]])
  })

  it('keeps a sent mark until the next summary, which decides', async () => {
    const { changes } = await following()
    await changes.mark('src/a.rs', true)
    expect(changes.viewed('src/a.rs')).toBe(true)
    changes.see(told(summary([file('src/a.rs', 'a2')])))
    expect(paths(changes)).toEqual(['src/a.rs false'])
  })

  it('keeps a mark on its way through a summary from before it', async () => {
    const answer = later<Marked>()
    const { changes } = await following({ mark: () => answer.promise })
    const marking = changes.mark('src/a.rs', true)
    changes.see(told(summary([file('src/a.rs', 'a1')])))
    expect(paths(changes)).toEqual(['src/a.rs true'])
    answer.answer({ status: 'ok', data: null })
    await marking
    changes.see(told(summary([file('src/a.rs', 'a1', { viewed: true })])))
    expect(paths(changes)).toEqual(['src/a.rs true'])
  })

  it('leaves a later mark alone when an earlier one fails', async () => {
    const first = later<Marked>()
    const mark = vi.fn<Calls['mark']>().mockReturnValueOnce(first.promise).mockReturnValue(done(null))
    const { changes, tell } = await following({ mark })
    const marking = changes.mark('src/a.rs', true)
    await changes.mark('src/a.rs', false)
    first.answer({ status: 'error', error: { code: 'unanswered', message: 'no answer came back' } })
    await marking
    expect([changes.viewed('src/a.rs'), tell.mock.calls]).toEqual([
      false,
      [[{ level: 'error', message: 'no answer came back' }]],
    ])
  })
})

describe('ChangesState, its open diff', () => {
  it('loads the patch of the file in the mode shown', async () => {
    const { changes, calls } = await following()
    const opening = changes.open('src/a.rs')
    expect(changes.diff).toEqual({ path: 'src/a.rs', reading: { kind: 'loading' } })
    await opening
    expect(changes.diff).toEqual({ path: 'src/a.rs', reading: { kind: 'patch', patch: patch('src/a.rs', 'patch of src/a.rs') } })
    expect(calls.read.mock.calls).toEqual([[WORKSPACE, 'src/a.rs', 'uncommitted']])
  })

  it.each<[string, Calls['read'], string]>([
    ['refused', () => refusal('no file src/a.rs'), 'no file src/a.rs'],
    ['rejected', () => Promise.reject(new Error('IPC is gone')), 'IPC is gone'],
  ])('shows why a read was %s', async (_, read, reason) => {
    const { changes } = await following({ read })
    await changes.open('src/a.rs')
    expect(changes.diff?.reading).toEqual({ kind: 'failed', reason })
  })

  it('drops the late answer of an earlier read', async () => {
    const first = later<Read>()
    const read = vi.fn<Calls['read']>().mockReturnValueOnce(first.promise).mockReturnValue(done(patch('src/b.rs', 'b9')))
    const { changes } = await following({ read })
    const opening = changes.open('src/a.rs')
    await changes.open('src/b.rs')
    first.answer({ status: 'ok', data: patch('src/a.rs', 'a9') })
    await opening
    expect(changes.diff).toEqual({ path: 'src/b.rs', reading: { kind: 'patch', patch: patch('src/b.rs', 'b9') } })
  })

  it('drops the answer of a read once the diff is closed', async () => {
    const first = later<Read>()
    const { changes } = await following({ read: () => first.promise })
    const opening = changes.open('src/a.rs')
    changes.closeDiff()
    first.answer({ status: 'ok', data: patch('src/a.rs', 'a9') })
    await opening
    expect(changes.diff).toBeNull()
  })

  it('reads again when the digest of its file changes, and only then', async () => {
    const { changes, calls } = await following()
    await changes.open('src/a.rs')
    changes.see(told(summary([file('src/a.rs', 'a1'), file('src/b.rs', 'b2')])))
    changes.see(told(summary([file('src/a.rs', 'a2'), file('src/b.rs', 'b2')])))
    await settle()
    expect(calls.read.mock.calls.map(([, path]) => path)).toEqual(['src/a.rs', 'src/a.rs'])
  })

  it('warns and closes when its file is no longer changed, dropping the read on its way', async () => {
    const answer = later<Read>()
    const read = vi.fn<Calls['read']>().mockReturnValueOnce(done(patch('src/a.rs', 'a1'))).mockReturnValue(answer.promise)
    const { changes, tell } = await following({ read })
    await changes.open('src/a.rs')
    changes.see(told(summary([file('src/a.rs', 'a2')])))
    changes.see(told(summary([file('src/b.rs', 'b1')])))
    answer.answer({ status: 'ok', data: patch('src/a.rs', 'a2') })
    await settle()
    expect([changes.diff, tell.mock.calls]).toEqual([
      { path: 'src/a.rs', reading: { kind: 'gone' } },
      [[{ level: 'info', message: goneMessage('src/a.rs') }]],
    ])
  })

  it('keeps the diff through a summary that failed to list the files', async () => {
    const { changes, tell } = await following()
    await changes.open('src/a.rs')
    changes.see(told(summary([], { state: 'failed', error: 'git failed' })))
    expect([changes.diff?.reading.kind, tell.mock.calls]).toEqual(['patch', []])
  })

  it('closes the diff when the mode changes', async () => {
    const { changes } = await following()
    await changes.open('src/a.rs')
    await changes.show('all')
    expect(changes.diff).toBeNull()
  })
})
