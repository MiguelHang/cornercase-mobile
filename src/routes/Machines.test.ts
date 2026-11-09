import { render } from 'svelte/server'
import { describe, expect, it } from 'vitest'
import type { ConnectionView_Serialize, MachineView_Serialize, ProbeView } from '../lib/bindings'
import { Probes } from '../lib/probes.svelte'
import { ThisDevice, type DeviceCalls } from '../lib/thisDevice.svelte'
import rowSource from '../lib/ui/MachineRow.svelte?raw'
import Machines from './Machines.svelte'
import machinesSource from './Machines.svelte?raw'

const machine = (id: number, name: string): MachineView_Serialize => ({
  id,
  name,
  routes: [{ kind: 'bridge', url: `ws://${name.toLowerCase()}.lan:9000/`, has_token: true }],
  last_route: null,
})

const MACHINES = [machine(1, 'Studio'), machine(2, 'Desk'), machine(3, 'Laptop'), machine(4, 'Attic')]

const CONNECTED: ConnectionView_Serialize = { state: 'connected', route: 'ws://192.168.1.20:7000/?token=abc' }

const PROBED: Record<number, ProbeView> = {
  2: { state: 'answers', route: 'ws://desk.lan:9000/' },
  3: {
    state: 'no_answer',
    failures: [
      { route: 'ws://laptop.lan:9000/', error: { code: 'timeout', message: 'timed out' } },
      { route: 'ws://100.64.0.3:9000/', error: { code: 'connect', message: 'connection refused' } },
    ],
  },
}

const probed = async () => {
  const probes = new Probes({
    machineProbe: (id) => (id in PROBED ? Promise.resolve({ status: 'ok', data: PROBED[id] }) : new Promise(() => {})),
  })
  void probes.check([4])
  await probes.check([2, 3])
  return probes
}

const loaded = async (identityFingerprint: DeviceCalls['identityFingerprint']) => {
  const device = new ThisDevice({ identityFingerprint })
  await device.load()
  return device
}

const FINGERPRINT = () => Promise.resolve({ status: 'ok' as const, data: '0a1b2c3d4e' })

const screen = async (
  device = loaded(FINGERPRINT),
  connection: ConnectionView_Serialize = CONNECTED,
  probes: Promise<Probes> = probed(),
) =>
  render(Machines, {
    props: {
      machines: MACHINES,
      active: 1,
      connection,
      onback: () => {},
      onpick: () => {},
      onadded: () => {},
      onrouted: () => {},
      onedit: () => {},
      onrescan: () => {},
      probes: await probes,
      device: await device,
    },
  }).body

