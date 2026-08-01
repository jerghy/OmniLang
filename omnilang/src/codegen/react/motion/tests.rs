// ═══════════════════════════════════════════════════════════
// Motion snapshot tests for React codegen
// ═══════════════════════════════════════════════════════════
//
// Tests the full pipeline: .omni source → parse → IR → React TSX
// Covers @enter, @hover, @in-view, @scroll, and combined triggers.
//
// Previously zero motion-specific test coverage existed in the
// React backend (ADR-024 documented gap).

use rustc_hash::FxHashMap;
use std::fmt::Write;

/// Compile a single .omni source string to React output, returning
/// all generated files concatenated for snapshot testing.
fn compile_fixture_to_react(source: &str) -> String {
    let mut sources = FxHashMap::default();
    sources.insert("input.omni".to_string(), source.to_string());
    let path_mapping = crate::path_mapping::derive_path_mapping(&sources);
    let result = crate::compile_multi_internal(
        sources,
        crate::codegen::Target::React,
        path_mapping,
        false,
        false,
        None,
    );
    assert!(
        result.success,
        "compilation should succeed, errors: {:?}",
        result.errors
    );
    // Format output files for snapshot
    let mut output = String::new();
    let mut keys: Vec<&String> = result.files.keys().collect();
    keys.sort();
    for path in keys {
        let content = &result.files[path];
        let _ = write!(output, "=== {} ===\n", path);
        output.push_str(content);
        output.push('\n');
    }
    output
}

/// Load fixture content by name from `tests/fixtures/react/`.
/// Uses `include_str!` so the file is embedded at compile time.
macro_rules! fixture {
    ($name:expr) => {
        include_str!(concat!("../../../../tests/fixtures/react/", $name, ".omni"))
    };
}

#[test]
fn inview_trigger_renamed_to_kebab_case() {
    // @inView was renamed to @in-view per semantic-level DSL spec §3.2.
    // The kebab-case form must emit a per-ref IntersectionObserver effect.
    let source = r#"
@motion fadeIn {
    from { opacity: 0 }
    to { opacity: 1 }
    over: 300ms
}
component Test {
    view {
        <div @enter={fadeIn} @in-view={fadeIn}>hello</div>
    }
}
"#;
    let output = compile_fixture_to_react(source);
    assert!(
        output.contains("new IntersectionObserver"),
        "Expected per-ref IntersectionObserver in output, got partial:\n{}",
        &output[..output.len().min(500)]
    );
    assert!(
        output.contains("_M[\"fadeIn\"].kf"),
        "Expected the in-view effect to reference the fadeIn motion, got partial:\n{}",
        &output[..output.len().min(500)]
    );
}

#[test]
fn camelcase_inview_no_longer_emits_trigger() {
    // The removed camelCase form @inView must no longer emit a motion trigger
    // (it now lexes as a plain ident and is treated as an event binding).
    let source = r#"
@motion fadeIn {
    from { opacity: 0 }
    to { opacity: 1 }
    over: 300ms
}
component Test {
    view {
        <div @inView={fadeIn}>hello</div>
    }
}
"#;
    let output = compile_fixture_to_react(source);
    assert!(
        !output.contains("IntersectionObserver") && !output.contains("_M["),
        "camelCase @inView must no longer emit any motion output"
    );
}

#[test]
fn steps_easing_emitted_as_waapi_string() {
    // steps(6, end) must reach the output as the WAAPI-native "steps(6,end)"
    // string — not silently fall back to "ease-out" via the named table.
    let source = r#"
@motion stepFade {
    from { opacity: 0 }
    to { opacity: 1 }
    over: 300ms
    easing: steps(6, end)
}
component Test {
    view {
        <div @enter={stepFade}>hello</div>
    }
}
"#;
    let output = compile_fixture_to_react(source);
    assert!(
        output.contains("easing:\"steps(6,end)\""),
        "Expected steps easing passthrough in output, got partial:\n{}",
        &output[..output.len().min(800)]
    );
    assert!(
        !output.contains("easing:\"ease-out\""),
        "steps easing must not silently fall back to ease-out"
    );
}

