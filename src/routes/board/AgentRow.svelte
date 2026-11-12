<script lang="ts">
  import type { AgentRow_Serialize as Agent } from '../../lib/bindings'
  import { disclosureName, elapsed, label, place, statusName, type Row } from '../../lib/board'
  import Hidden from '../../lib/ui/Hidden.svelte'
  import { ENTER_MS, prefersReducedMotion } from '../../lib/ui/motion'
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

  const turn = prefersReducedMotion() ? 0 : ENTER_MS
</script>

<div class="agent" style:--depth={item.depth}>
  {#if item.hasChildren}
    <button
      class="fold"
      type="button"
      aria-label={disclosureName(item.total, item.folded)}
      aria-expanded={!item.folded}
      onclick={() => onfold(row.pane)}
      ontouchstart={() => {}}
    >
      <span aria-hidden="true"><StatusDot status={row.status} /></span>
      <svg
        class="turn"
        class:open={!item.folded}
        style:--turn="{turn}ms"
        aria-hidden="true"
        width="14"
        height="14"
        viewBox="0 0 24 24"
        fill="currentColor"
      >
        <path d="M6.5 5v14l11-7z" stroke="currentColor" stroke-width="2" stroke-linejoin="round" />
      </svg>
    </button>
  {/if}
  <button class="row" type="button" onclick={() => onopen(row.pane)} ontouchstart={() => {}}>
    <span class="mark">
      {#if item.hasChildren}
        <span class="spoken">{statusName(row.status)}</span>
      {:else}
        <StatusDot status={row.status} />
      {/if}
    </span>
    <span class="lines">
      <span class="title">{label(row, agents)}</span>
      <span class="place">{place(row, agents)}</span>
    </span>
    <Hidden hidden={item.hidden} attention={item.attention} />
    {#if time !== ''}
      <span class="time">{time}</span>
    {/if}
  </button>
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
    flex-direction: column;
    align-items: center;
    gap: 6px;
    padding: 0;
    color: var(--soft);
  }

  .turn {
    transition: transform var(--turn);
  }

  .turn.open {
    transform: rotate(90deg);
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
