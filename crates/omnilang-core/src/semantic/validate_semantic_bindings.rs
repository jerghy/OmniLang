//! Validate semantic ref bindings on elements.
//!
//! Semantic bindings (`focus={sig}`, `scroll-x={sig}`, `scroll-y={sig}`,
//! `value={sig}`) are compiler-recognized element attributes (semantic-level
//! DSL spec §2.2) that bind a bare signal to an element's ref. They must
//! appear only on elements with a `@ref` binding. This pass enforces that
//! contract after component lowering, where the bindings have been extracted
//! into `ElementIR.semantic_bindings`.
//!
//! Runs after component lowering (`ir/lower/mod.rs`), alongside
//! `validate_ref_ops`. Ref declaration checks (`ref name: Element` exists,
//! single binding per ref) are handled by `validate_refs`; this pass only
//! checks the semantic-binding → @ref relationship, so it deliberately does
//! not re-check whether a ref name is declared.

use crate::error::{Diagnostic, DiagnosticKind, Severity};
use crate::ir::{ComponentIR, ViewTree};

/// Validates semantic ref bindings on elements.
///
/// Semantic bindings (focus={sig}, scroll-x={sig}, scroll-y={sig}, value={sig})
/// are compiler-recognized element attributes that must appear only on
/// elements with a @ref binding. This pass enforces that contract.
pub fn validate_semantic_bindings(component: &ComponentIR, diagnostics: &mut Vec<Diagnostic>) {
    let Some(ref view) = component.view else {
        return;
    };

    walk_view_tree(view, component, diagnostics);
}

/// Recursively walk the view tree, checking `semantic_bindings` on every
/// element: each binding requires a `@ref` on the element itself, and the
/// binding kind must be applicable to the element's tag.
fn walk_view_tree(tree: &ViewTree, component: &ComponentIR, diagnostics: &mut Vec<Diagnostic>) {
    match tree {
        ViewTree::Element(el) => {
            // Check semantic_bindings
            for binding in el.semantic_bindings.keys() {
                match &el.ref_binding {
                    Some(_) => {
                        if !binding_applies_to_tag(binding, &el.tag) {
                            diagnostics.push(warning_diag(
                                component,
                                format!(
                                    "semantic binding '{binding}' is not applicable to element '<{}>'",
                                    el.tag
                                ),
                            ));
                        }
                    }
                    None => {
                        diagnostics.push(error_diag(
                            component,
                            format!("semantic binding '{binding}' requires @ref on this element"),
                        ));
                    }
                }
            }
            // Recurse
            for child in &el.children {
                walk_view_tree(child, component, diagnostics);
            }
        }
        ViewTree::Conditional(cond) => {
            for branch in &cond.branches {
                if let Some(ref body) = branch.body {
                    walk_view_tree(body, component, diagnostics);
                }
            }
            if let Some(ref fallback) = cond.fallback {
                walk_view_tree(fallback, component, diagnostics);
            }
        }
        ViewTree::ListRendering(list) => {
            if let Some(ref body) = list.body {
                walk_view_tree(body, component, diagnostics);
            }
            if let Some(ref empty) = list.empty_body {
                walk_view_tree(empty, component, diagnostics);
            }
        }
        ViewTree::Slot { ref fallback, .. } => {
            for child in fallback {
                walk_view_tree(child, component, diagnostics);
            }
        }
        ViewTree::Fragment(children) => {
            for child in children {
                walk_view_tree(child, component, diagnostics);
            }
        }
        ViewTree::Interpolation(_) | ViewTree::Text(_) => {}
    }
}

/// Whether a semantic binding is meaningful for an element tag.
///
/// Conservative, warning-only check: `focus` on never-focusable container
/// tags and `value` on tags without a value concept produce a warning.
/// Scroll bindings apply to any overflow-scrollable element, so they are
/// always accepted. Unknown tags (custom components) are accepted.
fn binding_applies_to_tag(binding: &str, tag: &str) -> bool {
    match binding {
        "focus" => !matches!(
            tag,
            "div"
                | "span"
                | "p"
                | "section"
                | "article"
                | "header"
                | "footer"
                | "nav"
                | "main"
                | "aside"
                | "ul"
                | "ol"
                | "li"
                | "form"
                | "label"
                | "img"
                | "h1"
                | "h2"
                | "h3"
                | "h4"
                | "h5"
                | "h6"
                | "table"
                | "tr"
                | "td"
                | "th"
                | "strong"
                | "em"
                | "b"
                | "i"
                | "blockquote"
                | "figure"
                | "figcaption"
                | "br"
                | "hr"
        ),
        "value" => matches!(
            tag,
            "input"
                | "textarea"
                | "select"
                | "option"
                | "progress"
                | "meter"
                | "li"
                | "button"
                | "data"
                | "param"
        ),
        // scroll-x / scroll-y: any element can scroll via overflow.
        _ => true,
    }
}

/// Build an error diagnostic at the component's source location, following
/// the convention used by `validate_refs` / `validate_ref_ops`.
fn error_diag(component: &ComponentIR, message: String) -> Diagnostic {
    make_diag(component, Severity::Error, message)
}

/// Build a warning diagnostic at the component's source location.
fn warning_diag(component: &ComponentIR, message: String) -> Diagnostic {
    make_diag(component, Severity::Warning, message)
}

