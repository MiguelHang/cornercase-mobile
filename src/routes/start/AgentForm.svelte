<script lang="ts">
  import type { StartForm } from '../../lib/startForm.svelte'
  import Button from '../../lib/ui/Button.svelte'
  import Field from '../../lib/ui/Field.svelte'
  import TextArea from '../../lib/ui/TextArea.svelte'
  import AgentPicker from './AgentPicker.svelte'
  import FormFrame from './FormFrame.svelte'
  import WorkspacePicker from './WorkspacePicker.svelte'

  let { form, sendable }: { form: StartForm; sendable: boolean } = $props()
</script>

<FormFrame problem={form.formError} onsubmit={() => void form.start()}>
  <WorkspacePicker {form} />
  <AgentPicker {form} />
  <Field label="Model" placeholder="The agent's default" bind:value={form.model} disabled={form.sending} />
  <TextArea label="Prompt" bind:value={form.prompt} error={form.promptError} disabled={form.sending} />
  {#snippet button()}
    <Button type="submit" resting={!sendable || form.noAgents} busy={form.sending}>
      {form.sending ? 'Starting…' : 'Start'}
    </Button>
  {/snippet}
</FormFrame>
