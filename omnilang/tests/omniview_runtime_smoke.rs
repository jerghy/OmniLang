//! OmniView runtime smoke test.
//!
//! Compiles a minimal `.omni` fixture to OmniView output via
//! `omnilang::compile_omniview()`, writes the generated JS+HTML into
//! a standalone HTML document, then spawns `node smoke_test.js` to
//! verify the runtime executes without errors in jsdom.
//!
//! Requires:
//!   - Node.js (`node --version`)
//!   - jsdom (`npm install jsdom` — see CI smoke-test job)
//!
//! Skips gracefully when Node.js is not available.

#[cfg(feature = "codegen-omniview")]
use std::process::Command;

#[cfg(feature = "codegen-omniview")]
#[test]
fn omniview_runtime_smoke() {
    // ── Skip if Node.js is not available ──
    if Command::new("node").arg("--version").output().is_err() {
        eprintln!("SKIP: Node.js not found -- skipping OmniView runtime smoke test");
        return;
    }

    // ── Compile the fixture ──
    let source = include_str!("fixtures/omniview_smoke.omni");
    let result = omnilang::compile_omniview(source, false, false);
    assert!(
        result.success,
        "OmniView compilation failed: {:?}",
        result.errors
    );

    let files = match result.files {
        Some(f) => f,
        None => {
            panic!("compile_omniview returned success but no files");
        }
    };

    // ── Build test HTML ──
    // gen.html already has <div id="root" data-component="..."> from emit_view().
    // gen.js contains runtime + innerHTML assignment + deferred effects.
    let test_html = format!(
        r#"<!DOCTYPE html>
<html><head><meta charset="UTF-8"><title>OmniView Smoke Test</title></head>
<body>
{}
<script>{}</script>
</body></html>"#,
        files.html, files.js
    );

    // ── Write to temp file ──
    let tmp_dir = std::env::temp_dir().join("omni-smoke-test");
    let _ = std::fs::create_dir_all(&tmp_dir);
    let html_path = tmp_dir.join("index.html");
    std::fs::write(&html_path, &test_html).expect("write test HTML");

    // ── Run smoke test via Node.js ──
    let script = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/smoke_test.js");
    let output = Command::new("node")
        .arg(&script)
        .arg(html_path.to_str().expect("valid temp path"))
        .output()
        .expect("failed to execute node");

    // Clean up temp file
    let _ = std::fs::remove_file(&html_path);

    // Forward Node.js stdout so CI logs show pass/fail details
    let stdout = String::from_utf8_lossy(&output.stdout);
    if !stdout.is_empty() {
        println!("{}", stdout);
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !stderr.is_empty() {
        eprintln!("{}", stderr);
    }

    assert!(
        output.status.success(),
        "OmniView runtime smoke test FAILED (exit code: {:?})",
        output.status.code()
    );
}
