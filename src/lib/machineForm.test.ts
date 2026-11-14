import { describe, expect, it } from 'vitest'
import type { ErrorCode, MergeView, SameWhy } from './bindings'
import * as form from './machineForm'

const URL_TEXT = 'ws://studio.lan:9000/?token=c3R1ZGlvLXNlY3JldA'

const CODE = 'ws://192.168.1.20:9000/?token=c3R1ZGlvLXNlY3JldA&also=100.101.102.103&also=fd7a:115c:a1e0::1'

const named = (name: string) => `${CODE}&name=${name}`

const PAIRING = 'ws://192.168.1.20:47682/?code=4f9a0c1d2e3b&also=[fd7a:115c:a1e0::1]:47682&name=Studio'

describe('checkUrl', () => {
  it.each<[string, string, string]>([
    ['nothing', '', form.EMPTY],
    ['only blanks', ' \n\t ', form.EMPTY],
    ['no scheme', 'studio.lan:9000/?token=abc', form.NO_SCHEME],
    ['plain http', 'http://studio.lan:9000/?token=abc', form.NO_SCHEME],
    ['a secure scheme the bridge does not serve', 'wss://studio.lan:9000/?token=abc', form.NO_SCHEME],
    ['no host', 'ws://', form.NOT_A_URL],
    ['a space in the host', 'ws://studio lan:9000/?token=abc', form.NOT_A_URL],
    ['no token', 'ws://studio.lan:9000/', form.NO_TOKEN],
    ['an empty token', 'ws://studio.lan:9000/?token=', form.NO_TOKEN],
    ['another parameter', 'ws://studio.lan:9000/?tokens=abc', form.NO_TOKEN],
    ['an empty pairing code', 'ws://studio.lan:47682/?code=&name=Studio', form.NO_TOKEN],
    ['empty code and token', 'ws://studio.lan:47682/?code=&token=', form.NO_TOKEN],
  ])('refuses %s', (_, raw, error) => {
    expect(form.checkUrl(raw)).toEqual({ ok: false, error })
  })

  it.each<[string, string]>([
    ['as printed', URL_TEXT],
    ['with spaces around', `  ${URL_TEXT}  `],
    ['with a line break after it', `${URL_TEXT}\n`],
    ['with a line break before it', `\r\n${URL_TEXT}`],
  ])('accepts the URL %s, trimmed', (_, raw) => {
    expect(form.checkUrl(raw)).toEqual({ ok: true, url: URL_TEXT })
  })

  it('accepts a machine code with more addresses and the name of the Mac, trimmed', () => {
    expect(form.checkUrl(` ${named('Studio')}\n`)).toEqual({ ok: true, url: named('Studio') })
  })

  it("accepts cornercase's pairing code, trimmed", () => {
    expect(form.checkUrl(` ${PAIRING}\n`)).toEqual({ ok: true, url: PAIRING })
  })

  it('leaves a code with both a pairing code and a token to the app, which names what is wrong', () => {
    expect(form.checkUrl('ws://studio.lan:47682/?code=4f9a&token=abc').ok).toBe(true)
  })

  it('accepts the scheme in capitals', () => {
    expect(form.checkUrl('WS://studio.lan:9000/?token=abc').ok).toBe(true)
  })
})

describe('nameFor', () => {
  it('keeps a name, trimmed', () => {
    expect(form.nameFor('  Studio  ', URL_TEXT)).toBe('Studio')
  })

  it.each<[string, string]>([
    ['empty', ''],
    ['blank', ' \n '],
  ])('uses the host when the name is %s', (_, name) => {
    expect(form.nameFor(name, ` ${URL_TEXT}\n`)).toBe('studio.lan')
  })

  it('gives no name when neither the name nor the URL has one', () => {
    expect(form.nameFor('', 'studio.lan')).toBe('')
  })

  it.each<[string, string, string]>([
    ['plain', 'Studio', 'Studio'],
    ['with spaces', 'Mac%20Studio', 'Mac Studio'],
    ['with accents', 'Mac%20de%20Mar%C3%ADa', 'Mac de María'],
    ['with an emoji', 'Studio%20%F0%9F%96%A5', 'Studio 🖥'],
    ['with blanks around', '%20%20Studio%20', 'Studio'],
  ])('takes the name of the code, %s, before the host', (_, name, shown) => {
    expect(form.nameFor('', named(name))).toBe(shown)
  })

  it('keeps a typed name over the name of the code', () => {
    expect(form.nameFor(' Desk ', named('Studio'))).toBe('Desk')
  })

  it('cuts a name of the code longer than the longest name, counting an emoji as one', () => {
    expect(form.nameFor('', named('%F0%9F%96%A5'.repeat(70)))).toBe('🖥'.repeat(form.LONGEST_NAME))
    expect(form.nameFor('', named(`${'x'.repeat(63)}%20y`))).toBe('x'.repeat(63))
  })

  it.each<[string, string]>([
    ['empty', ''],
    ['blank', '%20%20'],
  ])('uses the host when the name of the code is %s', (_, name) => {
    expect(form.nameFor('', named(name))).toBe('192.168.1.20')
  })

  it('uses the host when another machine has the name of the code, whatever the case', () => {
    expect(form.nameFor('', named('Studio'), ['Desk', 'STUDIO'])).toBe('192.168.1.20')
    expect(form.nameFor('', named('Studio'), ['Desk'])).toBe('Studio')
  })

  it("takes the name or the host from cornercase's pairing code", () => {
    expect(form.nameFor('', PAIRING)).toBe('Studio')
    expect(form.nameFor('', PAIRING, ['studio'])).toBe('192.168.1.20')
  })
})

