<script lang="ts">
  import type { StartForm } from '../../lib/startForm.svelte'
  import Button from '../../lib/ui/Button.svelte'
  import Field from '../../lib/ui/Field.svelte'
  import FormFrame from './FormFrame.svelte'
  import ProjectPicker from './ProjectPicker.svelte'

  let { form, sendable }: { form: StartForm; sendable: boolean } = $props()
</script>

<FormFrame problem={form.formError} onsubmit={() => void form.create()}>
  <ProjectPicker {form} />
  <label class="switch">
    <span class="words">Own worktree</span>
    <input class="toggle" type="checkbox" role="switch" bind:checked={form.ownWorktree} disabled={form.sending} />
  </label>
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

<style>
  .switch {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--gap);
    min-height: 44px;
  }

  .words {
    color: var(--text);
    font-size: 16px;
  }

  .toggle {
    position: relative;
    flex: none;
    width: 51px;
    height: 31px;
    margin: 0;
    border-radius: 16px;
    background: var(--edge);
    -webkit-appearance: none;
    appearance: none;
  }

  .toggle::after {
    content: '';
    position: absolute;
    top: 2px;
    left: 2px;
    width: 27px;
    height: 27px;
    border-radius: 50%;
    background: var(--text);
  }

  .toggle:checked {
    background: var(--brand);
  }

  .toggle:checked::after {
    transform: translateX(20px);
  }

  .toggle:disabled {
    opacity: 0.6;
  }
</style>
