<script lang="ts">
  import type { ConnectionView_Serialize } from '../bindings'
  import { status } from '../connection'
  import { retryClock } from '../retryClock.svelte'
  import Icon from './Icon.svelte'
  import ToneDot from './ToneDot.svelte'

  let {
    connection,
    machineName,
    height = $bindable(0),
    onmachines,
  }: {
    connection: ConnectionView_Serialize
    machineName: string
    height?: number
    onmachines: () => void
  } = $props()

  const clock = retryClock(() => connection)

  const shown = $derived(status(connection, machineName, clock.elapsed))
  const counting = $derived(connection.state === 'retrying')
</script>

<div class="connection" role="status">
  <div class="row" bind:clientHeight={height}>
    <ToneDot tone={shown.tone} pulsing={shown.pulsing} />
    <span class="lines">
      <span class="line text" aria-hidden={counting ? 'true' : undefined}>{shown.text}</span>
      {#if counting}
        <span class="spoken">Retrying</span>
      {/if}
      {#if shown.detail !== undefined}
        <span class="line detail">{shown.detail}</span>
      {/if}
    </span>
    <button class="machines" type="button" aria-label="Machines" onclick={onmachines} ontouchstart={() => {}}>
      <Icon name="server" size={20} />
    </button>
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

  .machines {
    display: flex;
    flex: none;
    align-items: center;
    justify-content: center;
    width: 44px;
    height: 44px;
    margin: calc(-1 * var(--gap)) calc(-1 * var(--gap)) calc(-1 * var(--gap)) 0;
    padding: 0;
    border: 0;
    border-radius: var(--radius-control);
    background: none;
    color: var(--soft);
  }

  .machines:active {
    background: var(--hover);
  }
</style>
