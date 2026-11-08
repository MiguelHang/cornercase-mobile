<script lang="ts">
  let { size = 22 }: { size?: number } = $props()
</script>

<span class="wordmark" style:font-size="{size}px">cornercase</span>

<style>
  .wordmark {
    color: var(--text);
    font-weight: 700;
  }
</style>
