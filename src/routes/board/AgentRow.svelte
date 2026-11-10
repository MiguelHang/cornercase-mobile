<script lang="ts">
  import type { AgentRow_Serialize as Agent } from '../../lib/bindings'
  import { elapsed, label, place, type Row } from '../../lib/board'
  import Hidden from '../../lib/ui/Hidden.svelte'
  import Icon from '../../lib/ui/Icon.svelte'
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
    <Hidden hidden={item.hidden} attention={item.attention} />
  </button>
  {#if item.hasChildren}
    <button
      class="fold"
      type="button"
      aria-label="Subagents of {label(row, agents)}"
      aria-expanded={!item.folded}
      onclick={() => onfold(row.pane)}
      ontouchstart={() => {}}
    >
      <Icon name={item.folded ? 'chevron-right' : 'chevron-down'} size={18} />
    </button>
  {:else if item.depth > 0}
    <span class="slot"></span>
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
    display: flex;
    flex: none;
    align-items: center;
    justify-content: center;
    min-width: 44px;
    padding: 0;
    color: var(--soft);
  }

  .slot {
    flex: none;
    width: 44px;
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
</style>
