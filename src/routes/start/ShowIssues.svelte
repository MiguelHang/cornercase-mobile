<script lang="ts">
  import type { StartForm } from '../../lib/startForm.svelte'
  import Button from '../../lib/ui/Button.svelte'
  import FormFrame from './FormFrame.svelte'
  import ProjectPicker from './ProjectPicker.svelte'

  let { form, sendable, onshow }: { form: StartForm; sendable: boolean; onshow: (project: number) => void } = $props()

  const show = () => {
    const project = form.chooseProject()
    if (project !== null) {
      onshow(project)
    }
  }
</script>

<FormFrame problem={null} onsubmit={show}>
  <ProjectPicker {form} />
  <p class="note">Pick one of the project's issues to start an agent on it.</p>
  {#snippet button()}
    <Button type="submit" resting={!sendable}>Show issues</Button>
  {/snippet}
</FormFrame>

<style>
  .note {
    margin: 0;
    color: var(--muted);
    font-size: 13px;
  }
</style>
