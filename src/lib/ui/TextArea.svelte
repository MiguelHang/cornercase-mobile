<script lang="ts">
  import type { HTMLTextareaAttributes } from 'svelte/elements'

  let {
    label,
    value = $bindable(''),
    error,
    ...rest
  }: {
    label: string
    value?: string
    error?: string | null
  } & Omit<HTMLTextareaAttributes, 'value'> = $props()

  const id = $props.id()
  const field = $derived(error !== undefined)
</script>

{#snippet area()}
  <div class="grow" data-value={value}>
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
  <div class="field">
    <label class="label" for="{id}-input">{label}</label>
    {@render area()}
    <p class="error" id="{id}-error">{error ?? ''}</p>
  </div>
{:else}
  {@render area()}
{/if}

<style>
  .field {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .label {
    color: var(--soft);
    font-size: 13px;
    font-weight: 500;
  }

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
    padding: 10px 12px;
    border: 1px solid var(--edge);
    font: inherit;
    font-size: 16px;
    line-height: 22px;
  }

  .area {
    overflow-y: auto;
    resize: none;
    border-radius: var(--radius-control);
    outline: none;
    background: var(--field);
    color: var(--text);
    -webkit-user-select: text;
    user-select: text;
  }

  .area:focus {
    border-color: var(--accent);
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--brand) 32%, transparent);
  }

  .area:disabled {
    opacity: 0.6;
  }

  .area::placeholder {
    color: var(--faint);
  }

  .error {
    min-height: 16px;
    margin: 0;
    color: var(--failed);
    font-size: 13px;
    line-height: 16px;
  }
</style>
