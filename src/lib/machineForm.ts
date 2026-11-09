import type { CommandError, SameWhy } from './bindings'

export type Checked = { ok: true; url: string } | { ok: false; error: string }

export type NameChecked = { ok: true; name: string } | { ok: false; error: string }

export type Field = 'url' | 'name' | 'form'

export type Offer = { url: string; name: string; machine: number; saved: string; why: SameWhy; checked: boolean }

export const EMPTY = 'Paste the URL the bridge prints'
export const NO_SCHEME = 'The URL starts with ws://'
export const NOT_A_URL = 'This is not a URL'
export const NO_TOKEN = 'The URL has no token=: paste it whole'

export const LONGEST_NAME = 64
export const NAME_EMPTY = 'the name is empty'
export const NAME_TOO_LONG = `the name is longer than ${LONGEST_NAME} characters`
export const NAME_TAKEN = 'another machine already has that name'

export const ALREADY_SAVED = 'This URL is already saved: there is nothing to add.'
export const UNCHECKED = 'The bridge did not answer at this URL, so it could not be asked which machine it is.'
export const NEW_MACHINE = 'Add as a new machine'

const NAME = 'Name'

const REASONS: Record<SameWhy, (saved: string) => string> = {
  address: (saved) => `${saved} already has a route at this address.`,
  boot: (saved) => `This bridge is the one ${saved} answers from.`,
  name: (saved) => `A machine is already called ${saved}.`,
}

const parsed = (url: string) => {
  try {
    return new URL(url)
  } catch {
    return null
  }
}

const refused = (error: string): Checked => ({ ok: false, error })

export const checkUrl = (raw: string): Checked => {
  const url = raw.trim()
  if (url === '') {
    return refused(EMPTY)
  }
  if (!/^ws:\/\//i.test(url)) {
    return refused(NO_SCHEME)
  }
  const found = parsed(url)
  if (found === null) {
    return refused(NOT_A_URL)
  }
  return found.searchParams.get('token') ? { ok: true, url } : refused(NO_TOKEN)
}

const hostOf = (raw: string) => {
  const checked = checkUrl(raw)
  return checked.ok ? new URL(checked.url).hostname : ''
}

export const nameFor = (name: string, url: string) => name.trim() || hostOf(url)

export const placeholderName = (url: string) => hostOf(url) || NAME

const folded = (name: string) => name.trim().toLowerCase()

export const checkName = (raw: string, taken: readonly string[] = []): NameChecked => {
  const name = raw.trim()
  if (name === '') {
    return { ok: false, error: NAME_EMPTY }
  }
  if ([...name].length > LONGEST_NAME) {
    return { ok: false, error: NAME_TOO_LONG }
  }
  if (taken.some((other) => folded(other) === folded(name))) {
    return { ok: false, error: NAME_TAKEN }
  }
  return { ok: true, name }
}

export const offerText = (offer: Pick<Offer, 'saved' | 'why'>) => ({
  reason: REASONS[offer.why](offer.saved),
  keep: offer.why === 'address' ? `Update the token of ${offer.saved}` : `Add as a route of ${offer.saved}`,
})

export const fieldOf = (error: CommandError): Field => {
  if (error.code.startsWith('route_')) {
    return 'url'
  }
  return error.code.startsWith('name_') ? 'name' : 'form'
}
