//! Validate element event names against the shared canonical event table
//! (Task 4.2, D-B) — unknown event names warn with the nearest legal
//! suggestion instead of compiling clean and emitting a dead handler.
//!
//! Failure mode this closes (ai HIGH): `@onTotallyBogus={f}` used to report
//! "No issues found" — React emitted `onTotallyBogus` verbatim (runtime
//! no-op), Flutter only failed later in `dart analyze` with no .omni-side
//! hint. Every backend now gets a compile-time warning naming the event and
//! suggesting the nearest legal spelling.
//!
//! Whitelist = the canonical table's accepted names (the `press`/`click`/
//! `tap` alias families are built into `canonical_event_name`, so aliases
//! are legal by construction) PLUS custom events declared anywhere in the
//! program (`event onAdd: String;` — bound as `@onAdd={...}` on component
//! instances by the corpus). Events on component-instance elements are
//! custom-event bindings and are not validated against the DOM table.
//!
//! Runs post-IR-merge (see `analyse.rs` Phase 3d) so cross-file custom event
//! declarations and component names are visible — the per-file `lower()`
//! stage cannot distinguish an imported component's custom-event binding
//! from a typo'd DOM event.

use crate::codegen::shared::event_names::{canonical_event_name, nearest_event_suggestion};
use crate::error::{Diagnostic, DiagnosticKind, Severity};
use crate::ir::{ComponentIR, IRProgram, ViewTree};
use crate::lexer::Span;
use rustc_hash::FxHashSet;

/// Validate every element event name in every component view.
pub fn validate_event_names(ir: &IRProgram, diagnostics: &mut Vec<Diagnostic>) {
    // Program-wide sets: component tags (elements with these tags carry
    // custom-event bindings, not DOM events) and declared custom events
    // (`event onX` — legal to bind regardless of the DOM table).
    let mut component_names: FxHashSet<&str> = FxHashSet::default();
    let mut custom_events: FxHashSet<&str> = FxHashSet::default();
    for module in ir.modules.values() {
        for component in module.components.values() {
            component_names.insert(component.name.as_str());
            for e in &component.events {
                custom_events.insert(e.name.as_str());
            }
        }
    }

    for module in ir.modules.values() {
        for component in module.components.values() {
            if let Some(ref view) = component.view {
                walk_view_tree(
                    view,
                    component,
                    &component_names,
                    &custom_events,
                    diagnostics,
                );
            }
        }
    }
}

/// Recursively walk the view tree, validating each element's event names.
fn walk_view_tree(
    tree: &ViewTree,
    component: &ComponentIR,
    component_names: &FxHashSet<&str>,
    custom_events: &FxHashSet<&str>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    match tree {
        ViewTree::Element(el) => {
            // Component instantiations bind custom events — skip the DOM
            // table check (matches codegen's component_map behavior).
            if !component_names.contains(el.tag.as_str()) {
                for name in el.events.keys() {
                    if canonical_event_name(name).is_none()
                        && !custom_events.contains(name.as_str())
                    {
                        diagnostics.push(warning_diag(
                            component,
                            el.source_span,
                            match nearest_event_suggestion(name) {
                                Some(s) => format!("未知事件名 \"@{name}\"——最近合法建议 \"@{s}\""),
                                None => format!("未知事件名 \"@{name}\""),
                            },
                        ));
                    }
                }
            }
            for child in &el.children {
                walk_view_tree(
                    child,
                    component,
                    component_names,
                    custom_events,
                    diagnostics,
                );
            }
        }
        ViewTree::Conditional(cond) => {
            for branch in &cond.branches {
                if let Some(ref body) = branch.body {
                    walk_view_tree(body, component, component_names, custom_events, diagnostics);
                }
            }
            if let Some(ref fallback) = cond.fallback {
                walk_view_tree(
                    fallback,
                    component,
                    component_names,
                    custom_events,
                    diagnostics,
                );
            }
        }
        ViewTree::ListRendering(list) => {
            if let Some(ref body) = list.body {
                walk_view_tree(body, component, component_names, custom_events, diagnostics);
            }
            if let Some(ref empty) = list.empty_body {
                walk_view_tree(
                    empty,
                    component,
                    component_names,
                    custom_events,
                    diagnostics,
                );
            }
        }
        ViewTree::Slot { ref fallback, .. } => {
            for child in fallback {
                walk_view_tree(
                    child,
                    component,
                    component_names,
                    custom_events,
                    diagnostics,
                );
            }
        }
        ViewTree::Fragment(children) => {
            for child in children {
                walk_view_tree(
                    child,
                    component,
                    component_names,
                    custom_events,
                    diagnostics,
                );
            }
        }
        ViewTree::Interpolation(_) | ViewTree::Text(_) => {}
    }
}

/// Build a warning diagnostic (E0006) positioned at the offending element's
/// source span when available (falls back to the component span).
fn warning_diag(component: &ComponentIR, el_span: Option<Span>, message: String) -> Diagnostic {
    let (line, col) = el_span
        .or(component.source_span)
        .map(|s| (s.line, s.col))
        .unwrap_or((0, 0));
    Diagnostic {
        severity: Severity::Warning,
        kind: DiagnosticKind::E0006,
        message,
        file: component.source_file.clone(),
        line,
        col,
        end_line: None,
        end_col: None,
        suggestion: None,
    }
}
