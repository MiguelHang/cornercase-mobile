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

  it('takes a tap inside the gesture, so the keyboard opens, and never a cancelled touch', () => {
    expect(source).toContain('pinch.lift(event.pointerId) if (swipe.lift(event.pointerId)) ontap()')
    expect(source).toContain(
      'const cancelled = (event: PointerEvent) => { pinch.lift(event.pointerId) swipe.lift(event.pointerId) }',
    )
    expect(source).toContain('pointercancel: (event) => event instanceof PointerEvent && cancelled(event),')
  })

  it('points the wheel at the cell under the finger at the zoom it shows', () => {
    expect(source).toContain('const { left, top } = (drawn ?? box).getBoundingClientRect()')
    expect(source).toContain('cell: (cell * size) / BASE, line: size * LINE, origin: [left, top], room: roomIn(box)')
  })

  it('keeps its last row in view as its height changes', () => {
    expect(source).toContain('const anchored = (box: HTMLElement) => { if (height > 0) anchor(box) }')
    expect(source).toContain('{@attach anchored}')
  })
})
