//! Cross-backend regression tests — inline-style expression channel (域 8,
//! Task 1.3, review finding C2 root cause).
//!
//! The style channel (`style: { prop: expr }`) lowers expressions to JSON
//! markers (`$signal` / `$field` / `$call` / `$concat`) consumed by each
//! backend. Before the fix, ANY `obj.field` in a style expression was
//! mangled to `$signal: "obj_field"` — correct only for genuine store
//! signal aliases (`ThemeStore.currentBg` → `ThemeStore_currentBg`); for
//! loop-item fields like `row.score` it produced the undefined identifier
//! `row_score` (verified in generated React AND OmniView output).
//!
//! Contract under test:
//!   - `obj.field` where obj is a store alias  → `$signal: "obj_field"` (unchanged)
//!   - `obj.field` where obj is a signal       → `$field` with `$signal` obj
//!     (React emits `obj.field`, OmniView emits `obj.value.field`)
//!   - `obj.field` where obj is a local        → `$field` with `$ident` obj
//!     (both targets emit `obj.field`)

use rustc_hash::FxHashMap;

use omnilang::codegen::{compile, Target};
use omnilang::ir::lower;
use omnilang::lexer::tokenize;
use omnilang::parser;

/// Minimal C2 fixture: `@for` item field read inside a style expression.
/// Mirrors learn-omniview DataTable.omni (`getScoreColor(row.score)`).
const FOR_ITEM_FIELD_FIXTURE: &str = r##"
import omniview;

struct RowItem {
    id: i32,
    score: i32,
}

component App {
    signal rows: Array<RowItem> = [];

    fn getScoreColor(score: i32) -> String {
        if score >= 90 { return "#16a34a"; }
        return "#dc2626";
    }

    view {
        <div>
            @for row in rows {
                <span style: { color: getScoreColor(row.score) }>{row.score}</span>
            }
        </div>
    }
}
"##;

/// Genuine store alias fixture: `PaletteStore.color` must keep the
/// `$signal: "PaletteStore_color"` channel (store fields are real signals
/// in every backend — NOT real JS field access).
const STORE_FIELD_FIXTURE: &str = r##"
import omniview;

store PaletteStore {
    signal color: String = "#ff0000";
}

component App {
    @store PaletteStore;

    view {
        <div style: { color: PaletteStore.color }></div>
    }
}
"##;

/// Signal-object fixture: `user.name` where `user` is a signal holding an
/// object — must be REAL field access on the signal's value, not a mangled
/// `user_name` alias.
const SIGNAL_OBJECT_FIELD_FIXTURE: &str = r##"
import omniview;

component App {
    signal user: Map<String, String> = { name: "Ada", theme: "#0f0" };

    view {
        <div style: { color: user.theme }></div>
    }
}
"##;

/// Compile a source string to a target, returning all generated file
/// contents concatenated. Panics on parse/compile errors.
fn compile_source_all_files(source: &str, target: Target) -> String {
    let tokens = tokenize(source);
    let (program, parse_errors) = parser::parse(tokens, source);
    assert!(
        parse_errors.is_empty(),
        "parse errors: {:?}",
        parse_errors
            .iter()
            .map(|e| e.message.clone())
            .collect::<Vec<_>>()
    );
    // Mirror the real pipeline (run_pipeline): semantic analysis populates
    // the symbol table BEFORE lowering — the style-channel lowering consults
    // it to distinguish store aliases from plain locals.
    let mut semantics = omnilang::semantic::analyze(&program);
    assert!(
        semantics.diagnostics.is_empty(),
        "semantic diagnostics: {:?}",
        semantics
            .diagnostics
            .iter()
            .map(|d| d.message.clone())
            .collect::<Vec<_>>()
    );
    let ir = lower(&program, &mut semantics, None);
    let source_files: FxHashMap<String, String> =
        FxHashMap::from_iter([("input.omni".to_string(), source.to_string())]);
    let output = compile(&ir, target, &source_files);
    assert!(
        output.errors.is_empty(),
        "compile errors: {:?}",
        output
            .errors
            .iter()
            .map(|e| e.message.clone())
            .collect::<Vec<_>>()
    );
    output
        .files
        .iter()
        .map(|f| f.content.clone())
        .collect::<Vec<_>>()
        .join("\n")
}

fn compile_react(source: &str) -> String {
    compile_source_all_files(source, Target::React)
}

#[cfg(feature = "codegen-omniview")]
fn compile_omniview(source: &str) -> omnilang::OmniViewResult {
    omnilang::compile_omniview(source, false, false)
}

// ── @for item field: must NOT be mangled to `row_score` ─────

#[test]
fn react_style_expr_for_item_field_not_mangled() {
    let out = compile_react(FOR_ITEM_FIELD_FIXTURE);
    assert!(
        out.contains("getScoreColor(row.score)"),
        "React style must emit the real field access `getScoreColor(row.score)`. Got:\n{}",
        out
    );
    assert!(
        !out.contains("row_score"),
        "React style must NOT mangle `row.score` into `row_score`. Got:\n{}",
        out
    );
}

