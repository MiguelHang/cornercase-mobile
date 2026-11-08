import { render } from 'svelte/server'
import { describe, expect, it } from 'vitest'
import App from './App.svelte'

describe('App', () => {
  it('renders an empty main', () => {
    expect(render(App).body).toContain('<main></main>')
  })
})
