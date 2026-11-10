<script lang="ts">
  import { formatBytes } from '../../lib/attach.svelte'
  import type { ReadKind_Serialize as ReadKind, Read_Serialize as Read } from '../../lib/bindings'
  import { CLOSED } from '../../lib/changes.svelte'
  import type { FilesState } from '../../lib/files.svelte'
  import CodeBox from '../../lib/ui/CodeBox.svelte'
  import NavBar from '../../lib/ui/NavBar.svelte'
  import Spinner from '../../lib/ui/Spinner.svelte'

  const NOTICES: Partial<Record<ReadKind, (read: Read) => string>> = {
    binary: () => 'Binary file, not shown',
    'too-large': (read) => `Too large to show (${formatBytes(read.bytes ?? 0)})`,
    missing: (read) => `${read.path} no longer exists`,
    folder: (read) => `${read.path} is a folder`,
    other: () => 'Not a file, not shown',
  }

  let {
    path,
    files,
    backTo,
    onback,
  }: {
    path: string
    files: FilesState
    backTo: string | null
    onback: () => void
  } = $props()

  const linesOf = (text: string) => {
    const lines = text.split(/\r?\n/)
    if (lines.at(-1) === '') {
      lines.pop()
    }
    return lines
  }

  const mimeOf = (format: string) => `image/${format.toLowerCase()}`

  const reading = $derived(files.file?.path === path ? files.file.reading : null)
  const shown = $derived(reading?.kind === 'shown' ? reading.shown : null)
  const read = $derived(shown?.shown === 'read' ? shown.read : null)
  const lines = $derived(linesOf(read?.text ?? ''))
  const digits = $derived(String(lines.length).length)
  const notice = $derived(read === null ? undefined : NOTICES[read.kind]?.(read))
</script>

<section class="screen">
  <NavBar {backTo} {onback} topmost>
    {#snippet center()}
      <h1 class="path"><span dir="ltr">{path}</span></h1>
    {/snippet}
  </NavBar>
  {#if files.gone}
    <p class="note">{CLOSED}.</p>
  {:else if reading?.kind === 'failed'}
    <p class="note">{reading.reason}</p>
  {:else if shown?.shown === 'too_large_here'}
    <p class="note">
      Too large to show on the phone ({formatBytes(shown.bytes)}, {shown.lines.toLocaleString('en-US')} lines)
    </p>
  {:else if read === null}
    <p class="note"><Spinner />Reading the file…</p>
  {:else if notice !== undefined}
    <p class="note">{notice}</p>
  {:else if read.image !== null}
    <img
      class="picture"
      src="data:{mimeOf(read.image.format)};base64,{read.image.data}"
      width={read.image.width}
      height={read.image.height}
      alt={path}
    />
  {:else if lines.length === 0}
    <p class="note">Empty file</p>
  {:else}
    <CodeBox {digits}>
      {#each lines as line, at (at)}
        <div class="code-line">
          <span class="code-gutter"><span class="code-number">{at + 1}</span></span>
          <span class="code-text text">{line}</span>
        </div>
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

  .picture {
    align-self: flex-start;
    max-width: 100%;
    height: auto;
  }

  .text {
    padding-left: 6px;
  }
</style>
