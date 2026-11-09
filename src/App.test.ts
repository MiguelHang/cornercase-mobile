import { render } from 'svelte/server'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { describe, expect, it, vi } from 'vitest'
import App from './App.svelte'
import appSource from './App.svelte?raw'
import noMachinesSource from './routes/NoMachines.svelte?raw'

vi.mock('@tauri-apps/api/core', async (actual) => ({ ...(await actual<object>()), invoke: vi.fn() }))
vi.mock('@tauri-apps/api/event', async (actual) => ({ ...(await actual<object>()), listen: vi.fn() }))

describe('App', () => {
  const body = render(App).body.replaceAll('<!---->', '')

  it('opens on the screen with no machines, under the brand', () => {
    expect(body).toMatch(
      /<h1 class="name svelte-[\w-]+"><span class="wordmark svelte-[\w-]+" style="font-size: 22px;">cornercase<\/span><\/h1>/,
    )
    expect(body).toContain('--unit: 5px;')
    expect(body).toMatch(/<h2 class="title svelte-[\w-]+">No machines yet<\/h2>/)
  })

  it('says what a machine is and what it needs', () => {
    expect(body).toContain('A machine is a computer running cornercase whose agents you follow from here.')
    expect(body).toContain('It needs the cornercase bridge running on your Mac, and the URL it prints.')
  })

  it('offers adding a machine, ready to be pressed', () => {
    expect(body).toMatch(/<button class="button primary svelte-[\w-]+" type="button">(<!--[^>]*-->)* Add machine<\/button>/)
  })

  it('neither calls nor listens to Tauri while rendering on the server', () => {
    expect(render(App).body).toContain('No machines yet')
    expect(invoke).not.toHaveBeenCalled()
    expect(listen).not.toHaveBeenCalled()
  })

  it('shows the empty toast stack but no connection bar while there is no machine', () => {
    expect(body).toMatch(/<div class="toasts svelte-[\w-]+" aria-live="polite" style="--below: 0px;"><!--\[--><!--\]--><\/div>/)
    expect(body).not.toContain('role="status"')
    expect(body).not.toContain('class="home')
  })

  it('brings the card in with the shared motion', () => {
    expect(noMachinesSource).toMatch(/class="card"[^>]*\sin:riseIn[\s>]/)
    expect(noMachinesSource).toMatch(/class="card"[^>]*\sout:fadeOut[\s>]/)
  })
})

describe('App wiring', () => {
  const source = appSource.replace(/\s+/g, ' ')

  it('takes the base route from the screen on every change', () => {
    expect(source).toContain('const router = new Router(routeFor(idle, 0))')
    expect(source).toContain(
      'const show = (next: Screen) => { current = next router.setBase(routeFor(next, machines.length)) }',
    )
    expect(source.match(/ current = (?!\$state)/g)).toHaveLength(1)
  })

  it('connects to the last machine asked for, and a refused connect becomes an error toast', () => {
    expect(source).toContain(
      "const join = latestWins(async (machine: number) => { show(connecting(machine)) const connected = await commands.connect(machine) if (connected.status === 'error') { toasts.failed(connected.error) } })",
    )
  })

  it('goes home with a new machine, reloads the machines for its name and connects to it', () => {
    expect(source).toContain(
      'const added = (machine: number) => { show(connecting(machine)) Promise.all([commands.machinesList().then(remember), join(machine)]).catch(toasts.failed) }',
    )
    expect(source).toContain("machines.find((machine) => machine.id === current.machine)?.name ?? ''")
  })

  it('turns a failed machine list into an error toast and connects to nothing then', () => {
    expect(source).toContain("if (listed.status === 'error') { toasts.failed(listed.error) return { machines: [], active: null } }")
    expect(source).toContain('const first = firstToConnect(remember(listed), state)')
  })

  it('opens Machines from the bar, with the list, the active machine and its connection', () => {
    expect(source).toContain(
      "{:else if router.current.name === 'machines'} <Machines {machines} active={current.machine} connection={current.connection} onback={() => router.back()} onpick={(machine) => void join(machine)} onadded={added} onedit={(machine) => router.go({ name: 'machine', id: machine })} onrescan={(machine) => router.go({ name: 'machine', id: machine, adding: true })} />",
    )
  })

  it('edits the machine asked for, against the names of the others, and reloads the machines after each change', () => {
    expect(source).toContain(
      "const editing = $derived.by(() => { const route = router.current return route.name === 'machine' ? machines.find((machine) => machine.id === route.id) : undefined })",
    )
    expect(source).toContain(
      'const taken = $derived(machines.filter((machine) => machine.id !== editing?.id).map((machine) => machine.name))',
    )
    expect(source).toContain('const reload = () => commands.machinesList().then(remember).catch(toasts.failed)')
    expect(source).toContain(
      '{:else if editing !== undefined} <Machine machine={editing} {taken} connection={editing.id === current.machine ? current.connection : null} onback={() => router.back()} onchanged={reload} onremoved={removed} adding={rescanning} />',
    )
  })

  it('opens a machine with Add route open when Machines asks to scan it again', () => {
    expect(source).toContain(
      "const rescanning = $derived(router.current.name === 'machine' && router.current.adding === true)",
    )
  })

  it('leaves a deleted machine for Machines, connecting no other, and for the empty screen after the last', () => {
    expect(source).toContain(
      'const removed = async () => { router.back() await reload() router.setBase(routeFor(current, machines.length)) }',
    )
    expect(source.match(/join\(/g)).toHaveLength(3)
  })

  it('listens before loading, and sends toasts and failures to the stack', () => {
    expect(source).toContain('events.toast.listen((event) => toasts.push(event.payload)),')
    expect(source).toContain('Promise.all(listening).then(load).catch(toasts.failed)')
  })

  it('shows the connection bar only at home, and puts home and the toasts below it', () => {
    expect(source).toContain(
      '{#if home} <ConnectionBar connection={current.connection} {machineName} bind:height={barHeight} onmachines={() => router.go({ name: \'machines\' })} /> <Home screen={current} below={barHeight} />',
    )
    expect(source).toContain('{:else} <NoMachines onadded={added} /> {/if}')
    expect(source).toContain(
      '<ToastStack items={toasts.items} ondismiss={(id) => toasts.dismiss(id)} below={home ? barHeight : 0} />',
    )
  })
})