#[test]
fn semantic_props_convert_to_waapi_keyframes() {
    // Semantic properties must convert to WAAPI keyframes: opacity passes
    // through, transform-family props compose into a single `transform`.
    let source = r#"
@motion combo {
    from { opacity: 0, translateY: 20, scale: 0.8, rotate: -5 }
    to   { opacity: 1, translateY: 0, scale: 1, rotate: 0 }
    over: 300ms
}
component Test {
    view {
        <div @enter={combo}>hello</div>
    }
}
"#;
    let output = compile_fixture_to_react(source);
    assert!(
        output.contains("kf:[{offset:0,transform:\"translateY(20px) scale(0.8) rotate(-5deg)\",opacity:0},{offset:1,transform:\"translateY(0px) scale(1) rotate(0deg)\",opacity:1}]"),
        "Expected composed WAAPI keyframes, got partial:\n{}",
        &output[..output.len().min(800)]
    );
    assert!(
        !output.contains("properties:"),
        "WAAPI keyframes must be flat (no `properties` wrapper)"
    );
}

#[test]
fn motion_durations_emit_numeric_ms() {
    // P0 regression: WAAPI el.animate() rejects string durations — the
    // semantic DSL parses `over: 300ms` to a string literal, which must be
    // converted back to numeric ms at codegen (`duration:300`, not
    // `duration:"300ms"`). Also covers seconds ("1.5s" → 1500) and bare
    // numbers (>= 1.0 are ms, < 1.0 are seconds per CSS convention).
    let source = r#"
@motion fadeMs {
    from { opacity: 0 }
    to { opacity: 1 }
    over: 300ms
}
@motion fadeS {
    from { opacity: 0 }
    to { opacity: 1 }
    over: 1.5s
}
@motion fadeBare {
    from { opacity: 0 }
    to { opacity: 1 }
    over: 1
}
@motion fadeHalf {
    from { opacity: 0 }
    to { opacity: 1 }
    over: 0.5
}
component Test {
    view {
        <div @enter={fadeMs} @hover={fadeS} @focus={fadeBare} @in-view={fadeHalf}>hello</div>
    }
}
"#;
    let output = compile_fixture_to_react(source);
    for expected in [
        "duration:300,",
        "duration:1500,",
        "duration:1,",
        "duration:500,",
    ] {
        assert!(
            output.contains(expected),
            "expected {} in output, got partial:\n{}",
            expected,
            &output[..output.len().min(1200)]
        );
    }
    assert!(
        !output.contains("duration:\""),
        "duration must never emit as a string"
    );
}

#[test]
fn no_dom_scanning_in_emitted_code() {
    // The whole point of Task 4.4: no querySelectorAll, no MutationObserver,
    // no data-motion-* attributes in generated React code.
    let output = compile_fixture_to_react(fixture!("motion_combined"));
    for banned in [
        "document.querySelectorAll",
        "MutationObserver",
        "data-motion-",
        "@omni/animation",
    ] {
        assert!(
            !output.contains(banned),
            "emitted React code must not contain {:?}",
            banned
        );
    }
}

#[test]
fn focus_trigger_emits_focusin_focusout_listeners() {
    // @focus (TriggerKind::Focus) must emit per-ref focusin/focusout
    // listeners: focus plays the motion, blur plays it reversed.
    let source = r##"
@motion tint {
    from { color: "#111111" }
    to { color: "#888888" }
    over: 200ms
}
component Test {
    view {
        <input @focus={tint} />
    }
}
"##;
    let output = compile_fixture_to_react(source);
    assert!(
        output.contains("el.addEventListener('focusin', onFocus);"),
        "expected focusin listener, got partial:\n{}",
        &output[..output.len().min(800)]
    );
    assert!(
        output.contains("el.addEventListener('focusout', onBlur);"),
        "expected focusout listener, got partial:\n{}",
        &output[..output.len().min(800)]
    );
    assert!(
        output.contains("ref={__el_0Ref}"),
        "expected synthesized ref on the focus element"
    );
    assert!(
        output.contains("useRef<HTMLInputElement>(null)"),
        "expected input-typed internal ref"
    );
}

