export type Platform = 'ios' | 'android'

export const platformOf = (userAgent: string): Platform => (/android/i.test(userAgent) ? 'android' : 'ios')

export const platformFrom = (agent?: Pick<Navigator, 'userAgent'>): Platform => platformOf(agent?.userAgent ?? '')

export const platform = platformFrom(globalThis.navigator)
