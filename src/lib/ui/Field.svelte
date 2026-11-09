<script lang="ts">
  import type { HTMLInputAttributes } from 'svelte/elements'

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

<div class="field">
  <label class="label" for="{id}-input">{label}</label>
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
  <p class="error" id="{id}-error">{error ?? ''}</p>
</div>

<style>
  .field {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .label {
    color: var(--soft);
    font-size: 13px;
    font-weight: 500;
  }

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
    box-shadow: 0 0 0 3px rgb(0 95 255 / 0.32);
  }

  .input:disabled {
    opacity: 0.6;
  }

  .input::placeholder {
    color: var(--faint);
  }

  .error {
    min-height: 16px;
    margin: 0;
    color: var(--failed);
    font-size: 13px;
    line-height: 16px;
  }
</style>
