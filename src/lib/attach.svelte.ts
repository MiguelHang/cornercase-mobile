import type { AttachedImage_Serialize as AttachedImage, commands } from './bindings'
import { deliver, messageOf } from './toasts'

export type AttachImage = typeof commands.attach

export type PasteAttached = typeof commands.pasteAttached

export type Discard = typeof commands.discard

export type Images = { attach: AttachImage; paste: PasteAttached; discard: Discard }

export type Chip = { id: number; name: string; size: string; thumbnail: string | null; pasted: boolean }

export const MAX_ATTACHED = 4

const UNITS = ['B', 'KiB', 'MiB', 'GiB']

const STEP = 1024

export const formatBytes = (bytes: number) => {
  let value = bytes
  let unit = 0
  while (value >= STEP && unit < UNITS.length - 1) {
    value /= STEP
    unit += 1
  }
  const shown = unit === 0 || value >= 10 ? String(Math.round(value)) : value.toFixed(1)
  return `${shown} ${UNITS[unit]}`
}

export const nameOf = (path: string) => path.slice(path.lastIndexOf('/') + 1)

const chipOf = (image: AttachedImage): Chip => ({
  id: image.id,
  name: image.name,
  size: formatBytes(image.bytes),
  thumbnail: image.thumbnail,
  pasted: false,
})

export class Attachments {
  readonly pane: number
  attaching = $state(false)
  chips = $state<Chip[]>([])
  #images: Images

  constructor(pane: number, images: Images) {
    this.pane = pane
    this.#images = images
  }

  get full() {
    return this.chips.length >= MAX_ATTACHED
  }

  async attach(): Promise<string | null> {
    if (this.attaching || this.full) {
      return null
    }
    this.attaching = true
    try {
      const answer = await this.#images.attach(this.pane)
      if (answer.status === 'error') {
        return answer.error.message
      }
      if (answer.data !== null) {
        this.chips = [...this.chips, chipOf(answer.data)]
      }
      return null
    } catch (error) {
      return messageOf(error)
    } finally {
      this.attaching = false
    }
  }

  async paste(chip: Chip): Promise<string | null> {
    const failed = await deliver(() => this.#images.paste(this.pane, chip.id))
    if (failed === null) {
      chip.pasted = true
    }
    return failed
  }

  take(): Chip[] {
    const taken = this.chips
    this.chips = []
    return taken
  }

  giveBack(chips: Chip[]) {
    this.chips = [...chips, ...this.chips]
  }

  async remove(id: number): Promise<string | null> {
    const removed = this.chips.filter((chip) => chip.id === id)
    this.chips = this.chips.filter((chip) => chip.id !== id)
    return this.#discard(removed)
  }

  async release(chips = this.take()): Promise<string | null> {
    return this.#discard(chips)
  }

  async #discard(chips: Chip[]): Promise<string | null> {
    const ids = chips.filter((chip) => !chip.pasted).map((chip) => chip.id)
    if (ids.length === 0) {
      return null
    }
    try {
      await this.#images.discard(ids)
      return null
    } catch (error) {
      return messageOf(error)
    }
  }
}
