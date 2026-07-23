(function(){
"use strict";

// types.js
// ═══ Motion properties (Omni naming conventions) ═══
// These are the property names used in .omni @motion blocks.
// The library maps them to CSS properties.
{};

// easing.js
/** Named easing → WAAPI cubic-bezier / steps string. */
const NAMED_EASINGS = {
    linear: 'linear',
    ease: 'ease',
    easeIn: 'ease-in',
    easeOut: 'ease-out',
    easeInOut: 'ease-in-out',
    // power1 (quadratic)
    power1In: 'cubic-bezier(0.550,0.085,0.680,0.530)',
    power1Out: 'cubic-bezier(0.250,0.460,0.450,0.940)',
    power1InOut: 'cubic-bezier(0.455,0.030,0.515,0.955)',
    // power2 (cubic)
    power2In: 'cubic-bezier(0.550,0.055,0.675,0.190)',
    power2Out: 'cubic-bezier(0.215,0.610,0.355,1.000)',
    power2InOut: 'cubic-bezier(0.645,0.045,0.355,1.000)',
    // power3 (quartic)
    power3In: 'cubic-bezier(0.895,0.030,0.685,0.220)',
    power3Out: 'cubic-bezier(0.165,0.840,0.440,1.000)',
    power3InOut: 'cubic-bezier(0.770,0.000,0.175,1.000)',
    // power4 (quintic)
    power4In: 'cubic-bezier(0.755,0.050,0.855,0.060)',
    power4Out: 'cubic-bezier(0.230,1.000,0.320,1.000)',
    power4InOut: 'cubic-bezier(0.860,0.000,0.070,1.000)',
    // back
    backIn: 'cubic-bezier(0.600,-0.280,0.735,0.045)',
    backOut: 'cubic-bezier(0.175,0.885,0.320,1.275)',
    backInOut: 'cubic-bezier(0.680,-0.550,0.265,1.550)',
    // bounce (approximations)
    bounceIn: 'cubic-bezier(0.215,0.610,0.355,1.000)',
    bounceOut: 'cubic-bezier(0.215,0.610,0.355,1.000)',
    bounceInOut: 'cubic-bezier(0.645,0.045,0.355,1.000)',
    // elastic (approximations)
    elasticIn: 'cubic-bezier(0.600,0.040,0.980,0.335)',
    elasticOut: 'cubic-bezier(0.250,0.460,0.450,0.940)',
    elasticInOut: 'cubic-bezier(0.680,-0.550,0.265,1.550)',
    // spring → generic bouncy
    spring: 'cubic-bezier(0.250,0.460,0.450,0.940)',
    stepStart: 'steps(1,start)',
    stepEnd: 'steps(1,end)',
};
/**
 * Resolve an Omni EasingConfig to a WAAPI-compatible easing string.
 * All non-WAAPI-native easings (spring, back, elastic, bounce) are
 * approximated as cubic-bezier() since WAAPI does not support custom
 * easing functions.
 */
function resolveEasing(easing) {
    if (typeof easing === 'string') {
        // Check for parameterized form: backIn(1.5), elasticOut(1,0.5), etc.
        return NAMED_EASINGS[easing] ?? resolveParameterized(easing) ?? 'ease-out';
    }
    switch (easing.type) {
        case 'cubicBezier':
            return `cubic-bezier(${easing.x1},${easing.y1},${easing.x2},${easing.y2})`;
        case 'spring': {
            const t = easing.tension ?? 180;
            const f = easing.friction ?? 12;
            const ratio = t > 0 ? f / t : 1.0;
            if (ratio < 0.3) {
                const overshoot = Math.min(0.3, 1.0 - ratio * 2.0);
                return `cubic-bezier(0.250,${(0.460 + overshoot).toFixed(3)},0.450,${(0.940 + overshoot).toFixed(3)})`;
            }
            return 'cubic-bezier(0.250,0.460,0.450,0.940)';
        }
        case 'back': {
            const i = Math.min(1.5, Math.max(0.5, easing.intensity ?? 1.7));
            const d = easing.direction ?? 'out';
            if (d === 'in')
                return `cubic-bezier(0.600,${(-0.28 * i / 1.7).toFixed(3)},0.735,0.045)`;
            if (d === 'inOut')
                return `cubic-bezier(0.680,${(-0.55 * i / 1.7).toFixed(3)},0.265,${Math.min(1.8, 1.55 * i / 1.7).toFixed(3)})`;
            return `cubic-bezier(0.175,${(0.885 * i / 1.7).toFixed(3)},0.320,${Math.min(2.0, 1.275 * i / 1.7).toFixed(3)})`;
        }
        case 'elastic': {
            const d = easing.direction ?? 'out';
            if (d === 'in')
                return 'cubic-bezier(0.600,0.040,0.980,0.335)';
            if (d === 'inOut')
                return 'cubic-bezier(0.680,-0.550,0.265,1.550)';
            return 'cubic-bezier(0.250,0.460,0.450,0.940)';
        }
        case 'steps':
            return `steps(${easing.count},${easing.direction})`;
        default:
            return 'ease-out';
    }
}
/** Handle parameterized string easings like "backIn(1.5)" or "elasticOut(1,0.5)". */
function resolveParameterized(easing) {
    const match = easing.match(/^(backIn|backOut|backInOut)\(([\d.]+)\)$/);
    if (match) {
        return resolveEasing({ type: 'back', intensity: parseFloat(match[2]), direction: match[1].replace('back', '').toLowerCase() });
    }
    const elasticMatch = easing.match(/^(elasticIn|elasticOut|elasticInOut)\(([\d.]+),([\d.]+)\)$/);
    if (elasticMatch) {
        return resolveEasing({ type: 'elastic', amplitude: parseFloat(elasticMatch[2]), period: parseFloat(elasticMatch[3]), direction: elasticMatch[1].replace('elastic', '').toLowerCase() });
    }
    const springMatch = easing.match(/^spring\(([\d.]+),([\d.]+)(?:,([\d.]+))?\)$/);
    if (springMatch) {
        return resolveEasing({ type: 'spring', tension: parseFloat(springMatch[1]), friction: parseFloat(springMatch[2]), velocity: springMatch[3] ? parseFloat(springMatch[3]) : undefined });
    }
    return null;
}

// keyframes.js
/** Properties that must be mapped to CSS transform functions. */
const TRANSFORM_MAP = {
    x: 'translateX', y: 'translateY',
    scale: 'scale', scaleX: 'scaleX', scaleY: 'scaleY',
    rotate: 'rotate', rotateX: 'rotateX', rotateY: 'rotateY', rotateZ: 'rotateZ',
    skewX: 'skewX', skewY: 'skewY',
};
const TRANSFORM_PROPS = new Set(Object.keys(TRANSFORM_MAP));
/** Properties that need a "px" suffix when the value is a number. */
const PX_PROPS = new Set([
    'width', 'height', 'margin', 'padding', 'borderRadius', 'borderWidth',
    'marginTop', 'marginRight', 'marginBottom', 'marginLeft',
    'paddingTop', 'paddingRight', 'paddingBottom', 'paddingLeft',
    'borderTopLeftRadius', 'borderTopRightRadius',
    'borderBottomLeftRadius', 'borderBottomRightRadius',
    'gap', 'rowGap', 'columnGap', 'maxWidth', 'minWidth', 'maxHeight', 'minHeight',
    'top', 'left', 'right', 'bottom', 'fontSize', 'letterSpacing',
]);
/** Properties that map to CSS filter functions. */
const FILTER_MAP = {
    blur: 'blur', brightness: 'brightness',
    contrast: 'contrast', saturate: 'saturate', hueRotate: 'hue-rotate',
};
/**
 * Convert Omni animation property names + values to CSS property names + formatted values.
 * Transform properties (x, y, scale, rotate, etc.) are combined into one `transform` string.
 * Filter properties (blur, brightness, etc.) are combined into one `filter` string.
 */
function buildProperties(props) {
    const cssProps = {};
    const transforms = [];
    const filters = [];
    if (!props)
        return cssProps;
    for (const [key, value] of Object.entries(props)) {
        if (TRANSFORM_PROPS.has(key)) {
            const cssName = TRANSFORM_MAP[key];
            const num = typeof value === 'number' ? value : parseFloat(value);
            if (key === 'x' || key === 'y') {
                transforms.push(`${cssName}(${num}px)`);
            }
            else if (key.startsWith('rotate') || key.startsWith('skew')) {
                transforms.push(`${cssName}(${num}deg)`);
            }
            else {
                transforms.push(`${cssName}(${num})`);
            }
        }
        else if (key in FILTER_MAP) {
            const cssName = FILTER_MAP[key];
            const num = typeof value === 'number' ? value : parseFloat(value);
            if (key === 'blur')
                filters.push(`${cssName}(${num}px)`);
            else
                filters.push(`${cssName}(${num})`);
        }
        else {
            // Standard CSS property
            let formatted = value;
            if (typeof value === 'number' && PX_PROPS.has(key)) {
                formatted = `${value}px`;
            }
            cssProps[key] = String(formatted);
        }
    }
    if (transforms.length)
        cssProps.transform = transforms.join(' ');
    if (filters.length)
        cssProps.filter = filters.join(' ');
    return cssProps;
}
/**
 * Convert Omni KeyframeStop[] to WAAPI Keyframe[].
 * Handles offset → CSS property mapping, transform/filter merging.
 */
function buildKeyframes(stops) {
    const sorted = [...stops].sort((a, b) => a.offset - b.offset);
    return sorted.map(stop => {
        const kf = { offset: stop.offset };
        const cssProps = buildProperties(stop.properties);
        Object.assign(kf, cssProps);
        return kf;
    });
}

// animate.js


/**
 * Drive a single animation via WAAPI.
 * Returns the native Animation object — caller controls pause/cancel/seek.
 */
function animate(el, stops, options) {
    if (!el)
        return null;
    const keyframes = buildKeyframes(stops);
    const timing = {
        duration: options.duration,
        delay: options.delay ?? 0,
        easing: resolveEasing(options.easing ?? 'ease-out'),
        fill: options.fill ?? 'both',
        direction: options.direction ?? 'normal',
        iterations: options.iterations ?? 1,
    };
    return el.animate(keyframes, timing);
}

// timeline.js

/**
 * Orchestrate multi-element animations by absolute start time.
 * Returns Animation[] for unified control (pause all, cancel all).
 */
function timeline(steps) {
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

// stagger.js
/**
 * Compute stagger delay array for N items.
 * Direction controls the order: 'start' (first→last), 'end' (last→first),
 * 'center' (middle→outward), 'edges' (edges→inward).
 */
function staggerDelays(count, each, from = 'start') {
    const indices = [];
    if (from === 'start') {
        for (let i = 0; i < count; i++)
            indices.push(i);
    }
    else if (from === 'end') {
        for (let i = count - 1; i >= 0; i--)
            indices.push(i);
    }
    else if (from === 'edges') {
        let l = 0, r = count - 1;
        while (l <= r) {
            indices.push(l++);
            if (l <= r)
                indices.push(r--);
        }
    }
    else { // center
        const mid = Math.floor((count - 1) / 2);
        indices.push(mid);
        let l = mid - 1, r = mid + 1;
        while (l >= 0 || r < count) {
            if (l >= 0)
                indices.push(l--);
            if (r < count)
                indices.push(r++);
        }
    }
    const delays = new Array(count).fill(0);
    indices.forEach((origIdx, pos) => { delays[origIdx] = pos * each; });
    return delays;
}

// scroll.js
/**
 * Compute scroll-driven animation progress (0..1) based on "top 80%" / "center 50%"
 * style trigger configuration strings.
 */
function scrollProgress(el, startCfg, endCfg) {
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

// flip.js
// FLIP animation for @for list reordering
const flipState = new Map();
function flipBefore(container) {
    const containerEl = typeof container === 'string'
        ? document.querySelector(container)
        : container;
    if (!containerEl)
        return;
    containerEl.querySelectorAll('[data-for-key]').forEach(el => {
        const key = el.getAttribute('data-for-key');
        if (key)
            flipState.set(key, { rect: el.getBoundingClientRect(), element: el });
    });
}
function flipAfter(container, duration = 400, easing = 'ease-out') {
    const containerEl = typeof container === 'string'
        ? document.querySelector(container)
        : container;
    if (!containerEl)
        return;
    containerEl.querySelectorAll('[data-for-key]').forEach(el => {
        const key = el.getAttribute('data-for-key');
        if (!key)
            return;
        const prev = flipState.get(key);
        if (!prev)
            return;
        const newRect = el.getBoundingClientRect();
        const dx = prev.rect.left - newRect.left;
        const dy = prev.rect.top - newRect.top;
        if (dx === 0 && dy === 0)
            return;
        el.style.transform = `translate(${dx}px,${dy}px)`;
        el.style.transition = 'none';
        void el.offsetHeight // force reflow
        ;
        el.style.transition = `transform ${duration}ms ${easing}`;
        el.style.transform = 'translate(0,0)';
        el.addEventListener('transitionend', function handler() {
            ;
            el.style.transition = '';
            el.style.transform = '';
            el.removeEventListener('transitionend', handler);
        });
    });
    flipState.clear();
}

// hero.js
// @hero shared element transition
const heroState = new Map();
function heroBefore() {
    document.querySelectorAll('[data-hero]').forEach(el => {
        const tag = el.getAttribute('data-hero');
        if (!tag)
            return;
        const isVisible = el.offsetParent !== null
            || el.getBoundingClientRect().width > 0;
        if (!isVisible)
            return;
        if (!heroState.has(tag)) {
            heroState.set(tag, { rect: el.getBoundingClientRect(), element: el });
        }
    });
}
function heroAfter() {
    heroState.forEach((prev, tag) => {
        const visible = Array.from(document.querySelectorAll(`[data-hero="${tag}"]`)).find(el => el.offsetParent !== null
            || el.getBoundingClientRect().width > 0) ?? null;
        if (!visible || visible === prev.element)
            return;
        const newRect = visible.getBoundingClientRect();
        const dx = prev.rect.left - newRect.left;
        const dy = prev.rect.top - newRect.top;
        const dw = prev.rect.width / (newRect.width || 1);
        const dh = prev.rect.height / (newRect.height || 1);
        if (dx === 0 && dy === 0 && dw === 1 && dh === 1)
            return;
        const dur = 400;
        const ease = 'ease-out';
        visible.style.transformOrigin = '0 0';
        visible.style.transition = 'none';
        visible.style.transform = `translate(${dx}px,${dy}px) scale(${dw},${dh})`;
        void visible.offsetHeight;
        const kfs = [
            { transform: `translate(${dx}px,${dy}px) scale(${dw},${dh})`, offset: 0 },
            { transform: 'translate(0,0) scale(1,1)', offset: 1 }
        ];
        const anim = visible.animate(kfs, {
            duration: dur, easing: ease, fill: 'both'
        });
        anim.onfinish = () => {
            ;
            visible.style.transition = '';
            visible.style.transform = '';
            visible.style.transformOrigin = '';
        };
    });
    heroState.clear();
}


const Omni = { animate, timeline, buildKeyframes, buildProperties, resolveEasing, staggerDelays, scrollProgress, flipBefore, flipAfter, heroBefore, heroAfter };
window.Omni = Omni;
})();
