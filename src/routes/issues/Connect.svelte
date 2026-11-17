<script lang="ts">
  import type { SourceInfo_Serialize as SourceInfo } from '../../lib/bindings'
  import Button from '../../lib/ui/Button.svelte'
  import Glass from '../../lib/ui/Glass.svelte'

  let { sources, onretry }: { sources: SourceInfo[]; onretry: () => void } = $props()

  const inRepository = (source: SourceInfo) =>
    source.source === 'gitlab'
      ? 'On the machine, open a folder inside the git repository whose remote is on your GitLab server as the project.'
      : 'On the machine, open a folder inside the git repository as the project.'

  const signIn = (source: SourceInfo) =>
    source.source === 'github'
      ? "On the machine, sign in to gh in the project's folder."
      : "On the machine, connect it in cornercase's Settings › Issues."

  const howTo = (source: SourceInfo) => {
    const repository = source.missing.includes('repository')
    const settings = !repository || source.missing.some((what) => what !== 'repository')
    return [...(repository ? [inRepository(source)] : []), ...(settings ? [signIn(source)] : [])]
  }
</script>

<Glass>
  <div class="connect">
    <h2 class="heading">No issue tracker is connected</h2>
    {#if sources.length === 0}
      <p class="how">Connect one on the machine, in cornercase's Settings › Issues.</p>
    {:else}
      <ul class="trackers">
        {#each sources as source (source.source)}
          <li class="tracker">
            <span class="name">{source.name}</span>
            {#if source.missing.length > 0}
              <span class="why">Missing: {source.missing.join(', ')}</span>
            {/if}
            {#if source.reason !== null}
              <span class="why" class:failing={source.state === 'failing'}>{source.reason}</span>
            {/if}
            {#each howTo(source) as how (how)}
              <span class="how">{how}</span>
            {/each}
          </li>
        {/each}
      </ul>
    {/if}
    <div class="actions">
      <Button tone="plain" onclick={onretry}>Check again</Button>
    </div>
  </div>
</Glass>

<style>
  .connect {
    display: flex;
    flex-direction: column;
    gap: var(--inset);
    padding: var(--inset-wide);
  }

  .heading {
    margin: 0;
    font-size: 17px;
    font-weight: 600;
  }

  .trackers {
    display: flex;
    flex-direction: column;
    gap: var(--inset);
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .tracker {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .name {
    font-weight: 500;
  }

  .why,
  .how {
    margin: 0;
    color: var(--muted);
    font-size: 14px;
    overflow-wrap: anywhere;
  }

  .failing {
    color: var(--failed);
  }

  .actions {
    display: flex;
  }
</style>
