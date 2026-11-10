<script lang="ts">
  import type { FlowOption, FlowStart } from '../../lib/flowStart.svelte'
  import Picker from './Picker.svelte'

  let { flows, disabled }: { flows: FlowStart; disabled: boolean } = $props()

  const nameOf = (option: FlowOption) =>
    [option.name, option.agents, option.problem ?? option.description].filter((part) => part !== '').join(' · ')
</script>

<div class="flow">
  <Picker
    label="Flow"
    choices={flows.options}
    bind:value={flows.flow}
    of={(option) => ({ value: option.name, name: nameOf(option), disabled: option.problem !== null })}
    error={flows.flowError}
    reading="Reading the flows…"
    empty="cornercase has no flow for this project."
    {disabled}
    onretry={() => void flows.retry()}
  />
  {#if flows.team.length > 0}
    <ul class="team" aria-label="Team">
      {#each flows.team as member (member)}
        <li class="member">{member}</li>
      {/each}
    </ul>
  {/if}
</div>

<style>
  .flow {
    display: flex;
    flex-direction: column;
  }

  .team {
    display: flex;
    flex-direction: column;
    gap: 2px;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .member {
    color: var(--muted);
    font-size: 13px;
    overflow-wrap: anywhere;
  }
</style>
