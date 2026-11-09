<script lang="ts">
  import type { AgentRow_Serialize as AgentRow, ConnectionView_Serialize } from '../../lib/bindings'
  import { elapsed, label, place, statusName } from '../../lib/board'
  import type { Chat } from '../../lib/chat.svelte'
  import Button from '../../lib/ui/Button.svelte'
  import Icon from '../../lib/ui/Icon.svelte'
  import Mark from '../../lib/ui/Mark.svelte'
  import Markdown from '../../lib/ui/Markdown.svelte'
  import StatusDot from '../../lib/ui/StatusDot.svelte'
  import TextArea from '../../lib/ui/TextArea.svelte'
  import KeyRow from './KeyRow.svelte'

  const TICK_MS = 30_000

  let {
    chat,
    connection,
    below = 0,
    onback,
    onfailed,
    onterminal,
  }: {
    chat: Chat
    connection: ConnectionView_Serialize
    below?: number
    onback: () => void
    onfailed: (reason: string) => void
    onterminal: (row: AgentRow) => void
  } = $props()

  let now = $state(Date.now())

  $effect(() => {
    const timer = setInterval(() => (now = Date.now()), TICK_MS)
    return () => clearInterval(timer)
  })

  const row = $derived(chat.row)
  const reading = $derived(chat.reading)

  const send = async () => {
    const failed = await chat.send()
    if (failed !== null) {
      onfailed(failed)
    }
  }

  const attach = async () => {
    const failed = await chat.attach()
    if (failed !== null) {
      onfailed(failed)
    }
  }
</script>

<section class="agent" style:--below="{below}px">
  {#if chat.gone}
    <p class="gone">This agent is no longer running.</p>
    <div class="actions">
      <Button tone="plain" onclick={onback}>Back to the board</Button>
    </div>
  {:else}
    <div class="actions">
      <Button tone="plain" onclick={onback}>Back</Button>
      {#if row !== null}
        {@const opened = row}
        <Button tone="plain" onclick={() => onterminal(opened)}>Terminal</Button>
      {/if}
    </div>
    {#if row === null}
      <div class="waiting">
        <Mark unit={5} pulsing={connection.state === 'connecting'} />
      </div>
    {:else}
      <header class="head">
        <StatusDot status={row.status} size={20} />
        <div class="lines">
          <h1 class="title">{label(row)}</h1>
          <p class="place"><span class="status">{statusName(row.status)}</span> · {place(row)} › {row.tab_name}</p>
        </div>
      </header>
      <article class="message">
        {#if reading.kind === 'message'}
          {@const written = elapsed(reading.message.written, now)}
          {#if written !== ''}
            <p class="written">{written}</p>
          {/if}
          <Markdown text={reading.message.text} />
        {:else}
          <p class="reason">{reading.kind === 'loading' ? 'Reading its last message…' : reading.reason}</p>
        {/if}
      </article>
      <div class="composer">
        {#if chat.asking}
          <p class="note">{row.agent} is asking you something on its screen; answer with the keys.</p>
        {/if}
        <KeyRow {chat} {onfailed} />
        {#if chat.attachments.chips.length > 0}
          <ul class="chips" aria-label="Attached images">
            {#each chat.attachments.chips as chip, index (index)}
              <li class="chip"><span class="name">{chip.name}</span><span class="size">{chip.size}</span></li>
            {/each}
          </ul>
        {/if}
        <div class="fields">
          <div class="field">
            <TextArea label="Answer" placeholder="Answer {row.agent}" disabled={chat.sending} bind:value={chat.draft} />
          </div>
          <Button tone="plain" compact label="Attach image" busy={chat.attachments.attaching} onclick={attach}>
            {#if !chat.attachments.attaching}
              <Icon name="image-plus" size={20} />
            {/if}
          </Button>
          <Button resting={!chat.canSend && !chat.sending} busy={chat.sending} onclick={send}>Send</Button>
        </div>
      </div>
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

  .actions {
    display: flex;
    gap: var(--gap);
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

  .message {
    flex: 1;
    min-width: 0;
  }

  .written {
    margin: 0 0 var(--gap);
    color: var(--faint);
    font-size: 13px;
  }

  .reason,
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
</style>
