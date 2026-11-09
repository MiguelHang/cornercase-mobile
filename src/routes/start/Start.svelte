<script lang="ts">
  import { onMount, untrack } from 'svelte'
  import { commands, type ConnectionView_Serialize, type Started_Serialize as Started } from '../../lib/bindings'
  import { StartForm } from '../../lib/startForm.svelte'
  import Glass from '../../lib/ui/Glass.svelte'
  import Icon from '../../lib/ui/Icon.svelte'
  import AgentForm from './AgentForm.svelte'
  import WorkspaceForm from './WorkspaceForm.svelte'

  type Action = 'agent' | 'workspace'

  let {
    connection,
    boot = null,
    onback,
    onstarted,
    oncreated,
    form = new StartForm(
      commands,
      (started, agent) => onstarted(started, agent),
      (started, name) => oncreated(started, name),
    ),
  }: {
    connection: ConnectionView_Serialize
    boot?: string | null
    onback: () => void
    onstarted: (started: Started, agent: string) => void
    oncreated: (started: Started, name: string) => void
    form?: StartForm
  } = $props()

  const ACTIONS: [Action, string][] = [
    ['agent', 'Agent'],
    ['workspace', 'Workspace'],
  ]

  let action = $state<Action>('agent')

  const sendable = $derived(connection.state === 'connected')

  onMount(() => {
    void form.load()
  })

  $effect(() => {
    const seen = boot
    untrack(() => void form.see(seen))
  })
</script>

<section class="screen">
  <header class="top">
    <button class="back" type="button" aria-label="Back" onclick={onback} ontouchstart={() => {}}>
      <Icon name="chevron-left" size={24} />
    </button>
    <h1 class="title">Start</h1>
  </header>
  <div class="actions" role="group" aria-label="What to start">
    {#each ACTIONS as [name, words] (name)}
      <button
        class="action"
        type="button"
        aria-pressed={action === name}
        disabled={form.sending}
        onclick={() => (action = name)}
        ontouchstart={() => {}}
      >
        {words}
      </button>
    {/each}
  </div>
  {#if !sendable}
    <p class="offline">Not connected to the Mac. What you type stays here until it connects.</p>
  {/if}
  <Glass>
    {#if action === 'agent'}
      <AgentForm {form} {sendable} />
    {:else}
      <WorkspaceForm {form} {sendable} />
    {/if}
  </Glass>
</section>

<style>
  .screen {
    display: flex;
    flex-direction: column;
    gap: var(--inset-wide);
    max-width: 520px;
    margin: 0 auto;
    padding: calc(env(safe-area-inset-top) + var(--gap)) var(--inset-wide) 40px;
  }

  .top {
    display: flex;
    align-items: center;
    gap: var(--gap);
    margin-left: calc(-1 * var(--inset));
  }

  .back {
    display: flex;
    flex: none;
    align-items: center;
    justify-content: center;
    width: 44px;
    height: 44px;
    padding: 0;
    border: 0;
    border-radius: var(--radius-control);
    background: none;
    color: var(--accent);
  }

  .back:active {
    background: var(--hover);
  }

  .title {
    margin: 0;
    font-size: 22px;
    font-weight: 600;
  }

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

  .offline {
    margin: 0;
    padding: 0 var(--inset);
    color: var(--muted);
    font-size: 13px;
  }
</style>
