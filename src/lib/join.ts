export const oneAtATime = <A>(run: (arg: A) => Promise<void>) => {
  let pending: Promise<void> | null = null
  return (arg: A) => {
    pending ??= run(arg).finally(() => (pending = null))
    return pending
  }
}
