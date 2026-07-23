import type { KeyframeStop, MotionOptions } from './types'
import { buildKeyframes } from './keyframes'
import { resolveEasing } from './easing'

/**
 * Drive a single animation via WAAPI.
 * Returns the native Animation object — caller controls pause/cancel/seek.
 */
export function animate(
  el: Element | null,
  stops: KeyframeStop[],
  options: MotionOptions
): globalThis.Animation | null {
  if (!el) return null
  const keyframes = buildKeyframes(stops)

  const timing: KeyframeAnimationOptions = {
    duration: options.duration,
    delay: options.delay ?? 0,
    easing: resolveEasing(options.easing ?? 'ease-out'),
    fill: options.fill ?? 'both',
    direction: options.direction ?? 'normal',
    iterations: options.iterations ?? 1,
  }

  return el.animate(keyframes as unknown as PropertyIndexedKeyframes, timing)
}
