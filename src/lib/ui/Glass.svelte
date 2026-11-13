<script lang="ts">
  import type { Snippet } from 'svelte'

  let { sheet = false, children }: { sheet?: boolean; children: Snippet } = $props()
</script>

<div class="glass" class:sheet>
  <span class="highlight"></span>
  {@render children()}
</div>

<style>
  .glass {
    position: relative;
    border: 1px solid var(--border);
    border-radius: var(--radius-card);
    background: linear-gradient(180deg, #1c1f24e0, #101215e8);
    box-shadow:
      0 24px 60px rgb(0 0 0 / 0.5),
      0 4px 12px rgb(0 0 0 / 0.3);
    -webkit-backdrop-filter: blur(24px);
    backdrop-filter: blur(24px);
  }

  .highlight {
    position: absolute;
    top: 0;
    left: var(--radius-card);
    right: var(--radius-card);
    height: 1px;
    background: linear-gradient(90deg, rgb(236 238 241 / 0.18), rgb(236 238 241 / 0.02));
  }

  .sheet {
    border-bottom: 0;
    border-radius: var(--radius-sheet) var(--radius-sheet) 0 0;
  }

  .sheet .highlight {
    left: var(--radius-sheet);
    right: var(--radius-sheet);
  }
</style>
