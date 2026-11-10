<script lang="ts">
  import type { Snippet } from 'svelte'

  let {
    digits,
    children,
  }: {
    digits: number
    children: Snippet
  } = $props()
</script>

<div class="box">
  <div class="lines" style:--digits="{digits}ch">
    {@render children()}
  </div>
</div>

<style>
  .box {
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

  :global(.code-line) {
    --tint: transparent;
    display: flex;
    background: var(--tint);
    color: var(--soft);
    font-family: var(--font-mono);
    font-size: 13px;
    line-height: 20px;
    white-space: pre;
  }

  :global(.code-gutter) {
    position: sticky;
    left: 0;
    display: flex;
    flex: none;
    border-right: 1px solid var(--border);
    background: linear-gradient(var(--tint), var(--tint)), var(--raised);
    color: var(--faint);
    font-variant-numeric: tabular-nums;
  }

  :global(.code-number) {
    box-sizing: content-box;
    width: var(--digits);
    padding: 0 6px;
    text-align: right;
  }

  :global(.code-text) {
    padding-right: var(--inset);
  }
</style>
