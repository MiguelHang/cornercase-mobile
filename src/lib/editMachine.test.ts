import { describe, expect, it, vi } from 'vitest'
import type { CommandError, MachineView_Serialize } from './bindings'
import { EditMachine, type EditCalls } from './editMachine.svelte'
import { NAME_EMPTY, NAME_TAKEN, NO_TOKEN } from './machineForm'

const URL_TEXT = 'ws://100.64.0.3:9000/?token=abc'

const ROUTES = ['ws://studio.lan:9000/', 'ws://192.168.1.20:9000/']

const studio = (routes = ROUTES): MachineView_Serialize => ({
  id: 3,
  name: 'Studio',
  routes: routes.map((url) => ({ kind: 'bridge', url, has_token: true })),
  last_route: null,
})

const done = () => Promise.resolve({ status: 'ok' as const, data: null })

const refused = (error: CommandError) => Promise.resolve({ status: 'error' as const, error })

const gate = () => {
  let open: () => void = () => {}
  const promise = new Promise<void>((resolve) => (open = resolve))
  return { promise: promise.then(() => ({ status: 'ok' as const, data: null })), open }
}

const calls = (overrides: Partial<EditCalls> = {}) => ({
  machineRename: vi.fn(overrides.machineRename ?? done),
  machineRouteAdd: vi.fn(overrides.machineRouteAdd ?? done),
  machineRouteRemove: vi.fn(overrides.machineRouteRemove ?? done),
  machineRouteMove: vi.fn(overrides.machineRouteMove ?? done),
  machineRemove: vi.fn(overrides.machineRemove ?? done),
})

const editor = (overrides: Partial<EditCalls> = {}, machine = studio()) => {
  const commands = calls(overrides)
  const onchanged = vi.fn(() => Promise.resolve())
  const onremoved = vi.fn(() => Promise.resolve())
  const edit = new EditMachine(commands, () => machine, onchanged, onremoved)
  return { edit, commands, onchanged, onremoved }
}

describe('EditMachine renaming', () => {
  it('starts from the saved name and holds a name left as it is', async () => {
    const { edit, commands } = editor()
    expect(edit.name).toBe('Studio')
    expect(edit.unchanged).toBe(true)
    edit.name = ' Studio '
    await edit.rename([])
    expect(commands.machineRename).not.toHaveBeenCalled()
  })

  it('renames to the trimmed name, then reloads the machines', async () => {
    const { edit, commands, onchanged } = editor()
    edit.name = ' Office '
    await edit.rename(['Desk'])
    expect(commands.machineRename.mock.calls).toEqual([[3, 'Office']])
    expect(onchanged).toHaveBeenCalledOnce()
    expect([edit.nameError, edit.formError, edit.busy]).toEqual([null, null, false])
  })

  it.each([
    ['empty', '  ', NAME_EMPTY],
    ['of another machine', ' desk ', NAME_TAKEN],
  ])('refuses a name %s under the field without asking', async (_, name, error) => {
    const { edit, commands, onchanged } = editor()
    edit.name = name
    await edit.rename(['Desk'])
    expect(edit.nameError).toBe(error)
    expect(commands.machineRename).not.toHaveBeenCalled()
    expect(onchanged).not.toHaveBeenCalled()
  })

  it.each<[string, CommandError, 'nameError' | 'formError']>([
    ['a name error under the field', { code: 'name_taken', message: 'the name is taken' }, 'nameError'],
    ['any other under the page', { code: 'store', message: 'the disk is full' }, 'formError'],
  ])('puts %s, reloading nothing', async (_, error, field) => {
    const { edit, onchanged } = editor({ machineRename: () => refused(error) })
    edit.name = 'Office'
    await edit.rename([])
    expect(edit[field]).toBe(error.message)
    expect([edit.busy, onchanged.mock.calls.length]).toEqual([false, 0])
  })
})

describe('EditMachine routes', () => {
  it('moves a route up or down, then reloads the machines', async () => {
    const { edit, commands, onchanged } = editor()
    await edit.move(ROUTES[1], 'up')
    await edit.move(ROUTES[0], 'down')
    expect(commands.machineRouteMove.mock.calls).toEqual([
      [3, ROUTES[1], 'up'],
      [3, ROUTES[0], 'down'],
    ])
    expect(onchanged).toHaveBeenCalledTimes(2)
  })

  it('removes a route, then reloads the machines', async () => {
    const { edit, commands, onchanged } = editor()
    await edit.remove(ROUTES[0])
    expect(commands.machineRouteRemove.mock.calls).toEqual([[3, ROUTES[0]]])
    expect(onchanged).toHaveBeenCalledOnce()
  })

  it('never removes the last route', async () => {
    const { edit, commands } = editor({}, studio([ROUTES[0]]))
    await edit.remove(ROUTES[0])
    expect(commands.machineRouteRemove).not.toHaveBeenCalled()
  })

  it('tells a refused move or removal under the page, even a route error', async () => {
    const { edit } = editor({
      machineRouteMove: () => refused({ code: 'unknown_route', message: 'no such route' }),
      machineRouteRemove: () => refused({ code: 'route_port', message: 'bad port' }),
    })
    await edit.move(ROUTES[0], 'down')
    expect(edit.formError).toBe('no such route')
    await edit.remove(ROUTES[0])
    expect([edit.formError, edit.urlError]).toEqual(['bad port', null])
  })

  it('holds every change while one runs, until the machines are reloaded', async () => {
    const pending = gate()
    const { edit, commands } = editor({ machineRouteMove: () => pending.promise })
    const moving = edit.move(ROUTES[0], 'down')
    expect(edit.busy).toBe(true)
    edit.name = 'Office'
    await edit.rename([])
    await edit.remove(ROUTES[0])
    await edit.move(ROUTES[1], 'up')
    edit.url = URL_TEXT
    await edit.addRoute()
    expect(commands.machineRouteMove).toHaveBeenCalledOnce()
    expect([commands.machineRename, commands.machineRouteRemove, commands.machineRouteAdd].map((call) => call.mock.calls.length)).toEqual([0, 0, 0])
    pending.open()
    await moving
    expect(edit.busy).toBe(false)
  })
})

