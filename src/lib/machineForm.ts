import type { CommandError } from './bindings'

export type Checked = { ok: true; url: string } | { ok: false; error: string }

export type Field = 'url' | 'form'

export const EMPTY = 'Paste the URL the bridge prints'
export const NO_SCHEME = 'The URL starts with ws://'
export const NOT_A_URL = 'This is not a URL'
export const NO_TOKEN = 'The URL has no token=: paste it whole'

const NAME = 'Name'

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
  return found.searchParams.has('token') ? { ok: true, url } : refused(NO_TOKEN)
}

const hostOf = (raw: string) => {
  const checked = checkUrl(raw)
  return checked.ok ? new URL(checked.url).hostname : ''
}

export const nameFor = (name: string, url: string) => name.trim() || hostOf(url)

export const placeholderName = (url: string) => hostOf(url) || NAME

export const fieldOf = (error: CommandError): Field => (error.code.startsWith('route_') ? 'url' : 'form')
