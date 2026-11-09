<script module lang="ts">
  import { Zooms } from './zoom'

  const zooms = new Zooms()
</script>

<script lang="ts">
  import type { PaneState } from './pane.svelte'
  import { Pinch } from './pinch'
  import Row from './Row.svelte'
  import { paneColours, rgb } from './style'
  import { BASE, clamp, limits, type Place } from './zoom'

  const PROBE = 'M'.repeat(100)
  const ADVANCE = 0.6

  let { pane, place }: { pane: PaneState; place: Place } = $props()

  let width = $state(0)
  let measured = $state(0)
  let start = BASE
  let byGesture = false
  const pinch = new Pinch()

  const cell = $derived(measured > 0 ? measured : BASE * ADVANCE)
  const cols = $derived(pane.size.cols)
  const bounds = $derived(limits(cols, width, cell))
  const ink = $derived(paneColours(pane.colours))
  let size = $derived(zooms.recall(place, bounds))

  const measure = (probe: HTMLElement) => {
    measured = probe.getBoundingClientRect().width / PROBE.length
  }

  const zoom = (scale: number) => {
    size = clamp(start * scale, bounds)
    zooms.remember(place, size)
  }

  const scaleOf = (event: Event) => ('scale' in event && typeof event.scale === 'number' ? event.scale : 1)

  const listen = (element: HTMLElement, listeners: Record<string, (event: Event) => void>) => {
    const entries = Object.entries(listeners)
    for (const [name, listener] of entries) element.addEventListener(name, listener)
    return () => {
      for (const [name, listener] of entries) element.removeEventListener(name, listener)
    }
  }

  const gestures = (element: HTMLElement) =>
    listen(element, {
      gesturestart: (event) => {
        event.preventDefault()
        byGesture = true
        start = size
      },
      gesturechange: (event) => {
        event.preventDefault()
        zoom(scaleOf(event))
      },
    })

  const pressed = (event: PointerEvent) => {
    if (pinch.press(event.pointerId, [event.clientX, event.clientY])) start = size
  }

  const moved = (event: PointerEvent) => {
    const scale = pinch.move(event.pointerId, [event.clientX, event.clientY])
    if (scale !== null && !byGesture) zoom(scale)
  }

  const lifted = (event: PointerEvent) => pinch.lift(event.pointerId)

  const pointers = (element: HTMLElement) =>
    listen(element, {
      pointerdown: (event) => event instanceof PointerEvent && pressed(event),
      pointermove: (event) => event instanceof PointerEvent && moved(event),
      pointerup: (event) => event instanceof PointerEvent && lifted(event),
      pointercancel: (event) => event instanceof PointerEvent && lifted(event),
    })
</script>

<div class="terminal" style:background={rgb(ink.bg)} bind:clientWidth={width} {@attach gestures} {@attach pointers}>
  <div
    class="grid"
    style:font-size="{size}px"
    style:--cell="{cell / BASE}em"
    style:width="calc(var(--cell) * {cols})"
    style:color={rgb(ink.fg)}
  >
    {#each pane.rows as runs, row (row)}
      <Row {runs} colours={pane.colours} cell={cell / BASE} />
    {/each}
    {#if pane.cursor}
      <span
        class="cursor {pane.cursor.shape}"
        style:left="calc(var(--cell) * {pane.cursor.x})"
        style:top="calc(var(--line) * {pane.cursor.y})"
        style:--cursor={rgb(ink.cursor)}
      ></span>
    {/if}
  </div>
  <span class="probe" style:font-size="{BASE}px" aria-hidden="true" {@attach measure}>{PROBE}</span>
</div>

<style>
  .terminal {
    position: relative;
    width: 100%;
    height: 100%;
    overflow: auto;
    overscroll-behavior: contain;
    touch-action: pan-x pan-y;
  }

  .grid,
  .probe {
    font-family: var(--font-mono);
    white-space: pre;
  }

  .grid {
    --line: 1.3em;
    position: relative;
    line-height: var(--line);
  }

  .probe {
    position: absolute;
    top: 0;
    left: 0;
    visibility: hidden;
    pointer-events: none;
  }

  .cursor {
    position: absolute;
    box-sizing: border-box;
    width: var(--cell);
    height: var(--line);
    pointer-events: none;
  }

  .block {
    background: var(--cursor);
    mix-blend-mode: difference;
  }

  .block_hollow {
    border: 1px solid var(--cursor);
  }

  .bar {
    border-left: 2px solid var(--cursor);
  }

  .underline {
    border-bottom: 2px solid var(--cursor);
  }
</style>
