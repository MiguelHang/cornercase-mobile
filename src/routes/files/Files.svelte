<script lang="ts">
  import { CLOSED } from '../../lib/changes.svelte'
  import { ROOT, type FilesState, type Row } from '../../lib/files.svelte'
  import Icon from '../../lib/ui/Icon.svelte'
  import Spinner from '../../lib/ui/Spinner.svelte'
  import EntryRow from './EntryRow.svelte'
  import SearchResults from './SearchResults.svelte'

  let {
    title,
    files,
    onback,
    onopen,
  }: {
    title: string
    files: FilesState
    onback: () => void
    onopen: (path: string) => void
  } = $props()

  const root = $derived(files.folder(ROOT)?.listed ?? null)
  const rows = $derived(files.rows)

  const keyOf = (row: Row) => (row.kind === 'entry' ? row.path : `${row.kind}:${row.folder}`)

  const toggle = (path: string) => void files.toggle(path)
</script>

<section class="screen">
  <header class="top">
    <button class="back" type="button" aria-label="Back" onclick={onback} ontouchstart={() => {}}>
      <Icon name="chevron-left" size={24} />
    </button>
    <h1 class="title">{title}</h1>
  </header>
  {#if files.gone}
    <p class="note">{CLOSED}.</p>
  {:else}
    <label class="search">
      <Icon name="search" size={18} />
      <input
        class="query"
        type="search"
        aria-label="Search files"
        placeholder="Search files"
        autocomplete="off"
        autocapitalize="off"
        autocorrect="off"
        spellcheck="false"
        enterkeyhint="search"
        value={files.query}
        oninput={(event) => files.find(event.currentTarget.value)}
      />
    </label>
    {#if files.results !== null}
      <SearchResults results={files.results} {onopen} />
    {:else if rows.length > 0}
      <ul class="entries">
        {#each rows as row (keyOf(row))}
          <li><EntryRow {row} ontoggle={toggle} {onopen} /></li>
        {/each}
      </ul>
    {:else if root === null}
      <p class="note"><Spinner />Reading the files…</p>
    {:else}
      <p class="note">No files</p>
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

  .search {
    box-sizing: border-box;
    display: flex;
    align-items: center;
    gap: var(--gap);
    min-height: 44px;
    padding: 0 12px;
    border: 1px solid var(--edge);
    border-radius: var(--radius-control);
    background: var(--field);
    color: var(--faint);
  }

  .search:focus-within {
    border-color: var(--accent);
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--brand) 32%, transparent);
  }

  .query {
    flex: 1;
    min-width: 0;
    min-height: 42px;
    padding: 0;
    border: none;
    outline: none;
    background: none;
    color: var(--text);
    font: inherit;
    font-size: 16px;
    -webkit-user-select: text;
    user-select: text;
  }

  .query::placeholder {
    color: var(--faint);
  }

  .entries {
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
