import { describe, expect, it, vi } from 'vitest'
import {
  Attachments,
  formatBytes,
  MAX_ATTACHED,
  nameOf,
  type AttachImage,
  type Discard,
  type Images,
  type PasteAttached,
} from './attach.svelte'
import type { CommandError } from './bindings'

const PANE = 543

const PATH = '/Users/me/.local/state/cornercase/pasted/2026-11-09T19-40-12Z/IMG_0042.jpg'

const THUMBNAIL = 'data:image/jpeg;base64,/9j/2w=='

const image = (id: number, name = 'IMG_0042.jpg', bytes = 48213) => ({ id, name, bytes, thumbnail: THUMBNAIL })

type Answer = Awaited<ReturnType<AttachImage>>

const answered =
  (answer: Answer): AttachImage =>
  () =>
    Promise.resolve(answer)

const stored = answered({ status: 'ok', data: image(1) })

const numbered = (): AttachImage => {
  let last = 0
  return () => Promise.resolve({ status: 'ok', data: image(++last) })
}

const pasted: PasteAttached = (pane) => Promise.resolve({ status: 'ok', data: { pane, path: PATH, bytes: 48213 } })

const discarded: Discard = () => Promise.resolve()

const images = (changes: Partial<Images> = {}): Images => ({ attach: stored, paste: pasted, discard: discarded, ...changes })

const refused = (message: string, code: CommandError['code'] = 'failed'): Answer => ({
  status: 'error',
  error: { code, message },
})

const holding = async (count: number, changes: Partial<Images> = {}) => {
  const attachments = new Attachments(PANE, images({ attach: numbered(), ...changes }))
  for (let index = 0; index < count; index += 1) {
    await attachments.attach()
  }
  return attachments
}

describe('formatBytes', () => {
  it.each([
    ['nothing', 0, '0 B'],
    ['bytes', 512, '512 B'],
    ['a kibibyte', 1024, '1.0 KiB'],
    ['a few kibibytes', 1536, '1.5 KiB'],
    ['tens of kibibytes', 48213, '47 KiB'],
    ['mebibytes', 2.5 * 1024 * 1024, '2.5 MiB'],
    ['the most cornercase pastes', 32 * 1024 * 1024, '32 MiB'],
    ['more than any unit', 3 * 1024 ** 4, '3072 GiB'],
  ])('reads %s', (_, bytes, shown) => {
    expect(formatBytes(bytes)).toBe(shown)
  })
})

describe('nameOf', () => {
  it.each([
    ['the last part of a path', PATH, 'IMG_0042.jpg'],
    ['a bare name as it is', 'pasted.png', 'pasted.png'],
  ])('takes %s', (_, path, name) => {
    expect(nameOf(path)).toBe(name)
  })
})

describe('Attachments, attaching', () => {
  it('starts idle and without chips', () => {
    const attachments = new Attachments(PANE, images())
    expect([attachments.attaching, attachments.chips, attachments.full]).toEqual([false, [], false])
  })

  it('asks for its pane and shows the image kept on the phone as a chip, not pasted yet', async () => {
    const attach = vi.fn(stored)
    const attachments = new Attachments(PANE, images({ attach }))
    expect(await attachments.attach()).toBeNull()
    expect(attach.mock.calls).toEqual([[PANE]])
    expect(attachments.chips).toEqual([{ id: 1, name: 'IMG_0042.jpg', size: '47 KiB', thumbnail: THUMBNAIL, pasted: false }])
  })

  it('keeps every image attached, in order', async () => {
    const attach = vi
      .fn<AttachImage>()
      .mockResolvedValueOnce({ status: 'ok', data: image(1) })
      .mockResolvedValue({ status: 'ok', data: image(2, 'pasted.png', 900) })
    const attachments = new Attachments(PANE, images({ attach }))
    await attachments.attach()
    await attachments.attach()
    expect(attachments.chips.map((chip) => [chip.id, chip.name])).toEqual([
      [1, 'IMG_0042.jpg'],
      [2, 'pasted.png'],
    ])
  })

  it('shows no chip and gives no reason when the picker is cancelled', async () => {
    const attachments = new Attachments(PANE, images({ attach: answered({ status: 'ok', data: null }) }))
    expect(await attachments.attach()).toBeNull()
    expect([attachments.attaching, attachments.chips]).toEqual([false, []])
  })

  it('is attaching, and ignores a second tap, until the answer', async () => {
    let open: (answer: Answer) => void = () => {}
    const attach = vi.fn<AttachImage>(() => new Promise((resolve) => (open = resolve)))
    const attachments = new Attachments(PANE, images({ attach }))
    const attached = attachments.attach()
    expect(attachments.attaching).toBe(true)
    expect(await attachments.attach()).toBeNull()
    open({ status: 'ok', data: image(1) })
    await attached
    expect(attach).toHaveBeenCalledTimes(1)
    expect([attachments.attaching, attachments.chips.length]).toEqual([false, 1])
  })

  it.each<[string, Answer]>([
    ['a photo over the most cornercase pastes', refused('the image is over 32 MiB, the most cornercase pastes', 'image_too_large')],
    ['an empty photo', refused('the image is empty: there is nothing to paste', 'image_empty')],
    ['a photo that cannot be read', refused('cannot pick a photo: the photo cannot be read: gone', 'picker')],
    ['a full pane', refused('at most 4 images can wait to be sent', 'attached_full')],
    ['images taking too much memory', refused('the images waiting to be sent take too much memory', 'attached_full')],
    ['no machine', refused('no machine is connected', 'not_connected')],
  ])('gives the reason of %s, and shows no chip', async (_, answer) => {
    const attachments = new Attachments(PANE, images({ attach: answered(answer) }))
    expect(await attachments.attach()).toBe(answer.status === 'error' ? answer.error.message : null)
    expect([attachments.attaching, attachments.chips]).toEqual([false, []])
  })

  it('gives the message of a rejected call, and can attach again', async () => {
    const attach = vi.fn<AttachImage>().mockRejectedValueOnce(new Error('no window')).mockImplementation(stored)
    const attachments = new Attachments(PANE, images({ attach }))
    expect(await attachments.attach()).toBe('no window')
    expect(attachments.attaching).toBe(false)
    await attachments.attach()
    expect(attachments.chips.length).toBe(1)
  })
})

