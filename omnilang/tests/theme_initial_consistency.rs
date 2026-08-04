//! Theme-initial consistency + paused block-form migration hint
//! (Task 4.7, domain MEDIUM-2 + LOW-4).
//!
//! MEDIUM-2: `@default-theme light;` (design block) and the `@theme` signal's
//! literal initial (`signal mode: String @theme = "light"`) are TWO
//! declarations of the same fact — the program's startup theme — but were
//! never cross-validated: React starts from `@default-theme`, Flutter only
//! honors the signal initial for "dark", so a mismatch silently diverges
//! startup behavior across backends. The consistency check lives in
//! `omnilang-core/src/semantic/validate_theme.rs` (D-F decision); these
//! tests exercise it through the REAL compile pipeline
//! (`compile_multi_internal`), so the pass is actually in the path.
//!
//! LOW-4: the `@motion fadeIn { paused: true }` block form used to report
//! the generic "expected 'from' or 'to'" error instead of the E_PAUSED_REMOVED
//! migration hint (the hint only covered the keyframe-body map form). The
//! parser now routes the block form through the same hard error.
//!
//! NOTE deviations from the plan's literal snippets (same documented
//! precedent as a11y_role_aria.rs / event_name_validation.rs):
//! - the plan's literal `Target::All` does not exist — the diagnostic pass is
//!   target-independent, React stands in;
//! - the plan's literal `design { ... }` does not parse (design blocks require
//!   a name — "expected design block name"); the named `design D { ... }`
//!   below is the minimal valid form with identical assertions;
//! - the plan's literal `d.message.contains("paused")` passes even on the
//!   OLD generic error (it prints "got 'paused'") — the assertion is
//!   strengthened to require the E_PAUSED_REMOVED migration hint, which is
//!   what LOW-4 actually demands.

use rustc_hash::FxHashMap;

use omnilang::codegen::Target;
use omnilang::compile_multi_internal;
use omnilang::error::{Diagnostic, Severity};

/// Compile one source through the full pipeline and return its diagnostics
/// (errors + warnings — warning-severity diagnostics live in the `errors`
/// channel; see `compile_multi_internal_post`).
fn compile_with_diags(src: &str, target: Target) -> ((), Vec<Diagnostic>) {
    let sources = FxHashMap::from_iter([("input.omni".to_string(), src.to_string())]);
    let result = compile_multi_internal(sources, target, FxHashMap::default(), false, false, None);
    ((), result.errors)
}

/// Parse one source and return the collected parse diagnostics.
fn parse_and_diag(source: &str) -> Vec<omnilang_core::error::ParseError> {
    let tokens = omnilang_core::lexer::tokenize(source);
    let (_program, errors) = omnilang_core::parser::parse(tokens, source);
    errors
}

/// Task 4.7 Step 1 — `@default-theme light;` and `@theme` signal initial
/// `"dark"` are a double declaration of the program's startup theme that
/// silently diverges across backends (React starts from @default-theme,
/// Flutter only honors the signal initial for "dark"). The mismatch must
/// hard-error.
#[test]
fn default_theme_mismatch_errors() {
    let src = r#"
        design D { @themes { light, dark } @default-theme light; }
        store S { signal mode: String @theme = "dark"; }
    "#;
    let (_, diags) = compile_with_diags(src, Target::React);
    assert!(
        diags.iter().any(|d| d.severity == Severity::Error),
        "mismatch must error"
    );
}

/// Task 4.7 Step 1 — the `paused: true` BLOCK form (`@motion fadeIn { paused: true }`)
/// used to fall through to the generic "expected 'from' or 'to'" error; it
/// must now name `paused` with the migration hint, same as the keyframe-body
/// map form.
#[test]
fn paused_block_form_has_migration_hint() {
    let diags = parse_and_diag(r#"@motion fadeIn { paused: true }"#);
    assert!(
        diags.iter().any(|d| {
            d.kind == Some(omnilang_core::error::DiagnosticKind::EPausedRemoved)
                && d.message.contains("paused 已删除")
        }),
        "must name paused with the migration hint, got: {:?}",
        diags
            .iter()
            .map(|d| (&d.message, &d.kind))
            .collect::<Vec<_>>()
    );
}
