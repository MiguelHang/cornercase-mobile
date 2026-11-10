<script lang="ts">
  import Button from '../../lib/ui/Button.svelte'
  import FormFrame from './FormFrame.svelte'

  let { sendable, onshow }: { sendable: boolean; onshow: () => void } = $props()
</script>

<FormFrame problem={null} onsubmit={onshow}>
  <p class="note">See the issues of every project and pick one to start an agent on it.</p>
  {#snippet button()}
    <Button type="submit" resting={!sendable}>Show issues</Button>
  {/snippet}
</FormFrame>

<style>
  .note {
    margin: 0;
    color: var(--muted);
    font-size: 13px;
  }
</style>
