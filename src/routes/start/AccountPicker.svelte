<script lang="ts">
  import type { StartAccounts } from '../../lib/startAccounts.svelte'
  import Picker from './Picker.svelte'

  let {
    label,
    accounts,
    agent,
    disabled,
  }: { label: string; accounts: StartAccounts; agent: string; disabled: boolean } = $props()

  const options = $derived(accounts.optionsOf(agent))
</script>

{#if options.length > 0}
  <div class="account">
    <Picker
      {label}
      choices={{ kind: 'ready', items: options }}
      bind:value={() => accounts.chosen(agent), (name) => accounts.choose(agent, name)}
      of={(option) => option}
      error={null}
      empty=""
      {disabled}
    />
    {#if accounts.signedOut(agent)}
      <p class="note">This account is signed out: the agent will ask to log in on the machine.</p>
    {/if}
  </div>
{/if}

<style>
  .account {
    display: flex;
    flex-direction: column;
  }

  .note {
    margin: 0;
    color: var(--muted);
    font-size: 13px;
  }
</style>
