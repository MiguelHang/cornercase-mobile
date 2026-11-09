<script lang="ts">
  import type { Colours_Serialize } from '../bindings'
  import type { Row } from './pane.svelte'
  import { inline, runCss } from './style'

  let { runs, colours, cell }: { runs: Row; colours: Colours_Serialize; cell: number } = $props()
</script>

<div class="row">
  {#each runs as run, index (index)}
    <span class="run" style={inline(runCss(run.style, colours))} style:width="{run.width * cell}em"
      >{run.text}</span
    >
  {/each}
</div>

<style>
  .row {
    display: flex;
    height: 1lh;
  }

  .run {
    flex: none;
    overflow: hidden;
    white-space: pre;
  }
</style>
