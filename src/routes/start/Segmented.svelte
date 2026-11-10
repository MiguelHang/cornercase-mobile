<script lang="ts" generics="V extends string">
  let {
    label,
    options,
    value = $bindable(),
    disabled = false,
  }: {
    label: string
    options: [V, string][]
    value: V
    disabled?: boolean
  } = $props()
</script>

<div class="actions" role="group" aria-label={label}>
  {#each options as [name, words] (name)}
    <button
      class="action"
      type="button"
      aria-pressed={value === name}
      {disabled}
      onclick={() => (value = name)}
      ontouchstart={() => {}}
    >
      {words}
    </button>
  {/each}
</div>

<style>
  .actions {
    display: flex;
    gap: 2px;
    padding: 2px;
    border: 1px solid var(--border);
    border-radius: var(--radius-control);
    background: var(--field);
  }

  .action {
    flex: 1;
    min-height: 40px;
    border: 0;
    border-radius: calc(var(--radius-control) - 2px);
    background: none;
    color: var(--soft);
    font: inherit;
    font-size: 15px;
  }

  .action[aria-pressed='true'] {
    background: var(--selected);
    color: var(--text);
    font-weight: 500;
  }

  .action:disabled {
    opacity: 0.6;
  }
</style>
