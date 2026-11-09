import { describe, expect, it, vi } from 'vitest'
import { Attachments, formatBytes, nameOf, type AttachImage } from './attach.svelte'
import type { CommandError, Pasted_Serialize as Pasted } from './bindings'

const PANE = 543

const PATH = '/Users/miguelhang/.local/state/cornercase/pasted/2026-11-09T19-40-12Z/IMG_0042.jpg'

const PASTED: Pasted = { pane: PANE, path: PATH, bytes: 48213 }

type Answer = Awaited<ReturnType<AttachImage>>

const answered =
  (answer: Answer): AttachImage =>
  () =>
    Promise.resolve(answer)

const pasted = answered({ status: 'ok', data: PASTED })

const refused = (message: string, code: CommandError['code'] = 'failed'): Answer => ({
  status: 'error',
  error: { code, message },
})

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
    const attachments = new Attachments(PANE, pasted)
    expect([attachments.attaching, attachments.chips]).toEqual([false, []])
  })

  it('asks for its pane and shows what was pasted as a chip', async () => {
    const attachImage = vi.fn(pasted)
    const attachments = new Attachments(PANE, attachImage)
    expect(await attachments.attach()).toBeNull()
    expect(attachImage.mock.calls).toEqual([[PANE]])
    expect(attachments.chips).toEqual([{ name: 'IMG_0042.jpg', size: '47 KiB' }])
  })

  it('keeps every image attached, in order', async () => {
    const attachImage = vi
      .fn<AttachImage>()
      .mockResolvedValueOnce({ status: 'ok', data: PASTED })
      .mockResolvedValue({ status: 'ok', data: { pane: PANE, path: '/tmp/pasted.png', bytes: 900 } })
    const attachments = new Attachments(PANE, attachImage)
    await attachments.attach()
    await attachments.attach()
    expect(attachments.chips.map((chip) => chip.name)).toEqual(['IMG_0042.jpg', 'pasted.png'])
  })

  it('shows no chip and gives no reason when the picker is cancelled', async () => {
    const attachments = new Attachments(PANE, answered({ status: 'ok', data: null }))
    expect(await attachments.attach()).toBeNull()
    expect([attachments.attaching, attachments.chips]).toEqual([false, []])
  })

  it('is attaching, and ignores a second tap, until the answer', async () => {
    let open: (answer: Answer) => void = () => {}
    const attachImage = vi.fn<AttachImage>(() => new Promise((resolve) => (open = resolve)))
    const attachments = new Attachments(PANE, attachImage)
    const attached = attachments.attach()
    expect(attachments.attaching).toBe(true)
    expect(await attachments.attach()).toBeNull()
    open({ status: 'ok', data: PASTED })
    await attached
    expect(attachImage).toHaveBeenCalledTimes(1)
    expect([attachments.attaching, attachments.chips.length]).toEqual([false, 1])
  })

  it.each<[string, Answer]>([
    ['a photo over the most cornercase pastes', refused('the image is over 32 MiB, the most cornercase pastes', 'image_too_large')],
    ['an empty photo', refused('the image is empty: there is nothing to paste', 'image_empty')],
    ['a photo that cannot be read', refused('cannot pick a photo: the photo cannot be read: gone', 'picker')],
    ['an older cornercase', refused("error: unrecognized subcommand 'paste-file'")],
    ['a busy pane', refused('a message sent with `cornercase send` is between its paste and its Enter')],
    ['a pane that is gone', refused('there is no pane 543; `cornercase status` lists them')],
    ['a skipped job', refused('nothing was sent', 'not_sent')],
    ['a lost answer', refused('it may have been sent, but no answer came back', 'unanswered')],
    ['no machine', refused('no machine is connected', 'not_connected')],
  ])('gives the reason of %s, and shows no chip', async (_, answer) => {
    const attachments = new Attachments(PANE, answered(answer))
    expect(await attachments.attach()).toBe(answer.status === 'error' ? answer.error.message : null)
    expect([attachments.attaching, attachments.chips]).toEqual([false, []])
  })

  it('gives the message of a rejected call, and can attach again', async () => {
    const attachImage = vi.fn<AttachImage>().mockRejectedValueOnce(new Error('no window')).mockImplementation(pasted)
    const attachments = new Attachments(PANE, attachImage)
    expect(await attachments.attach()).toBe('no window')
    expect(attachments.attaching).toBe(false)
    await attachments.attach()
    expect(attachments.chips.length).toBe(1)
  })
})

describe('Attachments, clearing', () => {
  const twice = async () => {
    const attachments = new Attachments(PANE, pasted)
    await attachments.attach()
    await attachments.attach()
    return attachments
  }

  it('drops every chip', async () => {
    const attachments = await twice()
    attachments.clear()
    expect(attachments.chips).toEqual([])
  })

  it('drops only the chips sent, keeping those attached since', async () => {
    const attachments = await twice()
    attachments.clear(1)
    expect(attachments.chips.length).toBe(1)
  })
})
