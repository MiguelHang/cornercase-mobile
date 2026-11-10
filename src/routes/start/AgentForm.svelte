<script lang="ts">
  import type { StartForm } from '../../lib/startForm.svelte'
  import Button from '../../lib/ui/Button.svelte'
  import Field from '../../lib/ui/Field.svelte'
  import TextArea from '../../lib/ui/TextArea.svelte'
  import AgentPicker from './AgentPicker.svelte'
  import FormFrame from './FormFrame.svelte'
  import ProjectPicker from './ProjectPicker.svelte'

  let { form, sendable }: { form: StartForm; sendable: boolean } = $props()
</script>

<FormFrame problem={form.formError} onsubmit={() => void form.start()}>
  <ProjectPicker {form} />
  <AgentPicker {form} />
  <Field label="Model" placeholder="The agent's default" bind:value={form.model} disabled={form.sending} />
  <TextArea label="Prompt" bind:value={form.prompt} error={form.promptError} disabled={form.sending} />
  <div class="branch">
    <Field label="Branch" placeholder="None" bind:value={form.worktree} disabled={form.sending} />
    <p class="note">With a branch, the agent works in its own worktree. Without one, it starts in a new tab in the
      project's folder.</p>
  </div>
  {#snippet button()}
    <Button type="submit" resting={!sendable || form.noAgents} busy={form.sending}>
      {form.sending ? 'Starting…' : 'Start'}
    </Button>
  {/snippet}
</FormFrame>

<style>
  .branch {
    display: flex;
    flex-direction: column;
  }

  .note {
    margin: 0;
    color: var(--muted);
    font-size: 13px;
  }
</style>
