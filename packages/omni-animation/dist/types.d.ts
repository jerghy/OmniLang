/** Properties that become CSS transform functions. */
export type TransformProp = 'x' | 'y' | 'translateX' | 'translateY' | 'translateZ' | 'scale' | 'scaleX' | 'scaleY' | 'rotate' | 'rotateX' | 'rotateY' | 'rotateZ' | 'skewX' | 'skewY';
/** Properties that become CSS filter functions. */
export type FilterProp = 'blur' | 'brightness' | 'contrast' | 'saturate' | 'hueRotate';
/** All valid Omni animation property names. */
export type OmniProp = TransformProp | FilterProp | 'opacity' | 'width' | 'height' | 'backgroundColor' | 'color' | 'borderColor' | 'borderRadius' | 'margin' | 'padding' | 'boxShadow' | 'marginTop' | 'marginRight' | 'marginBottom' | 'marginLeft' | 'paddingTop' | 'paddingRight' | 'paddingBottom' | 'paddingLeft' | 'borderWidth' | 'top' | 'left' | 'right' | 'bottom' | 'fontSize' | 'letterSpacing' | 'offsetDistance';
/** Easing specification — string name or structured config. */
export type EasingConfig = string | {
    type: 'cubicBezier';
    x1: number;
    y1: number;
    x2: number;
    y2: number;
} | {
    type: 'back';
    direction?: 'in' | 'out' | 'inOut';
} | {
    type: 'elastic';
    direction?: 'in' | 'out' | 'inOut';
};
/** A single keyframe stop in Omni terms. */
export interface KeyframeStop {
    offset: number;
    properties: Partial<Record<OmniProp, number | string>>;
    easing?: EasingConfig;
}
/** A single keyframe in WAAPI terms (CSS properties only). */
export interface Keyframe {
    offset?: number;
    [cssProperty: string]: string | number | undefined;
}
/** Options passed to el.animate(). */
export interface MotionOptions {
    duration: number;
    delay?: number;
    easing?: string;
    fill?: 'none' | 'forwards' | 'backwards' | 'both';
    direction?: 'normal' | 'reverse' | 'alternate' | 'alternate-reverse';
    iterations?: number;
}
/** A single step in a timeline sequence. */
export interface TimelineStep {
    el: Element;
    keyframes: KeyframeStop[];
    options: MotionOptions;
    at: number;
}
