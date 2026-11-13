type Options = AddEventListenerOptions

export class Target {
  listeners = new Map<string, { listener: EventListener; options: Options }>()

  addEventListener(name: string, listener: EventListener, options: Options) {
    this.listeners.set(name, { listener, options })
  }

  removeEventListener(name: string, listener: EventListener) {
    if (this.listeners.get(name)?.listener === listener) this.listeners.delete(name)
  }

  fire(name: string, event: object) {
    this.listeners.get(name)?.listener(event as Event)
  }

  options() {
    return Object.fromEntries([...this.listeners].map(([name, { options }]) => [name, options]))
  }
}
