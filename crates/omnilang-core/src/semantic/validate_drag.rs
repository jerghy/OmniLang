//! Validate `@drag` / `@dragEnd` draggable-intent bindings (ADR-073).
//!
//! The directive expresses intent ("element is draggable") — the compiler
//! wires pointer capture per target (React setPointerCapture / OmniView
//! capturePointer / Flutter GestureDetector pan; ADR-071 decision 3). The
//! binding contract enforced here:
//!
//! 1. The element must carry a `@ref` binding — the target's capture
//!    mechanism needs the element handle (and the Phase 1 direct-write
//!    channel keys off `ref_binding`).
//! 2. The handler names must resolve to component functions (parser already
//!    guarantees bare names; this pass checks existence, because lowering
//!    stores the name verbatim and an unresolved name would silently emit a
//!    dead callback).
//! 3. A handler takes at most one parameter — the DragIntent event object
//!    `{ x, y, dx, dy, pointerId, startX, startY }` (design doc
//!    ref-high-frequency-direct-channel §5.2). Zero parameters is legal
//!    (matches the general event-handler contract: arity is not enforced
//!    for `@onX` handlers either); two or more is a hard error.
//!
//! Runs after component lowering (`ir/lower/mod.rs`) via `run_ir_passes`,
//! alongside `validate_semantic_bindings` / `validate_a11y` — the walker
//! pattern is shared (`ir::walk::walk_view` with a per-element callback).

use crate::error::{Diagnostic, DiagnosticKind, Severity};
use crate::ir::walk::walk_view;
use crate::ir::{ComponentIR, EventHandlerIR};
use crate::lexer::Span;

