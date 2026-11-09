export const latestWins = <A>(run: (arg: A) => Promise<void>) => {
  type Call = { arg: A; done: Promise<void> }
  let latest: Call | null = null
  let queue = Promise.resolve()
  return (arg: A) => {
    if (latest !== null && latest.arg === arg) {
      return latest.done
    }
    const call: Call = { arg, done: queue.then(() => (latest === call ? run(arg) : undefined)) }
    const settle = () => {
      if (latest === call) {
        latest = null
      }
    }
    latest = call
    queue = call.done.then(settle, settle)
    return call.done
  }
}
