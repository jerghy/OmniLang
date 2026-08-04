//! Motion-in-@for compile diagnostics (Task 4.9 — perf M2 + D-H): motion
//! triggers on elements inside an @for body silently degrade per backend —
//! Flutter allocates one controller per template ordinal (all N items replay
//! the same animation), React shares a single ref (only the last mounted
//! item animates). The semantic pass must warn (honest degradation, not
//! silent divergence) until per-item allocation lands (long-term item).
//!
//! The pass lives in `omnilang-core/src/semantic/validate_motion_triggers.rs`
//! (the ForDirective arm of the view-tree motion walk); these tests exercise
//! it through the REAL compile pipeline (`compile_multi_internal`), so the
//! pass is actually in the path.
//!
//! NOTE: the plan's literal source `view V { <for item in items key={...}>...`
//! does not parse — views only exist inside components, @for is an attribute
//! directive (`@for item in items { ... }`), not an element tag, and `@motion`
//! is the motion-definition keyword, not an element attribute (triggers are
//! `@enter={m}` / `@hover={m}` / ...). The component wrapper below is the
//! minimal valid form with identical assertions (same adaptation as Task
//! 4.2's event-name tests).

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

/// Task 4.9 Step 1 — a motion trigger on an element inside an @for body must
/// warn, naming both "motion" and "@for". Regression: this used to compile
/// clean ("No issues found") while React drove only the last mounted item
/// and Flutter replayed one shared controller for all N items — a silent
/// per-backend divergence (perf M2).
#[test]
fn motion_in_for_warns() {
    let src = r#"import omniview;
component V {
    signal items: Array<String> = ["a", "b"];
    @motion m { from { opacity: 0 } to { opacity: 1 } }
    view {
        @for item in items key={item} { <div @enter={m}>{item}</div> }
    }
}"#;
    let (_, diags) = compile_with_diags(src, Target::OmniView);
    assert!(
        diags
            .iter()
            .any(|d| d.message.contains("motion") && d.message.contains("@for")),
        "must warn on motion-in-for: {:?}",
        diags
    );
}

/// The check must recurse: a trigger on a nested element (e.g. inside an
/// @if branch) of the @for body is still per-item — and non-enter triggers
/// (@hover) degrade the same way.
#[test]
fn motion_nested_in_for_body_warns() {
    let src = r#"import omniview;
component V {
    signal items: Array<String> = ["a", "b"];
    signal open: Bool = true;
    @motion m { from { opacity: 0 } to { opacity: 1 } }
    view {
        @for item in items key={item} {
            <div>
                @if open { <span @hover={m}>{item}</span> }
            </div>
        }
    }
}"#;
    let (_, diags) = compile_with_diags(src, Target::OmniView);
    assert!(
        diags
            .iter()
            .any(|d| d.message.contains("motion") && d.message.contains("@for")),
        "must warn on motion nested in for-body: {:?}",
        diags
    );
}

/// Motion OUTSIDE any @for must not warn — zero false positives (the
/// warnings-baseline gate depends on this).
#[test]
fn motion_outside_for_does_not_warn() {
    let src = r#"import omniview;
component V {
    signal items: Array<String> = ["a", "b"];
    @motion m { from { opacity: 0 } to { opacity: 1 } }
    view {
        <div @enter={m}>header</div>
        @for item in items key={item} { <div>{item}</div> }
    }
}"#;
    let (_, diags) = compile_with_diags(src, Target::OmniView);
    let motion_in_for: Vec<&Diagnostic> = diags
        .iter()
        .filter(|d| d.message.contains("motion") && d.message.contains("@for"))
        .collect();
    assert!(
        motion_in_for.is_empty(),
        "motion outside @for must not warn, got: {:?}",
        motion_in_for.iter().map(|d| &d.message).collect::<Vec<_>>()
    );
}

/// Task 4.9 three-target fixture (perf M2) — the warning must surface on
/// every P0 backend, not just OmniView: React used to drive only the last
/// mounted item, Flutter only failed downstream in dart analyze.
#[test]
fn motion_in_for_warns_on_all_p0_targets() {
    let src = r#"import omniview;
component V {
    signal items: Array<String> = ["a", "b"];
    @motion m { from { opacity: 0 } to { opacity: 1 } }
    view {
        @for item in items key={item} { <div @enter={m}>{item}</div> }
    }
}"#;
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
            .find(|d| d.message.contains("motion") && d.message.contains("@for"));
        assert!(
            warn.is_some(),
            "{name}: motion-in-for must warn, got: {:?}",
            diags.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
        let warn = warn.expect("checked above");
        assert_eq!(
            warn.severity,
            Severity::Warning,
            "{name}: motion-in-for must be a warning, not an error"
        );
    }
}
