<script lang="ts">
  import type { StartForm } from '../../lib/startForm.svelte'
  import Button from '../../lib/ui/Button.svelte'
  import Field from '../../lib/ui/Field.svelte'
  import Switch from './Switch.svelte'
  import TextArea from '../../lib/ui/TextArea.svelte'
  import AgentPicker from './AgentPicker.svelte'
  import FormFrame from './FormFrame.svelte'
  import WorkspacePicker from './WorkspacePicker.svelte'

  let { form, sendable }: { form: StartForm; sendable: boolean } = $props()
</script>

<FormFrame problem={form.formError} onsubmit={() => void form.start()}>
  <WorkspacePicker {form} />
  <div class="where">
    <Switch label="Own worktree" bind:checked={form.ownWorktree} disabled={form.sending} />
    {#if form.ownWorktree}
      <Field label="Branch" bind:value={form.branch} error={form.branchError} disabled={form.sending} />
      <p class="note">The agent works in its own worktree on this branch, made from the workspace's project.</p>
    {:else}
      <p class="note">The agent starts in a new tab in this workspace.</p>
    {/if}
  </div>
  <AgentPicker {form} />
  <Field label="Model" placeholder="The agent's default" bind:value={form.model} disabled={form.sending} />
  <TextArea label="Prompt" bind:value={form.prompt} error={form.promptError} disabled={form.sending} />
  {#snippet button()}
    <Button type="submit" resting={!sendable || form.noAgents || form.noWorkspaces} busy={form.sending}>
      {form.sending ? 'Starting…' : 'Start'}
    </Button>
  {/snippet}
</FormFrame>

<style>
  .where {
    display: flex;
    flex-direction: column;
  }

  .note {
    margin: 0;
    color: var(--muted);
    font-size: 13px;
  }
</style>
