//! a11y role vocabulary integration tests (Task 4.3, domain HIGH-2) — the
//! semantic role vocabulary must be WAI-ARIA canonical
//! (`button/textbox/link/checkbox/radio/img`); legacy Flutter Semantics flag
//! names (`textField`/`image`) warn with the canonical suggestion; Flutter
//! maps the canonical names back to the internal Semantics flags
//! (`textbox → textField: true`, `img → image: true`) so web output never
//! carries illegal `role="textField"`.
//!
//! The validation lives in `omnilang-core/src/semantic/validate_a11y.rs`;
//! these tests exercise it through the REAL compile pipeline
//! (`compile_multi_internal`), so the pass is actually in the path.
//!
//! NOTE: the plan's literal source `view V { ... }` does not parse (views
//! only exist inside components — "Unexpected token at top level: View");
//! the component wrapper below is the minimal valid form with identical
//! assertions (same documented deviation as event_name_validation.rs).

use std::path::PathBuf;

use rustc_hash::FxHashMap;

use omnilang::codegen::Target;
use omnilang::compile_multi_internal;
use omnilang::error::Diagnostic;

/// Resolve the repo root (parent of the omnilang crate dir).
fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("should have parent directory")
        .to_path_buf()
}

/// Compile one source through the full pipeline and return its diagnostics
/// (errors + warnings — warning-severity diagnostics live in the `errors`
/// channel; see `compile_multi_internal_post`).
fn compile_with_diags(src: &str, target: Target) -> ((), Vec<Diagnostic>) {
    let sources = FxHashMap::from_iter([("input.omni".to_string(), src.to_string())]);
    let result = compile_multi_internal(sources, target, FxHashMap::default(), false, false, None);
    ((), result.errors)
}

/// Compile one source and return the concatenated generated file contents.
fn compile_output(src: &str, target: Target) -> (bool, String) {
    let sources = FxHashMap::from_iter([("input.omni".to_string(), src.to_string())]);
    let result = compile_multi_internal(sources, target, FxHashMap::default(), false, false, None);
    let mut code: Vec<String> = result.files.values().cloned().collect();
    code.sort();
    (result.success, code.join("\n"))
}

/// Task 4.3 Step 1 — the ARIA-canonical role `textbox` must compile clean on
/// every target: React emits the legal `role="textbox"` (no more
/// `role="textField"` in web output) and Flutter maps it to its internal
/// `textField: true` Semantics flag.
#[test]
fn a11y_role_uses_aria_vocabulary() {
    let src = r#"import omniview; component V { view { <div a11y: { role: "textbox" }/> } }"#;

    let (success, react_code) = compile_output(src, Target::React);
    assert!(success, "textbox must compile on React without errors");
    assert!(
        react_code.contains("role=\"textbox\""),
        "invalid ARIA passthrough: {}",
        react_code
    );

    #[cfg(feature = "codegen-flutter")]
    {
        let (success, dart_code) = compile_output(src, Target::Flutter);
        assert!(success, "textbox must compile on Flutter without errors");
        assert!(
            dart_code.contains("textField: true"),
            "must map to flutter flag: {}",
            dart_code
        );
    }
}

/// Task 4.3 Step 1 — the legacy Flutter flag name `textField` must warn with
/// the canonical ARIA suggestion (`textbox`), so migrated code stops
/// emitting illegal `role="textField"` on the web targets.
#[test]
fn legacy_flag_role_warns() {
    let (_, diags) = compile_with_diags(
        r#"import omniview; component V { view { <div a11y: { role: "textField" }/> } }"#,
        Target::React,
    );
    assert!(
        diags
            .iter()
            .any(|d| d.message.contains("textbox") && d.message.contains("textField")),
        "must suggest canonical name: {:?}",
        diags
    );
}

