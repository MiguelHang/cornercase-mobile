import { render } from 'svelte/server'
import { describe, expect, it } from 'vitest'
import type { ConnectionView_Serialize, MachineView_Serialize } from '../lib/bindings'
import { EditMachine } from '../lib/editMachine.svelte'
import Machine from './Machine.svelte'
import machineSource from './Machine.svelte?raw'

const ROUTES = ['ws://studio.lan:9000/', 'ws://192.168.1.20:9000/', 'ws://100.64.0.3:9000/']

const studio = (urls = ROUTES): MachineView_Serialize => ({
  id: 3,
  name: 'Studio',
  routes: urls.map((url, index) => ({ kind: 'bridge', url, has_token: index !== 2 })),
  last_route: null,
})

const CONNECTED: ConnectionView_Serialize = { state: 'connected', route: ROUTES[1] }

const done = () => Promise.resolve({ status: 'ok' as const, data: null })

const CALLS = {
  machineRename: done,
  machineMerge: () => Promise.resolve({ status: 'ok' as const, data: { added: [], token_updated: false, skipped: [] } }),
  machineRouteRemove: done,
  machineRouteMove: done,
  machineRemove: done,
}

const nothing = () => Promise.resolve()

const quiet = () => {}

const screen = (
  machine = studio(),
  connection: ConnectionView_Serialize | null = CONNECTED,
  edit = new EditMachine(CALLS, () => machine, nothing, nothing, quiet),
) =>
  render(Machine, {
    props: { machine, taken: ['Desk'], connection, onback: () => {}, onchanged: nothing, onremoved: nothing, edit },
  }).body

const rows = (body: string, kind: string) => body.split(new RegExp(`(?=<li class="${kind} )`)).slice(1)

const routes = (body: string) => rows(body, 'route')

const changes = (body: string) => rows(body, 'change')

const EDIT = /<details class="edit"><summary class="summary svelte-[\w-]+">Edit routes<\/summary>([^]*?)<\/details>/

const editing = (body: string) => body.match(EDIT)?.[1] ?? ''

const address = (row: string) => row.match(/<span class="address svelte-[\w-]+">([^<]*)<\/span>/)?.[1]

const tools = (row: string) =>
  [...row.matchAll(/<button class="tool svelte-[\w-]+" type="button" aria-label="([^"]+)"( disabled="")?/g)].map(
    ([, label, disabled]) => `${label}${disabled ? ' (off)' : ''}`,
  )

const flat = (text: string) => text.replace(/\s+/g, ' ')