describe('Attachments, full', () => {
  it(`is full with ${MAX_ATTACHED} images, and attaches no more`, async () => {
    const attach = vi.fn(numbered())
    const attachments = await holding(MAX_ATTACHED, { attach })
    expect(attachments.full).toBe(true)
    expect(await attachments.attach()).toBeNull()
    expect([attach.mock.calls.length, attachments.chips.length]).toEqual([MAX_ATTACHED, MAX_ATTACHED])
  })

  it('takes one more once an image is removed', async () => {
    const attachments = await holding(MAX_ATTACHED)
    await attachments.remove(2)
    expect(attachments.full).toBe(false)
  })
})

describe('Attachments, pasting', () => {
  it('pastes the image kept for its pane and marks it pasted', async () => {
    const paste = vi.fn(pasted)
    const attachments = await holding(2, { paste })
    expect(await attachments.paste(attachments.chips[1])).toBeNull()
    expect(paste.mock.calls).toEqual([[PANE, 2]])
    expect(attachments.chips.map((chip) => chip.pasted)).toEqual([false, true])
  })

  it.each<[string, PasteAttached, string]>([
    ['refused', () => Promise.resolve({ status: 'error', error: { code: 'failed', message: 'pane 543 is busy' } }), 'pane 543 is busy'],
    ['rejected', () => Promise.reject(new Error('IPC is gone')), 'IPC is gone'],
  ])('gives the reason of a paste %s, and leaves the image unpasted', async (_, paste, reason) => {
    const attachments = await holding(1, { paste })
    expect(await attachments.paste(attachments.chips[0])).toBe(reason)
    expect(attachments.chips[0].pasted).toBe(false)
  })
})

describe('Attachments, removing', () => {
  it('drops the chip and forgets its image on the phone, keeping the others', async () => {
    const discard = vi.fn(discarded)
    const attachments = await holding(3, { discard })
    expect(await attachments.remove(2)).toBeNull()
    expect(discard.mock.calls).toEqual([[[2]]])
    expect(attachments.chips.map((chip) => chip.id)).toEqual([1, 3])
  })

  it('gives the reason the image could not be forgotten, with its chip gone', async () => {
    const attachments = await holding(1, { discard: () => Promise.reject(new Error('IPC is gone')) })
    expect(await attachments.remove(1)).toBe('IPC is gone')
    expect(attachments.chips).toEqual([])
  })

  it('asks nothing for an image already pasted, which the phone no longer keeps', async () => {
    const discard = vi.fn(discarded)
    const attachments = await holding(2, { discard })
    await attachments.paste(attachments.chips[0])
    expect(await attachments.remove(1)).toBeNull()
    expect(discard).not.toHaveBeenCalled()
    expect(attachments.chips.map((chip) => chip.id)).toEqual([2])
  })
})

describe('Attachments, releasing', () => {
  it('forgets every image kept on the phone and drops their chips', async () => {
    const discard = vi.fn(discarded)
    const attachments = await holding(2, { discard })
    expect(await attachments.release()).toBeNull()
    expect(discard.mock.calls).toEqual([[[1, 2]]])
    expect(attachments.chips).toEqual([])
  })

  it('forgets only the images it is given', async () => {
    const discard = vi.fn(discarded)
    const attachments = await holding(2, { discard })
    await attachments.release([attachments.chips[0]])
    expect(discard.mock.calls).toEqual([[[1]]])
    expect(attachments.chips.length).toBe(2)
  })

  it('forgets only the images not pasted yet', async () => {
    const discard = vi.fn(discarded)
    const attachments = await holding(3, { discard })
    await attachments.paste(attachments.chips[1])
    expect(await attachments.release()).toBeNull()
    expect(discard.mock.calls).toEqual([[[1, 3]]])
  })

  it('asks nothing without images', async () => {
    const discard = vi.fn(discarded)
    expect(await new Attachments(PANE, images({ discard })).release()).toBeNull()
    expect(discard).not.toHaveBeenCalled()
  })
})

describe('Attachments, taking and giving back', () => {
  it('takes every chip out, and gives them back before those attached since', async () => {
    const attachments = await holding(1)
    const taken = attachments.take()
    expect(attachments.chips).toEqual([])
    await attachments.attach()
    attachments.giveBack(taken)
    expect(attachments.chips.map((chip) => chip.id)).toEqual([1, 2])
  })
})
