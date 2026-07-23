import type { TimelineStep } from './types'
import { animate } from './animate'

/**
 * Orchestrate multi-element animations by absolute start time.
 * Returns Animation[] for unified control (pause all, cancel all).
 */
export function timeline(steps: TimelineStep[]): globalThis.Animation[] {
  const animations: globalThis.Animation[] = []

  for (const step of steps) {
    const opts = { ...step.options, delay: (step.options.delay ?? 0) + step.at }
    const anim = animate(step.el, step.keyframes, opts)
    animations.push(anim)
  }

  return animations
}
