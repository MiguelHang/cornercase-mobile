<script lang="ts">
  import Button from './Button.svelte'

  let { ondemo }: { ondemo: () => void } = $props()
</script>

<p class="offer">
  No Mac nearby?
  <Button tone="link" onclick={ondemo}>Try a demo</Button>
</p>

<style>
  .offer {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: center;
    margin: 0;
    color: var(--muted);
    font-size: 15px;
  }
</style>
