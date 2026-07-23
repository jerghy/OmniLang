// FLIP animation for @for list reordering
const flipState = new Map<string, { rect: DOMRect; element: Element }>()

export function flipBefore(container: Element | string): void {
  const containerEl = typeof container === 'string'
    ? document.querySelector(container)
    : container;
  if (!containerEl) return
  containerEl.querySelectorAll('[data-for-key]').forEach(el => {
    const key = el.getAttribute('data-for-key')
    if (key) flipState.set(key, { rect: el.getBoundingClientRect(), element: el })
  })
}

export function flipAfter(
  container: Element | string,
  duration: number = 400,
  easing: string = 'ease-out'
): void {
  const containerEl = typeof container === 'string'
    ? document.querySelector(container)
    : container;
  if (!containerEl) return
  containerEl.querySelectorAll('[data-for-key]').forEach(el => {
    const key = el.getAttribute('data-for-key')
    if (!key) return
    const prev = flipState.get(key)
    if (!prev) return
    const newRect = el.getBoundingClientRect()
    const dx = prev.rect.left - newRect.left
    const dy = prev.rect.top - newRect.top
    if (dx === 0 && dy === 0) return
    ;(el as HTMLElement).style.transform = `translate(${dx}px,${dy}px)`
    ;(el as HTMLElement).style.transition = 'none'
    void (el as HTMLElement).offsetHeight // force reflow
    ;(el as HTMLElement).style.transition = `transform ${duration}ms ${easing}`
    ;(el as HTMLElement).style.transform = 'translate(0,0)'
    el.addEventListener('transitionend', function handler() {
      ;(el as HTMLElement).style.transition = ''
      ;(el as HTMLElement).style.transform = ''
      el.removeEventListener('transitionend', handler)
    })
  })
  flipState.clear()
}
