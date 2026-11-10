<script lang="ts">
  import type { StartForm } from '../../lib/startForm.svelte'
  import Button from '../../lib/ui/Button.svelte'
  import Field from '../../lib/ui/Field.svelte'
  import AgentPicker from './AgentPicker.svelte'
  import FormFrame from './FormFrame.svelte'

  let { form, sendable }: { form: StartForm; sendable: boolean } = $props()

  const starting = $derived(form.issue)
  const project = $derived(
    form.projects.kind === 'ready' ? form.projects.items.find((choice) => choice.id === form.project)?.name : undefined,
  )
</script>

<FormFrame problem={form.formError} onsubmit={() => void form.startIssue()}>
  {#if starting !== null}
    <div class="issue">
      <p class="line">
        <span class="key">{starting.issue.key}</span>
        {#if project !== undefined}
          <span class="project">{project}</span>
        {/if}
      </p>
      <p class="name">{starting.issue.title}</p>
    </div>
  {/if}
  <AgentPicker {form} />
  <Field label="Model" placeholder="The agent's default" bind:value={form.model} disabled={form.sending} />
  <div class="branch">
    <Field label="Branch" placeholder={form.issueBranch} bind:value={form.branch} disabled={form.sending} />
    <p class="note">The agent works in its own worktree on this branch. If the project's folder is not the root of a
      git repository, it starts in a new tab there instead.</p>
  </div>
  {#if starting !== null}
    <details class="prompt">
      <summary class="summary">Prompt cornercase will use</summary>
      <p class="said">{starting.prompt}</p>
    </details>
  {/if}
  {#snippet button()}
    <Button type="submit" resting={!sendable || form.noAgents} busy={form.sending}>
      {form.sending ? 'Starting…' : 'Start'}
    </Button>
  {/snippet}
</FormFrame>

<style>
  .issue,
  .branch {
    display: flex;
    flex-direction: column;
  }

  .issue {
    gap: 4px;
  }

  .line {
    display: flex;
    flex-wrap: wrap;
    gap: var(--gap);
    margin: 0;
    color: var(--muted);
    font-size: 13px;
  }

  .key {
    font-family: var(--font-mono);
  }

  .project {
    overflow-wrap: anywhere;
  }

  .name {
    margin: 0;
    font-size: 17px;
    font-weight: 600;
    overflow-wrap: anywhere;
  }

  .note {
    margin: 0;
    color: var(--muted);
    font-size: 13px;
  }

  .summary {
    min-height: 44px;
    color: var(--accent);
    font-size: 14px;
    line-height: 44px;
  }

  .said {
    margin: 0;
    color: var(--soft);
    font-size: 13px;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }
</style>