#[cfg(feature = "codegen-omniview")]
#[test]
fn omniview_style_expr_for_item_field_not_mangled() {
    let result = compile_omniview(FOR_ITEM_FIELD_FIXTURE);
    assert!(
        result.success,
        "OmniView compile failed: {:?}",
        result.errors
    );
    let js = result.files.expect("files").js;
    assert!(
        js.contains("row.score"),
        "OmniView JS must emit the real field access `row.score`. Got:\n{}",
        js
    );
    assert!(
        !js.contains("row_score"),
        "OmniView JS must NOT mangle `row.score` into `row_score`. Got:\n{}",
        js
    );
}

// ── Genuine store alias: `$signal` channel unchanged ────────

#[test]
fn react_style_expr_store_field_keeps_signal_channel() {
    let out = compile_react(STORE_FIELD_FIXTURE);
    assert!(
        out.contains("PaletteStore_color"),
        "React style must keep the store-field signal alias `PaletteStore_color`. Got:\n{}",
        out
    );
    assert!(
        !out.contains("PaletteStore.color"),
        "React style must NOT emit `PaletteStore.color` (store field is a signal, not JS field access). Got:\n{}",
        out
    );
}

#[cfg(feature = "codegen-omniview")]
#[test]
fn omniview_style_expr_store_field_keeps_signal_channel() {
    let result = compile_omniview(STORE_FIELD_FIXTURE);
    assert!(
        result.success,
        "OmniView compile failed: {:?}",
        result.errors
    );
    let js = result.files.expect("files").js;
    assert!(
        js.contains("PaletteStore_color.value"),
        "OmniView style must keep the store-field signal read `.value`. Got:\n{}",
        js
    );
}

// ── Signal-object field: real field access on the signal value ─

#[cfg(feature = "codegen-omniview")]
#[test]
fn omniview_style_expr_signal_object_field_resolves_value() {
    let result = compile_omniview(SIGNAL_OBJECT_FIELD_FIXTURE);
    assert!(
        result.success,
        "OmniView compile failed: {:?}",
        result.errors
    );
    let js = result.files.expect("files").js;
    assert!(
        js.contains("user.value.theme"),
        "OmniView style must read the field on the signal's value (`user.value.theme`). Got:\n{}",
        js
    );
    assert!(
        !js.contains("user_theme"),
        "OmniView style must NOT mangle `user.theme` into `user_theme`. Got:\n{}",
        js
    );
}

// ── Runtime: the style value must resolve correctly in node ──

/// Extract the style-binding expression from the OmniView JS output and
/// evaluate it in node with a sample `row`, proving the emitted field
/// access computes the right CSS value (no `row_score` ReferenceError).
#[cfg(feature = "codegen-omniview")]
#[test]
fn node_style_expr_field_value_resolves_at_runtime() {
    // Skip if Node.js is not available (mirrors tests/js_runtime_tests.rs)
    if std::process::Command::new("node")
        .arg("--version")
        .output()
        .is_err()
    {
        eprintln!("SKIP: Node.js not found -- skipping node style-value test");
        return;
    }

    let result = compile_omniview(FOR_ITEM_FIELD_FIXTURE);
    assert!(result.success, "compile failed: {:?}", result.errors);
    let js = result.files.expect("files").js;

    // The pre-compiled per-clone style binding emits:
    //   _el.style.color = __omni_sanitizeCss(<fn>(row.score));
    // Extract the inner expression. Match the ASSIGNMENT form
    // `=__omni_sanitizeCss(` so the runtime function definition
    // `function __omni_sanitizeCss(value) {` is not picked up.
    let marker = "=__omni_sanitizeCss(";
    let Some(open) = js.find(marker) else {
        panic!("no style binding assignment in OmniView JS:\n{}", js);
    };
    let rest = &js[open + marker.len()..];
    let close = rest.find(')').expect("closing paren");
    let expr = &rest[..=close];
    assert!(
        expr.contains("row.score"),
        "extracted style expression must read row.score, got: {}",
        expr
    );

    // Stub the component fn (prefixed, e.g. App_getScoreColor) so the
    // extracted expression evaluates in isolation.
    let fn_name = expr
        .split('(')
        .next()
        .unwrap_or_default()
        .trim()
        .to_string();
    assert!(
        !fn_name.is_empty()
            && fn_name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_'),
        "could not extract fn name from style expression: {}",
        expr
    );
    let script = format!(
        r##"
const __omni_sanitizeCss = (v) => v;
const {fn_name} = (score) => score >= 90 ? "#16a34a" : "#dc2626";
const row = {{ score: 95 }};
const color = ({}).trim();
console.log("STYLE_VALUE=" + color);
if (color !== "#16a34a") {{ throw new Error("expected #16a34a for score 95, got " + color); }}
"##,
        expr,
        fn_name = fn_name
    );
    let mut child = std::process::Command::new("node")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("failed to spawn node");
    use std::io::Write;
    child
        .stdin
        .as_mut()
        .expect("stdin")
        .write_all(script.as_bytes())
        .expect("write script");
    let output = child.wait_with_output().expect("node output");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "node style-value eval failed:\nstdout: {}\nstderr: {}\nexpr: {}",
        stdout,
        stderr,
        expr
    );
}
