<script lang="ts">
  import type { AgentRow_Serialize as AgentRow } from '../../lib/bindings'
  import { label, place, statusName } from '../../lib/board'
  import { compactHeader } from '../../lib/thread'
  import Button from '../../lib/ui/Button.svelte'
  import StatusDot from '../../lib/ui/StatusDot.svelte'

  let {
    row,
    below = 0,
    onback,
    onterminal,
  }: {
    row: AgentRow
    below?: number
    onback: () => void
    onterminal: (row: AgentRow) => void
  } = $props()

  let scrolled = $state(0)
  let tall = $state(0)

  const compact = $derived(compactHeader(scrolled, tall))

  $effect(() => {
    const seen = () => (scrolled = window.scrollY)
    seen()
    window.addEventListener('scroll', seen, { passive: true })
    return () => window.removeEventListener('scroll', seen)
  })
</script>

<div class="bar" class:compact style:--below="{below}px">
  <Button tone="plain" onclick={onback}>Back</Button>
  <div class="short" aria-hidden="true">
    <StatusDot status={row.status} />
    <span class="called">{label(row)}</span>
    <span class="state">{statusName(row.status)}</span>
  </div>
  <Button tone="plain" onclick={() => onterminal(row)}>Terminal</Button>
</div>
<header class="head" bind:clientHeight={tall}>
  <StatusDot status={row.status} size={20} />
  <div class="lines">
    <h1 class="title">{label(row)}</h1>
    <p class="place"><span class="status">{statusName(row.status)}</span> · {place(row)} › {row.tab_name}</p>
  </div>
</header>

<style>
  .bar {
    position: sticky;
    top: calc(env(safe-area-inset-top) + var(--below));
    z-index: 4;
    display: flex;
    align-items: center;
    gap: var(--gap);
    margin: calc(-1 * var(--gap)) calc(-1 * var(--inset-wide));
    padding: var(--gap) var(--inset-wide);
    border-bottom: 1px solid transparent;
    background: var(--veil);
    -webkit-backdrop-filter: blur(24px);
    backdrop-filter: blur(24px);
  }

  .short {
    display: flex;
    flex: 1;
    align-items: center;
    gap: 6px;
    min-width: 0;
    min-height: 44px;
    visibility: hidden;
    opacity: 0;
    transition:
      opacity 150ms ease-out,
      visibility 150ms;
  }

  .compact {
    border-bottom-color: var(--border);
  }

  .compact .short {
    visibility: visible;
    opacity: 1;
  }

  .called {
    overflow: hidden;
    color: var(--text);
    font-size: 15px;
    font-weight: 600;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .state {
    flex: none;
    color: var(--muted);
    font-size: 13px;
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

  .status {
    color: var(--soft);
  }

  @media (prefers-reduced-motion: reduce) {
    .short {
      transition: none;
    }
  }
</style>
