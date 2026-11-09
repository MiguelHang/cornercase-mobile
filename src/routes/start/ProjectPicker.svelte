<script lang="ts">
  import type { StartForm } from '../../lib/startForm.svelte'
  import Picker from './Picker.svelte'

  let { form }: { form: StartForm } = $props()
</script>

<Picker
  label="Project"
  choices={form.projects}
  bind:value={form.project}
  of={(choice) => ({ value: choice.id, name: choice.name })}
  error={form.projectError}
  reading="Reading the projects…"
  empty="No project is open on this Mac."
  disabled={form.sending}
  onretry={() => void form.loadProjects()}
/>
