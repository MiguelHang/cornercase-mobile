<script lang="ts">
  import type { RowStatus } from '../machineRows'
  import Button from './Button.svelte'
  import Spinner from './Spinner.svelte'
  import ToneDot from './ToneDot.svelte'

  let {
    name,
    shown,
    active,
    checking,
    rejected,
    onpick,
    oncheck,
    onrescan,
    onedit,
  }: {
    name: string
    shown: RowStatus
    active: boolean
    checking: boolean
    rejected: boolean
    onpick: () => void
    oncheck: () => void
    onrescan: () => void
    onedit: () => void
  } = $props()
</script>

{#snippet about()}
  <span class="name">{name}</span>
  <span class="state">
    {#if checking}
      <Spinner />
    {:else}
      <ToneDot tone={shown.tone} pulsing={shown.pulsing} />
    {/if}
    <span class="text">{shown.text}</span>
  </span>
  {#if shown.detail !== undefined}
    <span class="detail">{shown.detail}</span>
  {/if}
{/snippet}

<li class="machine">
  <div class="head">
    {#if active}
      <div class="pick" aria-current="true">{@render about()}</div>
    {:else}
      <button class="pick" type="button" onclick={onpick} ontouchstart={() => {}}>{@render about()}</button>
      <Button tone="plain" resting={checking} onclick={oncheck}>Check again</Button>
    {/if}
    {#if rejected}
      <Button tone="plain" onclick={onrescan}>Scan again</Button>
    {/if}
    <Button tone="plain" onclick={onedit}>Edit</Button>
  </div>
  {#if shown.failures.length > 0}
    <details class="failures">
      <summary class="summary">Errors by route</summary>
      <ul class="routes">
        {#each shown.failures as failure (failure.address)}
          <li class="route">
            <span class="address">{failure.address}</span>
            <span class="message">{failure.message}</span>
          </li>
        {/each}
      </ul>
    </details>
  {/if}
</li>

<style>
  .machine {
    display: flex;
    flex-direction: column;
    gap: var(--gap);
    padding: var(--inset) var(--inset-wide);
    border-top: 1px solid var(--border);
  }

  .machine:first-child {
    border-top: 0;
  }

  .head {
    display: flex;
    align-items: center;
    gap: var(--gap);
  }

  .pick {
    display: flex;
    flex: 1;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
    min-height: 44px;
    padding: 0;
    border: 0;
    background: none;
    color: var(--text);
    font: inherit;
    text-align: left;
  }

  .name {
    overflow: hidden;
    font-size: 16px;
    font-weight: 500;
    white-space: nowrap;
    text-overflow: ellipsis;
  }

  .state {
    display: flex;
    align-items: center;
    gap: var(--gap);
    color: var(--soft);
    font-size: 14px;
  }

  .text,
  .detail {
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }

  .detail {
    color: var(--muted);
    font-size: 12px;
  }

  .summary {
    min-height: 44px;
    color: var(--soft);
    font-size: 14px;
    line-height: 44px;
  }

  .routes {
    display: flex;
    flex-direction: column;
    gap: var(--gap);
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .route {
    display: flex;
    flex-direction: column;
    gap: 2px;
    font-size: 13px;
  }

  .address {
    color: var(--text);
    font-family: var(--font-mono);
  }

  .message {
    color: var(--muted);
  }
</style>
