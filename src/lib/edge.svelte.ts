export type Edge = 'idle' | 'pending' | 'claimed'

export class SharedEdge {
  state = $state<Edge>('idle')
}

export const edge = new SharedEdge()
