// @hero shared element transition
const heroState = new Map<string, { rect: DOMRect; element: Element }>()

export function heroBefore(): void {
  document.querySelectorAll('[data-hero]').forEach(el => {
    const tag = el.getAttribute('data-hero')
    if (!tag) return
    const isVisible = (el as HTMLElement).offsetParent !== null
      || el.getBoundingClientRect().width > 0
    if (!isVisible) return
    if (!heroState.has(tag)) {
      heroState.set(tag, { rect: el.getBoundingClientRect(), element: el })
    }
  })
}

export function heroAfter(): void {
  heroState.forEach((prev, tag) => {
    const visible = Array.from(
      document.querySelectorAll(`[data-hero="${tag}"]`)
    ).find(el =>
      (el as HTMLElement).offsetParent !== null
      || el.getBoundingClientRect().width > 0
    ) ?? null
    if (!visible || visible === prev.element) return

    const newRect = visible.getBoundingClientRect()
    const dx = prev.rect.left - newRect.left
    const dy = prev.rect.top - newRect.top
    const dw = prev.rect.width / (newRect.width || 1)
    const dh = prev.rect.height / (newRect.height || 1)
    if (dx === 0 && dy === 0 && dw === 1 && dh === 1) return

    const dur = 400
    const ease = 'ease-out'
    ;(visible as HTMLElement).style.transformOrigin = '0 0'
    ;(visible as HTMLElement).style.transition = 'none'
    ;(visible as HTMLElement).style.transform = `translate(${dx}px,${dy}px) scale(${dw},${dh})`
    void (visible as HTMLElement).offsetHeight

    const kfs = [
      { transform: `translate(${dx}px,${dy}px) scale(${dw},${dh})`, offset: 0 },
      { transform: 'translate(0,0) scale(1,1)', offset: 1 }
    ]
    const anim = (visible as HTMLElement).animate(kfs, {
      duration: dur, easing: ease, fill: 'both'
    })
    const cleanupStyles = () => {
      ;(visible as HTMLElement).style.transition = ''
      ;(visible as HTMLElement).style.transform = ''
      ;(visible as HTMLElement).style.transformOrigin = ''
    }
    const fallbackTimer = setTimeout(cleanupStyles, dur + 50)
    anim.onfinish = () => {
      clearTimeout(fallbackTimer)
      cleanupStyles()
    }
  })
  heroState.clear()
}
