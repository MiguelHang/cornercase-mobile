<script lang="ts">
  import Button from '../lib/ui/Button.svelte'
  import Emblem from '../lib/ui/Emblem.svelte'
  import Glass from '../lib/ui/Glass.svelte'
  import Mark from '../lib/ui/Mark.svelte'
  import Wordmark from '../lib/ui/Wordmark.svelte'
  import { fadeOut, riseIn } from '../lib/ui/motion'
  import MachineForm from './MachineForm.svelte'

  let { onadded, open = false }: { onadded: (machine: number) => void; open?: boolean } = $props()

  const LINES = [
    'A machine is a computer running cornercase whose agents you follow from here.',
    'It needs the cornercase bridge running on your Mac, and the URL it prints.',
  ]
</script>

<section class="screen">
  <header class="brand">
    <Mark unit={5} />
    <h1 class="name"><Wordmark size={22} /></h1>
  </header>
  <div class="card" in:riseIn out:fadeOut>
    <Glass>
      <Emblem icon="server" title="No machines yet" lines={LINES} />
      {#if open}
        <MachineForm {onadded} oncancel={() => (open = false)} />
      {:else}
        <div class="actions">
          <Button onclick={() => (open = true)}>Add machine</Button>
        </div>
      {/if}
    </Glass>
  </div>
</section>

<style>
  .screen {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 32px;
    padding: 40px var(--inset-wide);
  }

  .brand {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--inset);
  }

  .name {
    margin: 0;
    font-size: inherit;
  }

  .card {
    width: 100%;
    max-width: 520px;
  }

  .actions {
    display: flex;
    justify-content: center;
    padding: 0 var(--inset-card) var(--inset-card);
  }
</style>
