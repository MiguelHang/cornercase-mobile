import type { CommandError, MergeView, SameWhy } from './bindings'

export type Checked = { ok: true; url: string } | { ok: false; error: string }

export type NameChecked = { ok: true; name: string } | { ok: false; error: string }

export type Field = 'url' | 'name' | 'form'

export type Offer = { url: string; name: string; machine: number; saved: string; why: SameWhy; checked: boolean }

export const EMPTY = 'Paste the code from cornercase › Settings › Remote › Phone app'
export const NO_SCHEME = 'The code starts with ws://'
export const NOT_A_URL = 'This is not a machine code'
export const NO_CODE = 'The code is cut short: paste it whole'

export const LONGEST_NAME = 64
export const NAME_EMPTY = 'the name is empty'
export const NAME_TOO_LONG = `the name is longer than ${LONGEST_NAME} characters`
export const NAME_TAKEN = 'another machine already has that name'

export const UNCHECKED = 'cornercase did not answer, so it could not be asked which machine it is.'
export const NEW_MACHINE = 'Add as a new machine'

const NAME = 'Name'

const REASONS: Record<SameWhy, (saved: string) => string> = {
  address: (saved) => `${saved} is already saved at this address.`,
  boot: (saved) => `This code comes from the same machine as ${saved}.`,
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
  return found.searchParams.get('code') ? { ok: true, url } : refused(NO_CODE)
}

const folded = (name: string) => name.trim().toLowerCase()

const isTaken = (name: string, taken: readonly string[]) => taken.some((other) => folded(other) === folded(name))

const cut = (name: string) => [...name.trim()].slice(0, LONGEST_NAME).join('').trim()

const suggested = (raw: string, taken: readonly string[]) => {
  const checked = checkUrl(raw)
  if (!checked.ok) {
    return ''
  }
  const url = new URL(checked.url)
  const named = cut(url.searchParams.get('name') ?? '')
  return named !== '' && !isTaken(named, taken) ? named : url.hostname
}

export const nameFor = (name: string, url: string, taken: readonly string[] = []) =>
  name.trim() || suggested(url, taken)

export const placeholderName = (url: string, taken: readonly string[] = []) => suggested(url, taken) || NAME

export const checkName = (raw: string, taken: readonly string[] = []): NameChecked => {
  const name = raw.trim()
  if (name === '') {
    return { ok: false, error: NAME_EMPTY }
  }
  if ([...name].length > LONGEST_NAME) {
    return { ok: false, error: NAME_TOO_LONG }
  }
  if (isTaken(name, taken)) {
    return { ok: false, error: NAME_TAKEN }
  }
  return { ok: true, name }
}

export const offerText = (offer: Pick<Offer, 'saved' | 'why'>) => ({
  reason: REASONS[offer.why](offer.saved),
  keep: offer.why === 'address' ? `Update the token of ${offer.saved}` : `Add its addresses to ${offer.saved}`,
})

export const counted = (count: number, what: string) => `${count} ${what}${count === 1 ? '' : 's'}`

const mergeSummary = (merged: MergeView) => {
  const told = [
    merged.added.length > 0 ? `${counted(merged.added.length, 'route')} added` : '',
    merged.token_updated ? 'token updated' : '',
    merged.skipped.length > 0 ? `${counted(merged.skipped.length, 'route')} left to another machine` : '',
  ].filter((part) => part !== '')
  return told.length > 0 ? told.join(', ') : 'nothing to add'
}

export const mergedText = (saved: string, merged: MergeView) => `${saved} already saved: ${mergeSummary(merged)}.`

export const routesText = (machine: string, merged: MergeView) => `${machine}: ${mergeSummary(merged)}.`

export const fieldOf = (error: CommandError): Field => {
  if (error.code.startsWith('route_')) {
    return 'url'
  }
  return error.code.startsWith('name_') ? 'name' : 'form'
}
