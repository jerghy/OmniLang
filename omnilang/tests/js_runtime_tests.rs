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

/// End-to-end runtime proof for the stdlib gap modules (async/datetime/encode/
/// regex + math extensions): compile a program that uses every newly
/// registered std function through the real pipeline and execute the generated
/// JavaScript in node. Before the registration these calls emitted bare
/// `datetime_now()`-style identifiers with no runtime definition
/// (ReferenceError).
#[test]
fn js_runtime_stdlib_gap_modules_pass() {
    if Command::new("node").arg("--version").output().is_err() {
        eprintln!("SKIP: Node.js not found -- skipping JS runtime tests");
        return;
    }

    let mut sources = FxHashMap::default();
    sources.insert(
        "main.omni".to_string(),
        r#"
import { async_sleep, async_spawn, async_timeout } from "std/async";
import { datetime_now, datetime_format, datetime_year, datetime_month } from "std/datetime";
import { encode_base64_encode, encode_base64_decode,
         encode_hex_encode, encode_hex_decode,
         encode_url_encode, encode_url_decode } from "std/encode";
import { regex_match, regex_search, regex_replace } from "std/regex";
import { array_length } from "std/array";
import { string_length } from "std/string";
import { math_sin, math_pi, math_e, math_clamp, math_random_int,
         math_log2, math_log10 } from "std/math";

pub fn b64_ok() -> bool {
    return encode_base64_decode(encode_base64_encode("OmniLang")) == "OmniLang";
}

pub fn hex_ok() -> bool {
    return encode_hex_decode(encode_hex_encode("OmniLang")) == "OmniLang";
}

pub fn url_ok() -> bool {
    return encode_url_decode(encode_url_encode("hello world")) == "hello world";
}

pub fn regex_ok() -> bool {
    return regex_match("user@example.com", "^[\\w.]+@[\\w]+\\.[a-z]+$")
        && array_length(regex_search("abc 123 def 456", "\\d+")) == 2
        && regex_replace("Hello 2024", "\\d+", "2025") == "Hello 2025";
}

pub fn datetime_ok() -> bool {
    let now = datetime_now();
    let f = datetime_format(now, "%Y-%m-%d");
    return string_length(f) == 10 && datetime_year(now) >= 2020
        && datetime_month(now) >= 1 && datetime_month(now) <= 12;
}

pub fn math_ok() -> bool {
    return math_clamp(9.0, 1.0, 5.0) == 5.0
        && math_log2(8.0) == 3.0
        && math_log10(1000.0) == 3.0
        && math_pi() > 3.14 && math_e() > 2.7
        && math_random_int(5, 5) == 5
        && math_sin(0.0) == 0.0;
}

async fn quick_task() -> String {
    await async_sleep(1);
    return "quick";
}

pub async fn async_ok() -> String {
    let t = async_spawn(quick_task());
    return await async_timeout(t, 1000);
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
    for needle in [
        "new Promise",
        "new Date()",
        "btoa(",
        "new RegExp",
        "Math.log2(",
        "Math.PI",
    ] {
        assert!(
            js.contains(needle),
            "generated JS must contain {needle:?} (stdlib translation missing), got:\n{js}"
        );
    }
    js.push_str(
        "\nconsole.log('b64=' + b64_ok());\n\
         console.log('hex=' + hex_ok());\n\
         console.log('url=' + url_ok());\n\
         console.log('regex=' + regex_ok());\n\
         console.log('datetime=' + datetime_ok());\n\
         console.log('math=' + math_ok());\n\
         async_ok().then(r => console.log('async=' + r));\n",
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
    for (label, expected) in [
        ("b64", "b64=true"),
        ("hex", "hex=true"),
        ("url", "url=true"),
        ("regex", "regex=true"),
        ("datetime", "datetime=true"),
        ("math", "math=true"),
        ("async", "async=quick"),
    ] {
        assert!(
            stdout.contains(expected),
            "stdlib {label} runtime check failed, expected {expected:?}, got stdout: {}",
            stdout
        );
    }
}
