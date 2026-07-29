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
        output.push_str(&format!("=== {} ===\n", path));
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
fn dynamic_motion_params_emit_jsx_expressions() {
    let source = r#"
@motion fadeIn {
    from { opacity: 0 }
    to { opacity: 1 }
    over 300
}
component Test {
    signal threshold: f64 = 0.5
    view {
        <div @enter={fadeIn} @inViewThreshold={threshold}>hello</div>
    }
}
"#;
    let output = compile_fixture_to_react(source);
    // The generated TSX must contain data-inview-threshold={threshold}, NOT data-inview-threshold="threshold"
    // The generated TSX must contain {threshold} as a JSX expression, not a string attribute
    assert!(
        output.contains("{threshold}"),
        "Expected JSX expression {{threshold}} in output, got partial:\n{}",
        &output[..output.len().min(500)]
    );
    // Verify the motion engine receives the attrs correctly — threshold should NOT be in a quoted string
    assert!(
        !output.contains("\"{threshold}\"") && !output.contains("\"threshold\""),
        "Must not emit threshold as string attribute"
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
fn test_react_motion_combined() {
    let output = compile_fixture_to_react(fixture!("motion_combined"));
    insta::with_settings!({snapshot_path => "snapshots"}, {
        insta::assert_snapshot!("motion_combined_snapshot", &output);
    });
}
