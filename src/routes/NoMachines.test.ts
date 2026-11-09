import { Format } from '@tauri-apps/plugin-barcode-scanner'
import { render } from 'svelte/server'
import { describe, expect, it, vi } from 'vitest'
import { AddMachine } from '../lib/addMachine.svelte'
import { Scanner, type ScannerApi } from '../lib/scanner.svelte'
import ScanOverlay from '../lib/ui/ScanOverlay.svelte'
import overlaySource from '../lib/ui/ScanOverlay.svelte?raw'
import urlFieldSource from '../lib/ui/UrlField.svelte?raw'
import MachineForm from './MachineForm.svelte'
import NoMachines from './NoMachines.svelte'
import formSource from './MachineForm.svelte?raw'
import noMachinesSource from './NoMachines.svelte?raw'

type Permission = Awaited<ReturnType<ScannerApi['checkPermissions']>>

const SCANNED = 'ws://studio.lan:9000/?token=abc'

const camera = (permission: Permission, scan: ScannerApi['scan']) =>
  new Scanner(
    {
      checkPermissions: () => Promise.resolve(permission),
      requestPermissions: () => Promise.resolve(permission),
      scan,
      cancel: () => Promise.resolve(),
      openAppSettings: () => Promise.resolve(),
    },
    { add: () => {}, remove: () => {} },
  )

const qrCode = () => Promise.resolve({ content: SCANNED, format: Format.QRCode, bounds: null })

const overlay = async (scanner: Scanner) => {
  void scanner.scan()
  await new Promise((resolve) => setTimeout(resolve))
  return render(ScanOverlay, { props: { scanner } }).body
}

const flat = (text: string) => text.replace(/\s+/g, ' ')

const button = (body: string, label: string) =>
  body.match(new RegExp(`<button class="([^"]*)" type="(\\w+)"[^>]*>(?:<!--[^>]*-->| )*${label}<!---->`))

describe('NoMachines', () => {
  const body = render(NoMachines, { props: { onadded: () => {} } }).body

  it('offers adding a machine, ready to be pressed', () => {
    const add = button(body, 'Add machine')
    expect(add?.[1]).toMatch(/^button primary svelte-[\w-]+$/)
    expect(add?.[2]).toBe('button')
    expect(add?.[0]).not.toMatch(/disabled|aria-/)
  })

  it('shows no form yet', () => {
    expect(body).not.toContain('<form')
    expect(body).not.toContain('Bridge URL')
  })

  it('swaps the button for the form inside the card, under the emblem, and back on Cancel', () => {
    const markup = flat(noMachinesSource)
    expect(markup).toContain(
      '<Emblem icon="server" title="No machines yet" lines={LINES} /> {#if open} <MachineForm {onadded} oncancel={() => (open = false)} /> {:else}',
    )
    expect(markup).toContain('<Button onclick={() => (open = true)}>Add machine</Button>')
  })
})

