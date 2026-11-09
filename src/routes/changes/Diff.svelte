<script lang="ts">
  import type { Fold_Serialize as Fold } from '../../lib/bindings'
  import { CLOSED, goneMessage, type ChangesState } from '../../lib/changes.svelte'
  import { rows, shownPath } from '../../lib/diff'
  import Icon from '../../lib/ui/Icon.svelte'
  import Spinner from '../../lib/ui/Spinner.svelte'
  import DiffLine from './DiffLine.svelte'
  import Viewed from './Viewed.svelte'

  const NOTICES: Partial<Record<Fold, string>> = {
    binary: 'Binary file, not shown',
    large: 'Too large to show',
  }

  let {
    path,
    changes,
    onback,
  }: {
    path: string
    changes: ChangesState
    onback: () => void
  } = $props()

  const reading = $derived(changes.diff?.path === path ? changes.diff.reading : null)
  const patch = $derived(reading?.kind === 'patch' ? reading.patch : null)
  const lines = $derived(patch === null ? [] : rows(patch))
  const digits = $derived(lines.reduce((most, row) => Math.max(most, row.old.length, row.new.length), 1))
  const notice = $derived(patch === null ? undefined : NOTICES[patch.fold])
  const viewed = $derived(changes.viewed(path))
  const open = $derived(!changes.gone && reading?.kind !== 'gone')
</script>

<section class="screen">
  <header class="top">
    <button class="back" type="button" aria-label="Back" onclick={onback} ontouchstart={() => {}}>
      <Icon name="chevron-left" size={24} />
    </button>
    <h1 class="title"><span dir="ltr">{shownPath(patch ?? { path, old_path: null })}</span></h1>
    {#if open}
      <Viewed {viewed} onflip={() => void changes.mark(path, !viewed)} />
    {/if}
  </header>
  {#if changes.gone}
    <p class="note">{CLOSED}.</p>
  {:else if reading?.kind === 'gone'}
    <p class="note">{goneMessage(path)}.</p>
  {:else if reading?.kind === 'failed'}
    <p class="note">{reading.reason}</p>
  {:else if patch === null}
    <p class="note"><Spinner />Reading the diff…</p>
  {:else if notice !== undefined}
    <p class="note">{notice}</p>
  {:else if lines.length === 0}
    <p class="note">No lines changed</p>
  {:else}
    <div class="diff">
      <div class="lines">
        {#each lines as row, at (at)}
          <DiffLine {row} {digits} />
        {/each}
      </div>
    </div>
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
    margin: 0 calc(-1 * var(--inset));
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
    direction: rtl;
    font-size: 17px;
    font-weight: 600;
    text-align: left;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .diff {
    margin: 0 calc(-1 * var(--inset-wide));
    overflow-x: auto;
    overscroll-behavior-x: contain;
    border-block: 1px solid var(--border);
    background: var(--raised);
  }

  .lines {
    width: max-content;
    min-width: 100%;
  }

  .note {
    display: flex;
    align-items: center;
    gap: 6px;
    margin: 0;
    color: var(--muted);
  }
</style>
