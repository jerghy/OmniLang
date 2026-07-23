// FLIP animation for @for list reordering
const flipState = new Map();
export function flipBefore(containerSelector) {
    const container = document.querySelector(containerSelector);
    if (!container)
        return;
    container.querySelectorAll('[data-for-key]').forEach(el => {
        const key = el.getAttribute('data-for-key');
        if (key)
            flipState.set(key, { rect: el.getBoundingClientRect(), element: el });
    });
}
export function flipAfter(containerSelector, duration = 400, easing = 'ease-out') {
    const container = document.querySelector(containerSelector);
    if (!container)
        return;
    container.querySelectorAll('[data-for-key]').forEach(el => {
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
        el.addEventListener('transitionend', function handler() {
            ;
            el.style.transition = '';
            el.style.transform = '';
            el.removeEventListener('transitionend', handler);
        });
    });
    flipState.clear();
}
