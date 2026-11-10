export type Opened = ReadonlyMap<number, { boot: string; panes: ReadonlySet<number> }>

export const none: Opened = new Map()

const NO_PANES: ReadonlySet<number> = new Set()

export const expanded = (opened: Opened, machine: number, boot: string): ReadonlySet<number> => {
  const kept = opened.get(machine)
  return kept?.boot === boot ? kept.panes : NO_PANES
}

export const toggled = (opened: Opened, machine: number, boot: string, pane: number): Opened => {
  const panes = new Set(expanded(opened, machine, boot))
  if (!panes.delete(pane)) {
    panes.add(pane)
  }
  return new Map(opened).set(machine, { boot, panes })
}
