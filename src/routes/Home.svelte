<script lang="ts">
  import { describePanes, type Screen } from '../lib/connection'
  import Mark from '../lib/ui/Mark.svelte'

  let { screen, below = 0 }: { screen: Screen; below?: number } = $props()

  const panes = $derived(describePanes(screen.session))
</script>

<section class="home" style:--below="{below}px">
  <Mark unit={5} pulsing={screen.connection.state === 'connecting'} />
  {#if panes !== null}
    <p class="panes">{panes}</p>
  {/if}
</section>

<style>
  .home {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--inset);
    padding: calc(var(--below) + 40px) var(--inset-wide) 0;
  }

  .panes {
    margin: 0;
    color: var(--muted);
  }
</style>
