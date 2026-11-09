<script lang="ts">
  import { isTauri } from '@tauri-apps/api/core'
  import { onMount } from 'svelte'
  import { commands, events, type MachineView_Serialize } from './lib/bindings'
  import { apply, connecting, firstToConnect, screen, type Screen } from './lib/connection'
  import { latestWins } from './lib/join'
  import { Router, routeFor } from './lib/router.svelte'
  import { Toasts } from './lib/toasts.svelte'
  import ConnectionBar from './lib/ui/ConnectionBar.svelte'
  import ToastStack from './lib/ui/Toasts.svelte'
  import Home from './routes/Home.svelte'
  import Machine from './routes/Machine.svelte'
  import Machines from './routes/Machines.svelte'
  import NoMachines from './routes/NoMachines.svelte'

  type Listed = Awaited<ReturnType<typeof commands.machinesList>>

  const idle = screen({ machine: null, connection: { state: 'none' } }, null)

  let current = $state(idle)
  let machines = $state<MachineView_Serialize[]>([])
  let barHeight = $state(0)
  const router = new Router(routeFor(idle, 0))
  const toasts = new Toasts()

  const home = $derived(router.current.name === 'home')
  const editing = $derived.by(() => {
    const route = router.current
    return route.name === 'machine' ? machines.find((machine) => machine.id === route.id) : undefined
  })
  const taken = $derived(machines.filter((machine) => machine.id !== editing?.id).map((machine) => machine.name))
  const machineName = $derived(machines.find((machine) => machine.id === current.machine)?.name ?? '')

  const show = (next: Screen) => {
    current = next
    router.setBase(routeFor(next, machines.length))
  }

  const remember = (listed: Listed) => {
    if (listed.status === 'error') {
      toasts.failed(listed.error)
      return { machines: [], active: null }
    }
    machines = listed.data.machines
    return listed.data
  }

  const reload = () => commands.machinesList().then(remember).catch(toasts.failed)

  const removed = async () => {
    router.back()
    await reload()
    router.setBase(routeFor(current, machines.length))
  }

  const join = latestWins(async (machine: number) => {
    show(connecting(machine))
    const connected = await commands.connect(machine)
    if (connected.status === 'error') {
      toasts.failed(connected.error)
    }
  })

  const added = (machine: number) => {
    show(connecting(machine))
    Promise.all([commands.machinesList().then(remember), join(machine)]).catch(toasts.failed)
  }

  const load = async () => {
    const [listed, state, session] = await Promise.all([
      commands.machinesList(),
      commands.connectionState(),
      commands.session(),
    ])
    show(screen(state, session))
    const first = firstToConnect(remember(listed), state)
    if (first !== null) {
      await join(first)
    }
  }

  onMount(() => {
    if (!isTauri()) {
      return
    }
    const listening = [
      events.connectionChanged.listen((event) => show(apply(current, event.payload))),
      events.sessionChanged.listen((event) => show(apply(current, event.payload))),
      events.toast.listen((event) => toasts.push(event.payload)),
    ]
    Promise.all(listening).then(load).catch(toasts.failed)
    return () => listening.forEach((stop) => stop.then((unlisten) => unlisten()))
  })
</script>

<main>
  {#if home}
    <ConnectionBar
      connection={current.connection}
      {machineName}
      bind:height={barHeight}
      onmachines={() => router.go({ name: 'machines' })}
    />
    <Home screen={current} below={barHeight} />
  {:else if router.current.name === 'machines'}
    <Machines
      {machines}
      active={current.machine}
      connection={current.connection}
      onback={() => router.back()}
      onpick={(machine) => void join(machine)}
      onadded={added}
      onedit={(machine) => router.go({ name: 'machine', id: machine })}
    />
  {:else if editing !== undefined}
    <Machine
      machine={editing}
      {taken}
      connection={editing.id === current.machine ? current.connection : null}
      onback={() => router.back()}
      onchanged={reload}
      onremoved={removed}
    />
  {:else}
    <NoMachines onadded={added} />
  {/if}
  <ToastStack items={toasts.items} ondismiss={(id) => toasts.dismiss(id)} below={home ? barHeight : 0} />
</main>