/// Task 4.4 Step 1 (domain MEDIUM-1 + D-D) — raw `aria-*` attributes are a
/// deleted form: they pass through on the web targets but Flutter silently
/// drops them (accessible name lost) — one intent, not all surfaces. The
/// semantic pass must hard-error with the migration hint to the `a11y: {}`
/// block. (The plan's literal `Target::All` deviates per the file header
/// note — the diagnostic pass is target-independent; React stands in.)
#[test]
fn raw_aria_attributes_hard_diagnosed() {
    let (_, diags) = compile_with_diags(
        r#"import omniview; component V { view { <div aria-label="x"/> } }"#,
        Target::React,
    );
    let d = diags
        .iter()
        .find(|d| d.message.contains("aria"))
        .expect("must diagnose aria-*");
    assert!(
        d.message.contains("a11y"),
        "missing migration hint: {}",
        d.message
    );
    assert_eq!(
        d.severity,
        omnilang::error::Severity::Error,
        "aria-* must hard-error (Flutter drops the semantic), got: {:?}",
        d.severity
    );
}

/// Task 4.4 Step 1 (D-D boundary ruling) — `alt=` stays a plain DOM
/// attribute, NOT an a11y surface: it passes through verbatim on the web
/// targets (and maps to Flutter `semanticLabel` via the prop table). The
/// a11y semantic surface for image alternative text is `a11y: { alt }`.
#[test]
fn alt_remains_plain_dom_prop() {
    let (success, out_react) = compile_output(
        r#"import omniview; component V { view { <img alt="logo"/> } }"#,
        Target::React,
    );
    assert!(success, "alt must compile clean, got: {}", out_react);
    assert!(
        out_react.contains("alt=\"logo\""),
        "alt must stay a DOM prop: {}",
        out_react
    );
}

/// Corpus gate (Task 4.4 acceptance): the whole learn-omniview showcase
/// compiles through the REAL multi-file pipeline with ZERO raw `aria-*`
/// deletion diagnostics — after EventsDemo migrated to the a11y block, any
/// error here is a stale raw-aria usage that would silently lose its
/// accessible name on Flutter.
#[test]
fn learn_omniview_corpus_has_no_raw_aria_attributes() {
    let repo = repo_root();
    let src_dir = repo.join("test/projects/learn-omniview/src");
    let sources = omnilang::discovery::discover_omni_sources(
        &src_dir,
        omnilang::discovery::DiscoveryMode::Directory,
    )
    .expect("corpus discovery must succeed");
    assert!(!sources.is_empty(), "corpus must exist at {src_dir:?}",);
    let path_mapping = omnilang::path_mapping::derive_path_mapping(&sources);

    let result = compile_multi_internal(sources, Target::React, path_mapping, false, false, None);
    let aria_errors: Vec<&Diagnostic> = result
        .errors
        .iter()
        .filter(|d| d.message.contains("aria-* 原生属性已删除"))
        .collect();
    assert!(
        aria_errors.is_empty(),
        "corpus must compile with zero raw aria-* diagnostics, got: {:?}",
        aria_errors
            .iter()
            .map(|d| (&d.message, &d.file))
            .collect::<Vec<_>>()
    );
}

/// Corpus gate (Task 4.3 acceptance): the whole learn-omniview showcase
/// compiles through the REAL multi-file pipeline with ZERO a11y role
/// vocabulary warnings — after the demo migrated to canonical names
/// (`textbox`), any role warning here is a stale legacy usage that would
/// regenerate illegal `role="textField"` on the web targets.
#[test]
fn learn_omniview_corpus_has_no_a11y_role_warnings() {
    let repo = repo_root();
    let src_dir = repo.join("test/projects/learn-omniview/src");
    let sources = omnilang::discovery::discover_omni_sources(
        &src_dir,
        omnilang::discovery::DiscoveryMode::Directory,
    )
    .expect("corpus discovery must succeed");
    assert!(!sources.is_empty(), "corpus must exist at {src_dir:?}",);
    let path_mapping = omnilang::path_mapping::derive_path_mapping(&sources);

    let result = compile_multi_internal(sources, Target::React, path_mapping, false, false, None);
    let role_warnings: Vec<&Diagnostic> = result
        .errors
        .iter()
        .filter(|d| d.message.contains("a11y role"))
        .collect();
    assert!(
        role_warnings.is_empty(),
        "corpus must compile with zero a11y role warnings, got: {:?}",
        role_warnings
            .iter()
            .map(|d| (&d.message, &d.file))
            .collect::<Vec<_>>()
    );
}
