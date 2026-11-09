<script lang="ts">
  import type { Shown } from '../toasts'
  import Glass from './Glass.svelte'
  import { dropIn, fadeOut } from './motion'

  let {
    items,
    ondismiss,
    below = 0,
  }: {
    items: Shown[]
    ondismiss: (id: number) => void
    below?: number
  } = $props()
</script>

<div class="toasts" style:--below="{below}px" aria-live="polite">
  {#each items as toast (toast.id)}
    <button
      class="toast {toast.level}"
      type="button"
      onclick={() => ondismiss(toast.id)}
      ontouchstart={() => {}}
      in:dropIn
      out:fadeOut
    >
      <Glass>
        <span class="body">
          <span class="bar"></span>
          <span class="text">{toast.message}</span>
          {#if toast.count > 1}
            <span class="count">×{toast.count}</span>
          {/if}
        </span>
      </Glass>
    </button>
  {/each}
</div>

<style>
  .toasts {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    z-index: 10;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--gap);
    padding: calc(env(safe-area-inset-top) + var(--below) + var(--gap)) var(--inset-wide) 0;
    pointer-events: none;
  }

  .toast {
    width: 100%;
    max-width: 520px;
    padding: 0;
    border: 0;
    background: none;
    color: var(--text);
    font: inherit;
    text-align: left;
    pointer-events: auto;
  }

  .body {
    box-sizing: border-box;
    display: flex;
    align-items: center;
    gap: 10px;
    min-height: 44px;
    padding: 10px 14px;
  }

  .bar {
    flex: none;
    width: 3px;
    height: 18px;
    border-radius: 999px;
  }

  .info .bar {
    background: var(--accent);
    box-shadow: 0 0 8px rgb(116 163 255 / 0.6);
  }

  .error .bar {
    background: var(--failed);
    box-shadow: 0 0 8px rgb(255 107 139 / 0.6);
  }

  .text {
    flex: 1;
    min-width: 0;
    font-size: 14px;
    overflow-wrap: anywhere;
  }

  .count {
    flex: none;
    padding: 0 6px;
    border-radius: var(--radius-chip);
    background: var(--selected);
    color: var(--muted);
    font-size: 12px;
  }
</style>
