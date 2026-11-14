<script lang="ts">
  import type { Scanner } from '../scanner.svelte'
  import Button from './Button.svelte'
  import { overPage } from './overPage'

  let { scanner }: { scanner: Scanner } = $props()
</script>

{#if scanner.state === 'scanning'}
  <div class="layer" {@attach overPage}>
    <div class="frame"></div>
    <p class="hint">Point the camera at the QR code in cornercase › Settings › Remote › Phone app.</p>
    <Button tone="plain" onclick={() => void scanner.stop()}>Cancel</Button>
  </div>
{:else if scanner.state === 'denied'}
  <div class="notice" role="alert">
    <p class="text">Camera access is off. Turn it on in Settings, or paste the code below.</p>
    <Button tone="plain" onclick={() => void scanner.openSettings()}>Open Settings</Button>
  </div>
{:else if scanner.state === 'failed'}
  <div class="notice" role="alert">
    <p class="text">Could not scan the code. Paste the code below.</p>
    <p class="detail">{scanner.error}</p>
  </div>
{/if}

<style>
  .layer {
    position: fixed;
    inset: 0;
    z-index: 20;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 24px;
    padding: calc(env(safe-area-inset-top) + var(--inset-wide)) var(--inset-wide)
      calc(env(safe-area-inset-bottom) + var(--inset-wide));
    color: var(--text);
  }

  .frame {
    width: min(72vw, 280px);
    aspect-ratio: 1;
    border: 2px solid var(--text);
    border-radius: var(--radius-card);
    box-shadow: 0 0 0 100vmax var(--veil);
  }

  .hint {
    max-width: 280px;
    margin: 0;
    font-size: 15px;
    text-align: center;
    text-shadow: 0 1px 4px rgb(0 0 0 / 0.6);
  }

  .notice {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: var(--gap);
    padding: var(--inset);
    border: 1px solid var(--border);
    border-radius: var(--radius-control);
    background: var(--panel);
  }

  .text {
    margin: 0;
    color: var(--soft);
    font-size: 14px;
  }

  .detail {
    margin: 0;
    color: var(--failed);
    font-size: 13px;
    overflow-wrap: anywhere;
  }
</style>
