<script lang="ts">
  import { untrack } from 'svelte'
  import type { AgentRow_Serialize as AgentRow, ConnectionView_Serialize } from '../../lib/bindings'
  import { whileShown, type Chat } from '../../lib/chat.svelte'
  import Button from '../../lib/ui/Button.svelte'
  import Icon from '../../lib/ui/Icon.svelte'
  import Mark from '../../lib/ui/Mark.svelte'
  import NavBar from '../../lib/ui/NavBar.svelte'
  import Spinner from '../../lib/ui/Spinner.svelte'
  import TextArea from '../../lib/ui/TextArea.svelte'
  import ChatHeader from './ChatHeader.svelte'
  import KeyRow from './KeyRow.svelte'
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
    onterminal,
    onchanges,
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
    onterminal: (row: AgentRow) => void
    onchanges: (workspace: number, title: string) => void
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

  const row = $derived(chat.row)

  const attach = async () => {
    const failed = await chat.attach()
    if (failed !== null) {
      onfailed(failed)
    }
  }
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
    <ChatHeader {row} agents={agents ?? []} {backTo} {below} {onback} {onterminal} {onchanges} />
    {#key row.pane}
      <Subagents {row} agents={agents ?? []} {onopen} />
    {/key}
    <Thread {chat} {now} />
    <div class="composer">
      {#if chat.asking}
        <p class="note">{row.agent} is asking you something on its screen; answer with the keys.</p>
      {/if}
      <KeyRow {chat} {onfailed} />
      {#if chat.attachments.chips.length > 0}
        <ul class="chips" aria-label="Attached images">
          {#each chat.attachments.chips as chip (chip)}
            <li class="chip"><span class="name">{chip.name}</span><span class="size">{chip.size}</span></li>
          {/each}
        </ul>
      {/if}
      <div class="fields">
        <div class="field">
          <TextArea label="Answer" placeholder="Answer {row.agent}" bind:value={chat.draft} />
        </div>
        <Button tone="plain" compact label="Attach image" busy={chat.attachments.attaching} onclick={attach}>
          {#if !chat.attachments.attaching}
            <Icon name="image-plus" size={20} />
          {/if}
        </Button>
        <Button resting={!chat.canSend && !chat.sending} busy={chat.sending} onclick={() => void chat.send()}>Send</Button>
      </div>
    </div>
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

  .note {
    margin: 0;
    color: var(--muted);
  }

  .composer {
    position: sticky;
    bottom: 0;
    display: flex;
    flex-direction: column;
    gap: var(--gap);
    margin: 0 calc(-1 * var(--inset-wide)) calc(-1 * var(--inset-wide) - env(safe-area-inset-bottom));
    padding: var(--gap) var(--inset-wide) calc(var(--inset-wide) + env(safe-area-inset-bottom));
    background: var(--glass);
  }

  .fields {
    display: flex;
    align-items: flex-end;
    gap: var(--gap);
  }

  .field {
    flex: 1;
    min-width: 0;
  }

  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: var(--gap);
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .chip {
    display: flex;
    gap: 6px;
    min-width: 0;
    max-width: 100%;
    padding: 4px var(--gap);
    border: 1px solid var(--border);
    border-radius: var(--radius-chip);
    background: var(--hover);
    font-size: 13px;
  }

  .name {
    overflow: hidden;
    color: var(--soft);
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .size {
    flex: none;
    color: var(--faint);
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
