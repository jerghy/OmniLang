//! Stdlib method-name validation integration tests (Task 4.10, ai LOW) —
//! unknown methods on concrete receiver types must warn with the nearest
//! stdlib suggestion; known stdlib methods stay silent; the warning must
//! surface on every P0 target.
//!
//! The pass itself lives in `omnilang-core/src/semantic/validate_types.rs`
//! (`check_method_name`) and runs during the semantic resolve pass; these
//! tests exercise it through the REAL compile pipeline
//! (`compile_multi_internal`), not a shortcut, so the warning is actually
//! in the path.

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

/// Compile one source through the full pipeline and return its diagnostics
/// (errors + warnings — warning-severity diagnostics live in the `errors`
/// channel; see `compile_multi_internal_post`).
fn compile_with_diags(src: &str, target: Target) -> ((), Vec<Diagnostic>) {
    let sources = FxHashMap::from_iter([("input.omni".to_string(), src.to_string())]);
    let result = compile_multi_internal(sources, target, FxHashMap::default(), false, false, None);
    ((), result.errors)
}

/// Task 4.10 Step 1 — unknown method on a concrete receiver must warn AND
/// name the method AND suggest the stdlib spelling. Regression:
/// `s.toUpper()` (String receiver) used to compile clean ("No issues
/// found") and only fail at runtime with a TypeError (ai LOW).
///
/// NOTE: the plan's literal source `view V { ... }` does not parse (views
/// only exist inside components — "Unexpected token at top level: View")
/// and `Target::All` does not exist; the component wrapper below is the
/// minimal valid form with identical assertions.
#[test]
fn unknown_method_on_concrete_type_warns() {
    let src = r#"import omniview; component V { fn f(s: String) { s.toUpper() } view { <div/> } }"#;
    let (_, diags) = compile_with_diags(src, Target::React);
    let d = diags
        .iter()
        .find(|d| d.severity == Severity::Warning && d.message.contains("toUpper"))
        .expect("must warn");
    assert!(
        d.message.contains("to_uppercase"),
        "missing stdlib suggestion: {}",
        d.message
    );
}

/// Known stdlib methods on every concrete receiver must never warn
/// (zero false positives).
#[test]
fn known_stdlib_methods_do_not_warn() {
    let src = r#"
import omniview;
component KnownMethods {
    fn f(s: String, n: i32, items: Array<i32>, cfg: Map<String, i32>) {
        s.to_uppercase(); s.len(); s.get(0); s.starts_with("a");
        n.to_string();
        items.push(1); items.len(); items.filter(|x| x > 0);
        cfg.get("k"); cfg.contains("k");
    }
    view { <div/> }
}
"#;
    let (_, diags) = compile_with_diags(src, Target::React);
    let warnings: Vec<&Diagnostic> = diags
        .iter()
        .filter(|d| d.severity == Severity::Warning)
        .collect();
    assert!(
        warnings.is_empty(),
        "known methods must not warn, got: {:?}",
        warnings.iter().map(|d| &d.message).collect::<Vec<_>>()
    );
}

/// An untyped receiver (Any) must be skipped — warning on it would cascade
/// from a type the checker cannot see.
#[test]
fn any_receiver_is_skipped() {
    let src = r#"
import omniview;
component Untyped { fn f(x) { x.toUpper() } view { <div/> } }
"#;
    let (_, diags) = compile_with_diags(src, Target::React);
    let warnings: Vec<&Diagnostic> = diags
        .iter()
        .filter(|d| d.severity == Severity::Warning)
        .collect();
    assert!(
        warnings.is_empty(),
        "Any receiver must not warn, got: {:?}",
        warnings.iter().map(|d| &d.message).collect::<Vec<_>>()
    );
}

/// Signals own a method surface (set/inc/push/pop/...) — must not warn.
#[test]
fn signal_receiver_methods_do_not_warn() {
    let src = r#"
import omniview;
component SignalUser {
    signal s: i32 = 0;
    signal list: Array<String> = [];
    fn f() { s.set(1); s.inc(); list.push("x"); list.pop(); }
    view { <div/> }
}
"#;
    let (_, diags) = compile_with_diags(src, Target::React);
    let warnings: Vec<&Diagnostic> = diags
        .iter()
        .filter(|d| d.severity == Severity::Warning)
        .collect();
    assert!(
        warnings.is_empty(),
        "signal methods must not warn, got: {:?}",
        warnings.iter().map(|d| &d.message).collect::<Vec<_>>()
    );
}

/// Element refs dispatch to the ref-op API (focus/measure/scrollTo/...) —
/// not the stdlib table.
#[test]
fn ref_receiver_methods_do_not_warn() {
    let src = r#"
import omniview;
component RefUser {
    ref box: Element;
    fn f() { box.focus(); box.measure(); }
    view { <div @ref={box}/> }
}
"#;
    let (_, diags) = compile_with_diags(src, Target::React);
    let warnings: Vec<&Diagnostic> = diags
        .iter()
        .filter(|d| d.severity == Severity::Warning)
        .collect();
    assert!(
        warnings.is_empty(),
        "ref methods must not warn, got: {:?}",
        warnings.iter().map(|d| &d.message).collect::<Vec<_>>()
    );
}

/// Task 4.10 three-target fixture — the unknown-method warning must surface
/// on every P0 backend (semantic analysis is target-independent, so the
/// warning appears before codegen dispatch).
#[test]
fn unknown_method_warns_on_all_p0_targets() {
    let src = r#"
import omniview;
component Bogus {
    fn f(s: String) { s.toUpper() }
    view { <div/> }
}
"#;
    let backends: Vec<(Target, &'static str)> = vec![
        (Target::OmniView, "OmniView"),
        (Target::React, "React"),
        #[cfg(feature = "codegen-flutter")]
        (Target::Flutter, "Flutter"),
    ];
    for (target, name) in backends {
        let (_, diags) = compile_with_diags(src, target);
        let warn = diags
            .iter()
            .find(|d| d.severity == Severity::Warning && d.message.contains("toUpper"));
        assert!(
            warn.is_some(),
            "{name}: unknown method must warn, got: {:?}",
            diags.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }
}

/// Corpus gate (Task 4.10 acceptance): the whole learn-omniview showcase
/// compiles through the REAL multi-file pipeline with ZERO method-name
/// warnings — the corpus binds only canonical stdlib methods on typed
/// receivers (signal/ref/store receivers carry their own method surface
/// and are skipped by design), so any method-name warning here is a false
/// positive in the pass (e.g. a misspelled table entry).
#[test]
fn learn_omniview_corpus_has_no_method_name_warnings() {
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
    let method_warnings: Vec<&Diagnostic> = result
        .errors
        .iter()
        .filter(|d| d.severity == Severity::Warning && d.message.contains("未知方法"))
        .collect();
    assert!(
        method_warnings.is_empty(),
        "corpus must compile with zero method-name warnings, got: {:?}",
        method_warnings
            .iter()
            .map(|d| (&d.message, &d.file))
            .collect::<Vec<_>>()
    );
}
