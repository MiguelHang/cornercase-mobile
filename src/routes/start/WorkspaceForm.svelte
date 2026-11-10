<script lang="ts">
  import type { StartForm } from '../../lib/startForm.svelte'
  import Button from '../../lib/ui/Button.svelte'
  import Field from '../../lib/ui/Field.svelte'
  import Switch from '../../lib/ui/Switch.svelte'
  import FormFrame from './FormFrame.svelte'
  import ProjectPicker from './ProjectPicker.svelte'

  let { form, sendable }: { form: StartForm; sendable: boolean } = $props()
</script>

<FormFrame problem={form.formError} onsubmit={() => void form.create()}>
  <ProjectPicker {form} />
  <Switch label="Own worktree" bind:checked={form.ownWorktree} disabled={form.sending} />
  <Field
    label={form.ownWorktree ? 'Branch' : 'Name'}
    bind:value={form.name}
    error={form.nameError}
    disabled={form.sending}
  />
  {#snippet button()}
    <Button type="submit" resting={!sendable} busy={form.sending}>
      {form.sending ? 'Creating…' : 'Create workspace'}
    </Button>
  {/snippet}
</FormFrame>
