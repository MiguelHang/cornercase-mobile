<script lang="ts">
  import { untrack } from 'svelte'
  import type { AgentRow_Serialize as AgentRow, ConnectionView_Serialize, Toast } from '../../lib/bindings'
  import { whileShown, type Chat } from '../../lib/chat.svelte'
  import Mark from '../../lib/ui/Mark.svelte'
  import NavBar from '../../lib/ui/NavBar.svelte'
  import Spinner from '../../lib/ui/Spinner.svelte'
  import ChatHeader from './ChatHeader.svelte'
  import Composer from './Composer.svelte'
  import Subagents from './Subagents.svelte'
  import Thread from './Thread.svelte'

  const TICK_MS = 15_000

  let {
    chat,
    agents,
    connection,
    backTo,
    below = 0,
    starting = null,
    onback,
    onopen,
    onfailed,
    ontoast,
    onterminal,
    onchanges,
    onfiles,
  }: {
    chat: Chat
    agents: AgentRow[] | null
    connection: ConnectionView_Serialize
    backTo: string | null
    below?: number
    starting?: string | null
    onback: () => void
    onopen: (pane: number) => void
    onfailed: (reason: string) => void
    ontoast: (toast: Toast) => void
    onterminal: (row: AgentRow) => void
    onchanges: (workspace: number, title: string) => void
    onfiles: (workspace: number, title: string) => void
  } = $props()

  let now = $state(untrack(() => chat.now()))

  $effect(() => {
    const told = chat
    const timer = setInterval(() => (now = told.now()), TICK_MS)
    return () => clearInterval(timer)
  })

  $effect(() => {
    const followed = chat
    return untrack(() => whileShown(document, () => followed.follow()))
  })

  $effect(() => {
    const asked = chat
    return untrack(() => whileShown(document, () => asked.watchQuestion()))
  })

  const row = $derived(chat.row)
</script>

<section class="agent" style:--below="{below}px">
  {#if chat.gone}
    <NavBar {backTo} {onback} {below} />
    <p class="gone">This agent is no longer running.</p>
  {:else if row === null}
    <NavBar {backTo} {onback} {below} />
    {#if starting !== null}
      <p class="starting"><Spinner />Starting {starting}…</p>
    {:else}
      <div class="waiting">
        <Mark unit={5} pulsing={connection.state === 'connecting'} />
      </div>
    {/if}
  {:else}
    <ChatHeader {row} agents={agents ?? []} {backTo} {below} {onback} {onterminal} {onchanges} {onfiles} />
    {#key row.pane}
      <Subagents {row} agents={agents ?? []} {onopen} />
    {/key}
    <Thread {chat} {now} />
    <Composer {chat} agent={row.agent} {below} {onfailed} {ontoast} />
  {/if}
</section>

<style>
  .agent {
    box-sizing: border-box;
    display: flex;
    flex-direction: column;
    gap: var(--inset-wide);
    min-height: calc(100dvh - env(safe-area-inset-top) - env(safe-area-inset-bottom));
    padding: calc(var(--below) + var(--inset)) var(--inset-wide) var(--inset-wide);
  }

  .waiting {
    display: flex;
    justify-content: center;
    padding-top: 28px;
  }

  .gone {
    margin: 0;
    color: var(--muted);
  }

  .starting {
    display: flex;
    align-items: center;
    gap: 6px;
    margin: 0;
    color: var(--muted);
  }
</style>
