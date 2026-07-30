//! Integration tests for JS runtime functions (sanitizers, signals).
//!
//! These tests invoke the exact JS runtime functions emitted by `runtime.rs`
//! via Node.js. If Node.js is not available (e.g., in minimal CI images),
//! the tests gracefully skip with a diagnostic message.
//!
//! JS test files live in `tests/js_runtime/` alongside this file.

use std::process::Command;

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
