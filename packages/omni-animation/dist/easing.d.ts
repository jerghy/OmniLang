import type { EasingConfig } from './types';
/**
 * Resolve an Omni EasingConfig to a WAAPI-compatible easing string.
 * All non-WAAPI-native easings (spring, back, elastic, bounce) are
 * approximated as cubic-bezier() since WAAPI does not support custom
 * easing functions.
 */
export declare function resolveEasing(easing: EasingConfig): string;
