<script lang="ts">
  import type { Align, Cell, Node } from '../markdown'
  import MarkdownNode from './MarkdownNode.svelte'

  let { node }: { node: Node } = $props()
</script>

{#snippet all(nodes: Node[])}
  {#each nodes as child, index (index)}
    <MarkdownNode node={child} />
  {/each}
{/snippet}

{#snippet cells(row: Cell[], align: Align[], tag: 'th' | 'td')}
  <tr>
    {#each row as cell, index (index)}
      <svelte:element this={tag} class="cell" style:text-align={align[index] ?? undefined}>
        {@render all(cell.children)}
      </svelte:element>
    {/each}
  </tr>
{/snippet}

{#if node.type === 'text'}
  {node.value}
{:else if node.type === 'inlineCode'}
  <code class="inline">{node.value}</code>
{:else if node.type === 'code'}
  <div class="scroll"><pre class="code"><code>{node.value}</code></pre></div>
{:else if node.type === 'break'}
  <br />
{:else if node.type === 'thematicBreak'}
  <hr class="rule" />
{:else if node.type === 'paragraph'}
  <p class="block">{@render all(node.children)}</p>
{:else if node.type === 'emphasis'}
  <em>{@render all(node.children)}</em>
{:else if node.type === 'strong'}
  <strong>{@render all(node.children)}</strong>
{:else if node.type === 'delete'}
  <del>{@render all(node.children)}</del>
{:else if node.type === 'blockquote'}
  <blockquote class="block quote">{@render all(node.children)}</blockquote>
{:else if node.type === 'heading'}
  <svelte:element this={`h${node.depth}`} class="block heading">
    {@render all(node.children)}
  </svelte:element>
{:else if node.type === 'list'}
  <svelte:element this={node.ordered ? 'ol' : 'ul'} class="block list" start={node.start ?? undefined}>
    {#each node.children as item, index (index)}
      <li class:task={item.checked !== null}>
        {#if item.checked !== null}
          <input class="check" type="checkbox" checked={item.checked} disabled />
        {/if}
        {@render all(item.children)}
      </li>
    {/each}
  </svelte:element>
{:else if node.type === 'table'}
  {@const [head, ...body] = node.children}
  <div class="block scroll">
    <table class="table">
      {#if head}
        <thead>{@render cells(head.children, node.align, 'th')}</thead>
      {/if}
      {#if body.length > 0}
        <tbody>
          {#each body as row, index (index)}
            {@render cells(row.children, node.align, 'td')}
          {/each}
        </tbody>
      {/if}
    </table>
  </div>
{/if}

<style>
  .block,
  .scroll,
  .rule {
    margin: 0 0 12px;
  }

  .block:last-child,
  .scroll:last-child,
  .rule:last-child {
    margin-bottom: 0;
  }

  .heading {
    font-size: 16px;
    font-weight: 600;
    line-height: 1.35;
  }

  .quote {
    padding: 0 0 0 12px;
    border-left: 3px solid var(--edge);
    color: var(--soft);
  }

  .list {
    padding-left: 22px;
  }

  .list li + li {
    margin-top: 4px;
  }

  .list .task {
    list-style: none;
  }

  .list li > :global(p) {
    margin: 0;
  }

  .list .task > :global(p) {
    display: inline;
  }

  .check {
    margin: 0 6px 0 -20px;
    accent-color: var(--brand);
    vertical-align: -2px;
  }

  .inline,
  .code {
    font-family: var(--font-mono);
    font-size: 13px;
  }

  .inline {
    padding: 1px 4px;
    border-radius: var(--radius-chip);
    background: var(--hover);
  }

  .scroll {
    max-width: 100%;
    overflow-x: auto;
    border: 1px solid var(--border);
    border-radius: var(--radius-control);
    background: var(--panel);
  }

  .code {
    margin: 0;
    padding: 10px 12px;
    line-height: 1.45;
    white-space: pre;
  }

  .table {
    border-collapse: collapse;
    font-size: 14px;
  }

  .cell {
    padding: 6px 10px;
    border-bottom: 1px solid var(--border);
    text-align: left;
    white-space: nowrap;
  }

  tbody tr:last-child .cell {
    border-bottom: none;
  }

  th.cell {
    color: var(--soft);
    font-weight: 600;
  }

  .rule {
    border: none;
    border-top: 1px solid var(--border);
  }
</style>
