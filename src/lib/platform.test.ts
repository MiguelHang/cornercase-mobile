import { afterEach, describe, expect, it, vi } from 'vitest'
import { platformFrom, platformOf } from './platform'

const IPHONE =
  'Mozilla/5.0 (iPhone; CPU iPhone OS 27_2 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Mobile/15E148'
const PIXEL =
  'Mozilla/5.0 (Linux; Android 16; Pixel 9) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/140.0.0.0 Mobile Safari/537.36'

afterEach(() => {
  vi.unstubAllGlobals()
  vi.resetModules()
})

describe('platformOf', () => {
  it.each([
    ['an iPhone', IPHONE, 'ios'],
    ['an Android phone', PIXEL, 'android'],
    ['an Android name in lower case', 'okhttp android-webview', 'android'],
    ['an unknown agent', 'Node.js/22', 'ios'],
    ['an empty agent', '', 'ios'],
  ])('reads %s', (_, userAgent, expected) => {
    expect(platformOf(userAgent)).toBe(expected)
  })
})

describe('platformFrom', () => {
  it('reads the agent of the navigator it gets', () => {
    expect(platformFrom({ userAgent: PIXEL })).toBe('android')
  })

  it('takes iOS without a navigator', () => {
    expect(platformFrom()).toBe('ios')
  })
})

describe('platform', () => {
  it.each([
    ['an iPhone', IPHONE, 'ios'],
    ['an Android phone', PIXEL, 'android'],
  ])('is resolved from the navigator of %s when the module loads', async (_, userAgent, expected) => {
    vi.stubGlobal('navigator', { userAgent })
    const { platform } = await import('./platform')
    expect(platform).toBe(expected)
  })

  it('is iOS when the module loads without a navigator', async () => {
    vi.stubGlobal('navigator', undefined)
    const { platform } = await import('./platform')
    expect(platform).toBe('ios')
  })
})
