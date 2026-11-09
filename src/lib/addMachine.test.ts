import { describe, expect, it, vi } from 'vitest'
import { AddMachine, send, type MachineAdd } from './addMachine.svelte'
import type { CommandError } from './bindings'
import { EMPTY, NO_TOKEN } from './machineForm'

const URL_TEXT = 'ws://studio.lan:9000/?token=abc'

const added = (machine: number) => Promise.resolve({ status: 'ok' as const, data: machine })

const refused = (error: CommandError) => Promise.resolve({ status: 'error' as const, error })

const gate = () => {
  let open: (machine: number) => void = () => {}
  const promise = new Promise<number>((resolve) => (open = resolve))
  return { promise: promise.then((machine) => ({ status: 'ok' as const, data: machine })), open }
}

const form = (add: MachineAdd, url = URL_TEXT, name = '') => {
  const onadded = vi.fn()
  const draft = new AddMachine(add, onadded)
  draft.url = url
  draft.name = name
  return { draft, onadded }
}

describe('send', () => {
  it('hands the new machine over', async () => {
    expect(await send(() => added(7), URL_TEXT, 'Studio')).toEqual({ kind: 'added', machine: 7 })
  })

  it.each<[string, CommandError, string]>([
    ['a route error under the URL', { code: 'route_empty_token', message: 'the token is empty' }, 'url'],
    ['any other under the form', { code: 'store', message: 'the disk is full' }, 'form'],
  ])('puts %s', async (_, error, kind) => {
    expect(await send(() => refused(error), URL_TEXT, 'Studio')).toEqual({ kind, error: error.message })
  })

  it.each<[string, unknown, string]>([
    ['an error', new Error('IPC is gone'), 'IPC is gone'],
    ['an object with a message', { message: 'no window' }, 'no window'],
    ['anything else', 'refused', 'refused'],
  ])('tells a rejected call with %s under the form by its message', async (_, error, message) => {
    expect(await send(() => Promise.reject(error), URL_TEXT, 'Studio')).toEqual({ kind: 'form', error: message })
  })
})

describe('AddMachine', () => {
  it('sends the trimmed URL under the name typed, or the host without one', async () => {
    const add = vi.fn(() => added(1))
    await form(add, ` ${URL_TEXT}\n`, ' Studio ').draft.submit()
    await form(add, URL_TEXT).draft.submit()
    expect(add.mock.calls).toEqual([
      [URL_TEXT, 'Studio'],
      [URL_TEXT, 'studio.lan'],
    ])
  })

  it.each([
    ['empty', '', EMPTY],
    ['without its token', 'ws://studio.lan:9000/', NO_TOKEN],
  ])('puts the problem of a URL %s under it without sending it', async (_, url, error) => {
    const add = vi.fn(() => added(1))
    const { draft } = form(add, url)
    await draft.submit()
    expect([draft.urlError, draft.formError, draft.busy]).toEqual([error, null, false])
    expect(add).not.toHaveBeenCalled()
  })

  it('stays busy from sending until the machine is handed over, and after', async () => {
    const pending = gate()
    const { draft, onadded } = form(() => pending.promise)
    const sent = draft.submit()
    expect(draft.busy).toBe(true)
    pending.open(4)
    await sent
    expect(onadded.mock.calls).toEqual([[4]])
    expect(draft.busy).toBe(true)
  })

  it('ignores a second submit while busy, also once added', async () => {
    const pending = gate()
    const add = vi.fn(() => pending.promise)
    const { draft, onadded } = form(add)
    const first = draft.submit()
    await draft.submit()
    pending.open(4)
    await first
    await draft.submit()
    expect(add).toHaveBeenCalledTimes(1)
    expect(onadded).toHaveBeenCalledTimes(1)
  })

  it.each<[string, CommandError, [string | null, string | null]]>([
    ['under the URL', { code: 'route_scheme', message: 'only ws://' }, ['only ws://', null]],
    ['under the form', { code: 'store', message: 'the disk is full' }, [null, 'the disk is full']],
  ])('rests again after a refusal and puts it %s', async (_, error, shown) => {
    const { draft, onadded } = form(() => refused(error))
    await draft.submit()
    expect([draft.urlError, draft.formError, draft.busy]).toEqual([...shown, false])
    expect(onadded).not.toHaveBeenCalled()
  })

  it('clears the old problems on a new submit', async () => {
    const add = vi.fn<MachineAdd>(() => refused({ code: 'store', message: 'the disk is full' }))
    const { draft } = form(add, '')
    await draft.submit()
    draft.url = URL_TEXT
    await draft.submit()
    expect([draft.urlError, draft.formError]).toEqual([null, 'the disk is full'])
    add.mockImplementation(() => added(2))
    await draft.submit()
    expect([draft.urlError, draft.formError]).toEqual([null, null])
  })
})
