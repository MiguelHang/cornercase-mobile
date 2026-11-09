import type { Size_Deserialize } from '../bindings'

export const BASE = 13

export const LINE = 1.3

export type Limits = { min: number; max: number }

export type Place = { machine: number; boot: string; tab: number }

export const limits = (paneCols: number, screenWidth: number, cellWidth: number, base = BASE): Limits => {
  const width = paneCols * cellWidth
  const min = width > screenWidth ? (base * screenWidth) / width : base
  return { min, max: 2 * base }
}

export const fit = (width: number, height: number, cellWidth: number, base = BASE): Size_Deserialize => ({
  cols: Math.max(1, Math.floor(width / cellWidth)),
  rows: Math.max(1, Math.floor(height / (base * LINE))),
})

export const clamp = (size: number, { min, max }: Limits) => Math.min(max, Math.max(min, size))

const keyOf = ({ machine, boot, tab }: Place) => `${machine}:${boot}:${tab}`

export class Zooms {
  #sizes = new Map<string, number>()

  recall(place: Place, bounds: Limits) {
    return clamp(this.#sizes.get(keyOf(place)) ?? bounds.min, bounds)
  }

  remember(place: Place, size: number) {
    this.#sizes.set(keyOf(place), size)
  }
}
