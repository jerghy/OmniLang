// FLIP animation for @for list reordering
const flipState = new Map();
export function flipBefore(container) {
    const containerEl = typeof container === 'string'
        ? document.querySelector(container)
        : container;
    if (!containerEl)
        return;
    containerEl.querySelectorAll('[data-for-key]').forEach(el => {
        const key = el.getAttribute('data-for-key');
        if (key)
            flipState.set(key, { rect: el.getBoundingClientRect(), element: el });
    });
}
export function flipAfter(container, duration = 400, easing = 'ease-out') {
    const containerEl = typeof container === 'string'
        ? document.querySelector(container)
        : container;
    if (!containerEl)
        return;
    containerEl.querySelectorAll('[data-for-key]').forEach(el => {
        const key = el.getAttribute('data-for-key');
        if (!key)
            return;
        const prev = flipState.get(key);
        if (!prev)
            return;
        const newRect = el.getBoundingClientRect();
        const dx = prev.rect.left - newRect.left;
        const dy = prev.rect.top - newRect.top;
        if (dx === 0 && dy === 0)
            return;
        el.style.transform = `translate(${dx}px,${dy}px)`;
        el.style.transition = 'none';
        void el.offsetHeight // force reflow
        ;
        el.style.transition = `transform ${duration}ms ${easing}`;
        el.style.transform = 'translate(0,0)';
        const cleanupStyles = () => {
            ;
            el.style.transition = '';
            el.style.transform = '';
        };
        const fallbackTimer = setTimeout(cleanupStyles, duration + 50);
        el.addEventListener('transitionend', function handler() {
            clearTimeout(fallbackTimer);
            cleanupStyles();
            el.removeEventListener('transitionend', handler);
        });
    });
    flipState.clear();
}
