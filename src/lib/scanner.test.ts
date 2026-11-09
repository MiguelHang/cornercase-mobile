import { describe, expect, it, vi } from 'vitest'
import { Format, type Scanned } from '@tauri-apps/plugin-barcode-scanner'
import { Scanner, type ScannerApi } from './scanner.svelte'

type PermissionState = Awaited<ReturnType<ScannerApi['checkPermissions']>>

const URL_TEXT = 'ws://studio.lan:9000/?token=abc'

const read = (content: string): Scanned => ({ content, format: Format.QRCode, bounds: null })

const fakeRoot = () => {
  const classes = new Set<string>()
  const root = { add: (token: string) => classes.add(token), remove: (token: string) => classes.delete(token) }
  return { classes, root }
}

const plugin = (overrides: Partial<ScannerApi> = {}) => {
  const allowed = () => Promise.resolve<PermissionState>('granted')
  const api = {
    checkPermissions: vi.fn(overrides.checkPermissions ?? allowed),
    requestPermissions: vi.fn(overrides.requestPermissions ?? allowed),
    scan: vi.fn(overrides.scan ?? (() => Promise.resolve(read(URL_TEXT)))),
    cancel: vi.fn(overrides.cancel ?? (() => Promise.resolve())),
    openAppSettings: vi.fn(overrides.openAppSettings ?? (() => Promise.resolve())),
  }
  const { classes, root } = fakeRoot()
  return { api, classes, scanner: new Scanner(api, root) }
}

const pendingScan = () => {
  let close: (error: Error) => void = () => {}
  const scan = vi.fn<ScannerApi['scan']>(() => new Promise((_, reject) => (close = reject)))
  const cancel = vi.fn<ScannerApi['cancel']>(() => {
    close(new Error('cancelled'))
    return Promise.resolve()
  })
  return { scan, cancel }
}

const refusing: Partial<ScannerApi> = {
  checkPermissions: () => Promise.resolve('denied'),
  requestPermissions: () => Promise.resolve('denied'),
}

const settled = () => new Promise((resolve) => setTimeout(resolve))

