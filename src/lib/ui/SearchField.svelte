<script lang="ts">
  import type { FormEventHandler } from 'svelte/elements'
  import Icon from './Icon.svelte'

  let {
    label,
    value = $bindable(''),
    oninput,
  }: {
    label: string
    value?: string
    oninput?: FormEventHandler<HTMLInputElement>
  } = $props()
</script>

<label class="search">
  <Icon name="search" size={18} />
  <input
    class="query"
    type="search"
    aria-label={label}
    placeholder={label}
    autocomplete="off"
    autocapitalize="off"
    autocorrect="off"
    spellcheck="false"
    enterkeyhint="search"
    bind:value
    {oninput}
  />
</label>

<style>
  .search {
    box-sizing: border-box;
    display: flex;
    align-items: center;
    gap: var(--gap);
    min-height: 44px;
    padding: 0 12px;
    border: 1px solid var(--edge);
    border-radius: var(--radius-control);
    background: var(--field);
    color: var(--faint);
  }

  .search:focus-within {
    border-color: var(--accent);
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--brand) 32%, transparent);
  }

  .query {
    flex: 1;
    min-width: 0;
    min-height: 42px;
    padding: 0;
    border: none;
    outline: none;
    background: none;
    color: var(--text);
    font: inherit;
    font-size: 16px;
    -webkit-user-select: text;
    user-select: text;
  }

  .query::placeholder {
    color: var(--faint);
  }
</style>
