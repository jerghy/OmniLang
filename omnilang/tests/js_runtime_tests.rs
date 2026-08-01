//! Integration tests for JS runtime functions (sanitizers, signals).
//!
//! These tests invoke the exact JS runtime functions emitted by `runtime.rs`
//! via Node.js. If Node.js is not available (e.g., in minimal CI images),
//! the tests gracefully skip with a diagnostic message.
//!
//! JS test files live in `tests/js_runtime/` alongside this file.

use std::io::Write;
use std::process::{Command, Stdio};

use rustc_hash::FxHashMap;

#[test]
fn js_runtime_sanitizers_pass() {
    // Skip if Node.js is not available (e.g., in minimal CI images)
    if Command::new("node").arg("--version").output().is_err() {
        eprintln!("SKIP: Node.js not found -- skipping JS runtime tests");
        return;
    }

    let output = Command::new("node")
        .arg(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/js_runtime/test_sanitizers.js"
        ))
        .output()
        .expect("failed to execute node");

    assert!(
        output.status.success(),
        "JS sanitizer tests failed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn js_runtime_signals_pass() {
    if Command::new("node").arg("--version").output().is_err() {
        eprintln!("SKIP: Node.js not found -- skipping JS runtime tests");
        return;
    }

    let output = Command::new("node")
        .arg(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/js_runtime/test_signals.js"
        ))
        .output()
        .expect("failed to execute node");

    assert!(
        output.status.success(),
        "JS signal tests failed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

/// End-to-end runtime proof that match LITERAL arms actually fire on JS
/// targets: compile a match with literal patterns through the real pipeline
/// and execute the generated JavaScript in node. Before the fix the codegen
/// emitted `_v._variant === 0` on a raw primitive (always false), so every
/// literal arm silently fell through to the wildcard.
#[test]
fn js_runtime_match_literal_arms_pass() {
    if Command::new("node").arg("--version").output().is_err() {
        eprintln!("SKIP: Node.js not found -- skipping JS runtime tests");
        return;
    }

    let mut sources = FxHashMap::default();
    sources.insert(
        "main.omni".to_string(),
        r#"
pub fn describe(n: i32) -> String {
    return match n {
        0 => "zero",
        7 => "seven",
        _ => "many"
    };
}
"#
        .to_string(),
    );
    let path_mapping = omnilang::path_mapping::derive_path_mapping(&sources);
    let result = omnilang::compile_multi_internal(
        sources,
        omnilang::codegen::Target::JavaScript,
        path_mapping,
        false,
        false,
        None,
    );
    // The JavaScript backend is gated behind `codegen-javascript` (not in
    // default features) — mirror the graceful SKIP of the validation suite.
    if result
        .errors
        .iter()
        .any(|d| d.message.contains("is not enabled"))
    {
        eprintln!(
            "SKIP: JavaScript backend not compiled in -- rerun with \
             `--features codegen-javascript`"
        );
        return;
    }
    assert!(result.success, "compile failed: {:?}", result.errors);

    let mut js = String::new();
    for content in result.files.values() {
        js.push_str(content);
    }
    assert!(
        js.contains("_v === 0"),
        "literal arm must compare the subject value, generated JS:\n{}",
        js
    );
    assert!(
        !js.contains("_v._variant === 0"),
        "literal arm must not dispatch on _variant, generated JS:\n{}",
        js
    );
    js.push_str(
        "\nconsole.log(describe(0)); console.log(describe(7)); console.log(describe(42));\n",
    );

    let mut child = Command::new("node")
        .args(["--input-type=module", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn node");
    child
        .stdin
        .take()
        .expect("node stdin")
        .write_all(js.as_bytes())
        .expect("write generated JS to node stdin");
    let output = child.wait_with_output().expect("node output");

    assert!(
        output.status.success(),
        "generated JS failed to run:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    for (input, expected) in [("0", "zero"), ("7", "seven"), ("42", "many")] {
        assert!(
            stdout.contains(expected),
            "describe({input}) should return {expected:?}, got stdout: {}",
            stdout
        );
    }
}
