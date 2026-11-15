<script lang="ts">
  import type { Chat } from '../../lib/chat.svelte'
  import { agentLine, sendNews } from '../../lib/thread'
  import StatusDot from '../../lib/ui/StatusDot.svelte'

  let { chat, now }: { chat: Chat; now: number } = $props()

  const line = $derived(chat.row === null ? null : agentLine(chat.row, now, chat.lastNews))
</script>

<div class="line {line?.tone ?? ''}" class:spoken={line === null}>
  {#if line !== null}
    <StatusDot status={chat.row?.status} />
  {/if}
  <p class="said" aria-live="polite">
    <span class="spoken">{sendNews(chat.outgoing)}</span>
    {#if line !== null}
      <span class="phrase">{line.phrase}</span>
    {/if}
  </p>
  {#if line !== null && line.time !== ''}
    <span class="time">{line.stalled ? line.time : `· ${line.time}`}</span>
  {/if}
</div>

<style>
  .line {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px;
    min-width: 0;
    color: var(--soft);
    font-size: 15px;
  }

  .said {
    min-width: 0;
    margin: 0;
  }

  .time {
    color: var(--muted);
  }

  .muted,
  .muted .time {
    color: var(--muted);
  }

  .failed {
    color: var(--failed);
  }
</style>
