<script lang="ts">
  import type { Attachment } from 'svelte/attachments'
  import type { Card_Serialize as Card, CardOption_Serialize as CardOption } from '../../lib/bindings'
  import { revealBy, under } from '../../lib/reveal'
  import Button from '../../lib/ui/Button.svelte'
  import QuestionTabs from './QuestionTabs.svelte'

  let {
    card,
    agent,
    onoption,
    ontab,
  }: {
    card: Card
    agent: string
    onoption: (number: number) => void
    ontab: (index: number) => void
  } = $props()

  const named = (option: CardOption) =>
    [`Option ${option.number}`, option.label, ...(option.detail === null ? [] : [option.detail])].join(', ')

  const chosen = $derived(card.options.find((option) => option.selected)?.number ?? null)

  let tall = $state(0)

  const follow =
    (number: number | null): Attachment<HTMLElement> =>
    (box) => {
      void tall
      const option = number === null ? null : box.querySelector('.options [aria-current="true"]')
      const tabs = box.querySelector('[aria-label="Questions"]')
      if (option !== null) {
        const seen = under(box.getBoundingClientRect(), tabs?.getBoundingClientRect() ?? null)
        box.scrollTop += revealBy(seen, option.getBoundingClientRect())
      }
    }
</script>

<div class="card" role="group" aria-label="{agent} asks" bind:clientHeight={tall} {@attach follow(chosen)}>
  {#if card.tabs.length > 0}
    <QuestionTabs tabs={card.tabs} open={card.open} {ontab} />
  {/if}
  {#each card.blocks as block, at (at)}
    {#if block.kind === 'code'}
      <pre class="code">{block.text}</pre>
    {:else}
      <p class="text">{block.text}</p>
    {/if}
  {/each}
  <div class="options" role="group" aria-label="Options">
    {#each card.options as option (option.number)}
      <Button tone="plain" current={option.selected} label={named(option)} onclick={() => onoption(option.number)}
        ><span class="mark" aria-hidden="true">{option.selected ? '❯' : ''}</span><span class="answer"
          ><span class="label">{option.number}. {option.label}</span>{#if option.detail !== null}<span class="detail"
              >{option.detail}</span
            >{/if}</span
        ></Button
      >
    {/each}
  </div>
  {#each card.help as line, at (at)}
    <p class="help">{line}</p>
  {/each}
</div>

<style>
  .card {
    display: flex;
    flex: 0 1 auto;
    flex-direction: column;
    gap: var(--gap);
    min-width: 0;
    min-height: 0;
    max-height: 40dvh;
    overflow-x: hidden;
    overflow-y: auto;
    overscroll-behavior: contain;
    color: var(--text);
  }

  .text,
  .code,
  .help {
    margin: 0;
    overflow-wrap: anywhere;
  }

  .text {
    font-family: var(--font-ui);
    white-space: pre-line;
  }

  .code {
    padding: var(--gap);
    border-radius: var(--radius-control);
    background: var(--raised);
    font-family: var(--font-mono);
    font-size: 13px;
    white-space: pre-wrap;
  }

  .options {
    display: grid;
    grid-template-columns: minmax(0, 1fr);
    gap: var(--gap);
  }

  .mark {
    flex: none;
    width: 1em;
    color: var(--accent);
  }

  .answer {
    display: flex;
    flex: 1;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
    padding: var(--gap) 0;
    text-align: left;
    white-space: normal;
    overflow-wrap: anywhere;
  }

  .detail,
  .help {
    color: var(--muted);
  }

  .detail {
    font-size: 14px;
  }

  .help {
    font-size: 13px;
  }
</style>
