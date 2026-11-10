<script lang="ts">
  import type { StartForm } from '../../lib/startForm.svelte'
  import Picker from './Picker.svelte'

  let { form }: { form: StartForm } = $props()
</script>

<Picker
  label="Workspace"
  choices={form.workspaceChoices}
  bind:value={form.workspace}
  of={(choice) => ({ value: choice.id, name: choice.label })}
  error={form.workspaceError}
  reading="Reading the workspaces…"
  empty="No workspace is open on this Mac."
  disabled={form.sending}
  onretry={() => void form.loadProjects()}
/>
