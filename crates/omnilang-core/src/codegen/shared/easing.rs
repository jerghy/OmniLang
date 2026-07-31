/// Canonical named easing to WAAPI-compatible easing string and Flutter Curves constant.
///
/// Source of truth: packages/omni-animation/src/easing.ts
///
/// This table is the single shared source for all Rust codegen backends
/// and semantic validation passes. Any new easing must be added here
/// AND in the JS runtime.
///
/// Tuple: (omni_name, css_cubic_bezier, flutter_curves_constant)
pub const NAMED_EASINGS: &[(&str, &str, &str)] = &[
    // CSS standard keywords
    ("linear", "linear", "Curves.linear"),
    ("ease", "ease", "Curves.ease"),
    ("easeIn", "ease-in", "Curves.easeIn"),
    ("easeOut", "ease-out", "Curves.easeOut"),
    ("easeInOut", "ease-in-out", "Curves.easeInOut"),
    // Kebab-case canonical names (semantic DSL). The parser normalizes all
    // easing names to these; camelCase entries above remain for transition.
    ("ease-in", "ease-in", "Curves.easeIn"),
    ("ease-out", "ease-out", "Curves.easeOut"),
    ("ease-in-out", "ease-in-out", "Curves.easeInOut"),
    // GSAP power1 (quadratic)
    (
        "power1In",
        "cubic-bezier(0.550,0.085,0.680,0.530)",
        "Curves.easeInQuad",
    ),
    (
        "power1Out",
        "cubic-bezier(0.250,0.460,0.450,0.940)",
        "Curves.easeOutQuad",
    ),
    (
        "power1InOut",
        "cubic-bezier(0.455,0.030,0.515,0.955)",
        "Curves.easeInOutQuad",
    ),
    // GSAP power2 (cubic)
    (
        "power2In",
        "cubic-bezier(0.550,0.055,0.675,0.190)",
        "Curves.easeInCubic",
    ),
    (
        "power2Out",
        "cubic-bezier(0.215,0.610,0.355,1.000)",
        "Curves.easeOutCubic",
    ),
    (
        "power2InOut",
        "cubic-bezier(0.645,0.045,0.355,1.000)",
        "Curves.easeInOutCubic",
    ),
    // GSAP power3 (quartic)
    (
        "power3In",
        "cubic-bezier(0.895,0.030,0.685,0.220)",
        "Curves.easeInQuart",
    ),
    (
        "power3Out",
        "cubic-bezier(0.165,0.840,0.440,1.000)",
        "Curves.easeOutQuart",
    ),
    (
        "power3InOut",
        "cubic-bezier(0.770,0.000,0.175,1.000)",
        "Curves.easeInOutQuart",
    ),
    // GSAP power4 (quintic)
    (
        "power4In",
        "cubic-bezier(0.755,0.050,0.855,0.060)",
        "Curves.easeInQuint",
    ),
    (
        "power4Out",
        "cubic-bezier(0.230,1.000,0.320,1.000)",
        "Curves.easeOutQuint",
    ),
    (
        "power4InOut",
        "cubic-bezier(0.860,0.000,0.070,1.000)",
        "Curves.easeInOutQuint",
    ),
    // Kebab-case canonical names (power family)
    (
        "power1-in",
        "cubic-bezier(0.550,0.085,0.680,0.530)",
        "Curves.easeInQuad",
    ),
    (
        "power1-out",
        "cubic-bezier(0.250,0.460,0.450,0.940)",
        "Curves.easeOutQuad",
    ),
    (
        "power1-in-out",
        "cubic-bezier(0.455,0.030,0.515,0.955)",
        "Curves.easeInOutQuad",
    ),
    (
        "power2-in",
        "cubic-bezier(0.550,0.055,0.675,0.190)",
        "Curves.easeInCubic",
    ),
    (
        "power2-out",
        "cubic-bezier(0.215,0.610,0.355,1.000)",
        "Curves.easeOutCubic",
    ),
    (
        "power2-in-out",
        "cubic-bezier(0.645,0.045,0.355,1.000)",
        "Curves.easeInOutCubic",
    ),
    (
        "power3-in",
        "cubic-bezier(0.895,0.030,0.685,0.220)",
        "Curves.easeInQuart",
    ),
    (
        "power3-out",
        "cubic-bezier(0.165,0.840,0.440,1.000)",
        "Curves.easeOutQuart",
    ),
    (
        "power3-in-out",
        "cubic-bezier(0.770,0.000,0.175,1.000)",
        "Curves.easeInOutQuart",
    ),
    (
        "power4-in",
        "cubic-bezier(0.755,0.050,0.855,0.060)",
        "Curves.easeInQuint",
    ),
    (
        "power4-out",
        "cubic-bezier(0.230,1.000,0.320,1.000)",
        "Curves.easeOutQuint",
    ),
    (
        "power4-in-out",
        "cubic-bezier(0.860,0.000,0.070,1.000)",
        "Curves.easeInOutQuint",
    ),
    // Back easings (support overshoot in WAAPI)
    (
        "backIn",
        "cubic-bezier(0.600,-0.280,0.735,0.045)",
        "Curves.easeInBack",
    ),
    (
        "backOut",
        "cubic-bezier(0.175,0.885,0.320,1.275)",
        "Curves.easeOutBack",
    ),
    (
        "backInOut",
        "cubic-bezier(0.680,-0.550,0.265,1.550)",
        "Curves.easeInOutBack",
    ),
    // Bounce → cubic-bezier approximations
    // bounceIn: overshoot at start (bounce toward the peak)
    (
        "bounceIn",
        "cubic-bezier(0.215,0.610,0.355,1.000)",
        "Curves.bounceIn",
    ),
    // bounceOut: overshoot at end (bounce settling down)
    (
        "bounceOut",
        "cubic-bezier(0.175,0.885,0.320,1.275)",
        "Curves.bounceOut",
    ), // backOut-like overshoot at end
    (
        "bounceInOut",
        "cubic-bezier(0.645,0.045,0.355,1.000)",
        "Curves.bounceInOut",
    ),
    // Elastic → cubic-bezier approximations
    (
        "elasticIn",
        "cubic-bezier(0.600,0.040,0.980,0.335)",
        "Curves.easeInElastic",
    ),
    (
        "elasticOut",
        "cubic-bezier(0.250,0.460,0.450,0.940)",
        "Curves.easeOutElastic",
    ),
    (
        "elasticInOut",
        "cubic-bezier(0.680,-0.550,0.265,1.550)",
        "Curves.easeInOutElastic",
    ),
    // Spring → generic bouncy (no dedicated Flutter Curves constant)
    (
        "spring",
        "cubic-bezier(0.250,0.460,0.450,0.940)",
        "Curves.easeOutQuad",
    ),
    // Steps (discrete — no Flutter Curves equivalent)
    ("stepStart", "steps(1,start)", "Curves.linear"),
    ("stepEnd", "steps(1,end)", "Curves.linear"),
    // Kebab-case canonical names (back/bounce/elastic/steps family)
    (
        "back-in",
        "cubic-bezier(0.600,-0.280,0.735,0.045)",
        "Curves.easeInBack",
    ),
    (
        "back-out",
        "cubic-bezier(0.175,0.885,0.320,1.275)",
        "Curves.easeOutBack",
    ),
    (
        "back-in-out",
        "cubic-bezier(0.680,-0.550,0.265,1.550)",
        "Curves.easeInOutBack",
    ),
    (
        "bounce-in",
        "cubic-bezier(0.215,0.610,0.355,1.000)",
        "Curves.bounceIn",
    ),
    (
        "bounce-out",
        "cubic-bezier(0.175,0.885,0.320,1.275)",
        "Curves.bounceOut",
    ),
    (
        "bounce-in-out",
        "cubic-bezier(0.645,0.045,0.355,1.000)",
        "Curves.bounceInOut",
    ),
    (
        "elastic-in",
        "cubic-bezier(0.600,0.040,0.980,0.335)",
        "Curves.easeInElastic",
    ),
    (
        "elastic-out",
        "cubic-bezier(0.250,0.460,0.450,0.940)",
        "Curves.easeOutElastic",
    ),
    (
        "elastic-in-out",
        "cubic-bezier(0.680,-0.550,0.265,1.550)",
        "Curves.easeInOutElastic",
    ),
    ("step-start", "steps(1,start)", "Curves.linear"),
    ("step-end", "steps(1,end)", "Curves.linear"),
];

/// Resolve a named easing to its WAAPI-compatible CSS easing string.
///
/// Returns `None` if the name is not recognized.
pub fn resolve_easing(name: &str) -> Option<&'static str> {
    NAMED_EASINGS
        .iter()
        .find(|(n, _, _)| *n == name)
        .map(|(_, css, _)| *css)
}

/// Resolve a named easing to its Flutter Curves constant string.
///
/// Returns `None` if the name is not recognized.
pub fn resolve_easing_flutter(name: &str) -> Option<&'static str> {
    NAMED_EASINGS
        .iter()
        .find(|(n, _, _)| *n == name)
        .map(|(_, _, flutter)| *flutter)
}
