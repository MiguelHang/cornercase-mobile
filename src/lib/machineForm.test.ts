import { describe, expect, it } from 'vitest'
import type { ErrorCode, SameWhy } from './bindings'
import * as form from './machineForm'

const URL_TEXT = 'ws://studio.lan:9000/?token=c3R1ZGlvLXNlY3JldA'

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
    ['address', 'Studio already has a route at this address.', 'Update the token of Studio'],
    ['token', 'Studio uses the same bridge token.', 'Add as a route of Studio'],
    ['boot', 'This bridge is the one Studio answers from.', 'Add as a route of Studio'],
    ['name', 'A machine is already called Studio.', 'Add as a route of Studio'],
  ])('explains a match by %s and names the choice it keeps', (why, reason, keep) => {
    expect(form.offerText({ saved: 'Studio', why })).toEqual({ reason, keep })
  })
})
