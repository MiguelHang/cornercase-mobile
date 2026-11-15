import { describe, expect, it, vi } from 'vitest'
import { follow, opensOut } from './links'

describe('opensOut', () => {
  it.each(['https://example.com', 'http://example.com/a?b=1#c', 'HTTPS://EXAMPLE.COM/path'])('opens %s in the browser', (url) => {
    expect(opensOut(url)).toBe(true)
  })

  it.each([
    'javascript:alert(1)',
    'mailto:a@example.com',
    'file:///etc/hosts',
    'tauri://localhost',
    '/relative',
    'https://',
    'https:///path',
    'https://exa mple.com',
    ' https://example.com',
    '',
  ])('leaves %j alone', (url) => {
    expect(opensOut(url)).toBe(false)
  })
})

describe('follow', () => {
  it('asks the system to open a web link', async () => {
    const openUrl = vi.fn().mockResolvedValue(undefined)
    await follow({ openUrl }, 'https://example.com/a?b=1#c')
    expect(openUrl).toHaveBeenCalledWith('https://example.com/a?b=1#c')
  })

  it('writes the scheme and the host in lower case, as the scope of the app compares them', async () => {
    const openUrl = vi.fn().mockResolvedValue(undefined)
    await follow({ openUrl }, 'HTTPS://EXAMPLE.COM/path')
    expect(openUrl).toHaveBeenCalledWith('https://example.com/path')
  })

  it.each(['javascript:alert(1)', 'https://exa[mple.com'])('never asks for %s', async (url) => {
    const openUrl = vi.fn()
    await follow({ openUrl }, url)
    expect(openUrl).not.toHaveBeenCalled()
  })

  it('swallows a refusal, where the plugin is not there', async () => {
    const openUrl = vi.fn().mockRejectedValue(new Error('plugin not found'))
    await expect(follow({ openUrl }, 'https://example.com')).resolves.toBeUndefined()
  })
})
