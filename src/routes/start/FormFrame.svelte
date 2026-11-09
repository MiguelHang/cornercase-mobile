<script lang="ts">
  import type { Snippet } from 'svelte'

  let {
    problem,
    onsubmit,
    children,
    button,
  }: {
    problem: string | null
    onsubmit: () => void
    children: Snippet
    button: Snippet
  } = $props()

  const submit = (event: SubmitEvent) => {
    event.preventDefault()
    onsubmit()
  }
</script>

<form class="form" onsubmit={submit}>
  {@render children()}
  {#if problem !== null}
    <p class="problem" role="alert">{problem}</p>
  {/if}
  <div class="buttons">
    {@render button()}
  </div>
</form>

<style>
  .form {
    display: flex;
    flex-direction: column;
    gap: var(--gap);
    padding: var(--inset-card);
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
