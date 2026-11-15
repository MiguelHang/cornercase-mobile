import { describe, expect, it } from 'vitest'
import type { ConnectionView_Serialize, MachineView_Serialize } from './bindings'
import { status } from './connection'
import { CHECKING, rescannable, rowStatus, type Probe } from './machineRows'

const MACHINE: MachineView_Serialize = {
  id: 2,
  name: 'Studio',
  routes: [{ kind: 'cli', url: 'ws://studio.lan:9000/', has_token: true }],
  last_route: null,
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
      { state: 'answers', route: 'ws://studio.lan:9000/' },
      { text: 'Answers via studio.lan:9000', tone: 'done', pulsing: false },
    ],
    ['rejecting its token', { state: 'unauthorized' }, { text: 'Token rejected', tone: 'failed', pulsing: false }],
    ['forgetting this phone', { state: 'not_paired' }, { text: 'Not paired', tone: 'failed', pulsing: false }],
    [
      'without a token',
      { state: 'no_token' },
      { text: 'No secret: scan its code again', tone: 'failed', pulsing: false },
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

  it('carries no route errors while it answers', () => {
    expect(rowStatus(MACHINE, null, { state: 'answers', route: 'ws://studio.lan:9000/' }).failures).toEqual([])
  })
})

describe('rescannable', () => {
  it.each<[string, Probe, boolean]>([
    ['a rejected token', { state: 'unauthorized' }, true],
    ['no token', { state: 'no_token' }, true],
    ['a Mac that forgot this phone', { state: 'not_paired' }, true],
    ['an answer', { state: 'answers', route: 'ws://studio.lan:9000/' }, false],
    ['no answer', { state: 'no_answer', failures: [] }, false],
    ['a probe still running', CHECKING, false],
  ])('sends another machine with %s to scan again or not', (_, probe, expected) => {
    expect(rescannable(null, probe)).toBe(expected)
  })

  it.each<[ConnectionView_Serialize, boolean]>([
    [{ state: 'unauthorized' }, true],
    [{ state: 'not_paired' }, true],
    [{ state: 'connected', route: 'ws://studio.lan:9000/' }, false],
    [{ state: 'offline' }, false],
  ])('follows the connection of the active one, not its last probe (%o)', (connection, expected) => {
    expect(rescannable(connection, { state: 'no_token' })).toBe(expected)
  })
})
