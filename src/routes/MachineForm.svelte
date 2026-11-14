<script lang="ts">
  import { onMount } from 'svelte'
  import { AddMachine } from '../lib/addMachine.svelte'
  import { commands, events } from '../lib/bindings'
  import { placeholderName } from '../lib/machineForm'
  import Button from '../lib/ui/Button.svelte'
  import Field from '../lib/ui/Field.svelte'
  import SameOffer from '../lib/ui/SameOffer.svelte'
  import UrlField from '../lib/ui/UrlField.svelte'

  let {
    taken = [],
    onadded,
    onrouted,
    oncancel,
  }: {
    taken?: readonly string[]
    onadded: (machine: number) => void
    onrouted: (machine: number) => void
    oncancel: () => void
  } = $props()

  const form = new AddMachine(
    commands,
    (machine) => onadded(machine),
    (machine) => onrouted(machine),
    (toast) => void events.toast.emit(toast),
  )

  onMount(() => () => void form.close())

  const submit = (event: SubmitEvent) => {
    event.preventDefault()
    void form.submit(taken)
  }
</script>

<form class="form" onsubmit={submit}>
  <UrlField bind:value={form.url} error={form.urlError} disabled={form.held} onscanned={(url) => void form.fill(url, taken)} />
  <Field
    label="Name"
    bind:value={form.name}
    error={form.nameError}
    placeholder={placeholderName(form.url, taken)}
    disabled={form.held}
  />
  {#if form.formError !== null}
    <p class="problem" role="alert">{form.formError}</p>
  {/if}
  {#if form.offer === null}
    <div class="buttons">
      <Button tone="plain" resting={form.busy} onclick={oncancel}>Cancel</Button>
      <Button type="submit" busy={form.busy}>{form.busy ? 'Adding…' : 'Add'}</Button>
    </div>
  {:else}
    <SameOffer
      offer={form.offer}
      busy={form.busy}
      onroute={() => void form.addRoute()}
      onnew={() => void form.addNew()}
      onback={() => form.back()}
    />
  {/if}
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