const rows = (body: string) => body.split(/(?=<li class="machine )/).slice(1)

const flat = (text: string) => text.replace(/\s+/g, ' ')

const scanAgain = /<button class="button plain svelte-[\w-]+" type="button">(?:<!--[^>]*-->| )*Scan again/

const withRejected = async () => {
  const views: Record<number, ProbeView> = { 2: { state: 'unauthorized' }, 3: { state: 'no_token' }, 4: PROBED[2] }
  const probes = new Probes({ machineProbe: (id) => Promise.resolve({ status: 'ok', data: views[id] }) })
  await probes.check([2, 3, 4])
  return rows(await screen(loaded(FINGERPRINT), CONNECTED, Promise.resolve(probes)))
}

describe('Machines', () => {
  it('has a header with Back and its title', async () => {
    expect(await screen()).toMatch(
      /<header class="top svelte-[\w-]+"><button class="back svelte-[\w-]+" type="button" aria-label="Back"><svg[^]*?<\/svg><!----><\/button> <h1 class="title svelte-[\w-]+">Machines<\/h1><\/header>/,
    )
  })

  it('lists the machines in the order saved', async () => {
    const names = rows(await screen()).map((row) => row.match(/<span class="name svelte-[\w-]+">([^<]*)<\/span>/)?.[1])
    expect(names).toEqual(['Studio', 'Desk', 'Laptop', 'Attic'])
  })

  it('shows the active one with the text of the bar, marked current and not to be picked', async () => {
    const [active] = rows(await screen())
    expect(active).toContain('<div class="pick svelte-')
    expect(active).toContain('aria-current="true"')
    expect(active).toMatch(/<span class="text svelte-[\w-]+">Studio · via 192.168.1.20:7000<\/span>/)
    expect(active).toMatch(/<span class="dot done svelte-[\w-]+"><\/span>/)
    expect(active).not.toContain('Check again')
  })

  it('shows each other one as a button with what its probe said and Check again', async () => {
    const [, desk, laptop] = rows(await screen())
    for (const row of [desk, laptop]) {
      expect(row).toMatch(/<button class="pick svelte-[\w-]+" type="button">/)
      expect(row).toMatch(/<button class="button plain svelte-[\w-]+" type="button">(?:<!--[^>]*-->| )*Check again/)
    }
    expect(desk).toMatch(/<span class="dot done svelte-[\w-]+"><\/span><!--]--> <span class="text svelte-[\w-]+">Answers via desk.lan:9000<\/span>/)
    expect(laptop).toMatch(/<span class="dot failed svelte-[\w-]+"><\/span><!--]--> <span class="text svelte-[\w-]+">No answer<\/span>/)
  })

  it('folds the error of each route of one that did not answer', async () => {
    const [, desk, laptop] = rows(await screen())
    expect(desk).not.toContain('<details')
    expect(laptop).toMatch(/<details class="failures"><summary class="summary svelte-[\w-]+">Errors by route<\/summary>/)
    const routes = [...laptop.matchAll(/<span class="address svelte-[\w-]+">([^<]*)<\/span> <span class="message svelte-[\w-]+">([^<]*)<\/span>/g)]
    expect(routes.map(([, address, message]) => [address, message])).toEqual([
      ['laptop.lan:9000', 'timed out'],
      ['100.64.0.3:9000', 'connection refused'],
    ])
  })

  it('spins while a probe runs, and holds Check again until it ends', async () => {
    const attic = rows(await screen())[3]
    expect(attic).toMatch(/<span class="spinner svelte-[\w-]+" aria-hidden="true">[^]*?<\/span><!--]--> <span class="text svelte-[\w-]+">Checking…<\/span>/)
    expect(attic).not.toContain('class="dot')
    expect(attic.match(/<button[^]*?Check again/)?.[0]).toContain('disabled=""')
  })

  it('shows the fingerprint of this device in monospace', async () => {
    expect(await screen()).toMatch(
      /<h2 class="heading svelte-[\w-]+">This device<\/h2> <!--\[0--><p class="label svelte-[\w-]+">Identity fingerprint<\/p> <code class="fingerprint svelte-[\w-]+">0a1b2c3d4e<\/code>/,
    )
    expect(flat(machinesSource)).toContain('font-family: var(--font-mono);')
  })

  it('says why there is no fingerprint when the device has no identity', async () => {
    const body = await screen(
      loaded(() => Promise.resolve({ status: 'error', error: { code: 'no_identity', message: 'no identity yet' } })),
    )
    expect(body).toMatch(/<p class="reason svelte-[\w-]+">no identity yet<\/p>/)
    expect(body).not.toContain('<code')
  })

  it('offers Edit on every row, the active one too', async () => {
    for (const row of rows(await screen())) {
      expect(row).toMatch(/<button class="button plain svelte-[\w-]+" type="button">(?:<!--[^>]*-->| )*Edit/)
    }
  })

  it('offers Scan again on the active one while the bridge rejects its token, and on no row that answers', async () => {
    const [active, ...others] = rows(await screen(loaded(FINGERPRINT), { state: 'unauthorized' }))
    expect(active).toMatch(scanAgain)
    expect(active).toContain("The bridge rejected this phone's token: scan its code again in Machines")
    expect(others.filter((row) => scanAgain.test(row))).toEqual([])
    expect(rows(await screen()).filter((row) => scanAgain.test(row))).toEqual([])
  })

  it('offers Scan again on the others whose probe says their token is rejected or missing', async () => {
    expect((await withRejected()).map((row) => scanAgain.test(row))).toEqual([false, true, true, false])
  })

  it('puts Scan again in the place of Check again on a row whose token is rejected or missing', async () => {
    expect((await withRejected()).map((row) => row.includes('Check again'))).toEqual([false, false, false, true])
  })

  it('offers adding a machine and shows no form yet', async () => {
    const body = await screen()
    expect(body).toMatch(/<button class="button primary svelte-[\w-]+" type="button">(?:<!--[^>]*-->| )*Add machine/)
    expect(body).not.toContain('<form')
  })
})

describe('Machines wiring', () => {
  const script = flat(machinesSource)

  it('probes the others on opening, and the one that stops being active, without tracking the probes', () => {
    expect(script).toContain('void probes.open( machines.map(({ id }) => id), active, ) void device.load()')
    expect(script).toContain('$effect(() => { const now = active untrack(() => void probes.follow(now)) })')
  })

  it('connects to the one picked, probes one again on Check again, and goes back on Back', () => {
    expect(script).toContain('onpick={() => onpick(machine.id)} oncheck={() => void probes.check([machine.id])}')
    expect(script).toContain('onclick={onback}')
    expect(flat(rowSource)).toContain(
      '{:else if !active} <Button tone="plain" resting={checking} onclick={oncheck}>Check again</Button> {/if}',
    )
  })

  it('opens the machine edited', () => {
    expect(script).toContain('onedit={() => onedit(machine.id)}')
    expect(flat(rowSource)).toContain('{/if} <Button tone="plain" onclick={onedit}>Edit</Button>')
  })

  it('opens Add route on Scan again of a machine whose token is rejected or missing', () => {
    expect(script).toContain('const connectionOf = (machine: MachineView_Serialize) => (machine.id === active ? connection : null)')
    expect(script).toContain('rejected={rescannable(connectionOf(machine), probes.of(machine.id))}')
    expect(script).toContain('onrescan={() => onrescan(machine.id)}')
    expect(flat(rowSource)).toContain(
      '{#if rejected} <Button tone="plain" onclick={onrescan}>Scan again</Button> {:else if !active}',
    )
  })

  it('counts down a retry of the active one like the bar', () => {
    expect(script).toContain('const clock = retryClock(() => connection)')
    expect(script).toContain(
      'rowStatus(machine, connectionOf(machine), probes.of(machine.id), clock.elapsed)',
    )
  })

  it('closes the form once a route is added, probes that machine again unless active, and leaves the active one alone', () => {
    expect(script).toContain(
      'const routed = (machine: number) => { adding = false if (machine !== active) { void probes.check([machine]) } onrouted() }',
    )
  })

  it('swaps Add machine for the form, closing it once a machine is added', () => {
    expect(script).toContain('const added = (machine: number) => { adding = false onadded(machine) }')
    expect(script).toContain('<MachineForm onadded={added} onrouted={routed} oncancel={() => (adding = false)} />')
    expect(script).toContain('<Button onclick={() => (adding = true)}>Add machine</Button>')
  })
})