describe('MachineForm', () => {
  const body = render(MachineForm, { props: { onadded: () => {}, oncancel: () => {} } }).body

  it('is a form', () => {
    expect(body).toMatch(/^<!--\[--><form class="form svelte-[\w-]+">/)
  })

  it('asks for the bridge URL in a field that leaves it as pasted', () => {
    expect(body).toMatch(
      /<label class="label svelte-[\w-]+" for="(\w+)-input">Bridge URL<\/label> <input id="\1-input" class="input svelte-[\w-]+" type="text" autocomplete="off" autocapitalize="off" autocorrect="off" spellcheck="false" inputmode="url" value=""\/>/,
    )
  })

  it('offers scanning the QR code above the URL field', () => {
    expect(body).toMatch(
      /<button class="button plain svelte-[\w-]+" type="button"[^>]*>(?:<!--[^>]*-->| )*<svg[^]*?<\/svg><!----> Scan QR code<!---->/,
    )
    expect(body.indexOf('Scan QR code')).toBeLessThan(body.indexOf('Bridge URL'))
  })

  it('asks for a name, with a placeholder until there is a URL', () => {
    expect(body).toMatch(
      /<label class="label svelte-[\w-]+" for="(\w+)-input">Name<\/label> <input id="\1-input" class="input svelte-[\w-]+" type="text" autocomplete="off" placeholder="Name" value=""\/>/,
    )
  })

  it('ends with Cancel, plain, and Add, the primary that submits', () => {
    expect(button(body, 'Cancel')?.slice(1)).toEqual([expect.stringMatching(/^button plain svelte-[\w-]+$/), 'button'])
    expect(button(body, 'Add')?.slice(1)).toEqual([expect.stringMatching(/^button primary svelte-[\w-]+$/), 'submit'])
  })

  it('shows no problem before it is sent', () => {
    expect(body).not.toMatch(/role="alert"|aria-invalid|disabled/)
  })

  const script = flat(formSource)

  it('submits through AddMachine, with the commands and the handover', () => {
    expect(script).toContain('const form = new AddMachine(commands.machineAdd, (machine) => onadded(machine))')
    expect(script).toContain('event.preventDefault() void form.submit()')
  })

  it('holds still while adding: fields and Cancel off, the spinner and Adding… on Add', () => {
    expect(script).toContain(
      '<UrlField bind:value={form.url} error={form.urlError} disabled={form.busy} onscanned={(url) => void form.fill(url)} />',
    )
    expect(script).toContain(
      '<Field label="Name" bind:value={form.name} placeholder={placeholderName(form.url)} disabled={form.busy} />',
    )
    expect(script).toContain('<Button tone="plain" resting={form.busy} onclick={oncancel}>Cancel</Button>')
    expect(script).toContain("<Button type=\"submit\" busy={form.busy}>{form.busy ? 'Adding…' : 'Add'}</Button>")
  })

  it('announces a problem of the form below its fields', () => {
    expect(script).toContain('{#if form.formError !== null} <p class="problem" role="alert">{form.formError}</p> {/if}')
  })

  it.each([
    'padding: 0 var(--inset-card) var(--inset-card);',
    '.problem { margin: 0; color: var(--failed); font-size: 13px; }',
    'justify-content: flex-end; gap: var(--gap);',
  ])('holds %s', (text) => {
    expect(script).toContain(text)
  })
})

describe('scanning a QR code', () => {
  it('fills the URL with the code read and sends it like a pasted one', async () => {
    const add = vi.fn(() => Promise.resolve({ status: 'ok' as const, data: 5 }))
    const onadded = vi.fn()
    const form = new AddMachine(add, onadded)
    const url = await camera('granted', qrCode).scan()
    await form.fill(url ?? '')
    expect([form.url, add.mock.calls, onadded.mock.calls]).toEqual([SCANNED, [[SCANNED, 'studio.lan']], [[5]]])
  })

  it('hands the code read to the form, and nothing when there is none', () => {
    const script = flat(urlFieldSource)
    expect(script).toContain('const scanner = new Scanner(barcode, pageClasses)')
    expect(script).toContain('const url = await scanner.scan() if (url !== null) { onscanned(url) }')
    expect(script).toContain('<Button tone="plain" resting={disabled} onclick={() => void scan()}>')
  })

  it('shows nothing over the form until a scan starts', () => {
    const body = render(ScanOverlay, { props: { scanner: camera('granted', qrCode) } }).body
    expect(body).not.toMatch(/layer|notice|button/)
  })

  it('puts a frame and Cancel over the camera while it scans', async () => {
    const body = await overlay(camera('granted', () => new Promise(() => {})))
    expect(body).toMatch(/<div class="layer svelte-[\w-]+"><div class="frame svelte-[\w-]+"><\/div>/)
    expect(body).toContain('Point the camera at the QR code the bridge prints.')
    expect(button(body, 'Cancel')?.slice(1)).toEqual([expect.stringMatching(/^button plain svelte-[\w-]+$/), 'button'])
  })

  it('offers Open Settings when the camera is not allowed, with the field still there to paste', async () => {
    const body = await overlay(camera('denied', qrCode))
    expect(body).toContain('Camera access is off. Turn it on in Settings, or paste the URL below.')
    expect(button(body, 'Open Settings')?.slice(1)).toEqual([
      expect.stringMatching(/^button plain svelte-[\w-]+$/),
      'button',
    ])
    expect(body).not.toContain('layer')
  })

  it('says why the scan failed and leaves pasting to the field', async () => {
    const body = await overlay(camera('granted', () => Promise.reject(new Error('No camera available'))))
    expect(body).toContain('Could not scan the code. Paste the URL below.')
    expect(body).toMatch(/<p class="detail svelte-[\w-]+">No camera available<\/p>/)
  })

  const markup = flat(overlaySource)

  it('cancels with the scanner and opens the settings through it', () => {
    expect(markup).toContain('<Button tone="plain" onclick={() => void scanner.stop()}>Cancel</Button>')
    expect(markup).toContain(
      '<Button tone="plain" onclick={() => void scanner.openSettings()}>Open Settings</Button>',
    )
  })

  it('lays the frame over the whole page, out of the hidden app', () => {
    expect(markup).toContain('<div class="layer" {@attach overPage}>')
    expect(markup).toContain('document.body.append(node) return () => node.remove()')
    expect(markup).toContain('.layer { position: fixed; inset: 0; z-index: 20;')
    expect(markup).toContain('box-shadow: 0 0 0 100vmax var(--veil);')
  })
})
