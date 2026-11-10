<script lang="ts">
  import { untrack } from 'svelte'
  import type { Chat } from '../../lib/chat.svelte'
  import { DIGIT_ROW, ROW, type KeyName } from '../../lib/keys'
  import Button from '../../lib/ui/Button.svelte'
  import Icon from '../../lib/ui/Icon.svelte'

  const ARMED = 'Tap again to interrupt'
  const AGAIN = 'Again?'

  let {
    chat,
    initialDigits = false,
    onfailed,
  }: {
    chat: Chat
    initialDigits?: boolean
    onfailed: (reason: string) => void
  } = $props()

  let digits = $state(untrack(() => initialDigits))

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
      {#each DIGIT_ROW as button (button.key)}
        <Button tone="plain" compact label={button.name} onclick={() => void press(button.key)}>{button.label}</Button>
      {/each}
    </div>
  {/if}
  <div class="row" role="group" aria-label="Keys">
    {#each ROW as button (button.key)}
      {@const armed = button.key === 'ctrl+c' && chat.interrupting}
      <Button
        tone={armed ? 'danger' : 'plain'}
        compact
        resting={!chat.canPress}
        label={armed ? ARMED : button.name}
        onclick={() => void press(button.key)}
      >
        {#if button.icon !== undefined}
          <Icon name={button.icon} size={20} />
        {:else if button.key === 'ctrl+c'}
          <span class="steady"><span class:shown={!armed}>{button.label}</span><span class:shown={armed}>{AGAIN}</span></span>
        {:else}
          {button.label}
        {/if}
      </Button>
    {/each}
    <Button tone="plain" compact label="Digits" expanded={digits} onclick={() => (digits = !digits)}>0–9</Button>
  </div>
  <span class="spoken" aria-live="polite">{chat.interrupting ? ARMED : ''}</span>
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

  .steady {
    display: grid;
  }

  .steady > span {
    grid-area: 1 / 1;
    visibility: hidden;
  }

  .steady > .shown {
    visibility: visible;
  }
</style>
