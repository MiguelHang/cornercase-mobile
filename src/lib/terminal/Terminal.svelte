<script module lang="ts">
  import { Zooms } from './zoom'

  const zooms = new Zooms()
</script>

<script lang="ts">
  import type { Size_Deserialize } from '../bindings'
  import { edge } from '../edge.svelte'
  import type { InputEvent } from './keys'
  import type { PaneState } from './pane.svelte'
  import { Pinch, type Point } from './pinch'
  import Row from './Row.svelte'
  import { paneColours, rgb } from './style'
  import { Swipe, type Grid } from './swipe'
  import { anchor, ended, followed, roomIn, slide, Tap } from './touch'
  import { BASE, clamp, fit, limits, LINE, type Place } from './zoom'

  const PROBE = 'M'.repeat(100)
  const ADVANCE = 0.6

  let {
    pane,
    place,
    onfit,
    ontap,
    onsend,
    onscroll,
  }: {
    pane: PaneState
    place: Place
    onfit: (size: Size_Deserialize) => void
    ontap: () => void
    onsend: (events: InputEvent[]) => void
    onscroll: (rowsUp: number) => void
  } = $props()

  let width = $state(0)
  let height = $state(0)
  let measured = $state(0)
  let start = BASE
  let byGesture = false
  const tap = new Tap()
  const pinch = new Pinch()
  const swipe = new Swipe()
  let drawn: HTMLElement | undefined

  const cell = $derived(measured > 0 ? measured : BASE * ADVANCE)
  const cols = $derived(pane.size.cols)
  const bounds = $derived(limits(cols, width, cell))
  const ink = $derived(paneColours(pane.colours))
  let size = $derived(zooms.recall(place, bounds))

  $effect(() => {
    if (measured > 0 && width > 0 && height > 0) {
      onfit(fit(width, height, measured))
    }
  })

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

  const pointOf = (event: PointerEvent): Point => [event.clientX, event.clientY]

  const gridIn = (box: HTMLElement): Grid => {
    const { left, top } = (drawn ?? box).getBoundingClientRect()
    const { modes, size: cells } = pane
    return { modes, size: cells, cell: (cell * size) / BASE, line: size * LINE, origin: [left, top], room: roomIn(box) }
  }

  const pressed = (event: PointerEvent) => {
    swipe.press(event.pointerId, pointOf(event))
    if (pinch.press(event.pointerId, pointOf(event))) start = size
  }

  const moved = (box: HTMLElement, event: PointerEvent) => {
    if (!followed(edge.state)) return
    const scale = pinch.move(event.pointerId, pointOf(event))
    if (scale !== null && !byGesture) zoom(scale)
    const scroll = swipe.move(event.pointerId, pointOf(event), () => gridIn(box))
    if (scroll !== null) slide(box, scroll, { send: onsend, scroll: onscroll })
  }

  const lifted = (event: PointerEvent) => {
    pinch.lift(event.pointerId)
    ended(tap, edge.state, swipe.lift(event.pointerId))
  }

  const cancelled = (event: PointerEvent) => {
    pinch.lift(event.pointerId)
    swipe.lift(event.pointerId)
    tap.cancelled()
  }

  const clicked = () => {
    if (tap.clicked()) ontap()
  }

  const pointers = (element: HTMLElement) =>
    listen(element, {
      pointerdown: (event) => event instanceof PointerEvent && pressed(event),
      pointermove: (event) => event instanceof PointerEvent && moved(element, event),
      pointerup: (event) => event instanceof PointerEvent && lifted(event),
      pointercancel: (event) => event instanceof PointerEvent && cancelled(event),
      click: clicked,
    })

  const anchored = (box: HTMLElement) => {
    if (height > 0) anchor(box)
  }
</script>

<div
  class="terminal"
  style:background={rgb(ink.bg)}
  bind:clientWidth={width}
  bind:clientHeight={height}
  {@attach gestures}
  {@attach pointers}
  {@attach anchored}
>
  <div
    bind:this={drawn}
    class="grid"
    style:font-size="{size}px"
    style:--cell="{cell / BASE}em"
    style:--line="{LINE}em"
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
  <span class="probe" aria-hidden="true"
    ><span style:font-size="{BASE}px" {@attach measure}>{PROBE}</span></span
  >
</div>

<style>
  .terminal {
    position: relative;
    width: 100%;
    height: 100%;
    overflow: auto;
    overscroll-behavior: contain;
    touch-action: pan-x;
  }

  .grid,
  .probe {
    font-family: var(--font-mono);
    white-space: pre;
  }

  .grid {
    position: relative;
    line-height: var(--line);
  }

  .probe {
    position: absolute;
    top: 0;
    left: 0;
    width: 0;
    height: 0;
    overflow: hidden;
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
