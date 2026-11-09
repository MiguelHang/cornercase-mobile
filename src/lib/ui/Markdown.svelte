<script lang="ts">
  import { parse } from '../markdown'
  import MarkdownNode from './MarkdownNode.svelte'

  let { text }: { text: string } = $props()

  const nodes = $derived(parse(text))
</script>

<div class="markdown">
  {#each nodes as node, index (index)}
    <MarkdownNode {node} />
  {/each}
</div>

<style>
  .markdown {
    min-width: 0;
    color: var(--text);
    font-size: 15px;
    line-height: 1.5;
    overflow-wrap: anywhere;
    -webkit-user-select: text;
    user-select: text;
    -webkit-touch-callout: default;
  }
</style>
