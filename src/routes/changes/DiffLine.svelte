<script lang="ts">
  import type { Row } from '../../lib/diff'

  let {
    row,
    digits,
  }: {
    row: Row
    digits: number
  } = $props()
</script>

<div class="line {row.kind}" style:--digits="{digits}ch">
  <span class="gutter"><span class="number">{row.old}</span><span class="number">{row.new}</span></span>
  <span class="sign">{row.sign}</span>
  <span class="text">{#each row.spans as span, at (at)}<span class:strong={span.strong}>{span.text}</span>{/each}</span>
</div>

<style>
  .line {
    --tint: transparent;
    display: flex;
    background: var(--tint);
    color: var(--soft);
    font-family: var(--font-mono);
    font-size: 13px;
    line-height: 20px;
    white-space: pre;
  }

  .gutter {
    position: sticky;
    left: 0;
    display: flex;
    flex: none;
    border-right: 1px solid var(--border);
    background: linear-gradient(var(--tint), var(--tint)), var(--raised);
    color: var(--faint);
    font-variant-numeric: tabular-nums;
  }

  .number {
    box-sizing: content-box;
    width: var(--digits);
    padding: 0 6px;
    text-align: right;
  }

  .sign {
    flex: none;
    width: 2ch;
    text-align: center;
  }

  .text {
    padding-right: var(--inset);
  }

  .hunk {
    --tint: var(--hover);
    color: var(--accent);
  }

  .added {
    --tint: var(--diff-added);
    color: var(--text);
  }

  .removed {
    --tint: var(--diff-removed);
    color: var(--text);
  }

  .added .sign {
    color: var(--done);
  }

  .removed .sign {
    color: var(--failed);
  }

  .added .strong {
    background: var(--diff-added-strong);
  }

  .removed .strong {
    background: var(--diff-removed-strong);
  }
</style>
