import { buildKeyframes } from './keyframes';
/**
 * Drive a single animation via WAAPI.
 * Returns the native Animation object — caller controls pause/cancel/seek.
 */
export function animate(el, stops, options) {
    const keyframes = buildKeyframes(stops);
    const timing = {
        duration: options.duration,
        delay: options.delay ?? 0,
        easing: options.easing ?? 'ease-out',
        fill: options.fill ?? 'both',
        direction: options.direction ?? 'normal',
        iterations: options.iterations ?? 1,
    };
    return el.animate(keyframes, timing);
}
