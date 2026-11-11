export type Span = Pick<DOMRect, 'top' | 'bottom'>

export const under = (view: Span, header: Span | null): Span => ({
  top: header === null ? view.top : Math.max(view.top, header.bottom),
  bottom: view.bottom,
})

export const revealBy = (view: Span, item: Span): number => {
  if (item.top < view.top) {
    return item.top - view.top
  }
  if (item.bottom > view.bottom) {
    return Math.min(item.bottom - view.bottom, item.top - view.top)
  }
  return 0
}
