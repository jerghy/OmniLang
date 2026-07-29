/// Canonical named easing to WAAPI-compatible easing string mapping.
///
/// Source of truth: packages/omni-animation/src/easing.ts
///
/// This table is the single shared source for all Rust codegen backends
/// and semantic validation passes. Any new easing must be added here
/// AND in the JS runtime.
pub const NAMED_EASINGS: &[(&str, &str)] = &[
    // CSS standard keywords
    ("linear", "linear"),
    ("ease", "ease"),
    ("easeIn", "ease-in"),
    ("easeOut", "ease-out"),
    ("easeInOut", "ease-in-out"),
    // GSAP power1 (quadratic)
    ("power1In", "cubic-bezier(0.550,0.085,0.680,0.530)"),
    ("power1Out", "cubic-bezier(0.250,0.460,0.450,0.940)"),
    ("power1InOut", "cubic-bezier(0.455,0.030,0.515,0.955)"),
    // GSAP power2 (cubic)
    ("power2In", "cubic-bezier(0.550,0.055,0.675,0.190)"),
    ("power2Out", "cubic-bezier(0.215,0.610,0.355,1.000)"),
    ("power2InOut", "cubic-bezier(0.645,0.045,0.355,1.000)"),
    // GSAP power3 (quartic)
    ("power3In", "cubic-bezier(0.895,0.030,0.685,0.220)"),
    ("power3Out", "cubic-bezier(0.165,0.840,0.440,1.000)"),
    ("power3InOut", "cubic-bezier(0.770,0.000,0.175,1.000)"),
    // GSAP power4 (quintic)
    ("power4In", "cubic-bezier(0.755,0.050,0.855,0.060)"),
    ("power4Out", "cubic-bezier(0.230,1.000,0.320,1.000)"),
    ("power4InOut", "cubic-bezier(0.860,0.000,0.070,1.000)"),
    // Back easings (support overshoot in WAAPI)
    ("backIn", "cubic-bezier(0.600,-0.280,0.735,0.045)"),
    ("backOut", "cubic-bezier(0.175,0.885,0.320,1.275)"),
    ("backInOut", "cubic-bezier(0.680,-0.550,0.265,1.550)"),
    // Bounce → cubic-bezier approximations
    // bounceIn: overshoot at start (bounce toward the peak)
    ("bounceIn", "cubic-bezier(0.215,0.610,0.355,1.000)"),
    // bounceOut: overshoot at end (bounce settling down)
    ("bounceOut", "cubic-bezier(0.175,0.885,0.320,1.275)"),  // backOut-like overshoot at end
    ("bounceInOut", "cubic-bezier(0.645,0.045,0.355,1.000)"),
    // Elastic → cubic-bezier approximations
    ("elasticIn", "cubic-bezier(0.600,0.040,0.980,0.335)"),
    ("elasticOut", "cubic-bezier(0.250,0.460,0.450,0.940)"),
    ("elasticInOut", "cubic-bezier(0.680,-0.550,0.265,1.550)"),
    // Spring → generic bouncy
    ("spring", "cubic-bezier(0.250,0.460,0.450,0.940)"),
    // Steps
    ("stepStart", "steps(1,start)"),
    ("stepEnd", "steps(1,end)"),
];

/// Resolve a named easing to its WAAPI-compatible CSS easing string.
///
/// Returns `None` if the name is not recognized.
pub fn resolve_easing(name: &str) -> Option<&'static str> {
    NAMED_EASINGS
        .iter()
        .find(|(n, _)| *n == name)
        .map(|(_, v)| *v)
}
