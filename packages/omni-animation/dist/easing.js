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
export function resolveEasing(easing) {
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
