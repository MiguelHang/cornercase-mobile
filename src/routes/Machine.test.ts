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

const CALLS = { machineRename: done, machineRouteAdd: done, machineRouteRemove: done, machineRouteMove: done }

const screen = (
  machine = studio(),
  connection: ConnectionView_Serialize | null = CONNECTED,
  edit = new EditMachine(CALLS, () => machine, () => Promise.resolve()),
) =>
  render(Machine, {
    props: { machine, taken: ['Desk'], connection, onback: () => {}, onchanged: () => Promise.resolve(), edit },
  }).body

const routes = (body: string) => body.split(/(?=<li class="route )/).slice(1)

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
    const edit = new EditMachine(CALLS, () => machine, () => Promise.resolve())
    edit.name = 'desk'
    await edit.rename(['Desk'])
    expect(screen(machine, CONNECTED, edit)).toMatch(/<p class="error svelte-[\w-]+" id="[^"]+">another machine already has that name<\/p>/)
  })

  it('lists the routes in their order by address', () => {
    const addresses = routes(screen()).map((row) => row.match(/<span class="address svelte-[\w-]+">([^<]*)<\/span>/)?.[1])
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

  it('moves each route up or down but not past the ends, and removes any', () => {
    expect(routes(screen()).map(tools)).toEqual([
      ['Move up (off)', 'Move down', 'Remove route'],
      ['Move up', 'Move down', 'Remove route'],
      ['Move up', 'Move down (off)', 'Remove route'],
    ])
  })

  it('never removes the last route', () => {
    expect(routes(screen(studio([ROUTES[0]]))).map(tools)).toEqual([
      ['Move up (off)', 'Move down (off)', 'Remove route (off)'],
    ])
  })

  it('tells a refused change of the routes under them', async () => {
    const machine = studio()
    const refused = () =>
      Promise.resolve({ status: 'error' as const, error: { code: 'store' as const, message: 'the disk is full' } })
    const edit = new EditMachine({ ...CALLS, machineRouteMove: refused }, () => machine, () => Promise.resolve())
    await edit.move(ROUTES[0], 'down')
    expect(screen(machine, CONNECTED, edit)).toMatch(
      /<\/ul>(?:<[^p][^>]*>|<!--[^>]*-->| )*<p class="problem svelte-[\w-]+" role="alert">the disk is full<\/p>/,
    )
  })

  it('offers Add route, and opens its form with the scanner and the URL', () => {
    expect(screen()).toMatch(/<button class="button primary svelte-[\w-]+" type="button">(?:<!--[^>]*-->| )*Add route/)
    expect(screen()).not.toContain('Bridge URL')
    const machine = studio()
    const edit = new EditMachine(CALLS, () => machine, () => Promise.resolve())
    edit.open()
    const body = screen(machine, CONNECTED, edit)
    expect(body).toMatch(/<h2 class="heading svelte-[\w-]+">Add route<\/h2>/)
    expect(body).toContain('Scan QR code')
    expect(body).toContain('Bridge URL')
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

  it('reloads the machines after each change and goes back on Back', () => {
    expect(script).toContain('() => machine, () => onchanged(),')
    expect(script).toContain('onclick={onback}')
  })
})
