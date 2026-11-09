<script lang="ts">
  import type { AgentRow_Serialize as Agent } from '../../lib/bindings'
  import { elapsed, label, place, type Row } from '../../lib/board'
  import StatusDot from '../../lib/ui/StatusDot.svelte'

  let {
    item,
    agents,
    now,
    onopen,
    onfold,
  }: {
    item: Row
    agents: Agent[]
    now: number
    onopen: (pane: number) => void
    onfold: (pane: number) => void
  } = $props()

  const row = $derived(item.row)
  const time = $derived(elapsed(row.since, now))
</script>

<div class="agent" style:--depth={item.depth}>
  <button class="row" type="button" onclick={() => onopen(row.pane)} ontouchstart={() => {}}>
    <StatusDot status={row.status} />
    <span class="lines">
      <span class="title">{label(row, agents)}</span>
      <span class="place">{place(row, agents)}</span>
    </span>
    {#if time !== ''}
      <span class="time">{time}</span>
    {/if}
    {#if item.attention !== null}
      <StatusDot status={item.attention} size={10} />
    {/if}
    {#if item.hidden > 0}
      <span class="hidden">+{item.hidden}</span>
    {/if}
  </button>
  {#if item.hasChildren}
    <button
      class="fold"
      type="button"
      aria-label="Subagents"
      aria-expanded={!item.folded}
      onclick={() => onfold(row.pane)}
      ontouchstart={() => {}}
    >
      {item.folded ? '▸' : '▾'}
    </button>
  {/if}
</div>

<style>
  .agent {
    display: flex;
    align-items: center;
    padding-left: calc(var(--depth) * var(--inset-wide));
  }

  .row,
  .fold {
    box-sizing: border-box;
    min-height: 44px;
    border: none;
    border-radius: var(--radius-control);
    background: none;
    color: var(--text);
    font: inherit;
  }

  .row:active,
  .fold:active {
    background: var(--hover);
  }

  .row {
    display: flex;
    flex: 1;
    align-items: center;
    gap: var(--inset);
    min-width: 0;
    padding: var(--gap) var(--inset);
    text-align: left;
  }

  .fold {
    flex: none;
    min-width: 44px;
    color: var(--muted);
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
    flex: 0 1 auto;
    min-width: 0;
    overflow: hidden;
    color: var(--faint);
    font-size: 13px;
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }

  .hidden {
    flex: none;
    color: var(--muted);
    font-size: 13px;
    font-variant-numeric: tabular-nums;
  }
</style>
