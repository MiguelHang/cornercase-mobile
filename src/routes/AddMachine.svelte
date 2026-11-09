<script lang="ts">
  import { commands } from '../lib/bindings'

  let { onadded }: { onadded: (machine: number) => void } = $props()

  let url = $state('')
  let name = $state('')
  let problem = $state<string | null>(null)

  const add = async (event: SubmitEvent) => {
    event.preventDefault()
    problem = null
    const added = await commands.machineAdd(url, name)
    if (added.status === 'error') {
      problem = added.error.message
      return
    }
    onadded(added.data)
    const connected = await commands.connect(added.data)
    if (connected.status === 'error') {
      problem = connected.error.message
    }
  }
</script>

<form class="add" onsubmit={add}>
  <input type="url" placeholder="Bridge URL" autocomplete="off" bind:value={url} />
  <input type="text" placeholder="Name" autocomplete="off" bind:value={name} />
  <button type="submit">Add and connect</button>
  {#if problem !== null}
    <p class="problem">{problem}</p>
  {/if}
</form>

<style>
  .add {
    display: flex;
    flex-direction: column;
    gap: var(--inset);
    max-width: 520px;
    margin: 0 auto;
    padding: var(--inset-wide);
  }

  .problem {
    margin: 0;
    color: var(--muted);
  }
</style>
