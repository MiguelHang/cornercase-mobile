import type { KeyName } from './keys'

export const arrowsTo = (open: number | null, target: number): KeyName[] => {
  if (open === null || open === target) {
    return []
  }
  const key: KeyName = target > open ? 'right' : 'left'
  return Array.from({ length: Math.abs(target - open) }, () => key)
}
