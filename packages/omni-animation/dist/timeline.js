import { animate } from './animate';
/**
 * Orchestrate multi-element animations by absolute start time.
 * Returns Animation[] for unified control (pause all, cancel all).
 */
export function timeline(steps) {
    const animations = [];
    for (const step of steps) {
        // Skip steps with no keyframes (e.g. motion references that failed to resolve)
        if (!step.keyframes || step.keyframes.length === 0) {
            animations.push(null);
            continue;
        }
        const stepOptions = step.options ?? {};
        const opts = { ...stepOptions, delay: (stepOptions.delay ?? 0) + step.at };
        const anim = animate(step.el, step.keyframes, opts);
        animations.push(anim);
    }
    return animations;
}
