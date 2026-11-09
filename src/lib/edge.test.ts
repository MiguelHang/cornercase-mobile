import { describe, expect, it } from 'vitest'
import { edge, SharedEdge } from './edge.svelte'

describe('the edge', () => {
  it('starts with no finger on it', () => {
    expect(new SharedEdge().state).toBe('idle')
  })

  it('is one for the whole app, idle as it starts', () => {
    expect([edge instanceof SharedEdge, edge.state]).toEqual([true, 'idle'])
  })
})
