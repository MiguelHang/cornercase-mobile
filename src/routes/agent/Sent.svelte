<script lang="ts">
  import type { HistoryEntry_Serialize as Entry } from '../../lib/bindings'
  import type { Chat } from '../../lib/chat.svelte'
  import { sendNews, shownState } from '../../lib/thread'
  import Button from '../../lib/ui/Button.svelte'
  import Icon from '../../lib/ui/Icon.svelte'
  import Spinner from '../../lib/ui/Spinner.svelte'

  type You = Extract<Entry, { kind: 'you' }>

  const NAMES: Record<ReturnType<typeof shownState>, string> = {
    sending: 'sending',
    sent: 'sent',
    failed: 'not sent',
    unconfirmed: 'not confirmed',
  }

  let { entry, chat }: { entry: You; chat: Chat | null } = $props()

  const shown = $derived(shownState(entry.state, chat !== null))
</script>

<article class="sent" aria-label="Your message, {NAMES[shown]}">
  <div class="bubble">
    {#if entry.text !== ''}
      <p class="text">{entry.text}</p>
    {/if}
    {#if entry.chips.length > 0}
      <ul class="chips">
        {#each entry.chips as chip, index (index)}
          <li class="chip">{chip}</li>
        {/each}
      </ul>
    {/if}
  </div>
  {#if shown === 'sending'}
    <p class="state"><Spinner />Sending…</p>
  {:else if shown === 'sent'}
    <p class="state"><Icon name="check" size={14} />Sent</p>
  {:else if shown === 'failed'}
    <p class="state failed"><Icon name="circle-alert" size={14} />{sendNews(entry)}</p>
    {#if chat !== null}
      <div class="actions">
        <Button tone="plain" resting={!chat.canRetry} onclick={() => void chat.retry()}>
          <Icon name="rotate-ccw" size={16} />Retry
        </Button>
        <Button tone="plain" onclick={() => chat.edit()}><Icon name="pencil" size={16} />Edit</Button>
      </div>
    {/if}
  {:else}
    <p class="state"><Icon name="circle-alert" size={14} />Not confirmed: the app closed before an answer came back</p>
  {/if}
</article>

<style>
  .sent {
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    gap: 6px;
    align-self: flex-end;
    max-width: 85%;
    min-width: 0;
  }

  .bubble {
    display: flex;
    flex-direction: column;
    gap: var(--gap);
    max-width: 100%;
    padding: var(--gap) var(--inset);
    border: 1px solid var(--border);
    border-radius: var(--radius-card);
    background: var(--raised);
    color: var(--text);
  }

  .text {
    margin: 0;
    overflow-wrap: anywhere;
    white-space: pre-wrap;
  }

  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .chip {
    max-width: 100%;
    overflow: hidden;
    padding: 2px 6px;
    border: 1px solid var(--border);
    border-radius: var(--radius-chip);
    background: var(--hover);
    color: var(--soft);
    font-size: 13px;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .state {
    display: flex;
    align-items: center;
    gap: 6px;
    margin: 0;
    color: var(--muted);
    font-size: 13px;
  }

  .failed {
    color: var(--failed);
  }

  .actions {
    display: flex;
    gap: var(--gap);
  }
</style>