describe('Machine', () => {
  it('has a header with Back and the name of the machine', () => {
    expect(screen()).toMatch(
      /<button class="back svelte-[\w-]+" type="button" aria-label="Back"><svg[^]*?<\/svg><!----><\/button> <h1 class="title svelte-[\w-]+">Studio<\/h1>/,
    )
  })

  it('fills the name with the one saved and holds Rename until it changes', () => {
    const body = screen()
    expect(body).toMatch(/<input[^>]*value="Studio"/)
    expect(body.match(/<button class="button primary[^>]*>(?:<!--[^>]*-->| )*Rename/)?.[0]).toContain('disabled=""')
  })

  it('tells a refused name under the field', async () => {
    const machine = studio()
    const edit = new EditMachine(CALLS, () => machine, nothing, nothing, quiet)
    edit.name = 'desk'
    await edit.rename(['Desk'])
    expect(screen(machine, CONNECTED, edit)).toMatch(/<p class="error svelte-[\w-]+" id="[^"]+">another machine already has that name<\/p>/)
  })

  it('lists the routes in their order by address', () => {
    const addresses = routes(screen()).map(address)
    expect(addresses).toEqual(['studio.lan:9000', '192.168.1.20:9000', '100.64.0.3:9000'])
  })

  it('marks the route it is connected through and the one without a token', () => {
    const [first, second, third] = routes(screen())
    expect(first).not.toContain('class="mark')
    expect(second).toMatch(/<span class="mark done svelte-[\w-]+">Connected<\/span>/)
    expect(third).toMatch(/<span class="mark failed svelte-[\w-]+">No token<\/span>/)
    expect(third).not.toContain('Connected')
  })

  it('marks no route when the machine is not the one connected', () => {
    expect(screen(studio(), null)).not.toContain('Connected')
    expect(screen(studio(), { state: 'connecting' })).not.toContain('Connected')
  })

  it('shows the routes as information, with no button to change them', () => {
    const shown = screen().split('<details')[0]
    expect(routes(shown).map(tools)).toEqual([[], [], []])
    expect(shown).not.toContain('Add route')
  })

  it('keeps moving, removing and adding routes in Edit routes, closed until opened', () => {
    const body = screen()
    expect(body).not.toMatch(/<details[^>]* open/)
    expect(changes(editing(body)).map(address)).toEqual(['studio.lan:9000', '192.168.1.20:9000', '100.64.0.3:9000'])
    expect(changes(editing(body)).some((row) => row.includes('class="mark'))).toBe(false)
    expect(editing(body)).toMatch(/<button class="button plain svelte-[\w-]+" type="button">(?:<!--[^>]*-->| )*Add route/)
  })

  it('moves each route up or down but not past the ends, and removes any', () => {
    expect(changes(editing(screen())).map(tools)).toEqual([
      ['Move up (off)', 'Move down', 'Remove route'],
      ['Move up', 'Move down', 'Remove route'],
      ['Move up', 'Move down (off)', 'Remove route'],
    ])
  })

  it('never removes the last route', () => {
    expect(changes(editing(screen(studio([ROUTES[0]])))).map(tools)).toEqual([
      ['Move up (off)', 'Move down (off)', 'Remove route (off)'],
    ])
  })

  it('tells a refused change of the routes under them', async () => {
    const machine = studio()
    const refused = () =>
      Promise.resolve({ status: 'error' as const, error: { code: 'store' as const, message: 'the disk is full' } })
    const edit = new EditMachine({ ...CALLS, machineRouteMove: refused }, () => machine, nothing, nothing, quiet)
    await edit.move(ROUTES[0], 'down')
    expect(screen(machine, CONNECTED, edit)).toMatch(
      /<\/details>(?:<[^p][^>]*>|<!--[^>]*-->| )*<p class="problem svelte-[\w-]+" role="alert">the disk is full<\/p>/,
    )
  })

  it('opens the form for the code with Add route, outside Edit routes, which then offers it no more', () => {
    expect(screen()).not.toContain('Machine code')
    const machine = studio()
    const edit = new EditMachine(CALLS, () => machine, nothing, nothing, quiet)
    edit.open()
    const body = screen(machine, CONNECTED, edit)
    const after = body.slice(body.indexOf('</details>'))
    expect(after).toMatch(/<h2 class="heading svelte-[\w-]+">Scan or paste its code<\/h2>/)
    expect(after).toContain('Scan QR code')
    expect(after).toContain('Machine code')
    expect(body).not.toContain('Add route')
  })

  it('opens with Add route open when asked to scan again', () => {
    const body = render(Machine, {
      props: {
        machine: studio(),
        taken: [],
        connection: { state: 'unauthorized' },
        onback: () => {},
        onchanged: nothing,
        onremoved: nothing,
        adding: true,
      },
    }).body
    const after = body.slice(body.indexOf('</details>'))
    expect(after).toMatch(/<h2 class="heading svelte-[\w-]+">Scan or paste its code<\/h2>/)
    expect(after).toContain('Scan QR code')
    expect(body).not.toMatch(/<details[^>]* open/)
  })

  it('offers deleting the machine in the tone of danger', () => {
    expect(screen()).toMatch(/<button class="button danger svelte-[\w-]+" type="button">(?:<!--[^>]*-->| )*Delete machine/)
    expect(screen()).not.toContain('Delete Studio?')
  })

  it('asks before deleting, with the name and what goes, instead of Add route and Delete machine', () => {
    const machine = studio()
    const edit = new EditMachine(CALLS, () => machine, nothing, nothing, quiet)
    edit.askToDelete()
    const body = screen(machine, CONNECTED, edit)
    expect(body).toMatch(/<h2 class="heading svelte-[\w-]+">Delete Studio\?<\/h2>/)
    expect(body).toContain(
      'Its routes and their tokens are removed from this phone. The Mac and its bridge are not touched.',
    )
    expect(body).toMatch(/<button class="button plain svelte-[\w-]+" type="button">(?:<!--[^>]*-->| )*Cancel/)
    expect(body).toMatch(/<button class="button danger svelte-[\w-]+" type="button">(?:<!--[^>]*-->| )*Delete(?:<!--[^>]*-->)*<\/button>/)
    expect(body).not.toContain('Add route')
    expect(body).not.toContain('Delete machine')
  })
})

describe('Machine wiring', () => {
  const script = flat(machineSource)

  it('renames against the names of the others', () => {
    expect(script).toContain('void edit.rename(taken)')
    expect(script).toContain('<Button type="submit" resting={edit.unchanged} busy={edit.busy}>Rename</Button>')
  })

  it('moves and removes the route of its row', () => {
    expect(script).toContain("onclick={() => void edit.move(route.url, 'up')}")
    expect(script).toContain("onclick={() => void edit.move(route.url, 'down')}")
    expect(script).toContain('onclick={() => void edit.remove(route.url)}')
  })

  it('adds the route pasted or scanned, and cancels', () => {
    expect(script).toContain('void edit.addRoute()')
    expect(script).toContain('onscanned={(url) => void edit.fill(url)}')
    expect(script).toContain('onclick={() => edit.close()}>Cancel</Button>')
    expect(script).toContain('onclick={() => edit.open()}>Add route</Button>')
  })

  it('asks, cancels and deletes the machine', () => {
    expect(script).toContain('onclick={() => edit.askToDelete()}>Delete machine</Button>')
    expect(script).toContain('onclick={() => edit.keep()}>Cancel</Button>')
    expect(script).toContain('onclick={() => void edit.deleteMachine()}>Delete</Button>')
  })

  it('reloads the machines on opening, since the bridge may have told new routes', () => {
    expect(script).toContain('onMount(() => { void onchanged() })')
  })

  it('opens Add route once, on arriving to scan again', () => {
    expect(script).toContain('untrack(() => { if (adding) { edit.open() } })')
  })

  it('reloads the machines after each change, leaves once deleted and goes back on Back', () => {
    expect(script).toContain('() => machine, () => onchanged(), () => onremoved(),')
    expect(script).toContain('onclick={onback}')
  })
})
