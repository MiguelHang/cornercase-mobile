<script lang="ts">
  import type { HTMLInputAttributes } from 'svelte/elements'
  import Labelled from './Labelled.svelte'

  const URL_MODE = {
    autocapitalize: 'off',
    autocorrect: 'off',
    spellcheck: false,
    inputmode: 'url',
  } as const

  let {
    label,
    value = $bindable(''),
    error = null,
    url = false,
    ...rest
  }: {
    label: string
    value?: string
    error?: string | null
    url?: boolean
  } & Omit<HTMLInputAttributes, 'value'> = $props()

  const id = $props.id()
</script>

<Labelled {id} {label} {error}>
  <input
    id="{id}-input"
    class="input"
    type="text"
    autocomplete="off"
    {...(url ? URL_MODE : {})}
    {...rest}
    aria-invalid={error === null ? undefined : 'true'}
    aria-describedby={error === null ? undefined : `${id}-error`}
    bind:value
  />
</Labelled>

<style>
  .input {
    box-sizing: border-box;
    min-height: 44px;
    padding: 0 12px;
    border: 1px solid var(--edge);
    border-radius: var(--radius-control);
    outline: none;
    background: var(--field);
    color: var(--text);
    font: inherit;
    font-size: 16px;
    -webkit-user-select: text;
    user-select: text;
  }

  .input:focus {
    border-color: var(--accent);
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--brand) 32%, transparent);
  }

  .input:disabled {
    opacity: 0.6;
  }

  .input::placeholder {
    color: var(--faint);
  }
</style>
