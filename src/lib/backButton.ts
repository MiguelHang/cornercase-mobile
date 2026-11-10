import { onBackButtonPress } from '@tauri-apps/api/app'
import type { Platform } from './platform'
import type { Route } from './router.svelte'

export type BackPress = 'none' | 'system' | 'scanner' | 'back'

export const backPressed = (on: Platform, below: Route | null, scanning: boolean): BackPress => {
  if (on !== 'android') return 'none'
  if (scanning) return 'scanner'
  return below === null ? 'system' : 'back'
}

export const handled = (press: BackPress) => press === 'scanner' || press === 'back'

export type Unregistering = { unregister: () => Promise<void> }

export type Listen = (handler: () => void) => Promise<Unregistering>

export type BackWatching = {
  pressed: () => BackPress
  onback: () => void
  onscan: () => void
  failed: (error: unknown) => void
}

const answer = (watching: BackWatching) => {
  const press = watching.pressed()
  if (press === 'back') watching.onback()
  if (press === 'scanner') watching.onscan()
}

export const watchBackButton = (watching: BackWatching, listen: Listen = onBackButtonPress) => {
  let stopped = false
  const listening = listen(() => {
    if (!stopped) answer(watching)
  })
  listening.catch(watching.failed)
  return () => {
    stopped = true
    listening.then((listener) => listener.unregister(), () => undefined).catch(watching.failed)
  }
}
