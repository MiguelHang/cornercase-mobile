import type { ConnectionView_Serialize } from './bindings'

const TICK_MS = 1000

export const retryClock = (connection: () => ConnectionView_Serialize) => {
  let elapsed = $state(0)

  $effect(() => {
    elapsed = 0
    if (connection().state !== 'retrying') {
      return
    }
    const since = Date.now()
    const timer = setInterval(() => (elapsed = Date.now() - since), TICK_MS)
    return () => clearInterval(timer)
  })

  return {
    get elapsed() {
      return elapsed
    },
  }
}
