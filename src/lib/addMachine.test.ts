import { describe, expect, it, vi } from 'vitest'
import { AddMachine, send, type MachineCalls } from './addMachine.svelte'
import type { CommandError, MatchFound, SameWhy } from './bindings'
import { ALREADY_SAVED, EMPTY, NAME_TAKEN, NAME_TOO_LONG, NO_TOKEN } from './machineForm'

const URL_TEXT = 'ws://studio.lan:9000/?token=abc'

const ok = <T>(data: T) => Promise.resolve({ status: 'ok' as const, data })

const added = (machine: number) => ok(machine)

const refused = (error: CommandError) => Promise.resolve({ status: 'error' as const, error })

const matched = (found: MatchFound, checked = true) => ok({ found, checked })

const same = (why: SameWhy, checked = true) => matched({ kind: 'same', machine: 3, name: 'Studio', why }, checked)

const gate = () => {
  let open: (machine: number) => void = () => {}
  const promise = new Promise<number>((resolve) => (open = resolve))
  return { promise: promise.then((machine) => ({ status: 'ok' as const, data: machine })), open }
}

const calls = (overrides: Partial<MachineCalls> = {}) => ({
  machineAdd: vi.fn(overrides.machineAdd ?? (() => added(1))),
  machineMatch: vi.fn(overrides.machineMatch ?? (() => matched({ kind: 'none' }))),
  machineRouteAdd: vi.fn(overrides.machineRouteAdd ?? (() => ok(null))),
})

const form = (overrides: Partial<MachineCalls> = {}, url = URL_TEXT, name = '') => {
  const onadded = vi.fn()
  const commands = calls(overrides)
  const draft = new AddMachine(commands, onadded)
  draft.url = url
  draft.name = name
  return { draft, onadded, commands }
}

const offered = async (why: SameWhy, name = '') => {
  const made = form({ machineMatch: () => same(why) }, URL_TEXT, name)
  await made.draft.submit()
  return made
}

describe('send', () => {
  it('hands the answer over', async () => {
    expect(await send(() => added(7))).toEqual({ kind: 'done', data: 7 })
  })

  it.each<[string, CommandError, string]>([
    ['a route error under the URL', { code: 'route_empty_token', message: 'the token is empty' }, 'url'],
    ['a name error under the name', { code: 'name_taken', message: 'the name is taken' }, 'name'],
    ['any other under the form', { code: 'store', message: 'the disk is full' }, 'form'],
  ])('puts %s', async (_, error, kind) => {
    expect(await send(() => refused(error))).toEqual({ kind, error: error.message })
  })

  it.each<[string, unknown, string]>([
    ['an error', new Error('IPC is gone'), 'IPC is gone'],
    ['an object with a message', { message: 'no window' }, 'no window'],
    ['anything else', 'refused', 'refused'],
  ])('tells a rejected call with %s under the form by its message', async (_, error, message) => {
    expect(await send(() => Promise.reject(error))).toEqual({ kind: 'form', error: message })
  })
})

