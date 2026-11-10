<script lang="ts">
  import type { Fold_Serialize as Fold } from '../../lib/bindings'
  import { CLOSED, goneMessage, type ChangesState } from '../../lib/changes.svelte'
  import { rows, shownPath } from '../../lib/diff'
  import CodeBox from '../../lib/ui/CodeBox.svelte'
  import NavBar from '../../lib/ui/NavBar.svelte'
  import Spinner from '../../lib/ui/Spinner.svelte'
  import ToolButton from '../../lib/ui/ToolButton.svelte'
  import DiffLine from './DiffLine.svelte'
  import Viewed from './Viewed.svelte'

  const NOTICES: Partial<Record<Fold, string>> = {
    binary: 'Binary file, not shown',
    large: 'Too large to show',
  }

  let {
    path,
    changes,
    backTo,
    onback,
    onfile,
  }: {
    path: string
    changes: ChangesState
    backTo: string | null
    onback: () => void
    onfile: (path: string) => void
  } = $props()

  const reading = $derived(changes.diff?.path === path ? changes.diff.reading : null)
  const patch = $derived(reading?.kind === 'patch' ? reading.patch : null)
  const lines = $derived(patch === null ? [] : rows(patch))
  const digits = $derived(lines.reduce((most, row) => Math.max(most, row.old.length, row.new.length), 1))
  const notice = $derived(patch === null ? undefined : NOTICES[patch.fold])
  const viewed = $derived(changes.viewed(path))
  const open = $derived(!changes.gone && reading?.kind !== 'gone')
  const whole = $derived(open && patch !== null && patch.status !== 'deleted' ? patch.path : null)
</script>

<section class="screen">
  <NavBar {backTo} {onback} topmost>
    {#snippet center()}
      <h1 class="path"><span dir="ltr">{shownPath(patch ?? { path, old_path: null })}</span></h1>
    {/snippet}
    {#snippet right()}
      {#if whole !== null}
        <ToolButton label="Whole file" icon="file-text" size={22} onclick={() => onfile(whole)} />
      {/if}
      {#if open}
        <Viewed {viewed} onflip={() => void changes.mark(path, !viewed)} />
      {/if}
    {/snippet}
  </NavBar>
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
    <CodeBox {digits}>
      {#each lines as row, at (at)}
        <DiffLine {row} />
      {/each}
    </CodeBox>
  {/if}
</section>

<style>
  .screen {
    display: flex;
    flex-direction: column;
    gap: var(--inset-wide);
    padding: var(--gap) var(--inset-wide) 40px;
  }

  .path {
    margin: 0;
    overflow: hidden;
    direction: rtl;
    color: var(--text);
    font-size: 17px;
    font-weight: 600;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .note {
    display: flex;
    align-items: center;
    gap: 6px;
    margin: 0;
    color: var(--muted);
  }
</style>
