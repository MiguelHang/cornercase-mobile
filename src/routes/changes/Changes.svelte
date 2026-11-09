<script lang="ts">
  import type { Mode } from '../../lib/bindings'
  import { CLOSED, type ChangesState } from '../../lib/changes.svelte'
  import Icon from '../../lib/ui/Icon.svelte'
  import Spinner from '../../lib/ui/Spinner.svelte'
  import FileRow from './FileRow.svelte'

  const MODES: [Mode, string][] = [
    ['uncommitted', 'Uncommitted'],
    ['commits', 'Commits'],
    ['all', 'All'],
  ]

  let {
    title,
    changes,
    onback,
    onopen,
  }: {
    title: string
    changes: ChangesState
    onback: () => void
    onopen: (path: string) => void
  } = $props()

  const summary = $derived(changes.summary)
  const listed = $derived(changes.listed)
  const files = $derived(changes.files)

  const empty = (mode: Mode, base: string | null) => {
    if (mode === 'uncommitted') {
      return 'No uncommitted changes'
    }
    if (base === null) {
      return 'No changes'
    }
    return mode === 'commits' ? `No commits ahead of ${base}` : `Nothing changed since ${base}`
  }

  const viewed = (path: string, seen: boolean) => void changes.mark(path, seen)
</script>

<section class="screen">
  <header class="top">
    <button class="back" type="button" aria-label="Back" onclick={onback} ontouchstart={() => {}}>
      <Icon name="chevron-left" size={24} />
    </button>
    <h1 class="title">{title}</h1>
    {#if listed !== null}
      <p class="totals"><span class="plus">+{listed.added}</span> <span class="minus">−{listed.removed}</span></p>
    {/if}
  </header>
  {#if changes.gone}
    <p class="note">{CLOSED}.</p>
  {:else}
    <div class="modes" role="group" aria-label="Mode">
      {#each MODES as [mode, name] (mode)}
        <button
          class="mode"
          type="button"
          aria-pressed={changes.mode === mode}
          onclick={() => void changes.show(mode)}
          ontouchstart={() => {}}
        >
          {name}
        </button>
      {/each}
    </div>
    {#if summary?.state === 'failed'}
      <p class="note">{summary.error ?? 'Could not read the changes.'}</p>
    {:else if files.length > 0}
      <ul class="files">
        {#each files as file (file.path)}
          <li><FileRow {file} {onopen} onviewed={viewed} /></li>
        {/each}
      </ul>
    {:else if summary === null || summary.state === 'loading'}
      <p class="note"><Spinner />Reading the changes…</p>
    {:else}
      <p class="note">{empty(changes.mode, listed?.base ?? null)}</p>
    {/if}
  {/if}
</section>

<style>
  .screen {
    display: flex;
    flex-direction: column;
    gap: var(--inset-wide);
    padding: calc(env(safe-area-inset-top) + var(--gap)) var(--inset-wide) 40px;
  }

  .top {
    display: flex;
    align-items: center;
    gap: var(--gap);
    min-width: 0;
    margin-left: calc(-1 * var(--inset));
  }

  .back {
    display: flex;
    flex: none;
    align-items: center;
    justify-content: center;
    width: 44px;
    height: 44px;
    padding: 0;
    border: 0;
    border-radius: var(--radius-control);
    background: none;
    color: var(--accent);
  }

  .back:active {
    background: var(--hover);
  }

  .title {
    flex: 1;
    min-width: 0;
    margin: 0;
    overflow: hidden;
    font-size: 22px;
    font-weight: 600;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .totals {
    flex: none;
    margin: 0;
    font-variant-numeric: tabular-nums;
  }

  .plus {
    color: var(--done);
  }

  .minus {
    color: var(--failed);
  }

  .modes {
    display: flex;
    padding: 2px;
    border: 1px solid var(--border);
    border-radius: var(--radius-control);
    background: var(--field);
  }

  .mode {
    flex: 1;
    min-height: 44px;
    padding: 0 var(--gap);
    border: 0;
    border-radius: calc(var(--radius-control) - 2px);
    background: none;
    color: var(--muted);
    font: inherit;
    font-size: 15px;
  }

  .mode:active {
    background: var(--hover);
  }

  .mode[aria-pressed='true'] {
    background: var(--selected);
    color: var(--text);
  }

  .files {
    margin: 0 calc(-1 * var(--inset));
    padding: 0;
    list-style: none;
  }

  .note {
    display: flex;
    align-items: center;
    gap: 6px;
    margin: 0;
    color: var(--muted);
  }
</style>
