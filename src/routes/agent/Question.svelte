<script lang="ts">
  import type { Question } from '../../lib/question.svelte'
  import Row from '../../lib/terminal/Row.svelte'
  import { paneColours, rgb } from '../../lib/terminal/style'
  import { anchor } from '../../lib/terminal/touch'
  import { BASE, LINE } from '../../lib/terminal/zoom'
  import Button from '../../lib/ui/Button.svelte'
  import QuestionCard from './QuestionCard.svelte'

  const PROBE = 'M'.repeat(100)
  const ADVANCE = 0.6

  let {
    question,
    agent,
    onoption,
    ontab,
  }: {
    question: Question
    agent: string
    onoption: (number: number) => void
    ontab: (index: number) => void
  } = $props()

  let measured = $state(0)

  const cell = $derived(measured > 0 ? measured : ADVANCE)
  const ink = $derived(paneColours(question.colours))
  const cols = $derived(Math.max(0, ...question.rows.map((runs) => runs.reduce((sum, run) => sum + run.width, 0))))

  const measure = (probe: HTMLElement) => {
    measured = probe.getBoundingClientRect().width / PROBE.length / BASE
  }

  const anchored = (box: HTMLElement) => {
    if (question.rows.length > 0) anchor(box)
  }
</script>

{#if question.card !== null}
  <QuestionCard card={question.card} {agent} {onoption} {ontab} />
{:else}
  <div
    class="screen"
    role="group"
    aria-label="{agent} asks"
    style:background={rgb(ink.bg)}
    style:color={rgb(ink.fg)}
    style:font-size="{BASE}px"
    style:--line="{LINE}em"
    {@attach anchored}
  >
    <div class="rows" style:width="{cell * cols}em">
      {#each question.rows as runs, row (row)}
        <Row {runs} colours={question.colours} {cell} />
      {/each}
    </div>
    <span class="probe" aria-hidden="true"><span {@attach measure}>{PROBE}</span></span>
  </div>
  {#if question.options.length > 0}
    <div class="options" role="group" aria-label="Options">
      {#each question.options as option (option.number)}
        <Button tone="plain" label="Option {option.number}, {option.label}" onclick={() => onoption(option.number)}
          ><span class="label">{option.number}. {option.label}</span></Button
        >
      {/each}
    </div>
  {/if}
{/if}

<style>
  .screen {
    position: relative;
    flex: 0 1 auto;
    min-height: calc(3 * var(--line));
    max-height: 40dvh;
    overflow: auto;
    padding: var(--gap);
    overscroll-behavior: contain;
    border-radius: var(--radius-control);
  }

  .rows,
  .probe {
    font-family: var(--font-mono);
    white-space: pre;
  }

  .rows {
    line-height: var(--line);
  }

  .probe {
    position: absolute;
    top: 0;
    left: 0;
    width: 0;
    height: 0;
    overflow: hidden;
    visibility: hidden;
    pointer-events: none;
  }

  .options {
    display: grid;
    grid-template-columns: minmax(0, 1fr);
    gap: var(--gap);
  }

  .label {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-align: left;
    text-overflow: ellipsis;
  }
</style>
