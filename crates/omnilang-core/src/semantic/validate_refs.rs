//! Validate ref declarations and bindings for all components.
//!
//! Checks:
//! 1. Every ref name used in @ref={name} has a corresponding `ref name: Element` declaration.
//! 2. Each ref declaration is bound to exactly one element (no duplicates, no orphans).

use crate::error::{Diagnostic, DiagnosticKind, Severity};
use crate::ir::{ComponentIR, IRProgram, ViewTree};
use rustc_hash::FxHashSet;

/// Validate ref declarations and bindings across all components in the IR program.
///
/// Returns a list of diagnostics for each ref-related issue found.
pub fn validate_refs(program: &IRProgram) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    for (_comp_name, comp) in program.iter_all_components() {
        diagnostics.extend(validate_component_refs(comp));
    }

    diagnostics
}

/// Validate ref declarations and bindings for a single component.
fn validate_component_refs(comp: &ComponentIR) -> Vec<Diagnostic> {
    let mut warnings = Vec::new();

    // Collect declared ref names
    let declared: FxHashSet<&str> = comp.refs.iter().map(|r| r.name.as_str()).collect();

    // Collect ref bindings from the view tree
    let mut bindings: Vec<&str> = Vec::new();
    if let Some(ref view) = comp.view {
        collect_ref_bindings(view, &mut bindings);
        warn_refs_in_for_bodies(view, comp, false, &mut warnings);
    }

    let (line, col) = comp.source_span.map(|s| (s.line, s.col)).unwrap_or((0, 0));

    // Check: every binding must reference a declared ref
    for &binding in &bindings {
        if !declared.contains(binding) {
            warnings.push(Diagnostic {
                severity: Severity::Warning,
                kind: DiagnosticKind::E0006,
                message: format!(
                    "ref '{}' is used in @ref={{}} but no `ref {}: Element` declaration found",
                    binding, binding
                ),
                file: comp.source_file.clone(),
                line,
                col,
                end_line: None,
                end_col: None,
                suggestion: Some(format!("add 'ref {}: Element' to the component", binding)),
            });
        }
    }

    // Check: every declared ref should be bound somewhere (warning, not error)
    for decl in &declared {
        if !bindings.contains(decl) {
            warnings.push(Diagnostic {
                severity: Severity::Warning,
                kind: DiagnosticKind::E0006,
                message: format!(
                    "ref '{}' is declared but never bound to an element with @ref={{{}}}",
                    decl, decl
                ),
                file: comp.source_file.clone(),
                line,
                col,
                end_line: None,
                end_col: None,
                suggestion: Some(format!(
                    "add @ref={{{}}} to the element you want to reference",
                    decl
                )),
            });
        }
    }

    // Check: no duplicate bindings for the same ref name
    let mut seen: FxHashSet<&str> = FxHashSet::default();
    for &binding in &bindings {
        if !seen.insert(binding) {
            warnings.push(Diagnostic {
                severity: Severity::Warning,
                kind: DiagnosticKind::E0006,
                message: format!(
                    "ref '{}' is bound to multiple elements — each ref must be bound to exactly one element",
                    binding
                ),
                file: comp.source_file.clone(),
                line,
                col,
                end_line: None,
                end_col: None,
                suggestion: None,
            });
        }
    }

    warnings
}

/// Recursively collect all ref_binding values from a view tree.
fn collect_ref_bindings<'a>(view: &'a ViewTree, out: &mut Vec<&'a str>) {
    match view {
        ViewTree::Element(el) => {
            if let Some(ref name) = el.ref_binding {
                out.push(name.as_str());
            }
            for child in &el.children {
                collect_ref_bindings(child, out);
            }
        }
        ViewTree::Conditional(cond) => {
            for branch in &cond.branches {
                if let Some(ref body) = branch.body {
                    collect_ref_bindings(body, out);
                }
            }
            if let Some(ref fallback) = cond.fallback {
                collect_ref_bindings(fallback, out);
            }
        }
        ViewTree::ListRendering(list) => {
            if let Some(ref body) = list.body {
                collect_ref_bindings(body, out);
            }
            if let Some(ref empty) = list.empty_body {
                collect_ref_bindings(empty, out);
            }
        }
        ViewTree::Fragment(children) => {
            for child in children {
                collect_ref_bindings(child, out);
            }
        }
        ViewTree::Interpolation(_) | ViewTree::Text(_) | ViewTree::Slot { .. } => {}
    }
}

