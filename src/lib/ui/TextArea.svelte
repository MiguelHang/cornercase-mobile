<script lang="ts">
  import type { HTMLTextareaAttributes } from 'svelte/elements'

  let {
    label,
    value = $bindable(''),
    ...rest
  }: {
    label: string
    value?: string
  } & Omit<HTMLTextareaAttributes, 'value'> = $props()
</script>

<div class="grow" data-value={value}>
  <textarea class="area" rows="1" aria-label={label} {...rest} bind:value></textarea>
</div>

<style>
  .grow {
    display: grid;
    min-width: 0;
  }

  .grow::after {
    content: attr(data-value) ' ';
    visibility: hidden;
    overflow: hidden;
    white-space: pre-wrap;
    overflow-wrap: break-word;
  }

  .area,
  .grow::after {
    grid-area: 1 / 1;
    box-sizing: border-box;
    min-height: 44px;
    max-height: 152px;
    padding: 10px 12px;
    border: 1px solid var(--edge);
    font: inherit;
    font-size: 16px;
    line-height: 22px;
  }

  .area {
    overflow-y: auto;
    resize: none;
    border-radius: var(--radius-control);
    outline: none;
    background: var(--field);
    color: var(--text);
    -webkit-user-select: text;
    user-select: text;
  }

  .area:focus {
    border-color: var(--accent);
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--brand) 32%, transparent);
  }

  .area:disabled {
    opacity: 0.6;
  }

  .area::placeholder {
    color: var(--faint);
  }
</style>
