import type * as opener from '@tauri-apps/plugin-opener'

export type OpenerApi = Pick<typeof opener, 'openUrl'>

export const opensOut = (url: string) => /^https?:\/\/[^\s/?#]+\S*$/i.test(url)

const parsed = (url: string) => {
  try {
    return new URL(url).href
  } catch {
    return null
  }
}

export const follow = async (api: OpenerApi, url: string): Promise<void> => {
  const href = opensOut(url) ? parsed(url) : null
  if (href === null) return
  await api.openUrl(href).catch(() => undefined)
}
