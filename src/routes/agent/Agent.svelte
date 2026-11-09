<script lang="ts">
  import type { AgentRow } from '../../lib/bindings'
  import { label, place } from '../../lib/board'
  import Button from '../../lib/ui/Button.svelte'
  import StatusDot from '../../lib/ui/StatusDot.svelte'

  let { row, below = 0, onback }: { row: AgentRow | undefined; below?: number; onback: () => void } = $props()
</script>

<section class="agent" style:--below="{below}px">
  {#if row}
    <header class="head">
      <StatusDot status={row.status} size={20} />
      <div class="lines">
        <h1 class="title">{label(row)}</h1>
        <p class="place">{place(row)}</p>
      </div>
    </header>
  {:else}
    <p class="gone">This agent is no longer running.</p>
  {/if}
  <div class="actions">
    <Button tone="plain" onclick={onback}>Back</Button>
  </div>
</section>

<style>
  .agent {
    display: flex;
    flex-direction: column;
    gap: var(--inset-wide);
    padding: calc(var(--below) + var(--inset)) var(--inset-wide) var(--inset-wide);
  }

  .head {
    display: flex;
    align-items: center;
    gap: var(--inset);
  }

  .lines {
    min-width: 0;
  }

  .title {
    margin: 0;
    overflow: hidden;
    color: var(--text);
    font-size: 20px;
    font-weight: 600;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .place {
    margin: 0;
    overflow: hidden;
    color: var(--muted);
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .gone {
    margin: 0;
    color: var(--muted);
  }
</style>
