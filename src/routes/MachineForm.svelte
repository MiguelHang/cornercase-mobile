<script lang="ts">
  import { AddMachine } from '../lib/addMachine.svelte'
  import { commands } from '../lib/bindings'
  import { placeholderName } from '../lib/machineForm'
  import Button from '../lib/ui/Button.svelte'
  import Field from '../lib/ui/Field.svelte'
  import UrlField from '../lib/ui/UrlField.svelte'

  let { onadded, oncancel }: { onadded: (machine: number) => void; oncancel: () => void } = $props()

  const form = new AddMachine(commands.machineAdd, (machine) => onadded(machine))

  const submit = (event: SubmitEvent) => {
    event.preventDefault()
    void form.submit()
  }
</script>

<form class="form" onsubmit={submit}>
  <UrlField bind:value={form.url} error={form.urlError} disabled={form.busy} onscanned={(url) => void form.fill(url)} />
  <Field label="Name" bind:value={form.name} placeholder={placeholderName(form.url)} disabled={form.busy} />
  {#if form.formError !== null}
    <p class="problem" role="alert">{form.formError}</p>
  {/if}
  <div class="buttons">
    <Button tone="plain" resting={form.busy} onclick={oncancel}>Cancel</Button>
    <Button type="submit" busy={form.busy}>{form.busy ? 'Adding…' : 'Add'}</Button>
  </div>
</form>

<style>
  .form {
    display: flex;
    flex-direction: column;
    gap: var(--gap);
    padding: 0 var(--inset-card) var(--inset-card);
  }

  .problem {
    margin: 0;
    color: var(--failed);
    font-size: 13px;
  }

  .buttons {
    display: flex;
    justify-content: flex-end;
    gap: var(--gap);
    padding-top: var(--gap);
  }
</style>
