<script lang="ts">
  import type { Snippet } from 'svelte'
  import Spinner from './Spinner.svelte'

  let {
    tone = 'primary',
    type = 'button',
    resting = false,
    busy = false,
    compact = false,
    label,
    expanded,
    onclick,
    children,
  }: {
    tone?: 'primary' | 'plain' | 'danger'
    type?: 'button' | 'submit'
    resting?: boolean
    busy?: boolean
    compact?: boolean
    label?: string
    expanded?: boolean
    onclick?: (event: MouseEvent) => void
    children: Snippet
  } = $props()
</script>

<button
  class="button {tone}"
  class:compact
  class:resting
  class:busy
  {type}
  disabled={resting || busy}
  aria-disabled={resting ? 'true' : undefined}
  aria-busy={busy ? 'true' : undefined}
  aria-label={label}
  aria-expanded={expanded}
  {onclick}
  ontouchstart={() => {}}
>
  {#if busy}
    <Spinner />
  {/if}
  {@render children()}
</button>

<style>
  .button {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    min-height: 44px;
    padding: 0 var(--inset-wide);
    border-radius: var(--radius-control);
    font: inherit;
    font-size: 16px;
    white-space: nowrap;
  }

  .compact {
    min-width: 44px;
    padding: 0 var(--inset);
  }

  .primary {
    border: 1px solid rgb(43 123 255 / 0.7);
    background: linear-gradient(180deg, var(--brand-light), var(--brand));
    box-shadow: 0 0 10px rgb(0 95 255 / 0.28);
    color: var(--on-brand);
    font-weight: 500;
  }

  .plain {
    border: 1px solid var(--border);
    background: var(--hover);
    color: var(--soft);
  }

  .danger {
    border: 1px solid color-mix(in srgb, var(--failed) 35%, transparent);
    background: color-mix(in srgb, var(--failed) 10%, transparent);
    color: var(--failed);
  }

  .primary:not(.resting):active {
    background: var(--brand-hover);
  }

  .plain:not(.resting):active {
    border-color: var(--edge);
    background: var(--selected);
  }

  .danger:not(.resting):active {
    border-color: color-mix(in srgb, var(--failed) 70%, transparent);
    background: color-mix(in srgb, var(--failed) 18%, transparent);
  }

  .resting {
    opacity: 0.4;
    pointer-events: none;
  }

  .busy {
    pointer-events: none;
  }
</style>
