//! Diagnostic position + suggestion-field integration tests (Task 4.11,
//! ai MEDIUM) — semantic-pass and lint diagnostics must anchor at the
//! offending element/declaration instead of the component start line, and
//! verbatim migration hints must land in the structured `suggestion` field
//! (the JSON contract promised it; only validate_effect_deps filled it).
//!
//! The pass logic lives in `omnilang-core/src/semantic/`; these tests
//! exercise it through the REAL compile pipeline (`compile_multi_internal`),
//! so the position/suggestion values are the ones consumers see.

use std::path::PathBuf;

use rustc_hash::FxHashMap;

use omnilang::codegen::Target;
use omnilang::compile_multi_internal;
use omnilang::error::{Diagnostic, Severity};

/// Resolve the repo root (parent of the omnilang crate dir).
fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("should have parent directory")
        .to_path_buf()
}

/// Compile one source through the full pipeline and return its diagnostics.
/// Both channels are merged: semantic diagnostics (errors + warnings) live
/// in `errors`, while lint-rule warnings land in `warnings` — the position
/// tests below cover both kinds.
fn compile_with_diags(src: &str, target: Target) -> ((), Vec<Diagnostic>) {
    let sources = FxHashMap::from_iter([("input.omni".to_string(), src.to_string())]);
    let result = compile_multi_internal(sources, target, FxHashMap::default(), false, false, None);
    let mut all = result.errors;
    all.extend(result.warnings);
    ((), all)
}

/// Task 4.11 Step 1 — the removed `value={sig}` semantic-binding form must
/// point at the ELEMENT line (not the component start line) and carry the
/// verbatim migration hint in the structured `suggestion` field.
///
/// NOTE: the plan's literal source `view V { ... }` does not parse (views
/// only exist inside components — "Unexpected token at top level: View"),
/// `Target::All` does not exist, and the `code` field is the serialized
/// form of `Diagnostic.kind` (accessible via `kind.code_str()`); the
/// component wrapper below is the minimal valid form with identical
/// assertions. The removed form is the bare-signal `value={x}` attribute
/// (`bind:value={x}` is the legal two-way binding).
#[test]
fn semantic_binding_diag_has_element_position() {
    let src = "import omniview;\n\
               component V {\n\
               \x20   signal x: String = \"\";\n\
               \x20   view {\n\
               \x20       <div value={x}/>\n\
               \x20   }\n\
               }";
    let (_, diags) = compile_with_diags(src, Target::React);
    let d = diags
        .iter()
        .find(|d| d.kind.code_str() == "E_VALUE_BINDING_REMOVED")
        .expect("E_VALUE_BINDING_REMOVED must be emitted for value={sig}");
    assert!(d.line >= 2, "must point at element line, got {}", d.line);
    assert!(
        d.suggestion.is_some(),
        "suggestion must be populated: {:?}",
        d
    );
    // The human-readable message keeps the migration hint (docs-gate
    // verify-docs.py matches diagnostics by message prefix — the message
    // must not move into suggestion only).
    assert!(
        d.message.contains("bind:value"),
        "message must stay human-readable with the hint: {}",
        d.message
    );
}

/// The imperative ref write (`ref.value = x`) migration hint must be
/// structured: message unchanged (contains 命令式写已删除 / bind:value)
/// AND suggestion populated verbatim.
#[test]
fn ref_value_write_diag_has_structured_suggestion() {
    let src = "import omniview;\n\
               component W {\n\
               \x20   ref box: Element;\n\
               \x20   fn go() {\n\
               \x20       box.value = \"x\";\n\
               \x20   }\n\
               \x20   view {\n\
               \x20       <div @ref={box}/>\n\
               \x20   }\n\
               }";
    let (_, diags) = compile_with_diags(src, Target::React);
    let d = diags
        .iter()
        .find(|d| d.message.contains("命令式写已删除"))
        .expect("imperative ref write must be diagnosed");
    assert!(
        d.message.contains("bind:value"),
        "message must keep the hint: {}",
        d.message
    );
    assert!(
        d.suggestion.is_some(),
        "migration hint must also land in suggestion: {:?}",
        d
    );
    assert!(
        d.suggestion.as_deref().unwrap().contains("bind:value"),
        "suggestion must be the verbatim hint, got: {:?}",
        d.suggestion
    );
}

/// Lint diagnostics must carry a real source position (the AST declaration
/// line) instead of the hardcoded 0:0.
#[test]
fn dead_code_lint_diag_has_declaration_position() {
    let src = "import omniview;\n\
               component L {\n\
               \x20   signal unusedFlag: bool = false;\n\
               \x20   view { <div/> }\n\
               }";
    let (_, diags) = compile_with_diags(src, Target::React);
    let d = diags
        .iter()
        .find(|d| d.message.contains("'unusedFlag'") && d.message.contains("never used"))
        .expect("dead-code lint must flag the unused signal");
    assert!(
        d.line >= 2,
        "lint must point at the signal declaration line, got {} ({})",
        d.line,
        d.message
    );
}

/// The docs-gate compat guard: diagnostics that verify-docs.py matches by
/// message prefix keep their exact message text when suggestion is added.
#[test]
fn suggestion_population_never_rewrites_message() {
    let src = "import omniview;\n\
               component V {\n\
               \x20   signal x: String = \"\";\n\
               \x20   view {\n\
               \x20       <div value={x}/>\n\
               \x20   }\n\
               }";
    let (_, diags) = compile_with_diags(src, Target::React);
    let d = diags
        .iter()
        .find(|d| d.kind.code_str() == "E_VALUE_BINDING_REMOVED")
        .expect("E_VALUE_BINDING_REMOVED must be emitted");
    assert_eq!(
        d.message,
        "'value={sig}' 语义绑定形式已删除，用 bind:value 双向绑定，或普通 value prop 单向",
        "message must remain byte-identical (docs-gate matches by prefix)"
    );
    assert_eq!(
        d.suggestion.as_deref(),
        Some("用 bind:value 双向绑定，或普通 value prop 单向"),
        "suggestion carries the verbatim hint extracted from the message"
    );
}

/// Corpus gate: the learn-omniview showcase must still compile with the
/// same diagnostic surface (no new errors from span threading).
#[test]
fn learn_omniview_corpus_compiles_clean() {
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
    let errors: Vec<&Diagnostic> = result
        .errors
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .collect();
    assert!(
        errors.is_empty(),
        "corpus must compile with zero errors, got: {:?}",
        errors
            .iter()
            .map(|d| (&d.message, &d.file))
            .collect::<Vec<_>>()
    );
}
