<script lang="ts">
  import { isTauri } from '@tauri-apps/api/core'
  import { onMount, untrack } from 'svelte'
  import {
    commands,
    events,
    type AgentRow_Serialize as AgentRow,
    type MachineView_Serialize,
    type Started_Serialize as Started,
  } from './lib/bindings'
  import { Chat, touches } from './lib/chat.svelte'
  import { apply, connecting, firstToConnect, screen, type Screen } from './lib/connection'
  import { latestWins } from './lib/join'
  import { openedTerminal, Router, routeFor } from './lib/router.svelte'
  import { Toasts } from './lib/toasts.svelte'
  import ConnectionBar from './lib/ui/ConnectionBar.svelte'
  import ToastStack from './lib/ui/Toasts.svelte'
  import Agent from './routes/agent/Agent.svelte'
  import Board from './routes/board/Board.svelte'
  import Machine from './routes/Machine.svelte'
  import Machines from './routes/Machines.svelte'
  import NoMachines from './routes/NoMachines.svelte'
  import Start from './routes/start/Start.svelte'
  import Terminal from './routes/terminal/Terminal.svelte'

  type Listed = Awaited<ReturnType<typeof commands.machinesList>>

  const idle = screen({ machine: null, connection: { state: 'none' } }, null)

  let current = $state(idle)
  let machines = $state<MachineView_Serialize[]>([])
  let barHeight = $state(0)
  let launched = $state('')
  const router = new Router(routeFor(idle, 0))
  const toasts = new Toasts()

  const route = $derived(router.current)
  const barred = $derived(route.name === 'home' || route.name === 'agent' || route.name === 'terminal')
  const opening = $derived(router.opened)
  const pane = $derived(opening?.pane ?? null)
  const owner = $derived(opening?.machine ?? null)
  const chat = $derived(pane === null ? null : new Chat(owner, pane, commands.lastMessage, commands.send))
  const seen = $derived(chat?.row != null)
  const viewed = $derived(openedTerminal(route, current))
  const editing = $derived(route.name === 'machine' ? machines.find((machine) => machine.id === route.id) : undefined)
  const rescanning = $derived(route.name === 'machine' && route.adding === true)
  const taken = $derived(machines.filter((machine) => machine.id !== editing?.id).map((machine) => machine.name))
  const machineName = $derived(machines.find((machine) => machine.id === current.machine)?.name ?? '')
  const starting = $derived(route.name === 'agent' && route.starting === true ? launched : null)

  $effect(() => {
    const { machine, agents } = current
    if (chat !== null) {
      const waiting = starting !== null
      untrack(() => chat.see(machine, agents, waiting))
    }
  })

  $effect(() => {
    if (chat !== null && seen) {
      untrack(() => chat.refresh())
    }
  })

  const show = (next: Screen) => {
    current = next
    router.setBase(routeFor(next, machines.length))
  }

  const terminal = (row: AgentRow) => {
    if (current.machine !== null) {
      router.go({ name: 'terminal', machine: current.machine, row })
    }
  }

  const start = () => {
    if (current.machine !== null) {
      router.go({ name: 'start', machine: current.machine })
    }
  }

  const started = (placed: Started, agent: string) => {
    const at = router.current
    if (at.name === 'start') {
      launched = agent
      router.replace({ name: 'agent', machine: at.machine, pane: placed.pane, starting: true })
    } else {
      toasts.push({ level: 'info', message: `${agent} started` })
    }
    if (placed.warning !== null) {
      toasts.push({ level: 'info', message: placed.warning })
    }
  }

  const created = (name: string) => {
    toasts.push({ level: 'info', message: `Workspace ${name} created` })
    if (router.current.name === 'start') {
      router.back()
    }
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
    const [listed, state, agents] = await Promise.all([
      commands.machinesList(),
      commands.connectionState(),
      commands.agents(),
    ])
    const known = remember(listed)
    show(screen(state, agents))
    const first = firstToConnect(known, state)
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
        if (chat !== null && touches(event.payload, chat.machine, chat.pane)) {
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
  {#if viewed !== undefined}
    <Terminal
      machine={viewed.machine}
      row={viewed.row}
      onback={() => router.back()}
      onfailed={toasts.failed}
      bind:height={barHeight}
    />
  {:else if barred}
    <ConnectionBar
      connection={current.connection}
      {machineName}
      bind:height={barHeight}
      onmachines={() => router.go({ name: 'machines' })}
    />
    {#if route.name === 'home'}
      <Board
        agents={current.agents}
        connection={current.connection}
        below={barHeight}
        onopen={(pane) => router.go({ name: 'agent', machine: current.machine, pane })}
        onstart={start}
      />
    {:else if chat !== null}
      <Agent
        {chat}
        connection={current.connection}
        below={barHeight}
        {starting}
        onback={() => router.back()}
        onfailed={(message) => toasts.push({ level: 'error', message })}
        onterminal={terminal}
      />
    {/if}
  {:else if route.name === 'start'}
    <Start
      connection={current.connection}
      onback={() => router.back()}
      onstarted={started}
      oncreated={created}
    />
  {:else if route.name === 'machines'}
    <Machines
      {machines}
      active={current.machine}
      connection={current.connection}
      onback={() => router.back()}
      onpick={(machine) => void join(machine)}
      onadded={added}
      onrouted={reload}
      onedit={(machine) => router.go({ name: 'machine', id: machine })}
      onrescan={(machine) => router.go({ name: 'machine', id: machine, adding: true })}
    />
  {:else if editing !== undefined}
    <Machine
      machine={editing}
      {taken}
      connection={editing.id === current.machine ? current.connection : null}
      onback={() => router.back()}
      onchanged={reload}
      onremoved={removed}
      adding={rescanning}
    />
  {:else}
    <NoMachines onadded={added} />
  {/if}
  <ToastStack items={toasts.items} ondismiss={(id) => toasts.dismiss(id)} below={barred ? barHeight : 0} />
</main>
