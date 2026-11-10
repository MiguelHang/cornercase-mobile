<script lang="ts">
  import { placeKey, type StartForm } from '../../lib/startForm.svelte'
  import Button from '../../lib/ui/Button.svelte'
  import Field from '../../lib/ui/Field.svelte'
  import AgentPicker from './AgentPicker.svelte'
  import FormFrame from './FormFrame.svelte'
  import Picker from './Picker.svelte'

  let { form, sendable }: { form: StartForm; sendable: boolean } = $props()

  const starting = $derived(form.issue)
  const chosen = $derived(form.chosenPlace)
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
  <div class="where">
    <Picker
      label="Where"
      choices={form.placeChoices}
      bind:value={form.place}
      of={(place) => ({ value: placeKey(place), name: form.placeName(place) })}
      error={form.placeError}
      empty="There is no place to start an agent on this issue."
      disabled={form.sending}
    />
    {#if chosen?.worktree}
      <Field label="Branch" placeholder={form.issueBranch} bind:value={form.branch} disabled={form.sending} />
      {#if form.issueBranch === ''}
        <p class="note">Left empty, cornercase names the branch.</p>
      {/if}
      <p class="note">The agent works in its own worktree on this branch. If the project's folder is not the root of a
        git repository, it starts in a new tab there instead.</p>
    {:else if chosen !== undefined}
      <p class="note">The agent starts in a new tab there.</p>
    {/if}
  </div>
  <AgentPicker {form} />
  <Field label="Model" placeholder="The agent's default" bind:value={form.model} disabled={form.sending} />
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
  .where {
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
