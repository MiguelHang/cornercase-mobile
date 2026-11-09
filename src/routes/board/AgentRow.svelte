<script lang="ts">
  import type { AgentRow } from '../../lib/bindings'
  import { elapsed, label, place } from '../../lib/board'
  import StatusDot from '../../lib/ui/StatusDot.svelte'

  let { row, now, onopen }: { row: AgentRow; now: number; onopen: (pane: number) => void } = $props()

  const time = $derived(elapsed(row.since, now))
</script>

<button class="row" type="button" onclick={() => onopen(row.pane)} ontouchstart={() => {}}>
  <StatusDot status={row.status} />
  <span class="lines">
    <span class="title">{label(row)}</span>
    <span class="place">{place(row)}</span>
  </span>
  {#if time !== ''}
    <span class="time">{time}</span>
  {/if}
</button>

<style>
  .row {
    box-sizing: border-box;
    display: flex;
    align-items: center;
    gap: var(--inset);
    width: 100%;
    min-height: 44px;
    padding: var(--gap) var(--inset);
    border: none;
    border-radius: var(--radius-control);
    background: none;
    color: var(--text);
    font: inherit;
    text-align: left;
  }

  .row:active {
    background: var(--hover);
  }

  .lines {
    display: flex;
    flex: 1;
    flex-direction: column;
    min-width: 0;
  }

  .title,
  .place {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .place {
    color: var(--muted);
    font-size: 13px;
  }

  .time {
    flex: none;
    color: var(--faint);
    font-size: 13px;
    font-variant-numeric: tabular-nums;
  }
</style>
