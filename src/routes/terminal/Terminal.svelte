<script lang="ts">
  import { onMount } from 'svelte'
  import {
    commands,
    events,
    type AgentRow_Serialize as AgentRow,
    type Size_Deserialize,
  } from '../../lib/bindings'
  import { label, place } from '../../lib/board'
  import { Opening } from '../../lib/terminal/opening'
  import { Pane, shows } from '../../lib/terminal/pane.svelte'
  import TerminalView from '../../lib/terminal/Terminal.svelte'
  import Icon from '../../lib/ui/Icon.svelte'

  let {
    machine,
    row,
    onback,
    onfailed,
    height = $bindable(0),
  }: {
    machine: number
    row: AgentRow
    onback: () => void
    onfailed: (error: unknown) => void
    height?: number
  } = $props()

  const shown = new Pane()
  let wanted = $state.raw<Size_Deserialize | null>(null)
  let opening: Opening | null = null

  onMount(() => {
    const unlisten = events.terminalChanged.listen((event) => {
      if (shows(event.payload, machine, row.tab)) {
        shown.apply(event.payload)
      }
    })
    const open = (size: Size_Deserialize) => commands.terminalOpen(row.tab, row.pane, size)
    const close = () => commands.terminalClose(row.tab)
    opening = new Opening(unlisten, { open, close, failed: onfailed })
    return () => {
      unlisten.then((stop) => stop()).catch(onfailed)
      opening?.leave()
    }
  })

  $effect(() => {
    if (wanted !== null) {
      opening?.open(wanted)
    }
  })
</script>

<section class="screen">
  <header class="bar" bind:clientHeight={height}>
    <button class="back" type="button" aria-label="Back" onclick={onback}><Icon name="chevron-left" /></button>
    <span class="lines">
      <span class="line name">{label(row)}</span>
      <span class="line where">{row.tab_name} · {place(row)}</span>
    </span>
  </header>
  <div class="view">
    <TerminalView
      pane={shown.current}
      place={{ machine, boot: row.boot, tab: row.tab }}
      onfit={(size) => (wanted = size)}
    />
  </div>
  {#if shown.current.closed}
    <p class="closed" role="status">This tab was closed on the Mac</p>
  {/if}
</section>

<style>
  .screen {
    position: fixed;
    inset: 0;
    z-index: 1;
    box-sizing: border-box;
    display: flex;
    flex-direction: column;
    padding: env(safe-area-inset-top) env(safe-area-inset-right) env(safe-area-inset-bottom)
      env(safe-area-inset-left);
    background: var(--glass);
  }

  .bar {
    box-sizing: border-box;
    display: flex;
    flex: none;
    align-items: center;
    gap: var(--gap);
    min-height: 44px;
    padding: 0 var(--inset-wide) 0 var(--gap);
    border-bottom: 1px solid var(--border);
  }

  .back {
    display: inline-flex;
    flex: none;
    align-items: center;
    justify-content: center;
    width: 44px;
    height: 44px;
    padding: 0;
    border: 0;
    background: none;
    color: var(--soft);
  }

  .back:active {
    color: var(--text);
  }

  .lines {
    display: flex;
    flex: 1;
    flex-direction: column;
    min-width: 0;
  }

  .line {
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }

  .name {
    color: var(--text);
    font-size: 15px;
  }

  .where {
    color: var(--muted);
    font-size: 12px;
  }

  .view {
    flex: 1;
    min-height: 0;
  }

  .closed {
    flex: none;
    margin: 0;
    padding: var(--inset) var(--inset-wide);
    border-top: 1px solid var(--border);
    color: var(--failed);
    font-size: 14px;
  }
</style>
