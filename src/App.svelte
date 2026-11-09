<script lang="ts">
  import { isTauri } from '@tauri-apps/api/core'
  import { onMount } from 'svelte'
  import { commands, events, type MachineView_Serialize } from './lib/bindings'
  import { apply, connecting, firstToConnect, screen, type AgentTab, type Screen } from './lib/connection'
  import { oneAtATime } from './lib/join'
  import { Router, routeFor } from './lib/router.svelte'
  import { Toasts } from './lib/toasts.svelte'
  import ConnectionBar from './lib/ui/ConnectionBar.svelte'
  import ToastStack from './lib/ui/Toasts.svelte'
  import Home from './routes/Home.svelte'
  import NoMachines from './routes/NoMachines.svelte'
  import Terminal from './routes/terminal/Terminal.svelte'

  type Listed = Awaited<ReturnType<typeof commands.machinesList>>

  const idle = screen({ machine: null, connection: { state: 'none' } }, null)

  let current = $state(idle)
  let machines = $state<MachineView_Serialize[]>([])
  let barHeight = $state(0)
  const router = new Router(routeFor(idle))
  const toasts = new Toasts()

  const route = $derived(router.current)
  const home = $derived(route.name === 'home')
  const machineName = $derived(machines.find((machine) => machine.id === current.machine)?.name ?? '')

  const show = (next: Screen) => {
    current = next
    router.replace(routeFor(next))
  }

  const opened = (tab: AgentTab) => {
    if (current.machine !== null && current.session !== null) {
      router.go({ name: 'terminal', machine: current.machine, boot: current.session.boot, tab })
    }
  }

  const remember = (listed: Listed) => {
    if (listed.status === 'error') {
      toasts.failed(listed.error)
      return []
    }
    machines = listed.data
    return listed.data
  }

  const join = oneAtATime(async (machine: number) => {
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
  {#if route.name === 'terminal'}
    <Terminal
      machine={route.machine}
      boot={route.boot}
      tab={route.tab}
      onback={() => router.back()}
      onfailed={toasts.failed}
      bind:height={barHeight}
    />
  {:else if home}
    <ConnectionBar connection={current.connection} {machineName} bind:height={barHeight} />
    <Home screen={current} below={barHeight} onopen={opened} />
  {:else}
    <NoMachines onadded={added} />
  {/if}
  <ToastStack
    items={toasts.items}
    ondismiss={(id) => toasts.dismiss(id)}
    below={route.name === 'no-machines' ? 0 : barHeight}
  />
</main>
