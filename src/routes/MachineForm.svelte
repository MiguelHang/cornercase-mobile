<script lang="ts">
  import { commands } from '../lib/bindings'
  import { checkUrl, fieldOf, nameFor, placeholderName } from '../lib/machineForm'
  import Button from '../lib/ui/Button.svelte'
  import Field from '../lib/ui/Field.svelte'

  let { onadded, oncancel }: { onadded: (machine: number) => void; oncancel: () => void } = $props()

  let url = $state('')
  let name = $state('')
  let urlError = $state<string | null>(null)
  let formError = $state<string | null>(null)
  let busy = $state(false)

  const add = async () => {
    const checked = checkUrl(url)
    if (!checked.ok) {
      urlError = checked.error
      return
    }
    busy = true
    const added = await commands.machineAdd(checked.url, nameFor(name, url))
    busy = false
    if (added.status === 'ok') {
      onadded(added.data)
    } else if (fieldOf(added.error) === 'url') {
      urlError = added.error.message
    } else {
      formError = added.error.message
    }
  }

  const submit = (event: SubmitEvent) => {
    event.preventDefault()
    if (busy) {
      return
    }
    urlError = null
    formError = null
    add().catch((error: unknown) => {
      busy = false
      formError = String(error)
    })
  }
</script>

<form class="form" onsubmit={submit}>
  <Field label="Bridge URL" url bind:value={url} error={urlError} disabled={busy} />
  <Field label="Name" bind:value={name} placeholder={placeholderName(url)} disabled={busy} />
  {#if formError !== null}
    <p class="problem" role="alert">{formError}</p>
  {/if}
  <div class="buttons">
    <Button tone="plain" resting={busy} onclick={oncancel}>Cancel</Button>
    <Button type="submit" {busy}>{busy ? 'Adding…' : 'Add'}</Button>
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