#[test]
fn internal_refs_generated_for_motion_elements() {
    // Elements with motion triggers but no @ref get synthesized __el_N refs,
    // and @hover wires JSX handlers onto them.
    let output = compile_fixture_to_react(fixture!("motion_hover"));
    assert!(
        output.contains("const __el_0Ref = useRef<HTMLButtonElement>(null);"),
        "expected synthesized button ref, got partial:\n{}",
        &output[..output.len().min(800)]
    );
    assert!(
        output.contains("onMouseEnter={handleEnter_0}"),
        "expected onMouseEnter wiring, got partial:\n{}",
        &output[..output.len().min(800)]
    );
    assert!(
        output.contains("onMouseLeave={handleLeave_0}"),
        "expected onMouseLeave wiring, got partial:\n{}",
        &output[..output.len().min(800)]
    );
}

#[test]
fn test_react_motion_enter() {
    let output = compile_fixture_to_react(fixture!("motion_enter"));
    insta::with_settings!({snapshot_path => "snapshots"}, {
        insta::assert_snapshot!("motion_enter_snapshot", &output);
    });
}

#[test]
fn test_react_motion_hover() {
    let output = compile_fixture_to_react(fixture!("motion_hover"));
    insta::with_settings!({snapshot_path => "snapshots"}, {
        insta::assert_snapshot!("motion_hover_snapshot", &output);
    });
}

#[test]
fn test_react_motion_inview() {
    let output = compile_fixture_to_react(fixture!("motion_inview"));
    insta::with_settings!({snapshot_path => "snapshots"}, {
        insta::assert_snapshot!("motion_inview_snapshot", &output);
    });
}

#[test]
fn test_react_motion_scroll() {
    let output = compile_fixture_to_react(fixture!("motion_scroll"));
    insta::with_settings!({snapshot_path => "snapshots"}, {
        insta::assert_snapshot!("motion_scroll_snapshot", &output);
    });
}

#[test]
fn drag_without_constraints_derives_extent_from_keyframes() {
    // ADR-053 removed @drag-constraints — the drag progress must map onto the
    // motion's own keyframe range (translateX 0→300 ⇒ maxExtent 300) so the
    // element follows its animation instead of dividing by Infinity.
    let source = r#"
@motion dragX300 {
    from { translateX: 0 }
    to   { translateX: 300 }
    over: 300ms
}
component Test {
    view {
        <div @drag-horizontal={dragX300}>drag me</div>
    }
}
"#;
    let output = compile_fixture_to_react(source);
    assert!(
        output.contains("const maxExtent = 300;"),
        "expected keyframe-derived maxExtent, got partial:\n{}",
        &output[..output.len().min(600)]
    );
    assert!(
        !output.contains("Number.POSITIVE_INFINITY"),
        "derived extent must not fall back to the unbounded constant"
    );
}

#[test]
fn drag_vertical_uses_translate_y_range() {
    let source = r#"
@motion dragY100 {
    from { translateY: 0 }
    to   { translateY: 100 }
    over: 300ms
}
component Test {
    view {
        <div @drag-vertical={dragY100}>drag me</div>
    }
}
"#;
    let output = compile_fixture_to_react(source);
    assert!(
        output.contains("const maxExtent = 100;"),
        "expected translateY-derived maxExtent, got partial:\n{}",
        &output[..output.len().min(600)]
    );
}

#[test]
fn drag_without_translate_keyframes_falls_back_to_unbounded() {
    // A motion with no translate range in the drag axis must not emit NaN or
    // a bogus extent — it falls back to the previous unbounded (inert)
    // behavior and carries a compiler warning comment.
    let source = r#"
@motion fadeOnly {
    from { opacity: 0 }
    to { opacity: 1 }
    over: 300ms
}
component Test {
    view {
        <div @drag-vertical={fadeOnly}>drag me</div>
    }
}
"#;
    let output = compile_fixture_to_react(source);
    assert!(
        output.contains("const maxExtent = Number.POSITIVE_INFINITY;"),
        "no-translate motion must fall back to the unbounded extent, got partial:\n{}",
        &output[..output.len().min(600)]
    );
    assert!(
        output.contains("WARNING"),
        "fallback must carry a compiler warning comment"
    );
    assert!(!output.contains("NaN"), "fallback must not emit NaN");
}

#[test]
fn test_react_motion_combined() {
    let output = compile_fixture_to_react(fixture!("motion_combined"));
    insta::with_settings!({snapshot_path => "snapshots"}, {
        insta::assert_snapshot!("motion_combined_snapshot", &output);
    });
}
