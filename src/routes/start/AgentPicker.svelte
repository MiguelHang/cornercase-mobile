<script lang="ts">
  import type { StartForm } from '../../lib/startForm.svelte'
  import Picker from './Picker.svelte'

  let { form }: { form: StartForm } = $props()
</script>

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
