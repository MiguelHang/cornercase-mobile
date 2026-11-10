import type { Edge } from '../edge.svelte'
import type { InputEvent } from './keys'
import type { Room, Scroll } from './swipe'
import type { Limits } from './zoom'

export type Box = Pick<HTMLElement, 'scrollTop' | 'scrollHeight' | 'clientHeight'>

export type Moves = { send: (events: InputEvent[]) => void; scroll: (rowsUp: number) => void }

export const roomIn = ({ scrollTop, scrollHeight, clientHeight }: Box): Room => ({
  up: scrollTop,
  down: Math.max(0, scrollHeight - clientHeight - scrollTop),
})

const NONE: Room = { up: 0, down: 0 }

const NEAR_BASE = 0.5

export const pinned = (fitted: boolean, size: number, { min }: Limits) => fitted && size - min < NEAR_BASE

export const roomAt = (box: Box, held: boolean): Room => (held ? NONE : roomIn(box))

export const slide = (box: Box, { pan, send, rowsUp }: Scroll, moves: Moves) => {
  box.scrollTop += pan
  if (send.length > 0) moves.send(send)
  if (rowsUp !== 0) moves.scroll(rowsUp)
}

export const anchor = (box: Box) => {
  box.scrollTop = Math.max(0, box.scrollHeight - box.clientHeight)
}

export class Tap {
  #marked = false

  lifted(tapped: boolean) {
    this.#marked = tapped
  }

  cancelled() {
    this.#marked = false
  }

  clicked() {
    const tapped = this.#marked
    this.#marked = false
    return tapped
  }
}

export const followed = (edge: Edge) => edge === 'idle'

export const ended = (tap: Tap, edge: Edge, still: boolean) => {
  if (edge === 'claimed') tap.cancelled()
  else tap.lifted(still)
}
