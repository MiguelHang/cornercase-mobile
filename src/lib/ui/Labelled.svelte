<script lang="ts">
  import type { Snippet } from 'svelte'

  let {
    id,
    label,
    error,
    control = true,
    children,
  }: {
    id: string
    label: string
    error: string | null
    control?: boolean
    children: Snippet
  } = $props()
</script>

<div class="field">
  {#if control}
    <label class="label" for="{id}-input">{label}</label>
  {:else}
    <p class="label">{label}</p>
  {/if}
  {@render children()}
  <p class="error" id="{id}-error">{error ?? ''}</p>
</div>

<style>
  .field {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .label {
    margin: 0;
    color: var(--soft);
    font-size: 13px;
    font-weight: 500;
  }

  .error {
    min-height: 16px;
    margin: 0;
    color: var(--failed);
    font-size: 13px;
    line-height: 16px;
  }
</style>
