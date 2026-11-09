<script lang="ts">
  import { describe, describePanes, type Screen } from '../lib/connection'

  let { screen }: { screen: Screen } = $props()

  const panes = $derived(describePanes(screen.session))
</script>

<section class="connection">
  <p class="state">{describe(screen.connection)}</p>
  {#if panes !== null}
    <p class="panes">{panes}</p>
  {/if}
</section>

<style>
  .connection {
    padding: 40px var(--inset-wide) 0;
    text-align: center;
  }

  .state {
    margin: 0;
    color: var(--text);
  }

  .panes {
    margin: var(--inset) 0 0;
    color: var(--muted);
  }
</style>
