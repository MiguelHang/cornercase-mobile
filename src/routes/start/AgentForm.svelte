<script lang="ts">
  import type { StartForm } from '../../lib/startForm.svelte'
  import Button from '../../lib/ui/Button.svelte'
  import Field from '../../lib/ui/Field.svelte'
  import TextArea from '../../lib/ui/TextArea.svelte'
  import Picker from './Picker.svelte'
  import ProjectPicker from './ProjectPicker.svelte'

  let { form, sendable }: { form: StartForm; sendable: boolean } = $props()

  const submit = (event: SubmitEvent) => {
    event.preventDefault()
    void form.start()
  }
</script>

<form class="form" onsubmit={submit}>
  <ProjectPicker {form} />
  <Picker
    label="Agent"
    choices={form.agents}
    bind:value={form.agent}
    of={(choice) => ({ value: choice.agent, name: choice.agent })}
    error={form.agentError}
    reading="Reading the agents…"
    empty="No agent is installed on this Mac."
    disabled={form.sending}
    onretry={() => void form.loadAgents()}
  />
  <Field label="Model" placeholder="The agent's default" bind:value={form.model} disabled={form.sending} />
  <TextArea label="Prompt" bind:value={form.prompt} error={form.promptError} disabled={form.sending} />
  <div class="branch">
    <Field label="Branch" placeholder="None" bind:value={form.worktree} disabled={form.sending} />
    <p class="note">With a branch, the agent works in its own worktree. Without one, it starts in a new tab in the
      project's folder.</p>
  </div>
  {#if form.formError !== null}
    <p class="problem" role="alert">{form.formError}</p>
  {/if}
  <div class="buttons">
    <Button type="submit" resting={!sendable || form.noAgents} busy={form.sending}>
      {form.sending ? 'Starting…' : 'Start'}
    </Button>
  </div>
</form>

<style>
  .form {
    display: flex;
    flex-direction: column;
    gap: var(--gap);
    padding: var(--inset-card);
  }

  .branch {
    display: flex;
    flex-direction: column;
  }

  .note {
    margin: 0;
    color: var(--muted);
    font-size: 13px;
  }

  .problem {
    margin: 0;
    color: var(--failed);
    font-size: 13px;
    overflow-wrap: anywhere;
  }

  .buttons {
    display: flex;
    justify-content: flex-end;
  }
</style>
