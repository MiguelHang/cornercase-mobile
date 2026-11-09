import { render } from 'svelte/server'
import { describe, expect, it } from 'vitest'
import MachineForm from './MachineForm.svelte'
import NoMachines from './NoMachines.svelte'
import formSource from './MachineForm.svelte?raw'
import noMachinesSource from './NoMachines.svelte?raw'

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
    expect(script).toContain('<Field label="Bridge URL" url bind:value={form.url} error={form.urlError} disabled={form.busy} />')
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
