<script lang="ts">
  import type { Snippet } from 'svelte'
  import Glass from './Glass.svelte'
  import { sheetIn, sheetOut, veilIn, veilOut } from './motion'
  import { overPage } from './overPage'

  let { label, oncancel, children }: { label: string; oncancel: () => void; children: Snippet } = $props()

  const focused = (node: HTMLElement) => node.focus({ preventScroll: true })
</script>

<div class="layer" {@attach overPage}>
  <button class="scrim" type="button" tabindex="-1" aria-hidden="true" onclick={oncancel} in:veilIn out:veilOut></button>
  <div class="sheet" role="dialog" aria-modal="true" aria-label={label} tabindex="-1" {@attach focused} in:sheetIn out:sheetOut>
    <Glass sheet>
      <div class="body">
        {@render children()}
      </div>
    </Glass>
  </div>
</div>

<style>
  .layer {
    position: fixed;
    inset: 0;
    z-index: 8;
    display: flex;
    flex-direction: column;
    justify-content: flex-end;
  }

  .scrim {
    position: absolute;
    inset: 0;
    padding: 0;
    border: 0;
    background: var(--veil);
    touch-action: none;
  }

  .sheet {
    position: relative;
    width: 100%;
    max-width: 520px;
    margin: 0 auto;
    outline: none;
  }

  .body {
    display: flex;
    flex-direction: column;
    gap: var(--gap);
    padding: var(--inset-wide) var(--inset-wide) calc(env(safe-area-inset-bottom) + var(--inset-wide));
  }
</style>
