<script lang="ts">
  import { untrack } from 'svelte'
  import { commands, type ConnectionView_Serialize, type MachineView_Serialize } from '../lib/bindings'
  import { routeAddress } from '../lib/connection'
  import { EditMachine } from '../lib/editMachine.svelte'
  import Button from '../lib/ui/Button.svelte'
  import Field from '../lib/ui/Field.svelte'
  import Glass from '../lib/ui/Glass.svelte'
  import Icon from '../lib/ui/Icon.svelte'
  import NavBar from '../lib/ui/NavBar.svelte'
  import UrlField from '../lib/ui/UrlField.svelte'

  let {
    machine,
    taken,
    connection,
    backTo,
    onback,
    onchanged,
    onremoved,
    adding = false,
    edit = new EditMachine(
      commands,
      () => machine,
      () => onchanged(),
      () => onremoved(),
    ),
  }: {
    machine: MachineView_Serialize
    taken: string[]
    connection: ConnectionView_Serialize | null
    backTo: string | null
    onback: () => void
    onchanged: () => Promise<unknown>
    onremoved: () => Promise<unknown>
    adding?: boolean
    edit?: EditMachine
  } = $props()

  untrack(() => {
    if (adding) {
      edit.open()
    }
  })

  const connected = $derived(connection?.state === 'connected' ? connection.route : null)

  const rename = (event: SubmitEvent) => {
    event.preventDefault()
    void edit.rename(taken)
  }

  const addRoute = (event: SubmitEvent) => {
    event.preventDefault()
    void edit.addRoute()
  }
</script>

<section class="screen">
  <NavBar {backTo} title={machine.name} {onback} />
  <Glass>
    <form class="form" onsubmit={rename}>
      <Field label="Name" bind:value={edit.name} error={edit.nameError} disabled={edit.busy} />
      <div class="buttons">
        <Button type="submit" resting={edit.unchanged} busy={edit.busy}>Rename</Button>
      </div>
    </form>
  </Glass>
  <h2 class="heading">Routes</h2>
  <Glass>
    <ul class="routes">
      {#each machine.routes as route, index (route.url)}
        <li class="route">
          <span class="about">
            <span class="address">{routeAddress(route.url)}</span>
            {#if route.url === connected}
              <span class="mark done">Connected</span>
            {/if}
            {#if !route.has_token}
              <span class="mark failed">No token</span>
            {/if}
          </span>
          <button
            class="tool"
            type="button"
            aria-label="Move up"
            disabled={edit.busy || index === 0}
            onclick={() => void edit.move(route.url, 'up')}
            ontouchstart={() => {}}
          >
            <Icon name="chevron-up" size={20} />
          </button>
          <button
            class="tool"
            type="button"
            aria-label="Move down"
            disabled={edit.busy || index === machine.routes.length - 1}
            onclick={() => void edit.move(route.url, 'down')}
            ontouchstart={() => {}}
          >
            <Icon name="chevron-down" size={20} />
          </button>
          <button
            class="tool"
            type="button"
            aria-label="Remove route"
            disabled={edit.busy || machine.routes.length === 1}
            onclick={() => void edit.remove(route.url)}
            ontouchstart={() => {}}
          >
            <Icon name="trash-2" size={20} />
          </button>
        </li>
      {/each}
    </ul>
  </Glass>
  {#if edit.formError !== null}
    <p class="problem" role="alert">{edit.formError}</p>
  {/if}
  {#if edit.deleting}
    <Glass>
      <div class="form">
        <h2 class="heading">Delete {machine.name}?</h2>
        <p class="lost">Its routes and their tokens are removed from this phone. The Mac and its bridge are not touched.</p>
        <div class="buttons">
          <Button tone="plain" resting={edit.busy} onclick={() => edit.keep()}>Cancel</Button>
          <Button tone="danger" busy={edit.busy} onclick={() => void edit.deleteMachine()}>Delete</Button>
        </div>
      </div>
    </Glass>
  {:else if edit.adding}
    <Glass>
      <form class="form" onsubmit={addRoute}>
        <h2 class="heading">Add route</h2>
        <UrlField
          bind:value={edit.url}
          error={edit.urlError}
          disabled={edit.busy}
          onscanned={(url) => void edit.fill(url)}
        />
        <div class="buttons">
          <Button tone="plain" resting={edit.busy} onclick={() => edit.close()}>Cancel</Button>
          <Button type="submit" busy={edit.busy}>{edit.busy ? 'Adding…' : 'Add'}</Button>
        </div>
      </form>
    </Glass>
  {:else}
    <div class="actions">
      <Button resting={edit.busy} onclick={() => edit.open()}>Add route</Button>
      <Button tone="danger" resting={edit.busy} onclick={() => edit.askToDelete()}>Delete machine</Button>
    </div>
  {/if}
</section>

<style>
  .screen {
    display: flex;
    flex-direction: column;
    gap: var(--inset-wide);
    max-width: 520px;
    margin: 0 auto;
    padding: calc(env(safe-area-inset-top) + var(--gap)) var(--inset-wide) 40px;
  }

  .tool {
    display: flex;
    flex: none;
    align-items: center;
    justify-content: center;
    width: 44px;
    height: 44px;
    padding: 0;
    border: 0;
    border-radius: var(--radius-control);
    background: none;
    color: var(--accent);
  }

  .tool:active {
    background: var(--hover);
  }

  .tool:disabled {
    color: var(--faint);
    background: none;
  }

  .form {
    display: flex;
    flex-direction: column;
    gap: var(--gap);
    padding: var(--inset-card);
  }

  .buttons {
    display: flex;
    justify-content: flex-end;
    gap: var(--gap);
  }

  .heading {
    margin: 0;
    padding: 0 var(--inset);
    color: var(--soft);
    font-size: 13px;
    font-weight: 600;
  }

  .form .heading {
    padding: 0;
  }

  .routes {
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .route {
    display: flex;
    align-items: center;
    gap: 2px;
    padding: var(--gap) var(--inset) var(--gap) var(--inset-wide);
    border-top: 1px solid var(--border);
  }

  .route:first-child {
    border-top: 0;
  }

  .about {
    display: flex;
    flex: 1;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }

  .address {
    overflow: hidden;
    color: var(--text);
    font-family: var(--font-mono);
    font-size: 14px;
    white-space: nowrap;
    text-overflow: ellipsis;
  }

  .mark {
    font-size: 12px;
  }

  .done {
    color: var(--done);
  }

  .failed {
    color: var(--failed);
  }

  .problem {
    margin: 0;
    padding: 0 var(--inset);
    color: var(--failed);
    font-size: 13px;
  }

  .lost {
    margin: 0;
    color: var(--soft);
    font-size: 14px;
  }

  .actions {
    display: flex;
    justify-content: center;
    gap: var(--gap);
  }
</style>
