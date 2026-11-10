import { describe, expect, it } from 'vitest'
import terminalSource from './Terminal.svelte?raw'

const source = terminalSource.replace(/\s+/g, ' ')

describe('the terminal view', () => {
  it('measures its cell inside a box clipped to nothing, so the probe adds no room to scroll into', () => {
    expect(source).toContain(
      '<span class="probe" aria-hidden="true" ><span style:font-size="{BASE}px" {@attach measure}>{PROBE}</span></span >',
    )
    expect(source).toContain('.probe { position: absolute; top: 0; left: 0; width: 0; height: 0; overflow: hidden;')
  })

  it('scrolls only sideways by itself and leaves the vertical slide to the swipe', () => {
    expect(source).toContain('touch-action: pan-x;')
    expect(source).toContain('if (scroll !== null) slide(box, scroll, { send: onsend, scroll: onscroll })')
  })

  it('measures the grid only once a finger swipes, not on every move of a pinch', () => {
    expect(source).toContain('swipe.move(event.pointerId, pointOf(event), () => gridIn(box))')
  })

  it('takes a tap on the click that follows it, so the focus outlives the tap and the keyboard opens', () => {
    expect(source).toContain('pinch.lift(event.pointerId) ended(tap, edge.state, swipe.lift(event.pointerId))')
    expect(source).toContain('const clicked = () => { if (tap.clicked()) ontap() }')
    expect(source).toContain('click: clicked,')
  })

  it('never takes a cancelled touch as a tap', () => {
    expect(source).toContain(
      'const cancelled = (event: PointerEvent) => { pinch.lift(event.pointerId) swipe.lift(event.pointerId) tap.cancelled() }',
    )
    expect(source).toContain('pointercancel: (event) => event instanceof PointerEvent && cancelled(event),')
  })

  it('leaves a finger at the edge of the screen to the swipe back, before the pinch and the swipe see it', () => {
    expect(source).toContain(
      'const moved = (box: HTMLElement, event: PointerEvent) => { if (!followed(edge.state)) return const scale = pinch.move(',
    )
    expect(source).toContain("import { edge } from '../edge.svelte'")
  })

  it('points the wheel at the cell under the finger at the zoom it shows', () => {
    expect(source).toContain('const { left, top } = (drawn ?? box).getBoundingClientRect()')
    expect(source).toContain('cell: (cell * size) / BASE, line: size * LINE, origin: [left, top], room: roomIn(box)')
  })

  it('keeps its last row in view as its height changes', () => {
    expect(source).toContain('const anchored = (box: HTMLElement) => { if (height > 0) anchor(box) }')
    expect(source).toContain('{@attach anchored}')
  })

  it('recalls and remembers the zoom of the tab in the mode it shows', () => {
    expect(source).toContain('let size = $derived(zooms.recall(place, bounds))')
    expect(source).toContain('size = clamp(start * scale, bounds) zooms.remember(place, size)')
  })
})