/// Shared diagnostic construction: E0006 (validation error), positioned at
/// the component's `source_span` / `source_file`.
fn make_diag(component: &ComponentIR, severity: Severity, message: String) -> Diagnostic {
    let (line, col) = component
        .source_span
        .map(|s| (s.line, s.col))
        .unwrap_or((0, 0));
    Diagnostic {
        severity,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{
        BindingIR, ConstructionKind, ElementIR, ExprIR, ListRenderingIntent, RefDeclIR,
    };
    use rustc_hash::FxHashMap;

    /// Build a minimal ElementIR with a ref binding and semantic bindings.
    fn make_el(tag: &str, ref_binding: Option<&str>, semantic: &[(&str, &str)]) -> ElementIR {
        ElementIR {
            tag: tag.to_string(),
            ref_binding: ref_binding.map(String::from),
            semantic_bindings: semantic
                .iter()
                .map(|(k, v)| (k.to_string(), ExprIR::ident(v)))
                .collect(),
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
        }
    }

    /// Build a minimal ComponentIR with the given ref declarations and a
    /// view wrapping the given elements.
    fn make_component(name: &str, ref_names: &[&str], els: Vec<ViewTree>) -> ComponentIR {
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
            inline_stores: vec![],
            events: vec![],
            slots: vec![],
            styles: vec![],
            source_span: None,
        }
    }

    #[test]
    fn semantic_binding_without_ref_errors() {
        // focus={sig} on an element WITHOUT @ref → error
        let comp = make_component(
            "Test",
            &[],
            vec![ViewTree::Element(Box::new(make_el(
                "input",
                None,
                &[("focus", "active")],
            )))],
        );
        let mut diagnostics = Vec::new();
        validate_semantic_bindings(&comp, &mut diagnostics);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].severity, Severity::Error);
        assert!(
            diagnostics[0].message.contains("requires @ref"),
            "unexpected message: {}",
            diagnostics[0].message
        );
    }

    #[test]
    fn semantic_binding_with_ref_passes() {
        // focus={sig} on an element WITH @ref → no error
        let comp = make_component(
            "Test",
            &["inputBox"],
            vec![ViewTree::Element(Box::new(make_el(
                "input",
                Some("inputBox"),
                &[("focus", "active")],
            )))],
        );
        let mut diagnostics = Vec::new();
        validate_semantic_bindings(&comp, &mut diagnostics);
        assert!(
            diagnostics.is_empty(),
            "expected no diagnostics, got: {:?}",
            diagnostics
        );
    }

    #[test]
    fn semantic_binding_warns_on_wrong_element_type() {
        // focus on a non-focusable div → warning (not error); the ref itself
        // is declared and bound, so no other diagnostics.
        let comp = make_component(
            "Test",
            &["box"],
            vec![ViewTree::Element(Box::new(make_el(
                "div",
                Some("box"),
                &[("focus", "active")],
            )))],
        );
        let mut diagnostics = Vec::new();
        validate_semantic_bindings(&comp, &mut diagnostics);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].severity, Severity::Warning);
        assert!(
            diagnostics[0]
                .message
                .contains("not applicable to element '<div>'"),
            "unexpected message: {}",
            diagnostics[0].message
        );
    }

    #[test]
    fn semantic_binding_value_on_valid_tag_passes() {
        // value={sig} on an input → no warning
        let comp = make_component(
            "Test",
            &["field"],
            vec![ViewTree::Element(Box::new(make_el(
                "input",
                Some("field"),
                &[("value", "text")],
            )))],
        );
        let mut diagnostics = Vec::new();
        validate_semantic_bindings(&comp, &mut diagnostics);
        assert!(
            diagnostics.is_empty(),
            "expected no diagnostics, got: {:?}",
            diagnostics
        );
    }

    #[test]
    fn semantic_binding_inside_list_body_is_checked() {
        // Semantic bindings nested inside a @for body must also be validated.
        let comp = ComponentIR {
            name: "TestComp".into(),
            refs: vec![],
            view: Some(ViewTree::ListRendering(Box::new(ListRenderingIntent {
                iterable: ExprIR::ident("items"),
                item_binding: BindingIR {
                    name: "item".into(),
                    r#type: None,
                },
                key_expr: None,
                body: Some(ViewTree::Element(Box::new(make_el(
                    "input",
                    None,
                    &[("focus", "active")],
                )))),
                empty_body: None,
                scroll_hint: false,
            }))),
            source_file: Some("test.omni".into()),
            ..ComponentIR::default()
        };
        let mut diagnostics = Vec::new();
        validate_semantic_bindings(&comp, &mut diagnostics);
        assert!(
            diagnostics
                .iter()
                .any(|d| d.message.contains("requires @ref")),
            "expected error for semantic binding inside @for body, got: {:?}",
            diagnostics
        );
    }

    #[test]
    fn semantic_binding_with_ref_and_type_ok_no_diagnostics() {
        // scroll-x on a ref'd scrollable div → no diagnostics at all
        let comp = make_component(
            "Test",
            &["pane"],
            vec![ViewTree::Element(Box::new(make_el(
                "div",
                Some("pane"),
                &[("scroll-x", "pos")],
            )))],
        );
        let mut diagnostics = Vec::new();
        validate_semantic_bindings(&comp, &mut diagnostics);
        assert!(
            diagnostics.is_empty(),
            "expected no diagnostics, got: {:?}",
            diagnostics
        );
    }
}
