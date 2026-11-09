import { describe, expect, it } from 'vitest'
import type { ConnectionView_Serialize, MachineView_Serialize } from './bindings'
import { status } from './connection'
import { CHECKING, rowStatus, type Probe } from './machineRows'

const MACHINE: MachineView_Serialize = {
  id: 2,
  name: 'Studio',
  routes: [{ kind: 'bridge', url: 'ws://studio.lan:9000/', has_token: true }],
  last_route: null,
}

const shown = (probe: Probe) => {
  const { text, tone, pulsing } = rowStatus(MACHINE, null, probe)
  return { text, tone, pulsing }
}

describe('rowStatus of the active machine', () => {
  it.each<ConnectionView_Serialize>([
    { state: 'connected', route: 'ws://studio.lan:9000/?token=abc' },
    { state: 'connecting' },
    { state: 'offline' },
    { state: 'unauthorized' },
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

describe('rowStatus of another machine', () => {
  it.each<[string, Probe, ReturnType<typeof shown>]>([
    ['checking', CHECKING, { text: 'Checking…', tone: 'working', pulsing: true }],
    [
      'answering',
      { state: 'answers', route: 'ws://studio.lan:9000/?token=abc' },
      { text: 'Answers via studio.lan:9000', tone: 'done', pulsing: false },
    ],
    ['rejecting its token', { state: 'unauthorized' }, { text: 'Token rejected', tone: 'failed', pulsing: false }],
    [
      'without a token',
      { state: 'no_token' },
      { text: 'No token: scan its code again', tone: 'failed', pulsing: false },
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
        { route: 'ws://studio.lan:9000/?token=abc', error: { code: 'timeout', message: 'timed out' } },
        { route: 'ws://100.64.0.2:9000/?token=abc', error: { code: 'connect', message: 'refused' } },
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

  it('carries no route errors while it answers', () => {
    expect(rowStatus(MACHINE, null, { state: 'answers', route: 'ws://studio.lan:9000/' }).failures).toEqual([])
  })
})
