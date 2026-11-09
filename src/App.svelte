<script lang="ts">
  import { isTauri } from '@tauri-apps/api/core'
  import { onMount } from 'svelte'
  import { commands, events } from './lib/bindings'
  import { apply, connecting, firstToConnect, screen } from './lib/connection'
  import AddMachine from './routes/AddMachine.svelte'
  import Connection from './routes/Connection.svelte'
  import NoMachines from './routes/NoMachines.svelte'

  let current = $state(screen({ machine: null, connection: { state: 'none' } }, null))
  let notice = $state<string | null>(null)

  const join = async (machine: number) => {
    current = connecting(machine)
    const connected = await commands.connect(machine)
    if (connected.status === 'error') {
      notice = connected.error.message
    }
  }

  const load = async () => {
    const [machines, state, session] = await Promise.all([
      commands.machinesList(),
      commands.connectionState(),
      commands.session(),
    ])
    current = screen(state, session)
    if (machines.status === 'error') {
      notice = machines.error.message
      return
    }
    const first = firstToConnect(machines.data, state)
    if (first !== null) {
      await join(first)
    }
  }

  onMount(() => {
    if (!isTauri()) {
      return
    }
    const listening = [
      events.connectionChanged.listen((event) => (current = apply(current, event.payload))),
      events.sessionChanged.listen((event) => (current = apply(current, event.payload))),
      events.toast.listen((event) => (notice = event.payload.message)),
    ]
    load()
    return () => listening.forEach((stop) => stop.then((unlisten) => unlisten()))
  })
</script>

<main>
  {#if current.machine === null}
    <NoMachines />
  {:else}
    <Connection screen={current} />
  {/if}
  {#if notice !== null}
    <p class="notice">{notice}</p>
  {/if}
  <AddMachine onadded={(machine) => (current = connecting(machine))} />
</main>

<style>
  .notice {
    margin: 0;
    padding: 0 var(--inset-wide);
    color: var(--muted);
    text-align: center;
  }
</style>
