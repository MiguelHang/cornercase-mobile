<script lang="ts">
  import { hiddenName } from '../board'
  import StatusDot from './StatusDot.svelte'

  let { hidden, attention, gap }: { hidden: number; attention: string | null; gap?: string } = $props()
</script>

{#if hidden > 0}
  <span class="folded" role="img" aria-label={hiddenName(hidden, attention)}>
    <span class="marks" style:gap={gap} aria-hidden="true">
      {#if attention !== null}
        <StatusDot status={attention} size={10} />
      {/if}
      <span class="hidden">+{hidden}</span>
    </span>
  </span>
{/if}

<style>
  .folded,
  .marks {
    display: flex;
    flex: none;
    align-items: center;
  }

  .marks {
    gap: var(--inset);
  }

  .hidden {
    color: var(--muted);
    font-size: 13px;
    font-variant-numeric: tabular-nums;
  }
</style>
