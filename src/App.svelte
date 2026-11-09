<script lang="ts">
  import { isTauri } from '@tauri-apps/api/core'
  import { onMount, untrack } from 'svelte'
  import { commands, events, type MachineView_Serialize } from './lib/bindings'
  import { Chat, touches } from './lib/chat.svelte'
  import { apply, connecting, firstToConnect, screen, type Screen } from './lib/connection'
  import { oneAtATime } from './lib/join'
  import { Router, routeFor } from './lib/router.svelte'
  import { Toasts } from './lib/toasts.svelte'
  import ConnectionBar from './lib/ui/ConnectionBar.svelte'
  import ToastStack from './lib/ui/Toasts.svelte'
  import Agent from './routes/agent/Agent.svelte'
  import Board from './routes/board/Board.svelte'
  import NoMachines from './routes/NoMachines.svelte'

  type Listed = Awaited<ReturnType<typeof commands.machinesList>>

  const idle = screen({ machine: null, connection: { state: 'none' } }, null)

  let current = $state(idle)
  let machines = $state<MachineView_Serialize[]>([])
  let barHeight = $state(0)
  const router = new Router(routeFor(idle))
  const toasts = new Toasts()

  const route = $derived(router.current)
  const machine = $derived(route.name !== 'no-machines')
  const pane = $derived(route.name === 'agent' ? route.pane : null)
  const chat = $derived(pane === null ? null : new Chat(pane, commands.lastMessage, commands.send))
  const seen = $derived(chat?.row != null)
  const machineName = $derived(machines.find((machine) => machine.id === current.machine)?.name ?? '')

  $effect(() => {
    const agents = current.agents
    if (chat !== null) {
      untrack(() => chat.see(agents))
    }
  })

  $effect(() => {
    if (chat !== null && seen) {
      untrack(() => chat.refresh())
    }
  })

  const show = (next: Screen) => {
    current = next
    router.settle(routeFor(next))
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
    const [listed, state, agents] = await Promise.all([
      commands.machinesList(),
      commands.connectionState(),
      commands.agents(),
    ])
    show(screen(state, agents))
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
      events.agentsChanged.listen((event) => show(apply(current, event.payload))),
      events.paneStatusChanged.listen((event) => {
        if (chat !== null && touches(event.payload, current.machine, chat.pane)) {
          void chat.refresh()
        }
      }),
      events.toast.listen((event) => toasts.push(event.payload)),
    ]
    Promise.all(listening).then(load).catch(toasts.failed)
    return () => listening.forEach((stop) => stop.then((unlisten) => unlisten()))
  })
</script>

<main>
  {#if machine}
    <ConnectionBar connection={current.connection} {machineName} bind:height={barHeight} />
    {#if chat !== null}
      <Agent
        {chat}
        connection={current.connection}
        below={barHeight}
        onback={() => router.back()}
        onfailed={(message) => toasts.push({ level: 'error', message })}
      />
    {:else}
      <Board
        agents={current.agents}
        connection={current.connection}
        below={barHeight}
        onopen={(pane) => router.go({ name: 'agent', pane })}
      />
    {/if}
  {:else}
    <NoMachines onadded={added} />
  {/if}
  <ToastStack items={toasts.items} ondismiss={(id) => toasts.dismiss(id)} below={machine ? barHeight : 0} />
</main>
