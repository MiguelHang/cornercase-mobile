import { Format } from '@tauri-apps/plugin-barcode-scanner'
import type * as barcode from '@tauri-apps/plugin-barcode-scanner'
import { messageOf } from './toasts'

export type ScannerApi = Pick<
  typeof barcode,
  'checkPermissions' | 'requestPermissions' | 'scan' | 'cancel' | 'openAppSettings'
>

export type Classes = { add(token: string): void; remove(token: string): void }

export type ScanState = 'idle' | 'scanning' | 'denied' | 'failed'

const SCANNING = 'scanning'

export const pageClasses: Classes = {
  add: (token) => document.documentElement.classList.add(token),
  remove: (token) => document.documentElement.classList.remove(token),
}

type Permission = Awaited<ReturnType<ScannerApi['checkPermissions']>>

const granted = (state: Permission) => state === 'granted'

export class Shown {
  scanner = $state<Scanner | null>(null)

  get scanning() {
    return this.scanner?.state === 'scanning'
  }

  async stop() {
    await this.scanner?.stop()
  }
}

export const shown = new Shown()

export class Scanner {
  state = $state<ScanState>('idle')
  error = $state<string | null>(null)
  #api: ScannerApi
  #root: Classes
  #shown: Shown

  constructor(api: ScannerApi, root: Classes, on: Shown = shown) {
    this.#api = api
    this.#root = root
    this.#shown = on
  }

  async scan(): Promise<string | null> {
    if (this.state === 'scanning') {
      return null
    }
    this.state = 'scanning'
    this.error = null
    this.#shown.scanner = this
    try {
      return await this.#read()
    } catch (error) {
      this.#fail(error)
      return null
    } finally {
      this.#root.remove(SCANNING)
      await this.#api.cancel().catch(() => undefined)
    }
  }

  async stop() {
    this.state = 'idle'
    this.error = null
    this.#root.remove(SCANNING)
    await this.#api.cancel().catch(() => undefined)
  }

  async openSettings() {
    await this.#api.openAppSettings().catch((error: unknown) => this.#fail(error))
  }

  async #read() {
    const allowed = await this.#allowed()
    if (this.state !== 'scanning') {
      return null
    }
    if (!allowed) {
      this.state = 'denied'
      return null
    }
    this.#root.add(SCANNING)
    const scanned = await this.#api.scan({ windowed: true, formats: [Format.QRCode] })
    if (this.state !== 'scanning') {
      return null
    }
    this.state = 'idle'
    return scanned.content
  }

  async #allowed() {
    return granted(await this.#api.checkPermissions()) || granted(await this.#api.requestPermissions())
  }

  #fail(error: unknown) {
    if (this.state === 'idle') {
      return
    }
    this.state = 'failed'
    this.error = messageOf(error)
  }
}