describe('Scanner', () => {
  it('reads a QR code with the camera in a window over a see-through page', async () => {
    const { api, classes, scanner } = plugin()
    let during = false
    api.scan.mockImplementation(() => {
      during = classes.has('scanning')
      return Promise.resolve(read(URL_TEXT))
    })
    expect(await scanner.scan()).toBe(URL_TEXT)
    expect(api.scan.mock.calls).toEqual([[{ windowed: true, formats: [Format.QRCode] }]])
    expect(during).toBe(true)
    expect([scanner.state, scanner.error]).toEqual(['idle', null])
  })

  it.each<[string, PermissionState, number]>([
    ['granted', 'granted', 0],
    ['to be asked', 'prompt', 1],
  ])('asks for the camera only when it is not granted yet: %s', async (_, state, asked) => {
    const { api, scanner } = plugin({ checkPermissions: () => Promise.resolve(state) })
    expect(await scanner.scan()).toBe(URL_TEXT)
    expect(api.requestPermissions).toHaveBeenCalledTimes(asked)
  })

  it.each<[string, PermissionState]>([
    ['refused when asked', 'prompt'],
    ['refused before', 'denied'],
  ])('tells a camera %s apart, without scanning', async (_, state) => {
    const { api, scanner } = plugin({
      checkPermissions: () => Promise.resolve(state),
      requestPermissions: () => Promise.resolve('denied'),
    })
    expect(await scanner.scan()).toBeNull()
    expect([scanner.state, scanner.error]).toEqual(['denied', null])
    expect(api.scan).not.toHaveBeenCalled()
  })

  it('opens the app settings', async () => {
    const { api, scanner } = plugin()
    await scanner.openSettings()
    expect(api.openAppSettings).toHaveBeenCalledTimes(1)
    expect(scanner.state).toBe('idle')
  })

  it('fails with the reason when the settings do not open', async () => {
    const { scanner } = plugin({ ...refusing, openAppSettings: () => Promise.reject(new Error('no settings')) })
    await scanner.scan()
    await scanner.openSettings()
    expect([scanner.state, scanner.error]).toEqual(['failed', 'no settings'])
  })

  it.each<[string, Partial<ScannerApi>, string]>([
    ['the scan', { scan: () => Promise.reject(new Error('No camera available')) }, 'No camera available'],
    ['the permission check', { checkPermissions: () => Promise.reject('plugin not found') }, 'plugin not found'],
    ['the permission request', { requestPermissions: () => Promise.reject('busy') }, 'busy'],
  ])('fails with the reason of an error in %s', async (_, failing, error) => {
    const { scanner } = plugin({ checkPermissions: () => Promise.resolve('prompt'), ...failing })
    expect(await scanner.scan()).toBeNull()
    expect([scanner.state, scanner.error]).toEqual(['failed', error])
  })

  it('fails without a scanner, as on a computer, even when cancel fails too', async () => {
    const missing = () => Promise.reject(new Error('plugin barcode-scanner not found'))
    const { scanner } = plugin({ checkPermissions: missing, cancel: missing })
    expect(await scanner.scan()).toBeNull()
    expect([scanner.state, scanner.error]).toEqual(['failed', 'plugin barcode-scanner not found'])
  })

  it('rests without an error when the scan is cancelled', async () => {
    const { scan, cancel } = pendingScan()
    const { scanner } = plugin({ scan, cancel })
    const scanned = scanner.scan()
    await settled()
    expect(scanner.state).toBe('scanning')
    await scanner.stop()
    expect(await scanned).toBeNull()
    expect([scanner.state, scanner.error]).toEqual(['idle', null])
  })

  it('ignores a code read after the scan was cancelled', async () => {
    const { api, scanner } = plugin()
    api.scan.mockImplementation(async () => {
      await scanner.stop()
      return read(URL_TEXT)
    })
    expect(await scanner.scan()).toBeNull()
    expect(scanner.state).toBe('idle')
  })

  it('does not open the camera when cancelled while asking for it', async () => {
    let answer: (state: PermissionState) => void = () => {}
    const { api, classes, scanner } = plugin({
      checkPermissions: () => Promise.resolve('prompt'),
      requestPermissions: () => new Promise((resolve) => (answer = resolve)),
    })
    const scanned = scanner.scan()
    await settled()
    await scanner.stop()
    answer('granted')
    expect(await scanned).toBeNull()
    expect(api.scan).not.toHaveBeenCalled()
    expect([scanner.state, classes.has('scanning')]).toEqual(['idle', false])
  })

  it('closes a refusal or a failure', async () => {
    const { scanner } = plugin({ checkPermissions: () => Promise.reject(new Error('broken')) })
    await scanner.scan()
    await scanner.stop()
    expect([scanner.state, scanner.error]).toEqual(['idle', null])
  })

  it('ignores a second scan while one is open', async () => {
    const { scan, cancel } = pendingScan()
    const { api, scanner } = plugin({ scan, cancel })
    const first = scanner.scan()
    await settled()
    expect(await scanner.scan()).toBeNull()
    await scanner.stop()
    await first
    expect(api.scan).toHaveBeenCalledTimes(1)
  })

  it('clears the old error on a new scan', async () => {
    const { api, scanner } = plugin()
    api.scan.mockImplementationOnce(() => Promise.reject(new Error('No camera available')))
    await scanner.scan()
    expect(await scanner.scan()).toBe(URL_TEXT)
    expect([scanner.state, scanner.error]).toEqual(['idle', null])
  })

  it.each<[string, Partial<ScannerApi>]>([
    ['a code read', {}],
    ['a refusal', refusing],
    ['an error', { scan: () => Promise.reject(new Error('No camera available')) }],
    ['a cancel', pendingScan()],
  ])('cancels the camera and makes the page opaque again after %s', async (_, overrides) => {
    const { api, classes, scanner } = plugin(overrides)
    const scanned = scanner.scan()
    await settled()
    if (scanner.state === 'scanning') {
      await scanner.stop()
    }
    await scanned
    expect(api.cancel).toHaveBeenCalled()
    expect(classes.has('scanning')).toBe(false)
  })
})
