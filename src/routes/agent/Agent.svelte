<script lang="ts">
  import { untrack } from 'svelte'
  import type { AgentRow_Serialize as AgentRow, ConnectionView_Serialize, Toast } from '../../lib/bindings'
  import { label } from '../../lib/board'
  import { whileShown, type Chat } from '../../lib/chat.svelte'
  import type { CloseAgent } from '../../lib/closeAgent.svelte'
  import { Keyboard } from '../../lib/terminal/keyboard.svelte'
  import CloseConfirm from '../../lib/ui/CloseConfirm.svelte'
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
    closer,
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
    onstart,
  }: {
    chat: Chat
    agents: AgentRow[] | null
    connection: ConnectionView_Serialize
    closer: CloseAgent | null
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
    onstart: (workspace: number) => void
  } = $props()

  let now = $state(untrack(() => chat.now()))

  const keyboard = new Keyboard(globalThis)
  const seen = $derived(keyboard.covered ? keyboard.height : null)

  $effect(() => keyboard.watch())

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

  const closing = $derived(row === null ? undefined : closer?.stateOf(row.pane))

  const asking = $derived(closing?.kind === 'confirming' || closing?.kind === 'rechecking' ? closing : null)

  const closable = $derived(row !== null && connection.state === 'connected' && closer?.offered(row) === true)

  const ask = () => {
    if (row !== null && closable) {
      void closer?.ask(row)
    }
  }
</script>

<section
  class="agent"
  class:keyboard={seen !== null}
  style:--below="{below}px"
  style:top={seen === null ? undefined : `${keyboard.offset}px`}
  style:height={seen === null ? undefined : `${seen}px`}
>
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
    <ChatHeader
      {row}
      agents={agents ?? []}
      {backTo}
      {below}
      boxed={seen !== null}
      sendable={connection.state === 'connected'}
      {closable}
      progress={closer?.progress(row.pane) ?? null}
      {onback}
      {onterminal}
      {onchanges}
      {onfiles}
      {onstart}
      onclose={ask}
    />
    {#key row.pane}
      <Subagents {row} agents={agents ?? []} {onopen} />
    {/key}
    <Thread {chat} {now} boxed={seen !== null} />
    <Composer {chat} agent={row.agent} {below} {seen} {onfailed} {ontoast} />
    {#if asking !== null}
      {@const pane = row.pane}
      <CloseConfirm
        plan={asking.plan}
        label={label(row, agents ?? [])}
        rechecking={asking.kind === 'rechecking'}
        oncancel={() => closer?.cancel(pane)}
        onconfirm={() => void closer?.confirm(pane)}
      />
    {/if}
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

  .keyboard {
    position: fixed;
    right: 0;
    left: 0;
    min-height: 0;
    padding-top: calc(env(safe-area-inset-top) + var(--below) + var(--inset));
    padding-right: calc(env(safe-area-inset-right) + var(--inset-wide));
    padding-left: calc(env(safe-area-inset-left) + var(--inset-wide));
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
