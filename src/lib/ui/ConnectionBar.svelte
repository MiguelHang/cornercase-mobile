<script lang="ts">
  import type { ConnectionView_Serialize } from '../bindings'
  import { status } from '../connection'

  const TICK_MS = 1000

  let {
    connection,
    machineName,
    height = $bindable(0),
  }: {
    connection: ConnectionView_Serialize
    machineName: string
    height?: number
  } = $props()

  let elapsed = $state(0)

  $effect(() => {
    elapsed = 0
    if (connection.state !== 'retrying') {
      return
    }
    const since = Date.now()
    const timer = setInterval(() => (elapsed = Date.now() - since), TICK_MS)
    return () => clearInterval(timer)
  })

  const shown = $derived(status(connection, machineName, elapsed))
  const counting = $derived(connection.state === 'retrying')
</script>

<div class="connection" role="status">
  <div class="row" bind:clientHeight={height}>
    <span class="dot {shown.tone}" class:pulsing={shown.pulsing}></span>
    <span class="lines">
      <span class="line text" aria-hidden={counting ? 'true' : undefined}>{shown.text}</span>
      {#if counting}
        <span class="spoken">Retrying</span>
      {/if}
      {#if shown.detail !== undefined}
        <span class="line detail">{shown.detail}</span>
      {/if}
    </span>
  </div>
</div>

<style>
  .connection {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    z-index: 5;
    padding-top: env(safe-area-inset-top);
    border-bottom: 1px solid var(--border);
    background: var(--veil);
    -webkit-backdrop-filter: blur(24px);
    backdrop-filter: blur(24px);
  }

  .row {
    box-sizing: border-box;
    display: flex;
    align-items: center;
    gap: 10px;
    min-height: 44px;
    padding: var(--gap) var(--inset-wide);
  }

  .dot {
    flex: none;
    width: 8px;
    height: 8px;
    border-radius: 999px;
  }

  .working {
    background: var(--working);
  }

  .done {
    background: var(--done);
  }

  .failed {
    background: var(--failed);
  }

  .lines {
    display: flex;
    flex: 1;
    flex-direction: column;
    min-width: 0;
  }

  .line {
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }

  .text {
    color: var(--text);
    font-size: 14px;
  }

  .detail {
    color: var(--muted);
    font-size: 12px;
  }
</style>