describe('placeholderName', () => {
  it.each<[string, string, string]>([
    ['a valid URL', URL_TEXT, 'studio.lan'],
    ['an IPv6 address', 'ws://[fd7a::1]:9000/?token=abc', '[fd7a::1]'],
    ['nothing yet', '', 'Name'],
    ['a URL without its token', 'ws://studio.lan:9000/', 'Name'],
  ])('shows the host for %s', (_, url, shown) => {
    expect(form.placeholderName(url)).toBe(shown)
  })

  it('shows the name of the code, or the host when it is taken', () => {
    expect([form.placeholderName(named('Studio')), form.placeholderName(named('Studio'), ['studio'])]).toEqual([
      'Studio',
      '192.168.1.20',
    ])
  })
})

describe('fieldOf', () => {
  it.each<[ErrorCode, form.Field]>([
    ['route_no_token', 'url'],
    ['route_scheme', 'url'],
    ['route_token_characters', 'url'],
    ['name_empty', 'name'],
    ['name_too_long', 'name'],
    ['name_taken', 'name'],
    ['connect', 'form'],
    ['store', 'form'],
    ['unauthorized', 'form'],
  ])('puts %s under the %s', (code, field) => {
    expect(form.fieldOf({ code, message: 'refused' })).toBe(field)
  })
})

describe('checkName', () => {
  it.each<[string, string, string[], string]>([
    ['empty', '', [], form.NAME_EMPTY],
    ['only blanks', ' \t ', [], form.NAME_EMPTY],
    ['longer than 64 characters', 'x'.repeat(65), [], form.NAME_TOO_LONG],
    ['taken, whatever the case', 'STUDIO', ['studio'], form.NAME_TAKEN],
    ['taken, with spaces around either', ' Studio ', ['Desk', '  studio'], form.NAME_TAKEN],
  ])('refuses a name %s', (_, name, taken, error) => {
    expect(form.checkName(name, taken)).toEqual({ ok: false, error })
  })

  it.each<[string, string, string[], string]>([
    ['trimmed', '  Studio  ', [], 'Studio'],
    ['of 64 characters', 'x'.repeat(64), [], 'x'.repeat(64)],
    ['of 64 characters counted one per emoji', '🖥'.repeat(64), [], '🖥'.repeat(64)],
    ['close to a taken one', 'Studio 2', ['Studio'], 'Studio 2'],
  ])('accepts a name %s', (_, name, taken, kept) => {
    expect(form.checkName(name, taken)).toEqual({ ok: true, name: kept })
  })

  it('tells the same messages as the core', () => {
    expect([form.NAME_EMPTY, form.NAME_TOO_LONG, form.NAME_TAKEN]).toEqual([
      'the name is empty',
      'the name is longer than 64 characters',
      'another machine already has that name',
    ])
  })
})

describe('offerText', () => {
  it.each<[SameWhy, string, string]>([
    ['address', 'Studio is already saved at this address.', 'Update the token of Studio'],
    ['boot', 'This bridge is the one Studio answers from.', 'Add its addresses to Studio'],
    ['name', 'A machine is already called Studio.', 'Add its addresses to Studio'],
  ])('explains a match by %s and names the choice it keeps', (why, reason, keep) => {
    expect(form.offerText({ saved: 'Studio', why })).toEqual({ reason, keep })
  })
})

describe('mergedText', () => {
  const merged = (added: number, token_updated: boolean, skipped = 0): MergeView => ({
    added: Array.from({ length: added }, (_, at) => `ws://10.0.0.${at}:9000/`),
    token_updated,
    skipped: Array.from({ length: skipped }, (_, at) => `ws://10.0.1.${at}:9000/`),
  })

  it.each<[string, MergeView, string]>([
    ['nothing changed', merged(0, false), 'MacBook already saved: nothing to add.'],
    ['a route added and the token updated', merged(1, true), 'MacBook already saved: 1 route added, token updated.'],
    ['two routes added', merged(2, false), 'MacBook already saved: 2 routes added.'],
    ['only the token updated', merged(0, true), 'MacBook already saved: token updated.'],
    [
      'routes of another machine left alone',
      merged(1, false, 2),
      'MacBook already saved: 1 route added, 2 routes left to another machine.',
    ],
    ['only a route of another machine', merged(0, false, 1), 'MacBook already saved: 1 route left to another machine.'],
  ])('tells %s', (_, view, text) => {
    expect(form.mergedText('MacBook', view)).toBe(text)
  })

  it("tells the same of a route added from the machine's own screen, naming it alone", () => {
    expect(form.routesText('Studio', merged(0, false, 1))).toBe('Studio: 1 route left to another machine.')
  })
})

describe('the texts of adding a machine', () => {
  it('speak of the machine and its code, never of routes or URLs', () => {
    const texts = [form.EMPTY, form.NO_SCHEME, form.NOT_A_URL, form.NO_TOKEN, form.UNCHECKED]
    const offers = (['address', 'boot', 'name'] as const).map((why) => form.offerText({ saved: 'Studio', why }))
    const all = [...texts, form.NEW_MACHINE, ...offers.flatMap(({ reason, keep }) => [reason, keep])]
    expect(all.filter((text) => /route|url/i.test(text))).toEqual([])
    expect([form.EMPTY, form.NO_SCHEME, form.NOT_A_URL, form.NO_TOKEN]).toEqual([
      'Paste the code the bridge prints',
      'The code starts with ws://',
      'This is not a machine code',
      'The code is cut short: paste it whole',
    ])
  })
})
