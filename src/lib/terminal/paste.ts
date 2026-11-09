import { pasted, type InputEvent } from './keys'

export type Reader = Pick<Clipboard, 'readText'>

export const EMPTY = 'The clipboard holds no text to paste'

export const REFUSED = 'Could not read the clipboard'

const readable = async (clipboard: Reader | undefined) => {
  if (clipboard === undefined) throw new Error(REFUSED)
  return clipboard.readText().catch(() => {
    throw new Error(REFUSED)
  })
}

export const fromClipboard = async (clipboard: Reader | undefined): Promise<InputEvent> => {
  const event = pasted(await readable(clipboard))
  if (event === null) throw new Error(EMPTY)
  return event
}
