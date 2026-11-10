<script lang="ts">
  import type { Row } from '../../lib/diff'

  let { row }: { row: Row } = $props()
</script>

<div class="code-line {row.kind}">
  <span class="code-gutter"><span class="code-number">{row.old}</span><span class="code-number">{row.new}</span></span>
  <span class="sign">{row.sign}</span>
  <span class="code-text">{#each row.spans as span, at (at)}<span class:strong={span.strong}>{span.text}</span>{/each}</span>
</div>

<style>
  .sign {
    flex: none;
    width: 2ch;
    text-align: center;
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
