<script lang="ts">
  import { onMount, untrack } from 'svelte'
  import { commands, type ConnectionView_Serialize, type MachineView_Serialize } from '../lib/bindings'
  import { rescannable, rowStatus } from '../lib/machineRows'
  import { Probes } from '../lib/probes.svelte'
  import { retryClock } from '../lib/retryClock.svelte'
  import { ThisDevice } from '../lib/thisDevice.svelte'
  import Button from '../lib/ui/Button.svelte'
  import Glass from '../lib/ui/Glass.svelte'
  import MachineRow from '../lib/ui/MachineRow.svelte'
  import NavBar from '../lib/ui/NavBar.svelte'
  import MachineForm from './MachineForm.svelte'

  let {
    machines,
    active,
    connection,
    backTo,
    onback,
    onpick,
    onadded,
    onrouted,
    onedit,
    onrescan,
    probes = new Probes(commands),
    device = new ThisDevice(commands),
  }: {
    machines: MachineView_Serialize[]
    active: number | null
    connection: ConnectionView_Serialize
    backTo: string | null
    onback: () => void
    onpick: (machine: number) => void
    onadded: (machine: number) => void
    onrouted: () => void
    onedit: (machine: number) => void
    onrescan: (machine: number) => void
    probes?: Probes
    device?: ThisDevice
  } = $props()

  const clock = retryClock(() => connection)

  let adding = $state(false)

  const connectionOf = (machine: MachineView_Serialize) => (machine.id === active ? connection : null)

  const shownFor = (machine: MachineView_Serialize) =>
    rowStatus(machine, connectionOf(machine), probes.of(machine.id), clock.elapsed)

  const added = (machine: number) => {
    adding = false
    onadded(machine)
  }

  const routed = (machine: number) => {
    adding = false
    if (machine !== active) {
      void probes.check([machine])
    }
    onrouted()
  }

  onMount(() => {
    void probes.open(
      machines.map(({ id }) => id),
      active,
    )
    void device.load()
  })

  $effect(() => {
    const now = active
    untrack(() => void probes.follow(now))
  })
</script>

<section class="screen">
  <NavBar {backTo} title="Machines" {onback} />
  <Glass>
    <ul class="list">
      {#each machines as machine (machine.id)}
        <MachineRow
          name={machine.name}
          shown={shownFor(machine)}
          active={machine.id === active}
          checking={machine.id !== active && probes.of(machine.id).state === 'checking'}
          rejected={rescannable(connectionOf(machine), probes.of(machine.id))}
          onpick={() => onpick(machine.id)}
          oncheck={() => void probes.check([machine.id])}
          onrescan={() => onrescan(machine.id)}
          onedit={() => onedit(machine.id)}
        />
      {/each}
    </ul>
  </Glass>
  {#if adding}
    <Glass>
      <h2 class="heading adding">Add machine</h2>
      <MachineForm onadded={added} onrouted={routed} oncancel={() => (adding = false)} />
    </Glass>
  {:else}
    <div class="actions">
      <Button onclick={() => (adding = true)}>Add machine</Button>
    </div>
  {/if}
  <section class="device">
    <h2 class="heading">This device</h2>
    {#if device.shown?.kind === 'done'}
      <p class="label">Identity fingerprint</p>
      <code class="fingerprint">{device.shown.data}</code>
    {:else if device.shown !== null}
      <p class="reason">{device.shown.error}</p>
    {/if}
  </section>
</section>

<style>
  .screen {
    display: flex;
    flex-direction: column;
    gap: var(--inset-wide);
    max-width: 520px;
    margin: 0 auto;
    padding: calc(env(safe-area-inset-top) + var(--gap)) var(--inset-wide) 40px;
  }

  .list {
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .heading {
    margin: 0;
    color: var(--soft);
    font-size: 13px;
    font-weight: 600;
  }

  .adding {
    padding: var(--inset-card) var(--inset-card) var(--gap);
  }

  .actions {
    display: flex;
    justify-content: center;
  }

  .device {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 0 var(--inset);
  }

  .label,
  .reason {
    margin: 0;
    color: var(--muted);
    font-size: 13px;
  }

  .fingerprint {
    color: var(--text);
    font-family: var(--font-mono);
    font-size: 16px;
    letter-spacing: 0.05em;
  }
</style>
