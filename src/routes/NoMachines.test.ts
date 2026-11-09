import { render } from 'svelte/server'
import { describe, expect, it } from 'vitest'
import NoMachines from './NoMachines.svelte'
import formSource from './MachineForm.svelte?raw'
import noMachinesSource from './NoMachines.svelte?raw'

const card = (open: boolean) => render(NoMachines, { props: { onadded: () => {}, open } }).body

const flat = (text: string) => text.replace(/\s+/g, ' ')

const button = (body: string, label: string) =>
  body.match(new RegExp(`<button class="([^"]*)" type="(\\w+)"[^>]*>(?:<!--[^>]*-->| )*${label}<!---->`))

describe('NoMachines', () => {
  describe('closed', () => {
    const body = card(false)

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
  })

  describe('open', () => {
    const body = card(true)

    it('swaps the button for the form, inside the card under the emblem', () => {
      expect(body).not.toContain('Add machine')
      expect(body).toMatch(/No machines yet<\/h2>.*<\/div><!----> <!--\[0--><form class="form svelte-[\w-]+">/)
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
  })

  it('opens the form from the button and closes it on Cancel', () => {
    const markup = flat(noMachinesSource)
    expect(markup).toContain('<MachineForm {onadded} oncancel={() => (open = false)} />')
    expect(markup).toContain('<Button onclick={() => (open = true)}>Add machine</Button>')
  })
})

describe('MachineForm', () => {
  const script = flat(formSource)

  it('checks the URL before adding the machine, under the name the field or the URL gives', () => {
    expect(script).toContain(
      'const checked = checkUrl(url) if (!checked.ok) { urlError = checked.error return } busy = true const added = await commands.machineAdd(checked.url, nameFor(name, url)) busy = false',
    )
  })

  it('hands the new machine over, and puts a route error under the URL and the rest under the form', () => {
    expect(script).toContain(
      "if (added.status === 'ok') { onadded(added.data) } else if (fieldOf(added.error) === 'url') { urlError = added.error.message } else { formError = added.error.message }",
    )
  })

  it('ignores a second submit while busy and clears the old errors on a new one', () => {
    expect(script).toContain(
      'event.preventDefault() if (busy) { return } urlError = null formError = null add().catch((error: unknown) => { busy = false formError = String(error) })',
    )
  })

  it('holds still while adding: fields and Cancel off, the spinner and Adding… on Add', () => {
    expect(script).toContain('<Field label="Bridge URL" url bind:value={url} error={urlError} disabled={busy} />')
    expect(script).toContain('<Field label="Name" bind:value={name} placeholder={placeholderName(url)} disabled={busy} />')
    expect(script).toContain('<Button tone="plain" resting={busy} onclick={oncancel}>Cancel</Button>')
    expect(script).toContain("<Button type=\"submit\" {busy}>{busy ? 'Adding…' : 'Add'}</Button>")
  })

  it('announces a problem of the form below its fields', () => {
    expect(script).toContain('{#if formError !== null} <p class="problem" role="alert">{formError}</p> {/if}')
  })

  it.each([
    'padding: 0 var(--inset-card) var(--inset-card);',
    '.problem { margin: 0; color: var(--failed); font-size: 13px; }',
    'justify-content: flex-end; gap: var(--gap);',
  ])('holds %s', (text) => {
    expect(script).toContain(text)
  })
})