describe('AddMachine', () => {
  it('checks the trimmed URL under the name typed, or the host without one, then adds it', async () => {
    const first = form({}, ` ${URL_TEXT}\n`, ' Studio ')
    await first.draft.submit()
    const second = form({}, URL_TEXT)
    await second.draft.submit()
    expect([first.commands.machineMatch.mock.calls, second.commands.machineMatch.mock.calls]).toEqual([
      [[URL_TEXT, 'Studio']],
      [[URL_TEXT, 'studio.lan']],
    ])
    expect([first.commands.machineAdd.mock.calls, second.commands.machineAdd.mock.calls]).toEqual([
      [[URL_TEXT, 'Studio']],
      [[URL_TEXT, 'studio.lan']],
    ])
  })

  it('sends a URL filled in from a scan the same way as a pasted one', async () => {
    const { draft, onadded, commands } = form({ machineAdd: () => added(2) }, '', 'Studio')
    await draft.fill(URL_TEXT)
    expect([draft.url, commands.machineAdd.mock.calls, onadded.mock.calls]).toEqual([
      URL_TEXT,
      [[URL_TEXT, 'Studio']],
      [[2]],
    ])
  })

  it('leaves the URL alone when a scan ends while adding', async () => {
    const pending = gate()
    const { draft, commands } = form({ machineAdd: () => pending.promise })
    const sent = draft.submit()
    await draft.fill('ws://other.lan:9000/?token=xyz')
    pending.open(3)
    await sent
    expect([draft.url, commands.machineAdd.mock.calls]).toEqual([URL_TEXT, [[URL_TEXT, 'studio.lan']]])
  })

  it.each([
    ['empty', '', EMPTY],
    ['without its token', 'ws://studio.lan:9000/', NO_TOKEN],
  ])('puts the problem of a URL %s under it without sending it', async (_, url, error) => {
    const { draft, commands } = form({}, url)
    await draft.submit()
    expect([draft.urlError, draft.nameError, draft.formError, draft.busy]).toEqual([error, null, null, false])
    expect(commands.machineMatch).not.toHaveBeenCalled()
  })

  it('puts a name too long under it without sending anything', async () => {
    const { draft, commands } = form({}, URL_TEXT, 'x'.repeat(65))
    await draft.submit()
    expect([draft.urlError, draft.nameError, draft.busy]).toEqual([null, NAME_TOO_LONG, false])
    expect(commands.machineMatch).not.toHaveBeenCalled()
  })

  it('stays busy from checking until the machine is handed over, and after', async () => {
    const pending = gate()
    const { draft, onadded } = form({ machineAdd: () => pending.promise })
    const sent = draft.submit()
    expect(draft.busy).toBe(true)
    pending.open(4)
    await sent
    expect(onadded.mock.calls).toEqual([[4]])
    expect(draft.busy).toBe(true)
  })

  it('ignores a second submit while busy, also once added', async () => {
    const pending = gate()
    const { draft, onadded, commands } = form({ machineAdd: () => pending.promise })
    const first = draft.submit()
    await draft.submit()
    pending.open(4)
    await first
    await draft.submit()
    expect(commands.machineMatch).toHaveBeenCalledTimes(1)
    expect(onadded).toHaveBeenCalledTimes(1)
  })

  it.each<[string, CommandError, [string | null, string | null, string | null]]>([
    ['under the URL', { code: 'route_scheme', message: 'only ws://' }, ['only ws://', null, null]],
    ['under the name', { code: 'name_taken', message: NAME_TAKEN }, [null, NAME_TAKEN, null]],
    ['under the form', { code: 'store', message: 'the disk is full' }, [null, null, 'the disk is full']],
  ])('rests again after the core refuses to add and puts it %s', async (_, error, shown) => {
    const { draft, onadded } = form({ machineAdd: () => refused(error) })
    await draft.submit()
    expect([draft.urlError, draft.nameError, draft.formError, draft.busy]).toEqual([...shown, false])
    expect(onadded).not.toHaveBeenCalled()
  })

  it('puts a route error of the check under the URL without adding', async () => {
    const { draft, commands } = form({ machineMatch: () => refused({ code: 'route_port', message: 'bad port' }) })
    await draft.submit()
    expect([draft.urlError, draft.busy, draft.offer]).toEqual(['bad port', false, null])
    expect(commands.machineAdd).not.toHaveBeenCalled()
  })

  it('clears the old problems on a new submit', async () => {
    const add = vi.fn<MachineCalls['machineAdd']>(() => refused({ code: 'store', message: 'the disk is full' }))
    const { draft } = form({ machineAdd: add }, '')
    await draft.submit()
    draft.url = URL_TEXT
    await draft.submit()
    expect([draft.urlError, draft.formError]).toEqual([null, 'the disk is full'])
    add.mockImplementation(() => added(2))
    await draft.submit()
    expect([draft.urlError, draft.nameError, draft.formError]).toEqual([null, null, null])
  })
})

