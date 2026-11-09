import { describe, expect, it } from 'vitest'
import { EMPTY, fromClipboard, REFUSED, type Reader } from './paste'

const holding = (text: string): Reader => ({ readText: async () => text })

const refusing: Reader = { readText: () => Promise.reject(new Error('NotAllowedError')) }

describe('fromClipboard', () => {
  it('pastes the text of the clipboard', async () => {
    await expect(fromClipboard(holding('ls -la\n'))).resolves.toEqual({ paste: 'ls -la\n' })
  })

  it.each([
    ['an empty clipboard', holding(''), EMPTY],
    ['a clipboard it may not read', refusing, REFUSED],
    ['no clipboard at all', undefined, REFUSED],
  ])('says why it pastes nothing from %s', async (_, clipboard, message) => {
    await expect(fromClipboard(clipboard)).rejects.toThrow(message)
  })
})
