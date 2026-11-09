<script lang="ts">
  import type { Chat } from '../../lib/chat.svelte'
  import { DIGIT_ROW, ROW, type KeyName } from '../../lib/keys'
  import Button from '../../lib/ui/Button.svelte'
  import Icon from '../../lib/ui/Icon.svelte'

  const ARMED = 'Tap again to interrupt'

  let {
    chat,
    digits = false,
    onfailed,
  }: {
    chat: Chat
    digits?: boolean
    onfailed: (reason: string) => void
  } = $props()

  const press = async (key: KeyName) => {
    const failed = await chat.press(key)
    if (failed !== null) {
      onfailed(failed)
    }
  }
</script>

<div class="keys">
  {#if digits}
    <div class="row" role="group" aria-label="Digits">
      {#each DIGIT_ROW as key (key.key)}
        <Button tone="plain" compact label={key.name} onclick={() => void press(key.key)}>{key.label}</Button>
      {/each}
    </div>
  {/if}
  <div class="row" role="group" aria-label="Keys">
    {#each ROW as key (key.key)}
      {@const armed = key.key === 'ctrl+c' && chat.interrupting}
      <Button tone={armed ? 'danger' : 'plain'} compact label={armed ? ARMED : key.name} onclick={() => void press(key.key)}>
        {#if key.icon !== undefined}
          <Icon name={key.icon} size={20} />
        {:else if key.key === 'ctrl+c'}
          <span aria-live="polite">{armed ? ARMED : key.label}</span>
        {:else}
          {key.label}
        {/if}
      </Button>
    {/each}
    <Button tone="plain" compact label="Digits" expanded={digits} onclick={() => (digits = !digits)}>0–9</Button>
  </div>
</div>

<style>
  .keys {
    display: flex;
    flex-direction: column;
    gap: var(--gap);
  }

  .row {
    display: flex;
    flex-wrap: wrap;
    gap: var(--gap);
  }
</style>
