<script lang="ts">
  import { flip } from 'svelte/animate'
  import type { AgentRow_Serialize as Agent, ConnectionView_Serialize } from '../../lib/bindings'
  import { items } from '../../lib/board'
  import { folds } from '../../lib/folds.svelte'
  import Button from '../../lib/ui/Button.svelte'
  import Mark from '../../lib/ui/Mark.svelte'
  import { ENTER_MS, prefersReducedMotion } from '../../lib/ui/motion'
  import AgentRow from './AgentRow.svelte'

  const TICK_MS = 30_000

  let {
    machine,
    agents,
    connection,
    below = 0,
    onopen,
    onstart,
  }: {
    machine: number | null
    agents: Agent[] | null
    connection: ConnectionView_Serialize
    below?: number
    onopen: (pane: number) => void
    onstart: () => void
  } = $props()

  let now = $state(Date.now())

  $effect(() => {
    const timer = setInterval(() => (now = Date.now()), TICK_MS)
    return () => clearInterval(timer)
  })

  const boot = $derived(agents?.[0]?.boot ?? '')

  const opened = $derived(machine === null ? undefined : folds.expanded(machine, boot))

  const fold = (pane: number) => {
    if (machine !== null) {
      folds.toggle(machine, boot, pane)
    }
  }

  const moving = { duration: prefersReducedMotion() ? 0 : ENTER_MS }
</script>

<section class="board" style:--below="{below}px">
  <h1 class="spoken">Agents</h1>
  <div class="actions">
    <Button resting={connection.state !== 'connected'} onclick={onstart}>Start</Button>
  </div>
  {#if agents === null}
    <div class="waiting">
      <Mark unit={5} pulsing={connection.state === 'connecting'} />
    </div>
  {:else if agents.length === 0}
    <p class="empty">No agents running.</p>
  {:else}
    <ul class="items">
      {#each items(agents, opened) as item (item.key)}
        <li class="item" animate:flip={moving}>
          {#if item.kind === 'header'}
            <h2 class="section">{item.section}</h2>
          {:else}
            <AgentRow {item} {agents} {now} {onopen} onfold={fold} />
          {/if}
        </li>
      {/each}
    </ul>
  {/if}
</section>

<style>
  .board {
    padding: calc(var(--below) + var(--inset)) var(--inset-wide) var(--inset-wide);
  }

  .actions {
    display: flex;
    justify-content: flex-end;
    margin-bottom: var(--inset);
  }

  .waiting {
    display: flex;
    justify-content: center;
    padding-top: 28px;
  }

  .empty {
    margin: 40px 0 0;
    color: var(--muted);
    text-align: center;
  }

  .items {
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .section {
    margin: var(--inset) var(--inset) var(--gap);
    color: var(--muted);
    font-size: 13px;
    font-weight: 600;
  }

  .item:first-child .section {
    margin-top: 0;
  }
</style>
