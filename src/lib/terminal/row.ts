import type { InputEvent, RowKey, StickyCtrl } from './keys'
import { fromClipboard, type Reader } from './paste'

export type Pressing = {
  ctrl: StickyCtrl
  disabled: boolean
  send: (events: InputEvent[]) => void
  failed: (error: unknown) => void
  clipboard: Reader | undefined
}

export const press = async (key: RowKey, { ctrl, disabled, send, failed, clipboard }: Pressing) => {
  if (disabled) return
  if (key.kind === 'ctrl') ctrl.toggle()
  else if (key.kind === 'key') send(ctrl.apply([key.event]))
  else await fromClipboard(clipboard).then((event) => send(ctrl.apply([event])), failed)
}

export const byKeyboard = ({ detail }: Pick<MouseEvent, 'detail'>) => detail === 0
