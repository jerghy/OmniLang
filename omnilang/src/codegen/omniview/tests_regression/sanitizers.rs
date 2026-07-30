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
    let mut gen = OmniViewGenerator::default();
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
    let mut gen = OmniViewGenerator::default();
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

#[test]
fn design_block_css_values_are_sanitized() {
    use crate::codegen::omniview::generate;
    use crate::ir::design::*;
    use crate::ir::*;
    use rustc_hash::FxHashMap;

    // Build IR program with a design block containing a dangerous CSS value
    let design = DesignIR {
        name: "MyDesign".into(),
        tokens: FxHashMap::default(),
        classes: vec![ClassIR {
            name: "DangerCard".into(),
            properties: FxHashMap::from_iter([(
                "background".into(),
                "url(javascript:alert(1))".into(),
            )]),
            pseudo_states: vec![],
            source_span: None,
        }],
        variants: vec![],
        default_theme: None,
        themes: vec![],
        source_span: None,
    };
    let comp = ComponentIR {
        name: "Test".into(),
        source_file: Some("main.omni".into()),
        props: FxHashMap::default(),
        prop_defaults: FxHashMap::default(),
        signals: vec![],
        computeds: vec![],
        effects: vec![],
        lifecycles: vec![],
        fns: vec![],
        store_refs: vec![],
        refs: vec![],
        events: vec![],
        view: None,
        styles: vec![],
        source_span: None,
    };
    let mut module = ModuleIR {
        name: "main".into(),
        functions: FxHashMap::default(),
        structs: FxHashMap::default(),
        enums: FxHashMap::default(),
        type_aliases: FxHashMap::default(),
        components: FxHashMap::default(),
        stores: FxHashMap::default(),
        motions: vec![],
        imports: vec![],
        exports: vec![],
        designs: vec![design],
        source_span: None,
    };
    module.components.insert("Test".into(), comp);
    let ir = IRProgram {
        version: "0.1".into(),
        modules: FxHashMap::from_iter([("main".into(), module)]),
        source_map: FxHashMap::default(),
        stdlib_imports: Default::default(),
        source_span: None,
    };
    let output = generate(&ir, false, false, false);
    // javascript: URL in design-block CSS must be sanitized
    assert!(!output.css.contains("javascript:"));
    assert!(output.css.contains("url(") || output.css.contains("background"));
}

/// Verify both sanitizer functions exist in the emitted runtime.
#[test]
fn test_both_sanitizers_emitted() {
    let mut gen = OmniViewGenerator::default();
    gen.emit_signal_runtime();

    let js = &gen.js;

    assert!(js.contains("function __omni_sanitizeUrl"));
    assert!(js.contains("function __omni_sanitizeCss"));
    // Both functions must be ES5-compatible (no arrow functions, no const/let)
    assert!(!js.contains("const __omni_sanitizeUrl"));
    assert!(!js.contains("let __omni_sanitizeUrl"));
    assert!(!js.contains("=> __omni_sanitizeUrl"));
}
