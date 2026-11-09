import type { commands, Pasted_Serialize as Pasted } from './bindings'
import { messageOf } from './toasts'

export type AttachImage = typeof commands.attach

export type Chip = { name: string; size: string }

const UNITS = ['B', 'KB', 'MB', 'GB']

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

const chipOf = (pasted: Pasted): Chip => ({ name: nameOf(pasted.path), size: formatBytes(pasted.bytes) })

export class Attachments {
  readonly pane: number
  attaching = $state(false)
  chips = $state<Chip[]>([])
  #attachImage: AttachImage

  constructor(pane: number, attachImage: AttachImage) {
    this.pane = pane
    this.#attachImage = attachImage
  }

  async attach(): Promise<string | null> {
    if (this.attaching) {
      return null
    }
    this.attaching = true
    try {
      const answer = await this.#attachImage(this.pane)
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

  clear(sent = this.chips.length) {
    this.chips = this.chips.slice(sent)
  }
}