/// Emit warnings for @ref bindings inside @for loop bodies.
/// @ref inside @for has undefined semantics (Phase 2) — the ref would bind
/// to the template element or last clone, not the per-item element.
fn warn_refs_in_for_bodies(
    view: &ViewTree,
    comp: &ComponentIR,
    in_for_template: bool,
    warnings: &mut Vec<Diagnostic>,
) {
    let (line, col) = comp.source_span.map(|s| (s.line, s.col)).unwrap_or((0, 0));

    match view {
        ViewTree::Element(el) => {
            if in_for_template {
                if let Some(ref name) = el.ref_binding {
                    warnings.push(Diagnostic {
                        severity: Severity::Warning,
                        kind: DiagnosticKind::E0006,
                        message: format!(
                            "ref '{}' is used inside @for — each @ref inside a loop binds to the \
                             template element, not individual items. @ref inside @for is not yet \
                             supported (planned for Phase 2). Move the @ref outside the loop or \
                             use a different approach.",
                            name
                        ),
                        file: comp.source_file.clone(),
                        line,
                        col,
                        end_line: None,
                        end_col: None,
                        suggestion: Some(
                            "move @ref outside the @for loop, or wait for Phase 2 @for+@ref support"
                                .to_string(),
                        ),
                    });
                }
            }
            for child in &el.children {
                warn_refs_in_for_bodies(child, comp, in_for_template, warnings);
            }
        }
        ViewTree::Conditional(cond) => {
            for branch in &cond.branches {
                if let Some(ref body) = branch.body {
                    warn_refs_in_for_bodies(body, comp, in_for_template, warnings);
                }
            }
            if let Some(ref fallback) = cond.fallback {
                warn_refs_in_for_bodies(fallback, comp, in_for_template, warnings);
            }
        }
        ViewTree::ListRendering(list) => {
            if let Some(ref body) = list.body {
                warn_refs_in_for_bodies(body, comp, true, warnings);
            }
            if let Some(ref empty) = list.empty_body {
                warn_refs_in_for_bodies(empty, comp, in_for_template, warnings);
            }
        }
        ViewTree::Fragment(children) => {
            for child in children {
                warn_refs_in_for_bodies(child, comp, in_for_template, warnings);
            }
        }
        ViewTree::Interpolation(_) | ViewTree::Text(_) | ViewTree::Slot { .. } => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{
        BindingIR, ConstructionKind, ElementIR, ExprIR, ListRenderingIntent, RefDeclIR,
    };
    use rustc_hash::FxHashMap;

    /// Helper to build a minimal ElementIR with only the fields we need.
    fn make_el(tag: &str, ref_binding: Option<&str>) -> ElementIR {
        ElementIR {
            tag: tag.to_string(),
            ref_binding: ref_binding.map(String::from),
            children: vec![],
            props: FxHashMap::default(),
            bindings: FxHashMap::default(),
            events: FxHashMap::default(),
            style_classes: None,
            inline_style: None,
            construction_kind: ConstructionKind::Normal,
            class_map_bindings: FxHashMap::default(),
            source_span: None,
            motion: None,
            scroll: None,
            drag: None,
            semantic_bindings: FxHashMap::default(),
        }
    }

    /// Build a minimal ComponentIR with given ref names and bindings.
    fn make_component(name: &str, ref_names: &[&str], bindings: &[&str]) -> ComponentIR {
        let els: Vec<ViewTree> = bindings
            .iter()
            .map(|&binding_name| ViewTree::Element(Box::new(make_el("div", Some(binding_name)))))
            .collect();

        ComponentIR {
            name: name.to_string(),
            refs: ref_names
                .iter()
                .map(|n| RefDeclIR {
                    name: n.to_string(),
                    element_tag: None,
                })
                .collect(),
            view: if els.is_empty() {
                None
            } else {
                Some(ViewTree::Fragment(els))
            },
            source_file: None,
            props: FxHashMap::default(),
            prop_defaults: FxHashMap::default(),
            signals: vec![],
            computeds: vec![],
            effects: vec![],
            lifecycles: vec![],
            fns: vec![],
            store_refs: vec![],
            events: vec![],
            styles: vec![],
            source_span: None,
        }
    }

    #[test]
    fn test_orphan_ref_binding_warns() {
        // ref_binding "btn" but no refs declaration
        let comp = make_component("Test", &[], &["btn"]);
        let warnings = validate_component_refs(&comp);
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].message.contains("btn"));
        assert!(warnings[0]
            .message
            .contains("no `ref btn: Element` declaration"));
    }

    #[test]
    fn test_unbound_ref_decl_warns() {
        // refs has "btn" but no element binds to it
        let comp = make_component("Test", &["btn"], &[]);
        let warnings = validate_component_refs(&comp);
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].message.contains("btn"));
        assert!(warnings[0].message.contains("declared but never bound"));
    }

    #[test]
    fn test_duplicate_ref_binding_warns() {
        // Two elements both bind to "btn"
        let comp = ComponentIR {
            name: "Test".to_string(),
            refs: vec![RefDeclIR {
                element_tag: None,
                name: "btn".to_string(),
            }],
            view: Some(ViewTree::Fragment(vec![
                ViewTree::Element(Box::new(make_el("div", Some("btn")))),
                ViewTree::Element(Box::new(make_el("span", Some("btn")))),
            ])),
            source_file: None,
            props: FxHashMap::default(),
            prop_defaults: FxHashMap::default(),
            signals: vec![],
            computeds: vec![],
            effects: vec![],
            lifecycles: vec![],
            fns: vec![],
            store_refs: vec![],
            events: vec![],
            styles: vec![],
            source_span: None,
        };
        let warnings = validate_component_refs(&comp);
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].message.contains("bound to multiple elements"));
    }

    #[test]
    fn test_valid_ref_passes() {
        // refs has "btn", element binds to "btn"
        let comp = make_component("Test", &["btn"], &["btn"]);
        let warnings = validate_component_refs(&comp);
        assert!(
            warnings.is_empty(),
            "expected no warnings, got: {:?}",
            warnings
        );
    }

    #[test]
    fn test_multiple_refs_all_valid() {
        let comp = make_component("Test", &["btn", "input"], &["btn", "input"]);
        let warnings = validate_component_refs(&comp);
        assert!(
            warnings.is_empty(),
            "expected no warnings, got: {:?}",
            warnings
        );
    }

    #[test]
    fn test_mixed_issues() {
        // "btn" correctly bound, "input" orphan binding (not declared),
        // "header" declared but unbound, "btn2" duplicate
        let comp = ComponentIR {
            name: "Test".to_string(),
            refs: vec![
                RefDeclIR {
                    element_tag: None,
                    name: "btn".to_string(),
                },
                RefDeclIR {
                    element_tag: None,
                    name: "header".to_string(),
                },
            ],
            view: Some(ViewTree::Fragment(vec![
                ViewTree::Element(Box::new(make_el("button", Some("btn")))),
                ViewTree::Element(Box::new(make_el("input", Some("input")))),
                ViewTree::Element(Box::new(make_el("div", Some("btn2")))),
                ViewTree::Element(Box::new(make_el("span", Some("btn2")))),
            ])),
            source_file: None,
            props: FxHashMap::default(),
            prop_defaults: FxHashMap::default(),
            signals: vec![],
            computeds: vec![],
            effects: vec![],
            lifecycles: vec![],
            fns: vec![],
            store_refs: vec![],
            events: vec![],
            styles: vec![],
            source_span: None,
        };
        let warnings = validate_component_refs(&comp);
        // Expect: 1 orphan "input" + 2 orphan "btn2" (both occurrences)
        //         + 1 unbound "header" + 1 duplicate "btn2" = 5
        assert_eq!(warnings.len(), 5);
        let msgs: Vec<&str> = warnings.iter().map(|w| w.message.as_str()).collect();
        assert!(
            msgs.iter()
                .any(|m| m.contains("'input'") && m.contains("no `ref")),
            "expected orphan binding warning for 'input'"
        );
        assert!(
            msgs.iter()
                .any(|m| m.contains("'header'") && m.contains("declared but never bound")),
            "expected unbound declaration warning for 'header'"
        );
        assert!(
            msgs.iter()
                .any(|m| m.contains("'btn2'") && m.contains("bound to multiple elements")),
            "expected duplicate binding warning for 'btn2'"
        );
    }

    #[test]
    fn warns_on_ref_inside_for_loop() {
        let comp = ComponentIR {
            name: "TestComp".into(),
            refs: vec![RefDeclIR {
                element_tag: None,
                name: "box".into(),
            }],
            view: Some(ViewTree::ListRendering(Box::new(ListRenderingIntent {
                iterable: ExprIR::ident("items"),
                item_binding: BindingIR {
                    name: "item".into(),
                    r#type: None,
                },
                key_expr: None,
                body: Some(ViewTree::Element(Box::new(ElementIR {
                    tag: "div".into(),
                    ref_binding: Some("box".into()), // ← ref inside @for!
                    children: vec![],
                    props: FxHashMap::default(),
                    bindings: FxHashMap::default(),
                    events: FxHashMap::default(),
                    style_classes: None,
                    inline_style: None,
                    construction_kind: ConstructionKind::Normal,
                    class_map_bindings: FxHashMap::default(),
                    source_span: None,
                    motion: None,
                    scroll: None,
                    drag: None,
                    semantic_bindings: FxHashMap::default(),
                }))),
                empty_body: None,
            }))),
            source_file: Some("test.omni".into()),
            ..ComponentIR::default()
        };

        let warnings = validate_component_refs(&comp);

        // Should have a warning about @ref inside @for
        assert!(
            warnings.iter().any(|w| w.message.contains("@for")),
            "expected a warning about @ref inside @for, got: {:?}",
            warnings
        );
        let for_warning = warnings
            .iter()
            .find(|w| w.message.contains("@for"))
            .expect("expected a warning about @ref inside @for");
        assert!(
            for_warning.message.contains("inside @for"),
            "warning should mention @for: {}",
            for_warning.message
        );
        assert!(
            for_warning.suggestion.is_some(),
            "warning should include a suggestion"
        );
    }
}
