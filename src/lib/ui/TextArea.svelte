<script lang="ts">
  import type { HTMLTextareaAttributes } from 'svelte/elements'
  import Labelled from './Labelled.svelte'

  let {
    label,
    value = $bindable(''),
    error,
    bare = false,
    ...rest
  }: {
    label: string
    value?: string
    error?: string | null
    bare?: boolean
  } & Omit<HTMLTextareaAttributes, 'value'> = $props()

  const id = $props.id()
  const field = $derived(error !== undefined)
</script>

{#snippet area()}
  <div class="grow" class:framed={!bare} data-value={value}>
    <textarea
      id={field ? `${id}-input` : undefined}
      class="area"
      rows="1"
      aria-label={field ? undefined : label}
      {...rest}
      aria-invalid={error ? 'true' : undefined}
      aria-describedby={error ? `${id}-error` : undefined}
      bind:value
    ></textarea>
  </div>
{/snippet}

{#if field}
  <Labelled {id} {label} error={error ?? null}>
    {@render area()}
  </Labelled>
{:else}
  {@render area()}
{/if}

<style>
  .grow {
    display: grid;
    min-width: 0;
  }

  .grow::after {
    content: attr(data-value) ' ';
    visibility: hidden;
    overflow: hidden;
    white-space: pre-wrap;
    overflow-wrap: break-word;
  }

  .area,
  .grow::after {
    grid-area: 1 / 1;
    box-sizing: border-box;
    min-height: 44px;
    max-height: 152px;
    padding: 11px 4px;
    border: none;
    font: inherit;
    font-size: 16px;
    line-height: 22px;
  }

  .framed .area,
  .framed::after {
    padding: 10px 12px;
    border: 1px solid var(--edge);
  }

  .area {
    overflow-y: auto;
    resize: none;
    outline: none;
    background: none;
    color: var(--text);
    -webkit-user-select: text;
    user-select: text;
  }

  .framed .area {
    border-radius: var(--radius-control);
    background: var(--field);
  }

  .framed .area:focus {
    border-color: var(--accent);
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--brand) 32%, transparent);
  }

  .area:disabled {
    opacity: 0.6;
  }

  .area::placeholder {
    color: var(--faint);
  }
</style>
