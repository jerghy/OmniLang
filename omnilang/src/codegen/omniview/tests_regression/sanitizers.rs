//! Regression tests for runtime sanitization functions (__omni_sanitizeUrl
//! and __omni_sanitizeCss) embedded in the generated JS runtime.
//!
//! These tests verify that the generated function bodies contain the
//! correct blocking patterns. Full behavioral verification requires
//! a JS test runner; these tests provide compile-time regression
//! protection against accidental changes to the sanitizer logic.

use super::super::*;

/// Verify __omni_sanitizeUrl is emitted and blocks key dangerous schemes.
#[test]
fn test_sanitize_url_blocks_dangerous_schemes() {
    let mut gen = OmniViewGenerator::test_new();
    gen.emit_signal_runtime();

    let js = &gen.js;

    // Function is defined
    assert!(
        js.contains("function __omni_sanitizeUrl"),
        "sanitizeUrl function must be emitted"
    );

    // Uses String() coercion for toString bypass prevention
    assert!(
        js.contains("String(url)"),
        "must coerce input to string to prevent toString() bypass"
    );

    // Blocks javascript: scheme
    assert!(
        js.contains("javascript:") && js.contains("'#blocked'"),
        "must block javascript: URLs"
    );

    // Blocks data: scheme (all data: URIs, not just data:text/html)
    assert!(
        js.contains("data:") && js.contains("=== 0"),
        "must block all data: URI schemes"
    );

    // Blocks vbscript: scheme
    assert!(
        js.contains("vbscript:") && js.contains("'#blocked'"),
        "must block vbscript: URLs"
    );

    // Returns #blocked on match
    assert!(
        js.contains("return '#blocked'"),
        "must return '#blocked' for dangerous URLs"
    );
}

/// Verify __omni_sanitizeCss is emitted and blocks key injection patterns.
#[test]
fn test_sanitize_css_blocks_injection_patterns() {
    let mut gen = OmniViewGenerator::test_new();
    gen.emit_signal_runtime();

    let js = &gen.js;

    // Function is defined
    assert!(
        js.contains("function __omni_sanitizeCss"),
        "sanitizeCss function must be emitted"
    );

    // Uses String() coercion
    assert!(js.contains("String(value)"), "must coerce input to string");

    // Blocks core CSS injection patterns
    assert!(
        js.contains("javascript:") && js.contains("return ''"),
        "must block javascript: in CSS values"
    );
    assert!(
        js.contains("expression("),
        "must block CSS expression() injection"
    );
    assert!(
        js.contains("-moz-binding"),
        "must block -moz-binding injection"
    );
    assert!(js.contains("@import"), "must block @import injection");

    // Allows url() with safety check (Task 6)
    assert!(js.contains("url("), "must contain url() handling logic");
    // After Task 6: should contain url() validation but not blanket block
    assert!(
        js.contains("urlIdx") || js.contains("afterUrl"),
        "must validate url() content rather than blanket-block"
    );
}

/// Verify both sanitizer functions exist in the emitted runtime.
#[test]
fn test_both_sanitizers_emitted() {
    let mut gen = OmniViewGenerator::test_new();
    gen.emit_signal_runtime();

    let js = &gen.js;

    assert!(js.contains("function __omni_sanitizeUrl"));
    assert!(js.contains("function __omni_sanitizeCss"));
    // Both functions must be ES5-compatible (no arrow functions, no const/let)
    assert!(!js.contains("const __omni_sanitizeUrl"));
    assert!(!js.contains("let __omni_sanitizeUrl"));
    assert!(!js.contains("=> __omni_sanitizeUrl"));
}
