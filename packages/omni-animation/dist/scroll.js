/**
 * Compute scroll-driven animation progress (0..1) based on "top 80%" / "center 50%"
 * style trigger configuration strings.
 */
export function scrollProgress(el, startCfg, endCfg) {
    const vh = window.innerHeight;
    const startPos = parseScrollTrigger(startCfg, el, vh);
    const endPos = parseScrollTrigger(endCfg, el, vh);
    const raw = (window.scrollY - startPos) / (endPos - startPos);
    return Math.max(0, Math.min(1, raw));
}
function parseScrollTrigger(cfg, el, vh) {
    const parts = cfg.trim().split(/\s+/);
    if (parts.length < 2)
        return 0;
    const edge = parts[0];
    const offset = parts[1];
    const rect = el.getBoundingClientRect();
    let elPos;
    if (edge === 'top')
        elPos = rect.top;
    else if (edge === 'center')
        elPos = rect.top + rect.height / 2;
    else if (edge === 'bottom')
        elPos = rect.bottom;
    else
        elPos = rect.top;
    const absEdge = window.scrollY + elPos;
    if (offset.endsWith('%'))
        return absEdge - vh * (parseFloat(offset) / 100);
    if (offset === 'bottom')
        return absEdge - vh;
    if (offset === 'top')
        return absEdge;
    return absEdge - vh * 0.5;
}
