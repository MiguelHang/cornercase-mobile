import { describe, expect, it, vi } from 'vitest'
import { AddMachine, send, type MachineCalls } from './addMachine.svelte'
import type { CommandError, MatchFound, MergeView, SameWhy } from './bindings'
import { EMPTY, NAME_TAKEN, NAME_TOO_LONG, NO_TOKEN } from './machineForm'

const URL_TEXT = 'ws://studio.lan:9000/?token=abc'

const CODE = 'ws://192.168.1.20:9000/?token=abc&also=100.101.102.103&name=Mac%20Studio'

const MERGED: MergeView = { added: ['ws://100.101.102.103:9000/'], token_updated: true, skipped: [] }

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
  machineMerge: vi.fn(overrides.machineMerge ?? (() => ok(MERGED))),
  machineFormClosed: vi.fn(overrides.machineFormClosed ?? (() => Promise.resolve())),
})

const form = (overrides: Partial<MachineCalls> = {}, url = URL_TEXT, name = '') => {
  const onadded = vi.fn()
  const onrouted = vi.fn()
  const tell = vi.fn()
  const commands = calls(overrides)
  const draft = new AddMachine(commands, onadded, onrouted, tell)
  draft.url = url
  draft.name = name
  return { draft, onadded, onrouted, tell, commands }
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

describe('AddMachine with a machine code', () => {
  it('names the machine after the code and sends the code whole', async () => {
    const { draft, commands } = form({}, CODE)
    await draft.submit()
    expect([commands.machineMatch.mock.calls, commands.machineAdd.mock.calls]).toEqual([
      [[CODE, 'Mac Studio']],
      [[CODE, 'Mac Studio']],
    ])
  })

  it('names it after the host when another machine has the name of the code, compared by that name', async () => {
    const { draft, commands } = form({}, CODE)
    await draft.submit(['Desk', 'mac studio'])
    expect([commands.machineMatch.mock.calls, commands.machineAdd.mock.calls]).toEqual([
      [[CODE, 'Mac Studio']],
      [[CODE, '192.168.1.20']],
    ])
  })

  it('checks the names taken on a scan too', async () => {
    const { draft, commands } = form({}, '')
    await draft.fill(CODE, ['Mac Studio'])
    expect([commands.machineMatch.mock.calls, commands.machineAdd.mock.calls]).toEqual([
      [[CODE, 'Mac Studio']],
      [[CODE, '192.168.1.20']],
    ])
  })

  it('offers the machine of the same name and keeps the host for a new one', async () => {
    const { draft } = form({ machineMatch: () => same('name') }, CODE)
    await draft.submit(['Mac Studio'])
    expect([draft.offer?.why, draft.offer?.name]).toEqual(['name', '192.168.1.20'])
  })

  it('compares by the name typed when there is one', async () => {
    const { draft, commands } = form({}, CODE, 'Desk')
    await draft.submit(['Mac Studio'])
    expect(commands.machineMatch.mock.calls).toEqual([[CODE, 'Desk']])
  })
})

describe('AddMachine with a machine it can merge into', () => {
  const merging = (overrides: Partial<MachineCalls> = {}) =>
    form({ machineMatch: () => matched({ kind: 'merge', machine: 3, name: 'MacBook' }), ...overrides }, CODE)

  it('merges the code into it, tells what changed and hands it over as routed', async () => {
    const { draft, onadded, onrouted, tell, commands } = merging()
    await draft.submit()
    expect([commands.machineMerge.mock.calls, commands.machineAdd.mock.calls]).toEqual([[[3, CODE]], []])
    expect(tell.mock.calls).toEqual([[{ level: 'info', message: 'MacBook already saved: 1 route added, token updated.' }]])
    expect([onrouted.mock.calls, onadded.mock.calls, draft.offer, draft.formError]).toEqual([[[3]], [], null, null])
  })

  it('tells that there was nothing to add', async () => {
    const { draft, onrouted, tell } = merging({
      machineMerge: () => ok({ added: [], token_updated: false, skipped: [] }),
    })
    await draft.submit()
    expect(tell.mock.calls).toEqual([[{ level: 'info', message: 'MacBook already saved: nothing to add.' }]])
    expect(onrouted.mock.calls).toEqual([[3]])
  })

  it('stays busy from checking until the machine is handed over', async () => {
    const pending = gate()
    const { draft, onrouted } = merging({ machineMerge: () => pending.promise.then(() => ok(MERGED)) })
    const sent = draft.submit()
    expect(draft.busy).toBe(true)
    pending.open(3)
    await sent
    expect([onrouted.mock.calls, draft.busy]).toEqual([[[3]], true])
  })

  it('puts a failed merge under the form and tells nothing', async () => {
    const failing = { code: 'secrets', message: 'the keychain is locked' } as const
    const { draft, onrouted, tell } = merging({ machineMerge: () => refused(failing) })
    await draft.submit()
    expect([draft.formError, draft.busy, draft.offer]).toEqual([failing.message, false, null])
    expect([onrouted, tell].map((mock) => mock.mock.calls.length)).toEqual([0, 0])
  })
})

describe('AddMachine offering the same machine', () => {
  it.each<SameWhy>(['address', 'boot', 'name'])('offers it when it matches by %s, holding the form', async (why) => {
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

  it('adds the URL as a route of that machine and says so, without calling it added', async () => {
    const { draft, onadded, onrouted, tell, commands } = await offered('boot')
    await draft.addRoute()
    expect([commands.machineMerge.mock.calls, commands.machineAdd.mock.calls]).toEqual([[[3, URL_TEXT]], []])
    expect([onrouted.mock.calls, onadded.mock.calls]).toEqual([[[3]], []])
    expect(tell.mock.calls).toEqual([[{ level: 'info', message: 'Studio already saved: 1 route added, token updated.' }]])
  })

  it("says so when the address it offered is another machine's", async () => {
    const elsewhere = { added: [], token_updated: false, skipped: ['ws://studio.lan:9000/'] }
    const { draft, tell } = form({ machineMatch: () => same('boot'), machineMerge: () => ok(elsewhere) })
    await draft.submit()
    await draft.addRoute()
    const message = 'Studio already saved: 1 route left to another machine.'
    expect(tell.mock.calls).toEqual([[{ level: 'info', message }]])
  })

  it('adds the whole code to that machine', async () => {
    const { draft, commands } = form({ machineMatch: () => same('boot') }, CODE)
    await draft.submit()
    await draft.addRoute()
    expect(commands.machineMerge.mock.calls).toEqual([[3, CODE]])
  })

  it('adds a new machine instead under the name checked', async () => {
    const { draft, onadded, onrouted, commands } = await offered('boot', ' Desk ')
    await draft.addNew()
    expect([commands.machineAdd.mock.calls, commands.machineMerge.mock.calls]).toEqual([[[URL_TEXT, 'Desk']], []])
    expect([onadded.mock.calls, onrouted.mock.calls]).toEqual([[[1]], []])
  })

  it('asks for another name for a new machine when it matched by name, without sending', async () => {
    const { draft, commands } = await offered('name', 'studio')
    await draft.addNew()
    expect([draft.nameError, draft.offer, draft.busy]).toEqual([NAME_TAKEN, null, false])
    expect(commands.machineAdd).not.toHaveBeenCalled()
  })

  it('drops the offer and puts the error in its place when adding the route fails', async () => {
    const failing = { code: 'secrets', message: 'the keychain is locked' } as const
    const { draft, onrouted } = form({ machineMatch: () => same('address'), machineMerge: () => refused(failing) })
    await draft.submit()
    await draft.addRoute()
    expect([draft.formError, draft.offer, draft.busy]).toEqual([failing.message, null, false])
    expect(onrouted).not.toHaveBeenCalled()
  })

  it('goes back to the form on Back, and ignores the choices without an offer', async () => {
    const { draft, commands } = await offered('boot')
    draft.back()
    await draft.addRoute()
    await draft.addNew()
    expect([draft.offer, draft.held]).toEqual([null, false])
    expect([commands.machineMerge, commands.machineAdd].map((mock) => mock.mock.calls.length)).toEqual([0, 0])
  })

  it('keeps the secret on Back, for the same code sent again, and forgets it as the form closes', async () => {
    const { draft, commands } = await offered('boot')
    draft.back()
    expect(commands.machineFormClosed).not.toHaveBeenCalled()
    await draft.close()
    expect(commands.machineFormClosed).toHaveBeenCalledOnce()
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
    expect([commands.machineAdd.mock.calls.length, commands.machineMerge.mock.calls.length]).toEqual([1, 0])
  })

  it('ignores a submit or a scan while the offer is up', async () => {
    const { draft, commands } = await offered('boot')
    await draft.submit()
    await draft.fill('ws://other.lan:9000/?token=xyz')
    expect([draft.url, commands.machineMatch.mock.calls.length]).toEqual([URL_TEXT, 1])
  })
})
