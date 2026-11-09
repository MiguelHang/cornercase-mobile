<script lang="ts">
  import { agentTabs, describePanes, type AgentTab, type Screen } from '../lib/connection'
  import Mark from '../lib/ui/Mark.svelte'

  let {
    screen,
    below = 0,
    onopen,
  }: { screen: Screen; below?: number; onopen: (tab: AgentTab) => void } = $props()

  const panes = $derived(describePanes(screen.session))
  const tabs = $derived(agentTabs(screen.session))
</script>

<section class="home" style:--below="{below}px">
  <Mark unit={5} pulsing={screen.connection.state === 'connecting'} />
  {#if panes !== null}
    <p class="panes">{panes}</p>
  {/if}
  {#if tabs.length > 0}
    <ul class="tabs">
      {#each tabs as tab (tab.tab)}
        <li>
          <button class="tab" type="button" onclick={() => onopen(tab)}>
            <span class="line name">{tab.name}</span>
            <span class="line where">{tab.agent} · {tab.where}</span>
          </button>
        </li>
      {/each}
    </ul>
  {/if}
</section>

<style>
  .home {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--inset);
    padding: calc(var(--below) + 40px) var(--inset-wide) 0;
  }

  .panes {
    margin: 0;
    color: var(--muted);
  }

  .tabs {
    display: flex;
    flex-direction: column;
    gap: var(--gap);
    align-self: stretch;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .tab {
    box-sizing: border-box;
    display: flex;
    flex-direction: column;
    width: 100%;
    min-height: 44px;
    padding: var(--gap) var(--inset);
    border: 1px solid var(--border);
    border-radius: var(--radius-control);
    background: var(--panel);
    font: inherit;
    text-align: left;
  }

  .tab:active {
    background: var(--selected);
  }

  .line {
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }

  .name {
    color: var(--text);
    font-size: 15px;
  }

  .where {
    color: var(--muted);
    font-size: 12px;
  }
</style>
