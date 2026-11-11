<script lang="ts">
  import { tick, untrack } from 'svelte'
  import { elapsed } from '../../lib/board'
  import type { Chat } from '../../lib/chat.svelte'
  import { atEnd, readingNote, threadOf } from '../../lib/thread'
  import Markdown from '../../lib/ui/Markdown.svelte'
  import AgentLine from './AgentLine.svelte'
  import Sent from './Sent.svelte'

  let { chat, now, boxed = false }: { chat: Chat; now: number; boxed?: boolean } = $props()

  const bubbles = $derived(threadOf(chat.conversation, chat.outgoing !== null))
  const note = $derived(readingNote(chat.reading))

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

  let drawn = false
  let wasBoxed = untrack(() => boxed)
  let pinned = true

  $effect.pre(() => {
    void chat.reading
    void chat.outgoing
    const was = drawn
    drawn = chat.conversation.length > 0
    if (drawn && (!was || untrack(ending))) {
      void tick().then(toEnd)
    }
  })

  $effect(() => {
    void tall
    if (boxed !== wasBoxed || (boxed && pinned)) {
      wasBoxed = boxed
      toEnd()
    }
  })
</script>

<div class="thread" class:boxed bind:this={box} bind:clientHeight={tall} onscroll={() => (pinned = ending())}>
  {#each bubbles as { entry, live, going } (entry.id)}
    {#if entry.kind === 'you'}
      <Sent {entry} chat={live ? chat : null} />
    {:else if entry.kind === 'agent'}
      {@const written = elapsed(entry.written, now)}
      <article class="message">
        {#if written !== ''}
          <p class="written">{written}</p>
        {/if}
        <Markdown text={entry.text} />
        {#if going}
          <p class="going">More to come…</p>
        {/if}
      </article>
    {:else}
      <p class="gap">Earlier messages are on the Mac</p>
    {/if}
  {/each}
  {#if note !== null}
    <p class="reason">{note}</p>
  {/if}
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

  .gap {
    margin: 0;
    color: var(--muted);
    font-size: 13px;
    text-align: center;
  }
</style>
