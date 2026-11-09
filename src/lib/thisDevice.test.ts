import { describe, expect, it, vi } from 'vitest'
import { ThisDevice, type DeviceCalls } from './thisDevice.svelte'

const device = (identityFingerprint: DeviceCalls['identityFingerprint']) => {
  const calls = { identityFingerprint: vi.fn(identityFingerprint) }
  return { device: new ThisDevice(calls), calls }
}

describe('ThisDevice', () => {
  it('shows nothing before it loads, and asks nothing by itself', () => {
    const made = device(() => Promise.resolve({ status: 'ok', data: '0a1b2c3d4e' }))
    expect([made.device.shown, made.calls.identityFingerprint.mock.calls.length]).toEqual([null, 0])
  })

  it('keeps the fingerprint of the identity', async () => {
    const { device: made } = device(() => Promise.resolve({ status: 'ok', data: '0a1b2c3d4e' }))
    await made.load()
    expect(made.shown).toEqual({ kind: 'done', data: '0a1b2c3d4e' })
  })

  it.each<[string, DeviceCalls['identityFingerprint'], string]>([
    [
      'refused',
      () => Promise.resolve({ status: 'error', error: { code: 'no_identity', message: 'no identity on this device' } }),
      'no identity on this device',
    ],
    ['rejected', () => Promise.reject(new Error('IPC is gone')), 'IPC is gone'],
  ])('keeps the reason when the fingerprint is %s', async (_, identityFingerprint, message) => {
    const { device: made } = device(identityFingerprint)
    await made.load()
    expect(made.shown).toEqual({ kind: 'form', error: message })
  })
})