describe('EditMachine adding a route', () => {
  it('adds the URL pasted, then closes the form and reloads the machines', async () => {
    const { edit, commands, onchanged } = editor()
    edit.open()
    edit.url = ` ${URL_TEXT}\n`
    await edit.addRoute()
    expect(commands.machineRouteAdd.mock.calls).toEqual([[3, URL_TEXT]])
    expect(onchanged).toHaveBeenCalledOnce()
    expect([edit.adding, edit.url]).toEqual([false, ''])
  })

  it('adds the URL scanned', async () => {
    const { edit, commands } = editor()
    edit.open()
    await edit.fill(URL_TEXT)
    expect(commands.machineRouteAdd.mock.calls).toEqual([[3, URL_TEXT]])
    expect(edit.adding).toBe(false)
  })

  it('takes the machine code scanned again whole, for the core to add its addresses and update the token', async () => {
    const code = 'ws://192.168.1.20:9000/?token=new&also=100.64.0.3&name=Studio'
    const { edit, commands, onchanged } = editor()
    edit.open()
    await edit.fill(code)
    expect([commands.machineRouteAdd.mock.calls, edit.urlError, edit.adding]).toEqual([[[3, code]], null, false])
    expect(onchanged).toHaveBeenCalledOnce()
  })

  it('refuses a URL without a token under the field, keeping the form open', async () => {
    const { edit, commands } = editor()
    edit.open()
    edit.url = 'ws://100.64.0.3:9000/'
    await edit.addRoute()
    expect([edit.urlError, edit.adding]).toEqual([NO_TOKEN, true])
    expect(commands.machineRouteAdd).not.toHaveBeenCalled()
  })

  it.each<[string, CommandError, 'urlError' | 'formError']>([
    ['a route error under the field', { code: 'route_empty_token', message: 'the token is empty' }, 'urlError'],
    ['any other under the page', { code: 'secrets', message: 'the keychain is locked' }, 'formError'],
  ])('puts %s, keeping the form open', async (_, error, field) => {
    const { edit, onchanged } = editor({ machineRouteAdd: () => refused(error) })
    edit.open()
    await edit.fill(URL_TEXT)
    expect(edit[field]).toBe(error.message)
    expect([edit.adding, edit.url, onchanged.mock.calls.length]).toEqual([true, URL_TEXT, 0])
  })

  it('closes the form clearing what was typed, but not while adding', async () => {
    const pending = gate()
    const { edit } = editor({ machineRouteAdd: () => pending.promise })
    edit.open()
    const adding = edit.fill(URL_TEXT)
    edit.close()
    expect(edit.adding).toBe(true)
    pending.open()
    await adding
    edit.open()
    edit.url = 'ws://x'
    edit.urlError = NO_TOKEN
    edit.close()
    expect([edit.adding, edit.url, edit.urlError]).toEqual([false, '', null])
  })
})

describe('EditMachine deleting', () => {
  it('deletes nothing until asked and confirmed', async () => {
    const { edit, commands } = editor()
    await edit.deleteMachine()
    expect(commands.machineRemove).not.toHaveBeenCalled()
    edit.askToDelete()
    expect(edit.deleting).toBe(true)
  })

  it('deletes nothing when cancelled', async () => {
    const { edit, commands } = editor()
    edit.askToDelete()
    edit.formError = 'the disk is full'
    edit.keep()
    await edit.deleteMachine()
    expect([edit.deleting, edit.formError]).toEqual([false, null])
    expect(commands.machineRemove).not.toHaveBeenCalled()
  })

  it('deletes the machine once confirmed, then leaves its screen', async () => {
    const { edit, commands, onchanged, onremoved } = editor()
    edit.askToDelete()
    await edit.deleteMachine()
    expect(commands.machineRemove.mock.calls).toEqual([[3]])
    expect([onremoved.mock.calls.length, onchanged.mock.calls.length, edit.busy]).toEqual([1, 0, false])
  })

  it('tells a refused deletion under the routes, still asking', async () => {
    const { edit, onremoved } = editor({
      machineRemove: () => refused({ code: 'secrets', message: 'the keychain is locked' }),
    })
    edit.askToDelete()
    await edit.deleteMachine()
    expect([edit.formError, edit.deleting, edit.busy]).toEqual(['the keychain is locked', true, false])
    expect(onremoved).not.toHaveBeenCalled()
  })

  it('neither asks nor cancels while a change runs', async () => {
    const pending = gate()
    const { edit } = editor({ machineRemove: () => pending.promise })
    edit.askToDelete()
    const deleting = edit.deleteMachine()
    edit.keep()
    expect(edit.deleting).toBe(true)
    pending.open()
    await deleting
    const moved = gate()
    const other = editor({ machineRouteMove: () => moved.promise })
    const moving = other.edit.move(ROUTES[0], 'down')
    other.edit.askToDelete()
    expect(other.edit.deleting).toBe(false)
    moved.open()
    await moving
  })
})
