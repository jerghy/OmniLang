//! Integration tests that run the JS runtime behavioral tests via Node.js.
//!
//! These tests execute the actual signal/sanitizer runtime functions in a JS
//! engine, going beyond the Rust-side string-content snapshot tests.
//!
//! Requires Node.js to be installed. Skipped gracefully if not available.

use std::process::Command;

fn node_available() -> bool {
    Command::new("node")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn run_js_test(test_file: &str) {
    let output = Command::new("node")
        .arg(test_file)
        .output()
        .expect("failed to execute node");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "JS test {} failed.\nstdout:\n{}\nstderr:\n{}",
        test_file,
        stdout,
        stderr,
    );
}

#[test]
fn js_runtime_signals_tests() {
    if !node_available() {
        eprintln!("Skipping JS runtime tests: Node.js not available");
        return;
    }
    run_js_test("tests/js_runtime/test_signals.js");
}

#[test]
fn js_runtime_sanitizers_tests() {
    if !node_available() {
        eprintln!("Skipping JS runtime tests: Node.js not available");
        return;
    }
    run_js_test("tests/js_runtime/test_sanitizers.js");
}
