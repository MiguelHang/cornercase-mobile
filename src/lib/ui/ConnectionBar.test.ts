import { render } from 'svelte/server'
import { describe, expect, it } from 'vitest'
import type { ConnectionView_Serialize } from '../bindings'
import type { Tone } from '../connection'
import ConnectionBar from './ConnectionBar.svelte'
import barSource from './ConnectionBar.svelte?raw'

const RETRYING: ConnectionView_Serialize = {
  state: 'retrying',
  in_ms: 2000,
  error: { code: 'timeout', message: 'timed out' },
}
const MISSING: ConnectionView_Serialize = {
  state: 'missing',
  missing: [{ name: 'watch', kind: 'stream', reason: null }],
}

const bar = (connection: ConnectionView_Serialize) =>
  render(ConnectionBar, { props: { connection, machineName: 'MacBook' } }).body

const flat = (text: string) => text.replace(/\s+/g, ' ')

describe('ConnectionBar', () => {
  it.each<[string, ConnectionView_Serialize, string, Tone]>([
    ['connecting', { state: 'connecting' }, 'Connecting…', 'working'],
    ['connected', { state: 'connected', route: 'ws://192.168.1.20:7000/?token=abc' }, 'MacBook · via 192.168.1.20:7000', 'done'],
    ['retrying', RETRYING, 'Retrying in 2 s', 'failed'],
    ['missing', MISSING, 'cornercase on the Mac lacks watch: update it', 'failed'],
    ['unauthorized', { state: 'unauthorized' }, "The bridge rejected this phone's token", 'failed'],
  ])('shows %s with its text and the dot in its tone', (_, connection, text, tone) => {
    const body = bar(connection)
    expect(body).toMatch(new RegExp(`<span class="dot ${tone} svelte-[\\w-]+( pulsing)?"></span>`))
    expect(body.match(/<span class="line text svelte-[\w-]+">([^<]*)<\/span>/)?.[1]).toBe(text)
  })

  it('pulses the dot only while connecting', () => {
    expect(bar({ state: 'connecting' })).toMatch(/<span class="dot working svelte-[\w-]+ pulsing"><\/span>/)
    for (const connection of [RETRYING, MISSING, { state: 'connected', route: 'ws://h:1' } as const]) {
      expect(bar(connection)).not.toContain('pulsing')
    }
  })

  it('gives the reason of a retry its own line', () => {
    expect(bar(RETRYING)).toMatch(/<span class="line detail svelte-[\w-]+">timed out<\/span>/)
    expect(bar({ state: 'connecting' })).not.toContain('detail')
  })

  it('announces itself as a status', () => {
    expect(bar({ state: 'connecting' })).toMatch(/^<!--\[--><div class="connection svelte-[\w-]+" role="status">/)
  })

  it('counts down every second only while retrying, and stops when it leaves', () => {
    const script = flat(barSource)
    expect(script).toContain('const TICK_MS = 1000')
    expect(script).toContain(
      "$effect(() => { elapsed = 0 if (connection.state !== 'retrying') { return } const since = Date.now() const timer = setInterval(() => (elapsed = Date.now() - since), TICK_MS) return () => clearInterval(timer) })",
    )
    expect(script).toContain('status(connection, machineName, elapsed)')
  })

  it('tells its height below the safe area, for the toasts', () => {
    expect(barSource).toContain('<div class="row" bind:clientHeight={height}>')
  })

  it.each([
    'position: fixed; top: 0; left: 0; right: 0;',
    'padding-top: env(safe-area-inset-top);',
    'min-height: 44px;',
    'width: 8px; height: 8px; border-radius: 999px;',
    '.working { background: var(--working); }',
    '.done { background: var(--done); }',
    '.failed { background: var(--failed); }',
    '.pulsing { animation: pulse var(--pulse) ease-in-out infinite; }',
    '@keyframes pulse { 50% { opacity: 0.3; } }',
    '@media (prefers-reduced-motion: reduce) { .pulsing { animation: none; } }',
    'overflow: hidden; white-space: nowrap; text-overflow: ellipsis;',
  ])('holds %s', (text) => {
    expect(flat(barSource)).toContain(text)
  })
})
