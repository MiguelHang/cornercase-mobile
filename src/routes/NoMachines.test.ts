import { Format } from '@tauri-apps/plugin-barcode-scanner'
import { render } from 'svelte/server'
import { describe, expect, it, vi } from 'vitest'
import { AddMachine } from '../lib/addMachine.svelte'
import { Scanner, type ScannerApi } from '../lib/scanner.svelte'
import type { Offer } from '../lib/machineForm'
import SameOffer from '../lib/ui/SameOffer.svelte'
import ScanOverlay from '../lib/ui/ScanOverlay.svelte'
import overlaySource from '../lib/ui/ScanOverlay.svelte?raw'
import overPageSource from '../lib/ui/overPage.ts?raw'
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
    expect(body).not.toContain('Machine code')
  })

  it('swaps the button for the form inside the card, under the emblem, and back on Cancel', () => {
    const markup = flat(noMachinesSource)
    expect(markup).toContain(
      '<Emblem icon="server" title="No machines yet" lines={LINES} /> {#if open} <MachineForm {onadded} onrouted={onadded} oncancel={() => (open = false)} /> {:else}',
    )
    expect(markup).toContain('<Button onclick={() => (open = true)}>Add machine</Button>')
  })
})

describe('MachineForm', () => {
  const body = render(MachineForm, { props: { onadded: () => {}, onrouted: () => {}, oncancel: () => {} } }).body

  it('is a form', () => {
    expect(body).toMatch(/^<!--\[--><form class="form svelte-[\w-]+">/)
  })

  it('asks for the machine code in a field that leaves it as pasted', () => {
    expect(body).toMatch(
      /<label class="label svelte-[\w-]+" for="(\w+)-input">Machine code<\/label>(?:<!--[^>]*-->)* <input id="\1-input" class="input svelte-[\w-]+" type="text" autocomplete="off" autocapitalize="off" autocorrect="off" spellcheck="false" inputmode="url" value=""\/>/,
    )
  })

  it('offers scanning the QR code above the code field', () => {
    expect(body).toMatch(
      /<button class="button plain svelte-[\w-]+" type="button"[^>]*>(?:<!--[^>]*-->| )*<svg[^]*?<\/svg><!----> Scan QR code<!---->/,
    )
    expect(body.indexOf('Scan QR code')).toBeLessThan(body.indexOf('Machine code'))
  })

  it('asks for a name, with a placeholder until there is a code', () => {
    expect(body).toMatch(
      /<label class="label svelte-[\w-]+" for="(\w+)-input">Name<\/label>(?:<!--[^>]*-->)* <input id="\1-input" class="input svelte-[\w-]+" type="text" autocomplete="off" placeholder="Name" value=""\/>/,
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

  it('submits through AddMachine, with the commands, the handover and the toasts', () => {
    expect(script).toContain(
      'const form = new AddMachine( commands, (machine) => onadded(machine), (machine) => onrouted(machine), (toast) => void events.toast.emit(toast), )',
    )
    expect(script).toContain('event.preventDefault() void form.submit(taken)')
  })

  it('takes no names as taken unless told', () => {
    expect(script).toContain('let { taken = [],')
  })

  it('holds still while adding: fields and Cancel off, the spinner and Adding… on Add', () => {
    expect(script).toContain(
      '<UrlField bind:value={form.url} error={form.urlError} disabled={form.held} onscanned={(url) => void form.fill(url, taken)} />',
    )
    expect(script).toContain(
      '<Field label="Name" bind:value={form.name} error={form.nameError} placeholder={placeholderName(form.url, taken)} disabled={form.held} />',
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

describe('the same machine offer', () => {
  const offer = (why: Offer['why'], checked = true): Offer => ({
    url: SCANNED,
    name: 'studio.lan',
    machine: 3,
    saved: 'Studio',
    why,
    checked,
  })

  const shown = (made: Offer, busy = false) =>
    render(SameOffer, { props: { offer: made, busy, onroute: () => {}, onnew: () => {}, onback: () => {} } }).body

  it('says why, then offers the route first as the primary, a new machine and Back', () => {
    const body = shown(offer('boot'))
    expect(body).toMatch(/<div class="offer svelte-[\w-]+" role="group" aria-label="Same machine">/)
    expect(body).toContain('This bridge is the one Studio answers from.')
    expect(button(body, 'Add its addresses to Studio')?.[1]).toMatch(/^button primary svelte-[\w-]+$/)
    expect(button(body, 'Add as a new machine')?.[1]).toMatch(/^button plain svelte-[\w-]+$/)
    expect(button(body, 'Back')?.[1]).toMatch(/^button plain svelte-[\w-]+$/)
    expect(body.indexOf('Add its addresses')).toBeLessThan(body.indexOf('Add as a new machine'))
    expect(body).not.toContain('did not answer')
  })

  it('offers updating the token for the same address', () => {
    expect(shown(offer('address'))).toContain('Update the token of Studio')
  })

  it('warns when the bridge could not be asked', () => {
    expect(shown(offer('name', false))).toMatch(
      /<p class="note svelte-[\w-]+">The bridge did not answer, so it could not be asked which machine it is.<\/p>/,
    )
  })

  it('holds every choice while one runs', () => {
    expect(shown(offer('boot'), true).match(/<button[^>]* disabled=""/g)).toHaveLength(3)
  })

  it('takes the place of Cancel and Add, with the choices through AddMachine', () => {
    const script = flat(formSource)
    expect(script).toContain('{#if form.offer === null} <div class="buttons">')
    expect(script).toContain(
      '{:else} <SameOffer offer={form.offer} busy={form.busy} onroute={() => void form.addRoute()} onnew={() => void form.addNew()} onback={() => form.back()} /> {/if}',
    )
  })
})

describe('scanning a QR code', () => {
  it('fills the URL with the code read and sends it like a pasted one', async () => {
    const add = vi.fn(() => Promise.resolve({ status: 'ok' as const, data: 5 }))
    const onadded = vi.fn()
    const form = new AddMachine(
      {
        machineAdd: add,
        machineMatch: () => Promise.resolve({ status: 'ok', data: { found: { kind: 'none' }, checked: true } }),
        machineMerge: () => Promise.resolve({ status: 'ok', data: { added: [], token_updated: false, skipped: [] } }),
      },
      onadded,
      () => {},
      () => {},
    )
    const url = await camera('granted', qrCode).scan()
    await form.fill(url ?? '')
    expect([form.url, add.mock.calls, onadded.mock.calls]).toEqual([SCANNED, [[SCANNED, 'studio.lan']], [[5]]])
  })

  it('hands the code read to the form, and nothing when there is none', () => {
    const script = flat(urlFieldSource)
    expect(script).toContain('const scanner = new Scanner(barcode, pageClasses)')
    expect(script).toContain('$effect(() => () => void scanner.stop())')
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
    expect(body).toContain('Camera access is off. Turn it on in Settings, or paste the code below.')
    expect(button(body, 'Open Settings')?.slice(1)).toEqual([
      expect.stringMatching(/^button plain svelte-[\w-]+$/),
      'button',
    ])
    expect(body).not.toContain('layer')
  })

  it('says why the scan failed and leaves pasting to the field', async () => {
    const body = await overlay(camera('granted', () => Promise.reject(new Error('No camera available'))))
    expect(body).toContain('Could not scan the code. Paste the code below.')
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
    expect(markup).toContain("import { overPage } from './overPage'")
    expect(flat(overPageSource)).toContain('document.body.append(node) return () => node.remove()')
    expect(markup).toContain('.layer { position: fixed; inset: 0; z-index: 20;')
    expect(markup).toContain('box-shadow: 0 0 0 100vmax var(--veil);')
  })
})
