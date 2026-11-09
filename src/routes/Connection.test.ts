import { render } from 'svelte/server'
import { describe, expect, it } from 'vitest'
import { screen } from '../lib/connection'
import Connection from './Connection.svelte'

const draw = (session: Parameters<typeof screen>[1]) =>
  render(Connection, {
    props: { screen: screen({ machine: 1, connection: { state: 'connected', route: 'ws://mac:7000' } }, session) },
  }).body.replaceAll('<!---->', '')

describe('Connection', () => {
  it('tells the state and the panes of the session', () => {
    const body = draw({ boot: 'boot', revision: 1, version: '0.1.0', groups: [], projects: [] })
    expect(body).toMatch(/<p class="state svelte-[\w-]+">Connected through ws:\/\/mac:7000<\/p>/)
    expect(body).toMatch(/<p class="panes svelte-[\w-]+">0 panes<\/p>/)
  })

  it('leaves the panes out without a session', () => {
    expect(draw(null)).not.toContain('class="panes')
  })
})
