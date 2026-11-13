import type { Attachment } from 'svelte/attachments'

export const overPage: Attachment<HTMLElement> = (node) => {
  document.body.append(node)
  return () => node.remove()
}
