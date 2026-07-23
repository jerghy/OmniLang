import type { Keyframe, KeyframeStop, OmniProp } from './types';
/**
 * Convert Omni animation property names + values to CSS property names + formatted values.
 * Transform properties (x, y, scale, rotate, etc.) are combined into one `transform` string.
 * Filter properties (blur, brightness, etc.) are combined into one `filter` string.
 */
export declare function buildProperties(props: Partial<Record<OmniProp, number | string>>): Record<string, string>;
/**
 * Convert Omni KeyframeStop[] to WAAPI Keyframe[].
 * Handles offset → CSS property mapping, transform/filter merging.
 */
export declare function buildKeyframes(stops: KeyframeStop[]): Keyframe[];
