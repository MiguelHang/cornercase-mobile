export const fromEnd = ({ scrollTop, clientHeight, scrollHeight }: Element) => scrollHeight - scrollTop - clientHeight

export const keepFromEnd = (scrolled: Element, left: number) => {
  scrolled.scrollTo({
    top: scrolled.scrollHeight - scrolled.clientHeight - left,
  })
}
