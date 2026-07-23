import type { KeyframeStop, MotionOptions } from './types';
/**
 * Drive a single animation via WAAPI.
 * Returns the native Animation object — caller controls pause/cancel/seek.
 */
export declare function animate(el: Element, stops: KeyframeStop[], options: MotionOptions): globalThis.Animation;
