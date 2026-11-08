<script lang="ts">
  import type { Snippet } from 'svelte'

  let { resting = false, children }: { resting?: boolean; children: Snippet } = $props()
</script>

<button class="button" class:resting type="button" disabled={resting} aria-disabled={resting ? 'true' : undefined}>
  {@render children()}
</button>

<style>
  .button {
    min-height: 44px;
    padding: 0 var(--inset-wide);
    border: 1px solid rgb(43 123 255 / 0.7);
    border-radius: var(--radius-control);
    background: linear-gradient(180deg, var(--brand-light), var(--brand));
    box-shadow: 0 0 10px rgb(0 95 255 / 0.28);
    color: var(--on-brand);
    font: inherit;
    font-size: 16px;
    font-weight: 500;
    white-space: nowrap;
  }

  .button:not(.resting):active {
    background: var(--brand-hover);
  }

  .resting {
    opacity: 0.4;
    pointer-events: none;
  }
</style>
