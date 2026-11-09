<script lang="ts">
  import { onMount, untrack } from 'svelte'
  import { commands, type ConnectionView_Serialize, type MachineView_Serialize } from '../lib/bindings'
  import { rowStatus } from '../lib/machineRows'
  import { Probes } from '../lib/probes.svelte'
  import { retryClock } from '../lib/retryClock.svelte'
  import { ThisDevice } from '../lib/thisDevice.svelte'
  import Button from '../lib/ui/Button.svelte'
  import Glass from '../lib/ui/Glass.svelte'
  import Icon from '../lib/ui/Icon.svelte'
  import MachineRow from '../lib/ui/MachineRow.svelte'
  import MachineForm from './MachineForm.svelte'

  let {
    machines,
    active,
    connection,
    onback,
    onpick,
    onadded,
    probes = new Probes(commands),
    device = new ThisDevice(commands),
  }: {
    machines: MachineView_Serialize[]
    active: number | null
    connection: ConnectionView_Serialize
    onback: () => void
    onpick: (machine: number) => void
    onadded: (machine: number) => void
    probes?: Probes
    device?: ThisDevice
  } = $props()

  const clock = retryClock(() => connection)

  let adding = $state(false)

  const shownFor = (machine: MachineView_Serialize) =>
    rowStatus(machine, machine.id === active ? connection : null, probes.of(machine.id), clock.elapsed)

  const added = (machine: number) => {
    adding = false
    onadded(machine)
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
  <header class="top">
    <button class="back" type="button" aria-label="Back" onclick={onback} ontouchstart={() => {}}>
      <Icon name="chevron-left" size={24} />
    </button>
    <h1 class="title">Machines</h1>
  </header>
  <Glass>
    <ul class="list">
      {#each machines as machine (machine.id)}
        <MachineRow
          name={machine.name}
          shown={shownFor(machine)}
          active={machine.id === active}
          checking={machine.id !== active && probes.of(machine.id).state === 'checking'}
          onpick={() => onpick(machine.id)}
          oncheck={() => void probes.check([machine.id])}
        />
      {/each}
    </ul>
  </Glass>
  {#if adding}
    <Glass>
      <h2 class="heading adding">Add machine</h2>
      <MachineForm onadded={added} oncancel={() => (adding = false)} />
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

  .top {
    display: flex;
    align-items: center;
    gap: var(--gap);
    margin-left: calc(-1 * var(--inset));
  }

  .back {
    display: flex;
    flex: none;
    align-items: center;
    justify-content: center;
    width: 44px;
    height: 44px;
    padding: 0;
    border: 0;
    border-radius: var(--radius-control);
    background: none;
    color: var(--accent);
  }

  .back:active {
    background: var(--hover);
  }

  .title {
    margin: 0;
    font-size: 22px;
    font-weight: 600;
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
