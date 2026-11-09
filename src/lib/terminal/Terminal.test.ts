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
})
