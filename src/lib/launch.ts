import type { Screen } from './connection'
import { leave } from './ui/motion'

export const LAUNCH_TIMEOUT_MS = 3000

export const LEAVE_EASING = 'cubic-bezier(0.11, 0, 0.5, 0)'

export type After = (ms: number, then: () => void) => () => void

export type Layer = {
  style: { pointerEvents: string }
  animate: (keyframes: Keyframe[], options: KeyframeAnimationOptions) => { finished: Promise<unknown> }
  remove: () => void
}

const afterTimeout: After = (ms, then) => {
  const timer = setTimeout(then, ms)
  return () => clearTimeout(timer)
}

export const ready = (listed: boolean, current: Screen) => {
  if (!listed || current.machine === null) {
    return listed
  }
  switch (current.connection.state) {
    case 'connecting':
      return false
    case 'connected':
      return current.agents !== null
    default:
      return true
  }
}

export const dismiss = async (layer: Layer | null, reduced: boolean) => {
  if (layer === null) {
    return
  }
  layer.style.pointerEvents = 'none'
  const { duration } = leave(reduced)
  if (duration === 0) {
    layer.remove()
    return
  }
  const fading = layer.animate([{ opacity: 1 }, { opacity: 0 }], { duration, easing: LEAVE_EASING, fill: 'forwards' })
  const remove = () => layer.remove()
  await fading.finished.then(remove, remove)
}

export const exit = (go: () => void, after: After = afterTimeout) => {
  let gone = false
  let cancel = () => {}
  const now = () => {
    if (gone) {
      return
    }
    gone = true
    cancel()
    go()
  }
  cancel = after(LAUNCH_TIMEOUT_MS, now)
  return {
    when: (isReady: boolean) => {
      if (isReady) {
        now()
      }
    },
    stop: () => cancel(),
  }
}
