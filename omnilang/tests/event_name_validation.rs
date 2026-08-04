//! Event-name validation integration tests (Task 4.2) — unknown event names
//! must warn with the nearest legal suggestion; known events must stay
//! silent; the warning must surface on every P0 target.
//!
//! The pass itself lives in `omnilang-core/src/semantic/validate_event_names.rs`
//! (runs post-IR-merge so program-wide custom `event` declarations are
//! visible); these tests exercise it through the REAL compile pipeline
//! (`compile_multi_internal`), not the lower-only shortcut, so the pass is
//! actually in the path.

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

/// Task 4.2 Step 1 — unknown event name must warn AND name the event AND
/// suggest the nearest legal event. Regression: `@onTotallyBogus={f}` used to
/// compile clean ("No issues found") and emit the bogus handler verbatim to
/// React — a silent no-op at runtime (ai HIGH).
///
/// NOTE: the plan's literal source `view V { ... }` does not parse (views
/// only exist inside components — "Unexpected token at top level: View");
/// the component wrapper below is the minimal valid form with identical
/// assertions.
#[test]
fn unknown_event_warns_with_suggestion() {
    let src = r#"import omniview; component V { view { <div @onTotallyBogus={f}/> } }"#;
    let (_, diags) = compile_with_diags(src, Target::React);
    let d = diags
        .iter()
        .find(|d| d.severity == Severity::Warning)
        .expect("must warn");
    assert!(
        d.message.contains("onTotallyBogus"),
        "must name the event: {}",
        d.message
    );
    assert!(
        d.message.contains("@onPress")
            || d.message.contains("@onClick")
            || d.message.contains("@onTap"),
        "missing nearest suggestion: {}",
        d.message
    );
}

/// Known canonical events must never warn (zero false positives).
#[test]
fn known_events_do_not_warn() {
    let src = r#"
import omniview;
component KnownEvents {
    fn handle(e) {}
    view {
        <div
            @onClick={handle}
            @onPress={handle}
            @onTap={handle}
            @onDoubleClick={handle}
            @onKeyDown={handle}
            @onKeyUp={handle}
            @onChange={handle}
            @onInput={handle}
            @onFocus={handle}
            @onBlur={handle}
            @onMouseEnter={handle}
            @onMouseLeave={handle}
            @onScroll={handle}
            @onWheel={handle}
            @onTouchStart={handle}
            @onTouchEnd={handle}
            @onContextMenu={handle}
            @onDragStart={handle}
            @onDrop={handle}
            @onCopy={handle}
            @onCut={handle}
            @onPaste={handle}
            @onAnimationEnd={handle}
            @onTransitionEnd={handle}
        />
    }
}
"#;
    let (_, diags) = compile_with_diags(src, Target::React);
    let event_warnings: Vec<&Diagnostic> = diags
        .iter()
        .filter(|d| d.severity == Severity::Warning)
        .collect();
    assert!(
        event_warnings.is_empty(),
        "known events must not warn, got: {:?}",
        event_warnings
            .iter()
            .map(|d| &d.message)
            .collect::<Vec<_>>()
    );
}

/// Custom `event` declarations on component instances are legal bindings —
/// must NOT warn (the corpus binds `@onAdd`/`@onSwitch`/... this way).
#[test]
fn declared_custom_event_does_not_warn() {
    let src = r#"
import omniview;
component Child { event onCustom: String; view { <div/> } }
component App {
    fn handle(e) {}
    view { <Child @onCustom={handle} /> }
}
"#;
    let (_, diags) = compile_with_diags(src, Target::React);
    let event_warnings: Vec<&Diagnostic> = diags
        .iter()
        .filter(|d| d.severity == Severity::Warning)
        .collect();
    assert!(
        event_warnings.is_empty(),
        "declared custom event must not warn, got: {:?}",
        event_warnings
            .iter()
            .map(|d| &d.message)
            .collect::<Vec<_>>()
    );
}

/// Task 4.2 three-target fixture (ai HIGH) — the unknown-event warning must
/// surface on every P0 backend: React used to emit the bogus handler
/// verbatim, Flutter only failed downstream in dart analyze, OmniView
/// silently dropped it. All three must now warn at compile time.
#[test]
fn unknown_event_warns_on_all_p0_targets() {
    let src = r#"
import omniview;
component Bogus {
    fn handle(e) {}
    view { <div @onTotallyBogus={handle} /> }
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
            .find(|d| d.severity == Severity::Warning && d.message.contains("onTotallyBogus"));
        assert!(
            warn.is_some(),
            "{name}: unknown event must warn, got: {:?}",
            diags.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }
}

/// Corpus gate (Task 4.2 acceptance): the whole learn-omniview showcase
/// compiles through the REAL multi-file pipeline with ZERO event-name
/// warnings — the corpus binds only canonical `@onXxx` DOM events and
/// declared custom events (domain-verified), so any event-name warning here
/// is a false positive in the pass (e.g. a misspelled whitelist entry or a
/// cross-file custom-event name the merged-IR pass failed to see).
#[test]
fn learn_omniview_corpus_has_no_event_name_warnings() {
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
    let event_warnings: Vec<&Diagnostic> = result
        .errors
        .iter()
        .filter(|d| d.severity == Severity::Warning && d.message.contains("未知事件名"))
        .collect();
    assert!(
        event_warnings.is_empty(),
        "corpus must compile with zero event-name warnings, got: {:?}",
        event_warnings
            .iter()
            .map(|d| (&d.message, &d.file))
            .collect::<Vec<_>>()
    );
}
