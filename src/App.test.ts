import { render } from 'svelte/server'
import { describe, expect, it } from 'vitest'
import App from './App.svelte'
import noMachinesSource from './routes/NoMachines.svelte?raw'

describe('App', () => {
  const body = render(App).body

  it('opens on the screen with no machines, under the brand', () => {
    expect(body).toMatch(/<span class="wordmark svelte-[\w-]+" style="font-size: 22px;">cornercase<\/span>/)
    expect(body).toContain('--unit: 5px;')
    expect(body).toMatch(/<h2 class="title svelte-[\w-]+">No machines yet<\/h2>/)
  })

  it('says what a machine is and what it needs', () => {
    expect(body).toContain('A machine is a computer running cornercase whose agents you follow from here.')
    expect(body).toContain('It needs the cornercase bridge running on your Mac, and the URL it prints.')
  })

  it('offers adding a machine, not yet pressable', () => {
    expect(body).toMatch(/<button class="button svelte-[\w-]+ resting" type="button" disabled="" aria-disabled="true"><!---->Add machine<!----><\/button>/)
  })

  it('brings the card in with the shared motion', () => {
    expect(noMachinesSource).toContain('<div class="card" in:riseIn out:fadeOut>')
  })
})
