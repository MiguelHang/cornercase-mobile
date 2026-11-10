<script lang="ts" generics="T, V">
  import type { Choices } from '../../lib/startForm.svelte'
  import Button from '../../lib/ui/Button.svelte'
  import Labelled from '../../lib/ui/Labelled.svelte'
  import Spinner from '../../lib/ui/Spinner.svelte'

  let {
    label,
    choices,
    value = $bindable(),
    of,
    error,
    reading = '',
    empty,
    disabled = false,
    onretry,
  }: {
    label: string
    choices: Choices<T>
    value: V
    of: (item: T) => { value: V; name: string; disabled?: boolean }
    error: string | null
    reading?: string
    empty: string
    disabled?: boolean
    onretry?: () => void
  } = $props()

  const id = $props.id()
  const options = $derived(choices.kind === 'ready' ? choices.items.map(of) : [])
  const chosen = $derived(options.some((option) => option.value === value))
</script>

<Labelled {id} {label} {error} control={options.length > 0}>
  {#if options.length > 0}
    <select
      id="{id}-input"
      class="input"
      {disabled}
      aria-invalid={error === null ? undefined : 'true'}
      aria-describedby={error === null ? undefined : `${id}-error`}
      bind:value
    >
      {#if !chosen}
        <option value={value} disabled>Choose…</option>
      {/if}
      {#each options as option (option.value)}
        <option value={option.value} disabled={option.disabled}>{option.name}</option>
      {/each}
    </select>
  {:else}
    {#if choices.kind === 'loading'}
      <p class="state"><Spinner />{reading}</p>
    {:else if choices.kind === 'failed'}
      <div class="failed">
        <p class="state reason">{choices.reason}</p>
        {#if onretry !== undefined}
          <Button tone="plain" onclick={onretry}>Try again</Button>
        {/if}
      </div>
    {:else}
      <p class="state">{empty}</p>
    {/if}
  {/if}
</Labelled>

<style>
  .input {
    box-sizing: border-box;
    min-height: 44px;
    padding: 0 12px;
    border: 1px solid var(--edge);
    border-radius: var(--radius-control);
    outline: none;
    background: var(--field);
    color: var(--text);
    font: inherit;
    font-size: 16px;
  }

  .input:focus {
    border-color: var(--accent);
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--brand) 32%, transparent);
  }

  .input:disabled {
    opacity: 0.6;
  }

  .state {
    display: flex;
    align-items: center;
    gap: 6px;
    min-height: 44px;
    margin: 0;
    color: var(--muted);
    font-size: 14px;
  }

  .failed {
    display: flex;
    align-items: center;
    gap: var(--gap);
  }

  .reason {
    flex: 1;
    min-width: 0;
    color: var(--failed);
    overflow-wrap: anywhere;
  }
</style>
