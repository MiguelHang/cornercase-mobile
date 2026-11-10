<script lang="ts">
  import { tick, untrack } from 'svelte'
  import { elapsed } from '../../lib/board'
  import type { Chat } from '../../lib/chat.svelte'
  import { atEnd, threadOf } from '../../lib/thread'
  import Markdown from '../../lib/ui/Markdown.svelte'
  import AgentLine from './AgentLine.svelte'
  import Sent from './Sent.svelte'

  let { chat, now, boxed = false }: { chat: Chat; now: number; boxed?: boolean } = $props()

  const parts = $derived(threadOf(chat.reading, chat.outgoing))

  let box = $state<HTMLElement>()
  let tall = $state(0)

  const scroller = () => (boxed && box !== undefined ? box : document.documentElement)

  const toEnd = () => {
    const scrolled = scroller()
    scrolled.scrollTo({ top: scrolled.scrollHeight })
  }

  const ending = () => {
    const { scrollTop, clientHeight, scrollHeight } = scroller()
    return atEnd(scrollTop, clientHeight, scrollHeight)
  }

  let read = false
  let wasBoxed = untrack(() => boxed)

  $effect.pre(() => {
    const was = read
    read = chat.reading.kind === 'message'
    void chat.outgoing
    if (was && untrack(ending)) {
      void tick().then(toEnd)
    }
  })

  $effect(() => {
    void tall
    if (boxed || boxed !== wasBoxed) {
      wasBoxed = boxed
      toEnd()
    }
  })
</script>

<div class="thread" class:boxed bind:this={box} bind:clientHeight={tall}>
  {#each parts as part (part)}
    {#if part === 'you'}
      <Sent {chat} />
    {:else}
      <article class="message">
        {#if chat.reading.kind === 'message'}
          {@const message = chat.reading.message}
          {@const written = elapsed(message.written, now)}
          {#if written !== ''}
            <p class="written">{written}</p>
          {/if}
          <Markdown text={message.text} />
          {#if message.turn_over === false}
            <p class="going">More to come…</p>
          {/if}
        {:else}
          <p class="reason">{chat.reading.kind === 'loading' ? 'Reading its last message…' : chat.reading.reason}</p>
        {/if}
      </article>
    {/if}
  {/each}
  <AgentLine {chat} {now} />
</div>

<style>
  .thread {
    display: flex;
    flex: 1;
    flex-direction: column;
    gap: var(--inset-wide);
    min-width: 0;
  }

  .boxed {
    min-height: 0;
    overflow-y: auto;
    overscroll-behavior: contain;
  }

  .message {
    min-width: 0;
  }

  .written,
  .going {
    color: var(--faint);
    font-size: 13px;
  }

  .written {
    margin: 0 0 var(--gap);
  }

  .going {
    margin: var(--gap) 0 0;
  }

  .reason {
    margin: 0;
    color: var(--muted);
  }
</style>
