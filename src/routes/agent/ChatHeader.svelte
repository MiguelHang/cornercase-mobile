<script lang="ts">
  import type { AgentRow_Serialize as AgentRow } from '../../lib/bindings'
  import { label, place, statusName } from '../../lib/board'
  import { compactHeader } from '../../lib/thread'
  import Icon from '../../lib/ui/Icon.svelte'
  import NavBar from '../../lib/ui/NavBar.svelte'
  import StatusDot from '../../lib/ui/StatusDot.svelte'

  let {
    row,
    backTo,
    agents,
    below = 0,
    onback,
    onterminal,
    onchanges,
  }: {
    row: AgentRow
    backTo: string | null
    agents: AgentRow[]
    below?: number
    onback: () => void
    onterminal: (row: AgentRow) => void
    onchanges: (workspace: number, title: string) => void
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

<NavBar {backTo} {onback} {below} edge={compact}>
  {#snippet center()}
    <div class="short" class:compact aria-hidden="true">
      <StatusDot status={row.status} />
      <span class="called">{label(row, agents)}</span>
      <span class="state">{statusName(row.status)}</span>
    </div>
  {/snippet}
  {#snippet right()}
    <button class="tool" type="button" aria-label="Terminal" onclick={() => onterminal(row)} ontouchstart={() => {}}>
      <Icon name="square-terminal" />
    </button>
    <button
      class="tool"
      type="button"
      aria-label="Changes"
      onclick={() => onchanges(row.workspace, row.workspace_name)}
      ontouchstart={() => {}}
    >
      <Icon name="git-compare" />
    </button>
  {/snippet}
</NavBar>
<header class="head" bind:clientHeight={tall}>
  <StatusDot status={row.status} size={20} />
  <div class="lines">
    <h1 class="title">{label(row, agents)}</h1>
    <p class="place"><span class="status">{statusName(row.status)}</span> · {place(row)} › {row.tab_name}</p>
  </div>
</header>

<style>
  .tool {
    display: flex;
    flex: none;
    align-items: center;
    justify-content: center;
    width: 44px;
    height: 44px;
    padding: 0;
    border: 0;
    border-radius: var(--radius-control);
    background: none;
    color: var(--accent);
  }

  .tool:active {
    background: var(--hover);
  }

  .short {
    display: inline-flex;
    align-items: center;
    max-width: 0;
    overflow: hidden;
    vertical-align: middle;
    gap: 6px;
    min-width: 0;
    min-height: 44px;
    visibility: hidden;
    opacity: 0;
    transition:
      opacity 150ms ease-out,
      visibility 150ms,
      max-width 0s 150ms;
  }

  .short.compact {
    max-width: 100%;
    visibility: visible;
    opacity: 1;
    transition-delay: 0s;
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
