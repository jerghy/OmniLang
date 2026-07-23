import type { TimelineStep } from './types';
/**
 * Orchestrate multi-element animations by absolute start time.
 * Returns Animation[] for unified control (pause all, cancel all).
 */
export declare function timeline(steps: TimelineStep[]): globalThis.Animation[];