/// Validates `@drag` / `@dragEnd` bindings on all elements of a component.
pub fn validate_drag(component: &ComponentIR, diagnostics: &mut Vec<Diagnostic>) {
    let Some(ref view) = component.view else {
        return;
    };

    walk_view(view, &mut |el| {
        let Some(drag) = &el.drag_binding else {
            return;
        };

        // ① @drag requires @ref on the same element — the target's capture
        // mechanism needs the element handle.
        if el.ref_binding.is_none() {
            diagnostics.push(make_diag(
                component,
                el.source_span,
                Severity::Error,
                "@drag requires @ref on this element (e.g. @ref={box})",
            ));
        }

        // ②/③ handler existence + arity.
        for (slot, handler) in [("drag", &drag.on_drag)]
            .into_iter()
            .chain(drag.on_drag_end.as_ref().map(|h| ("dragEnd", h)))
        {
            if let EventHandlerIR::FnRef { r#ref, .. } = handler {
                let func = component.fns.iter().find(|f| f.name == *r#ref);
                if func.is_none() {
                    diagnostics.push(make_diag(
                        component,
                        el.source_span,
                        Severity::Error,
                        format!(
                            "@{slot} handler '{}' is not defined in this component",
                            r#ref
                        ),
                    ));
                    continue;
                }
                let func = func.expect("checked above");
                if func.params.len() > 1 {
                    diagnostics.push(make_diag(
                        component,
                        el.source_span,
                        Severity::Error,
                        format!(
                            "@{slot} handler '{}' takes at most one parameter (the drag event data {{ x, y, dx, dy, pointerId, startX, startY }})",
                            r#ref
                        ),
                    ));
                }
            }
        }
    });
}

/// Build a diagnostic at the offending element's source location (falling
/// back to the component's location), following the convention used by
/// `validate_semantic_bindings`.
fn make_diag(
    component: &ComponentIR,
    span: Option<Span>,
    severity: Severity,
    message: impl Into<String>,
) -> Diagnostic {
    let (line, col) = span
        .or(component.source_span)
        .map(|s| (s.line, s.col))
        .unwrap_or((0, 0));
    Diagnostic {
        severity,
        kind: DiagnosticKind::E0006,
        message: message.into(),
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
    use crate::ir::{BindingIR, ElementIR, ListRenderingIntent, RefDeclIR, ViewTree, Visibility};
    use rustc_hash::FxHashMap;

    /// Build a minimal ComponentIR with the given ref declarations, fns and
    /// a view wrapping the given elements.
    fn make_component(
        name: &str,
        ref_names: &[&str],
        fns: &[&str],
        els: Vec<ViewTree>,
    ) -> ComponentIR {
        ComponentIR {
            form_intent: None,
            load_intent: None,
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
            fns: fns
                .iter()
                .map(|n| crate::ir::logic::FunctionIR {
                    name: n.to_string(),
                    visibility: Visibility::Private,
                    params: vec![],
                    return_type: None,
                    body: crate::ir::BlockIR { statements: vec![] },
                    is_async: false,
                    source_span: None,
                })
                .collect(),
            source_file: None,
            props: FxHashMap::default(),
            prop_defaults: FxHashMap::default(),
            signals: vec![],
            computeds: vec![],
            effects: vec![],
            lifecycles: vec![],
            store_refs: vec![],
            inline_stores: vec![],
            events: vec![],
            slots: vec![],
            styles: vec![],
            source_span: None,
        }
    }

    /// Build an ElementIR with a drag binding (and optional ref).
    fn make_el(
        tag: &str,
        ref_binding: Option<&str>,
        on_drag: &str,
        on_drag_end: Option<&str>,
    ) -> ElementIR {
        ElementIR {
            tag: tag.to_string(),
            ref_binding: ref_binding.map(String::from),
            drag_binding: Some(crate::ir::DragBindingIR {
                on_drag: EventHandlerIR::FnRef {
                    r#ref: on_drag.to_string(),
                    params: vec![],
                    event_data: None,
                    used_fields: None,
                    needs_prevent_default: false,
                    needs_stop_propagation: false,
                },
                on_drag_end: on_drag_end.map(|n| EventHandlerIR::FnRef {
                    r#ref: n.to_string(),
                    params: vec![],
                    event_data: None,
                    used_fields: None,
                    needs_prevent_default: false,
                    needs_stop_propagation: false,
                }),
            }),
            children: vec![],
            props: FxHashMap::default(),
            bindings: FxHashMap::default(),
            events: FxHashMap::default(),
            style_classes: None,
            inline_style: None,
            construction_kind: crate::ir::ConstructionKind::Normal,
            class_map_bindings: FxHashMap::default(),
            source_span: None,
            motion: None,
            semantic_bindings: FxHashMap::default(),
            a11y: None,
        }
    }

    #[test]
    fn drag_with_ref_and_existing_handlers_passes() {
        let comp = make_component(
            "Test",
            &["box"],
            &["move", "end"],
            vec![ViewTree::Element(Box::new(make_el(
                "div",
                Some("box"),
                "move",
                Some("end"),
            )))],
        );
        let mut diagnostics = Vec::new();
        validate_drag(&comp, &mut diagnostics);
        assert!(
            diagnostics.is_empty(),
            "expected no diagnostics, got: {:?}",
            diagnostics
        );
    }

    #[test]
    fn drag_without_ref_errors() {
        let comp = make_component(
            "Test",
            &[],
            &["move"],
            vec![ViewTree::Element(Box::new(make_el(
                "div", None, "move", None,
            )))],
        );
        let mut diagnostics = Vec::new();
        validate_drag(&comp, &mut diagnostics);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].severity, Severity::Error);
        assert!(
            diagnostics[0].message.contains("@drag requires @ref"),
            "unexpected message: {}",
            diagnostics[0].message
        );
    }

    #[test]
    fn drag_handler_undefined_errors() {
        let comp = make_component(
            "Test",
            &["box"],
            &[],
            vec![ViewTree::Element(Box::new(make_el(
                "div",
                Some("box"),
                "missing",
                None,
            )))],
        );
        let mut diagnostics = Vec::new();
        validate_drag(&comp, &mut diagnostics);
        assert_eq!(diagnostics.len(), 1);
        assert!(
            diagnostics[0].message.contains("'missing' is not defined"),
            "unexpected message: {}",
            diagnostics[0].message
        );
    }

    #[test]
    fn drag_handler_with_too_many_params_errors() {
        let comp = make_component(
            "Test",
            &["box"],
            &["move"],
            vec![ViewTree::Element(Box::new(make_el(
                "div",
                Some("box"),
                "move",
                None,
            )))],
        );
        // Give "move" two params — the pass reads arity from the fn IR.
        let comp = ComponentIR {
            form_intent: None,
            load_intent: None,
            fns: vec![crate::ir::logic::FunctionIR {
                name: "move".into(),
                visibility: Visibility::Private,
                params: vec![
                    crate::ir::ParamIR {
                        name: "e".into(),
                        r#type: None,
                    },
                    crate::ir::ParamIR {
                        name: "extra".into(),
                        r#type: None,
                    },
                ],
                return_type: None,
                body: crate::ir::BlockIR { statements: vec![] },
                is_async: false,
                source_span: None,
            }],
            ..comp
        };
        let mut diagnostics = Vec::new();
        validate_drag(&comp, &mut diagnostics);
        assert_eq!(diagnostics.len(), 1);
        assert!(
            diagnostics[0].message.contains("at most one parameter"),
            "unexpected message: {}",
            diagnostics[0].message
        );
    }

    #[test]
    fn drag_inside_list_body_is_checked() {
        let comp = ComponentIR {
            form_intent: None,
            load_intent: None,
            name: "TestComp".into(),
            refs: vec![],
            view: Some(ViewTree::ListRendering(Box::new(ListRenderingIntent {
                iterable: crate::ir::ExprIR::ident("items"),
                item_binding: BindingIR {
                    name: "item".into(),
                    r#type: None,
                },
                key_expr: None,
                body: Some(ViewTree::Element(Box::new(make_el(
                    "div", None, "move", None,
                )))),
                empty_body: None,
                scroll_hint: false,
            }))),
            source_file: Some("test.omni".into()),
            ..ComponentIR::default()
        };
        let mut diagnostics = Vec::new();
        validate_drag(&comp, &mut diagnostics);
        assert!(
            diagnostics
                .iter()
                .any(|d| d.message.contains("@drag requires @ref")),
            "expected error for @drag inside @for body, got: {:?}",
            diagnostics
        );
    }
}