describe('AddMachine with a route already saved', () => {
  it('says so and adds nothing', async () => {
    const { draft, onadded, commands } = form({ machineMatch: () => matched({ kind: 'already_saved', machine: 3 }) })
    await draft.submit()
    expect([draft.formError, draft.offer, draft.busy]).toEqual([ALREADY_SAVED, null, false])
    expect([commands.machineAdd, commands.machineRouteAdd, onadded].map((mock) => mock.mock.calls.length)).toEqual([
      0, 0, 0,
    ])
  })
})

describe('AddMachine offering the same machine', () => {
  it.each<SameWhy>(['address', 'token', 'boot', 'name'])('offers it when it matches by %s, holding the form', async (why) => {
    const { draft, commands } = await offered(why)
    expect(draft.offer).toEqual({ url: URL_TEXT, name: 'studio.lan', machine: 3, saved: 'Studio', why, checked: true })
    expect([draft.busy, draft.held]).toEqual([false, true])
    expect(commands.machineAdd).not.toHaveBeenCalled()
  })

  it('keeps whether the bridge answered the check', async () => {
    const { draft } = form({ machineMatch: () => same('name', false) })
    await draft.submit()
    expect(draft.offer?.checked).toBe(false)
  })

  it('adds the URL as a route of that machine and hands it over', async () => {
    const { draft, onadded, commands } = await offered('token')
    await draft.addRoute()
    expect([commands.machineRouteAdd.mock.calls, commands.machineAdd.mock.calls]).toEqual([[[3, URL_TEXT]], []])
    expect(onadded.mock.calls).toEqual([[3]])
  })

  it('adds a new machine instead under the name checked', async () => {
    const { draft, onadded, commands } = await offered('boot', ' Desk ')
    await draft.addNew()
    expect([commands.machineAdd.mock.calls, commands.machineRouteAdd.mock.calls]).toEqual([[[URL_TEXT, 'Desk']], []])
    expect(onadded.mock.calls).toEqual([[1]])
  })

  it('asks for another name for a new machine when it matched by name, without sending', async () => {
    const { draft, commands } = await offered('name', 'studio')
    await draft.addNew()
    expect([draft.nameError, draft.offer, draft.busy]).toEqual([NAME_TAKEN, null, false])
    expect(commands.machineAdd).not.toHaveBeenCalled()
  })

  it('drops the offer and puts the error in its place when adding the route fails', async () => {
    const failing = { code: 'secrets', message: 'the keychain is locked' } as const
    const { draft, onadded } = form({ machineMatch: () => same('address'), machineRouteAdd: () => refused(failing) })
    await draft.submit()
    await draft.addRoute()
    expect([draft.formError, draft.offer, draft.busy]).toEqual([failing.message, null, false])
    expect(onadded).not.toHaveBeenCalled()
  })

  it('goes back to the form on Back, and ignores the choices without an offer', async () => {
    const { draft, commands } = await offered('token')
    draft.back()
    await draft.addRoute()
    await draft.addNew()
    expect([draft.offer, draft.held]).toEqual([null, false])
    expect([commands.machineRouteAdd, commands.machineAdd].map((mock) => mock.mock.calls.length)).toEqual([0, 0])
  })

  it('takes one choice at a time and holds Back while it runs', async () => {
    const pending = gate()
    const { draft, commands } = await offered('boot')
    commands.machineAdd.mockImplementation(() => pending.promise)
    const chosen = draft.addNew()
    await draft.addRoute()
    await draft.addNew()
    draft.back()
    expect(draft.offer).not.toBeNull()
    pending.open(6)
    await chosen
    expect([commands.machineAdd.mock.calls.length, commands.machineRouteAdd.mock.calls.length]).toEqual([1, 0])
  })

  it('ignores a submit or a scan while the offer is up', async () => {
    const { draft, commands } = await offered('token')
    await draft.submit()
    await draft.fill('ws://other.lan:9000/?token=xyz')
    expect([draft.url, commands.machineMatch.mock.calls.length]).toEqual([URL_TEXT, 1])
  })
})
