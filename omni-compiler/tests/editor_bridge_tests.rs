//! Tests for OmniView editor bridge code injection.
//!
//! When `editor_mode: true`, the codegen emits postMessage bridge code,
//! nid_map construction, highlight helpers, and component hot-replace.

use omni_compiler::codegen::omniview;
use omni_compiler::ir;
use omni_compiler::lexer;
use omni_compiler::parser;
use omni_compiler::semantic;

/// Helper: compile .omni source to IR, then generate OmniView output with editor_mode.
fn compile_omniview(source: &str, editor_mode: bool) -> omniview::OmniViewOutput {
    let tokens = lexer::tokenize(source);
    let program = parser::parse(&tokens, source).expect("parse failed");
    let semantics = semantic::analyze(&program);
    let ir_program = ir::lower(&program, &semantics);
    omniview::generate(&ir_program, editor_mode)
}

#[test]
fn editor_mode_false_produces_no_bridge_code() {
    let source = r#"
component App {
    view {
        <div id="App.root.0">Hello</div>
    }
}
"#;
    let normal = compile_omniview(source, false);
    let editor = compile_omniview(source, true);

    // Normal mode has no bridge code
    assert!(!normal.js.contains("window.__omni_signals"));
    // Editor mode has bridge code
    assert!(editor.js.contains("window.__omni_signals"));
}

#[test]
fn editor_mode_true_injects_bridge_code() {
    let source = r#"
component App {
    view {
        <div id="App.root.0">Hello</div>
    }
}
"#;
    let result = compile_omniview(source, true);
    let js = &result.js;
    assert!(js.contains("window.__omni_signals"), "missing __omni_signals");
    assert!(js.contains("window.__omni_components"), "missing __omni_components");
    assert!(js.contains("__omni_buildNidMap"), "missing __omni_buildNidMap");
    assert!(js.contains("postMessage"), "missing postMessage");
    assert!(js.contains("omni-select"), "missing omni-select");
    assert!(js.contains("omni-rendered"), "missing omni-rendered");
    assert!(js.contains("omni-replace-component"), "missing omni-replace-component");
}

#[test]
fn editor_mode_true_injects_highlight_helpers() {
    let source = r#"
component App {
    signal count: number = 0;
    view { <div id="App.root.0"></div> }
}
"#;
    let result = compile_omniview(source, true);
    let js = &result.js;
    assert!(js.contains("__omni_editor"), "missing __omni_editor");
    assert!(js.contains(".highlight"), "missing highlight");
    assert!(js.contains(".clearHighlight"), "missing clearHighlight");
}

#[test]
fn editor_bridge_uses_function_keyword() {
    let source = r#"
component App {
    view {
        <div id="App.root.0">Hello</div>
    }
}
"#;
    let result = compile_omniview(source, true);
    let js = &result.js;
    // The bridge section uses function() keyword, not arrow functions
    assert!(js.contains("function __omni_buildNidMap"), "missing function keyword for buildNidMap");
    assert!(js.contains("function __omni_replaceComponent"), "missing function keyword for replaceComponent");
}

#[test]
fn editor_mode_true_injects_replace_component() {
    let source = r#"
component App {
    signal count: number = 0;
    view { <div id="App.root.0"></div> }
}
"#;
    let result = compile_omniview(source, true);
    let js = &result.js;
    // Bridge supports __omni_replaceComponent which is the core of factory pattern
    assert!(js.contains("__omni_replaceComponent"), "missing __omni_replaceComponent");
    assert!(js.contains("teardown"), "missing teardown in replaceComponent");
}
