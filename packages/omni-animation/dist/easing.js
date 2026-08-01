/**
 * Named easing → WAAPI cubic-bezier / steps string.
 *
 * Each camelCase entry has a kebab-case alias — the semantic DSL compiler
 * emits canonical kebab names ("power2-out", "ease-in-out") into the
 * OmniView runtime via Omni.resolveEasing(), so both forms must resolve.
 * Mirrors the compiler's NAMED_EASINGS table (crates/omnilang-core/src/
 * codegen/shared/easing.rs).
 */
const NAMED_EASINGS = {
    linear: 'linear',
    ease: 'ease',
    easeIn: 'ease-in',
    easeOut: 'ease-out',
    easeInOut: 'ease-in-out',
    'ease-in': 'ease-in',
    'ease-out': 'ease-out',
    'ease-in-out': 'ease-in-out',
    // power1 (quadratic)
    power1In: 'cubic-bezier(0.550,0.085,0.680,0.530)',
    power1Out: 'cubic-bezier(0.250,0.460,0.450,0.940)',
    power1InOut: 'cubic-bezier(0.455,0.030,0.515,0.955)',
    'power1-in': 'cubic-bezier(0.550,0.085,0.680,0.530)',
    'power1-out': 'cubic-bezier(0.250,0.460,0.450,0.940)',
    'power1-in-out': 'cubic-bezier(0.455,0.030,0.515,0.955)',
    // power2 (cubic)
    power2In: 'cubic-bezier(0.550,0.055,0.675,0.190)',
    power2Out: 'cubic-bezier(0.215,0.610,0.355,1.000)',
    power2InOut: 'cubic-bezier(0.645,0.045,0.355,1.000)',
    'power2-in': 'cubic-bezier(0.550,0.055,0.675,0.190)',
    'power2-out': 'cubic-bezier(0.215,0.610,0.355,1.000)',
    'power2-in-out': 'cubic-bezier(0.645,0.045,0.355,1.000)',
    // power3 (quartic)
    power3In: 'cubic-bezier(0.895,0.030,0.685,0.220)',
    power3Out: 'cubic-bezier(0.165,0.840,0.440,1.000)',
    power3InOut: 'cubic-bezier(0.770,0.000,0.175,1.000)',
    'power3-in': 'cubic-bezier(0.895,0.030,0.685,0.220)',
    'power3-out': 'cubic-bezier(0.165,0.840,0.440,1.000)',
    'power3-in-out': 'cubic-bezier(0.770,0.000,0.175,1.000)',
    // power4 (quintic)
    power4In: 'cubic-bezier(0.755,0.050,0.855,0.060)',
    power4Out: 'cubic-bezier(0.230,1.000,0.320,1.000)',
    power4InOut: 'cubic-bezier(0.860,0.000,0.070,1.000)',
    'power4-in': 'cubic-bezier(0.755,0.050,0.855,0.060)',
    'power4-out': 'cubic-bezier(0.230,1.000,0.320,1.000)',
    'power4-in-out': 'cubic-bezier(0.860,0.000,0.070,1.000)',
    // back
    backIn: 'cubic-bezier(0.600,-0.280,0.735,0.045)',
    backOut: 'cubic-bezier(0.175,0.885,0.320,1.275)',
    backInOut: 'cubic-bezier(0.680,-0.550,0.265,1.550)',
    'back-in': 'cubic-bezier(0.600,-0.280,0.735,0.045)',
    'back-out': 'cubic-bezier(0.175,0.885,0.320,1.275)',
    'back-in-out': 'cubic-bezier(0.680,-0.550,0.265,1.550)',
    // Bounce approximations (WAAPI cannot natively represent bounce curves.
    // These cubic-bezier values approximate the directional asymmetry: bounceIn
    // has overshoot at the start, bounceOut has overshoot at the end.)
    bounceIn: 'cubic-bezier(0.215,0.610,0.355,1.000)',
    bounceOut: 'cubic-bezier(0.175,0.885,0.320,1.275)',
    bounceInOut: 'cubic-bezier(0.645,0.045,0.355,1.000)',
    'bounce-in': 'cubic-bezier(0.215,0.610,0.355,1.000)',
    'bounce-out': 'cubic-bezier(0.175,0.885,0.320,1.275)',
    'bounce-in-out': 'cubic-bezier(0.645,0.045,0.355,1.000)',
    // elastic (approximations)
    elasticIn: 'cubic-bezier(0.600,0.040,0.980,0.335)',
    elasticOut: 'cubic-bezier(0.250,0.460,0.450,0.940)',
    elasticInOut: 'cubic-bezier(0.680,-0.550,0.265,1.550)',
    'elastic-in': 'cubic-bezier(0.600,0.040,0.980,0.335)',
    'elastic-out': 'cubic-bezier(0.250,0.460,0.450,0.940)',
    'elastic-in-out': 'cubic-bezier(0.680,-0.550,0.265,1.550)',
    // spring → generic bouncy
    spring: 'cubic-bezier(0.250,0.460,0.450,0.940)',
    stepStart: 'steps(1,start)',
    stepEnd: 'steps(1,end)',
    'step-start': 'steps(1,start)',
    'step-end': 'steps(1,end)',
};
/**
 * Resolve an Omni EasingConfig to a WAAPI-compatible easing string.
 * Non-WAAPI-native easings (back, elastic, bounce) are
 * approximated as cubic-bezier() since WAAPI does not support custom
 * easing functions.
 */
export function resolveEasing(easing) {
    if (typeof easing === 'string') {
        // Named easings only (no parameterized forms)
        return NAMED_EASINGS[easing] ?? easing;
    }
    switch (easing.type) {
        case 'cubicBezier':
            return `cubic-bezier(${easing.x1},${easing.y1},${easing.x2},${easing.y2})`;
        case 'back': {
            const d = easing.direction ?? 'out';
            if (d === 'in')
                return 'cubic-bezier(0.600,-0.280,0.735,0.045)';
            if (d === 'inOut')
                return 'cubic-bezier(0.680,-0.550,0.265,1.550)';
            return 'cubic-bezier(0.175,0.885,0.320,1.275)';
        }
        case 'elastic': {
            const d = easing.direction ?? 'out';
            if (d === 'in')
                return 'cubic-bezier(0.600,0.040,0.980,0.335)';
            if (d === 'inOut')
                return 'cubic-bezier(0.680,-0.550,0.265,1.550)';
            return 'cubic-bezier(0.250,0.460,0.450,0.940)';
        }
        default:
            return 'ease-out';
    }
}
