import { describe, expect, it } from 'vitest'
import type { ConnectionView_Serialize, MachineView_Serialize } from './bindings'
import { status } from './connection'
import { CHECKING, rescannable, rowStatus, type Probe } from './machineRows'

const MACHINE: MachineView_Serialize = {
  id: 2,
  name: 'Studio',
  routes: [{ kind: 'cli', url: 'ws://studio.lan:9000/', has_token: true }],
  last_route: null,
  demo: false,
}

const shown = (probe: Probe) => {
  const { text, tone, pulsing } = rowStatus(MACHINE, null, probe)
  return { text, tone, pulsing }
}

describe('rowStatus of the active machine', () => {
  it.each<ConnectionView_Serialize>([
    { state: 'connected', route: 'ws://studio.lan:9000/' },
    { state: 'connecting' },
    { state: 'offline' },
    { state: 'not_paired' },
  ])('says what the bar says while $state, whatever the probe', (connection) => {
    expect(rowStatus(MACHINE, connection, { state: 'no_token' })).toEqual({
      ...status(connection, 'Studio'),
      failures: [],
    })
  })

  it('counts down while retrying like the bar', () => {
    const retrying: ConnectionView_Serialize = {
      state: 'retrying',
      in_ms: 5000,
      error: { code: 'timeout', message: 'timed out' },
    }
    expect(rowStatus(MACHINE, retrying, CHECKING, 2500)).toMatchObject({
      text: 'Retrying in 3 s',
      detail: 'timed out',
      tone: 'failed',
    })
  })
})

describe('rowStatus of the demo', () => {
  const DEMO: MachineView_Serialize = { id: 4294967295, name: 'Demo Mac', routes: [], last_route: null, demo: true }

  it('shows it connected by its name with no via', () => {
    expect(rowStatus(DEMO, { state: 'connected', route: '' }, CHECKING)).toEqual({
      text: 'Demo Mac',
      tone: 'done',
      pulsing: false,
      failures: [],
    })
  })
})

describe('rowStatus of another machine', () => {
  it.each<[string, Probe, ReturnType<typeof shown>]>([
    ['checking', CHECKING, { text: 'Checking…', tone: 'working', pulsing: true }],
    [
      'answering',
      { state: 'answers', route: 'ws://studio.lan:9000/' },
      { text: 'Answers via studio.lan:9000', tone: 'done', pulsing: false },
    ],
    ['forgetting this phone', { state: 'not_paired' }, { text: 'Not paired', tone: 'failed', pulsing: false }],
    [
      'without a secret',
      { state: 'no_token' },
      { text: 'No secret', tone: 'failed', pulsing: false },
    ],
    [
      'not answering',
      { state: 'no_answer', failures: [] },
      { text: 'No answer', tone: 'failed', pulsing: false },
    ],
  ])('tells it is %s', (_, probe, expected) => {
    expect(shown(probe)).toEqual(expected)
  })

  it('keeps the error of each route that did not answer, by its address', () => {
    const probe: Probe = {
      state: 'no_answer',
      failures: [
        { route: 'ws://studio.lan:9000/', error: { code: 'timeout', message: 'timed out' } },
        { route: 'ws://100.64.0.2:9000/', error: { code: 'connect', message: 'refused' } },
      ],
    }
    expect(rowStatus(MACHINE, null, probe).failures).toEqual([
      { address: 'studio.lan:9000', message: 'timed out' },
      { address: '100.64.0.2:9000', message: 'refused' },
    ])
  })

  it('tells why it could not be checked at all', () => {
    expect(rowStatus(MACHINE, null, { state: 'failed', message: 'unknown machine' })).toEqual({
      text: 'No answer',
      detail: 'unknown machine',
      tone: 'failed',
      pulsing: false,
      failures: [],
    })
  })

  it('answers with no address for a machine without one', () => {
    expect(shown({ state: 'answers', route: '' })).toEqual({ text: 'Answers', tone: 'done', pulsing: false })
  })

  it('carries no route errors while it answers', () => {
    expect(rowStatus(MACHINE, null, { state: 'answers', route: 'ws://studio.lan:9000/' }).failures).toEqual([])
  })
})

describe('rescannable', () => {
  it.each<[string, Probe, boolean]>([
    ['no secret', { state: 'no_token' }, true],
    ['a Mac that forgot this phone', { state: 'not_paired' }, true],
    ['an answer', { state: 'answers', route: 'ws://studio.lan:9000/' }, false],
    ['no answer', { state: 'no_answer', failures: [] }, false],
    ['a probe still running', CHECKING, false],
  ])('sends another machine with %s to scan again or not', (_, probe, expected) => {
    expect(rescannable(null, probe)).toBe(expected)
  })

  it.each<[ConnectionView_Serialize, boolean]>([
    [{ state: 'not_paired' }, true],
    [{ state: 'connected', route: 'ws://studio.lan:9000/' }, false],
    [{ state: 'offline' }, false],
  ])('follows the connection of the active one, not its last probe (%o)', (connection, expected) => {
    expect(rescannable(connection, { state: 'no_token' })).toBe(expected)
  })
})
