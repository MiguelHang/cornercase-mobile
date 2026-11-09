<script lang="ts">
  import type { AgentRow_Serialize as AgentRow } from '../../lib/bindings'
  import { label, place, subagents } from '../../lib/board'
  import StatusDot from '../../lib/ui/StatusDot.svelte'

  let {
    row,
    agents,
    onopen,
    open = $bindable(false),
  }: { row: AgentRow; agents: AgentRow[]; onopen: (pane: number) => void; open?: boolean } = $props()

  const summary = $derived(subagents(agents, row.pane))
</script>

{#if summary !== null}
  <div class="subagents">
    <button
      class="line"
      type="button"
      aria-expanded={open}
      onclick={() => (open = !open)}
      ontouchstart={() => {}}
    >
      <span class="count">{summary.line}</span>
      <span class="arrow">{open ? '▾' : '▸'}</span>
    </button>
    {#if open}
      <ul class="children" aria-label="Subagents">
        {#each summary.children as child (child.key)}
          <li>
            <button class="child" type="button" onclick={() => onopen(child.row.pane)} ontouchstart={() => {}}>
              <StatusDot status={child.row.status} />
              <span class="names">
                <span class="name">{label(child.row, agents)}</span>
                <span class="where">{place(child.row, agents)}</span>
              </span>
              {#if child.attention !== null}
                <StatusDot status={child.attention} size={10} />
              {/if}
              {#if child.hidden > 0}
                <span class="hidden">+{child.hidden}</span>
              {/if}
            </button>
          </li>
        {/each}
      </ul>
    {/if}
  </div>
{/if}

<style>
  .line,
  .child {
    box-sizing: border-box;
    display: flex;
    align-items: center;
    width: 100%;
    min-height: 44px;
    border: none;
    border-radius: var(--radius-control);
    background: none;
    font: inherit;
    text-align: left;
  }

  .line:active,
  .child:active {
    background: var(--hover);
  }

  .line {
    gap: var(--gap);
    padding: 0;
    color: var(--muted);
  }

  .count {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .arrow {
    flex: none;
    min-width: 44px;
    text-align: center;
  }

  .children {
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .child {
    gap: var(--inset);
    padding: var(--gap) var(--inset);
    color: var(--text);
  }

  .names {
    display: flex;
    flex: 1;
    flex-direction: column;
    min-width: 0;
  }

  .name,
  .where {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .where,
  .hidden {
    color: var(--muted);
    font-size: 13px;
  }

  .hidden {
    flex: none;
    font-variant-numeric: tabular-nums;
  }
</style>
