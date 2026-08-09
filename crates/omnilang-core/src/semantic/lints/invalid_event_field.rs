//! Checks that `e.field` references in event handlers refer to fields
//! that exist in the event's EventDataKind schema.
//!
//! Category: correctness
//! Severity: error

use super::{LintDiagnostic, LintRule, LintSeverity};
use crate::ir::{event_data_schema, IRProgram, ViewTree};
use rustc_hash::FxHashMap;
use rustc_hash::FxHashSet;

/// DOM-shaped event-object property names → semantic DSL field names, used
/// to attach migration hints to invalid-field errors (spec §3.5 V5).
/// The note (empty = none) explains semantic deltas — x/y switched from
/// viewport to element-local coordinates (D1).
const DOM_FIELD_MIGRATIONS: &[(&str, &str, &str)] = &[
    ("clientX", "x", "元素局部坐标"),
    ("clientY", "y", "元素局部坐标"),
    ("altKey", "alt", ""),
    ("ctrlKey", "ctrl", ""),
    ("shiftKey", "shift", ""),
    ("metaKey", "meta", ""),
];

/// Migration hint for a DOM-shaped field name, e.g. `clientX` →
/// ` 可改用 e.x（元素局部坐标）`. `None` when the name is not DOM-shaped or
/// the semantic counterpart is not a field of this event's schema (the
/// suggestion must always be actionable).
fn dom_field_migration_hint(field: &str, valid: &FxHashSet<&str>) -> Option<String> {
    DOM_FIELD_MIGRATIONS
        .iter()
        .find(|(dom, _, _)| *dom == field)
        .filter(|(_, semantic, _)| valid.contains(*semantic))
        .map(|(_, semantic, note)| {
            let note = if note.is_empty() {
                String::new()
            } else {
                format!("（{}）", note)
            };
            format!(" 可改用 e.{}{}", semantic, note)
        })
}

pub struct InvalidEventField;

impl LintRule for InvalidEventField {
    fn name(&self) -> &str {
        "invalid-event-field"
    }
    fn category(&self) -> &str {
        "correctness"
    }
    fn severity(&self) -> LintSeverity {
        LintSeverity::Error
    }

    fn check_ir(
        &self,
        ir: &IRProgram,
        file_map: &FxHashMap<String, String>,
    ) -> Vec<LintDiagnostic> {
        let mut diags = Vec::new();
        for (_comp_name, comp) in ir.all_components() {
            if let Some(ref view) = comp.view {
                let comp_file = comp
                    .source_file
                    .as_deref()
                    .or_else(|| file_map.keys().next().map(|s| s.as_str()))
                    .unwrap_or(&comp.name);
                check_view_invalid_fields(view, comp_file, self, &mut diags);
            }
        }
        diags
    }
}

fn check_view_invalid_fields(
    tree: &ViewTree,
    comp_file: &str,
    rule: &InvalidEventField,
    diags: &mut Vec<LintDiagnostic>,
) {
    match tree {
        ViewTree::Element(el) => {
            for handler in el.events.values() {
                if let (Some(kind), Some(used)) = (handler.event_data(), handler.used_fields()) {
                    let schema = event_data_schema(kind);
                    let valid: FxHashSet<&str> = schema.iter().map(|f| f.name.as_str()).collect();
                    for field in used {
                        if !valid.contains(field.as_str()) {
                            let hint = dom_field_migration_hint(field, &valid).unwrap_or_default();
                            diags.push(LintDiagnostic {
                                rule: rule.name().into(),
                                severity: rule.severity(),
                                message: format!(
                                    "`e.{}` is not a field of {:?}. Available fields: {:?}{}",
                                    field,
                                    kind,
                                    schema.iter().map(|f| f.name.as_str()).collect::<Vec<_>>(),
                                    hint
                                ),
                                file: comp_file.to_string(),
                                line: el.source_span.map_or(0, |s| s.line),
                                col: el.source_span.map_or(0, |s| s.col),
                            });
                        }
                    }
                }
            }
            for child in &el.children {
                check_view_invalid_fields(child, comp_file, rule, diags);
            }
        }
        ViewTree::Conditional(cond) => {
            for branch in &cond.branches {
                if let Some(ref body) = branch.body {
                    check_view_invalid_fields(body, comp_file, rule, diags);
                }
            }
            if let Some(ref fallback) = cond.fallback {
                check_view_invalid_fields(fallback, comp_file, rule, diags);
            }
        }
        ViewTree::ListRendering(list) => {
            if let Some(ref body) = list.body {
                check_view_invalid_fields(body, comp_file, rule, diags);
            }
            if let Some(ref empty) = list.empty_body {
                check_view_invalid_fields(empty, comp_file, rule, diags);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use crate::ir::{
        BlockIR, ComponentIR, ConstructionKind, ElementIR, EventDataKind, EventHandlerIR,
        IRProgram, ModuleIR, ViewTree,
    };
    use crate::lexer::Span;
    use rustc_hash::FxHashMap;

    use super::*;

    fn make_test_ir_with_view(comp_name: &str, view: Option<ViewTree>) -> IRProgram {
        let mut components = FxHashMap::default();
        components.insert(
            comp_name.to_string(),
            ComponentIR {
                form_intent: None,
                load_intent: None,
                name: comp_name.to_string(),
                props: FxHashMap::default(),
                prop_defaults: FxHashMap::default(),
                signals: vec![],
                computeds: vec![],
                effects: vec![],
                lifecycles: vec![],
                fns: vec![],
                store_refs: vec![],
                refs: vec![],
                events: vec![],
                view,
                styles: vec![],
                ..Default::default()
            },
        );
        let mut modules = FxHashMap::default();
        modules.insert(
            "main".to_string(),
            ModuleIR {
                name: "main".to_string(),
                components,
                stores: FxHashMap::default(),
                ..Default::default()
            },
        );
        IRProgram {
            version: "1.0".into(),
            modules,
            source_map: FxHashMap::default(),
            stdlib_imports: Default::default(),
            source_span: None,
            motion_index: FxHashMap::default(),
        }
    }

    #[test]
    fn test_invalid_event_field_detects_invalid_field() {
        let view = Some(ViewTree::Element(Box::new(ElementIR {
            tag: "input".into(),
            props: FxHashMap::default(),
            bindings: FxHashMap::default(),
            events: [(
                "onChange".into(),
                EventHandlerIR::InlineFn {
                    body: BlockIR { statements: vec![] },
                    params: vec![],
                    event_data: Some(EventDataKind::InputEvent),
                    used_fields: Some(vec!["nonexistent_field".into()]),
                    needs_prevent_default: false,
                    needs_stop_propagation: false,
                },
            )]
            .into_iter()
            .collect(),
            style_classes: None,
            inline_style: None,
            children: vec![],
            construction_kind: ConstructionKind::Normal,
            class_map_bindings: FxHashMap::default(),
            source_span: Some(Span {
                start: 0,
                end: 0,
                line: 10,
                col: 5,
            }),
            motion: None,
            ref_binding: None,
            semantic_bindings: FxHashMap::default(),
            a11y: None,
            drag_binding: None,
            modal: None,
            skeleton: None,
            error_boundary: None,
        })));
        let ir = make_test_ir_with_view("Form", view);

        let diags = InvalidEventField.check_ir(&ir, &FxHashMap::default());
        assert!(
            !diags.is_empty(),
            "Should detect invalid event field reference"
        );
        assert!(
            diags[0].message.contains("nonexistent_field"),
            "Message should mention the invalid field: {}",
            diags[0].message
        );
        assert!(
            diags[0].message.contains("InputEvent"),
            "Message should mention the event data kind: {}",
            diags[0].message
        );
        assert_eq!(diags[0].line, 10, "Line should match ElementIR.source_span");
        assert_eq!(diags[0].col, 5, "Col should match ElementIR.source_span");
    }

    #[test]
    fn test_invalid_event_field_allows_valid_field() {
        let view = Some(ViewTree::Element(Box::new(ElementIR {
            tag: "input".into(),
            props: FxHashMap::default(),
            bindings: FxHashMap::default(),
            events: [(
                "onChange".into(),
                EventHandlerIR::InlineFn {
                    body: BlockIR { statements: vec![] },
                    params: vec![],
                    event_data: Some(EventDataKind::InputEvent),
                    used_fields: Some(vec!["value".into()]),
                    needs_prevent_default: false,
                    needs_stop_propagation: false,
                },
            )]
            .into_iter()
            .collect(),
            style_classes: None,
            inline_style: None,
            children: vec![],
            construction_kind: ConstructionKind::Normal,
            class_map_bindings: FxHashMap::default(),
            source_span: None,
            motion: None,
            ref_binding: None,
            semantic_bindings: FxHashMap::default(),
            a11y: None,
            drag_binding: None,
            modal: None,
            skeleton: None,
            error_boundary: None,
        })));
        let ir = make_test_ir_with_view("Form", view);

        let diags = InvalidEventField.check_ir(&ir, &FxHashMap::default());
        assert!(
            diags.is_empty(),
            "Valid event field `value` should not trigger diagnostic"
        );
    }

    /// Build an IR whose single element has a PointerEvent handler reading
    /// exactly `used_fields`.
    fn make_pointer_event_ir(comp_name: &str, used_fields: Vec<String>) -> IRProgram {
        let view = Some(ViewTree::Element(Box::new(ElementIR {
            tag: "div".into(),
            props: FxHashMap::default(),
            bindings: FxHashMap::default(),
            events: [(
                "onPointerMove".into(),
                EventHandlerIR::InlineFn {
                    body: BlockIR { statements: vec![] },
                    params: vec![],
                    event_data: Some(EventDataKind::PointerEvent),
                    used_fields: Some(used_fields),
                    needs_prevent_default: false,
                    needs_stop_propagation: false,
                },
            )]
            .into_iter()
            .collect(),
            style_classes: None,
            inline_style: None,
            children: vec![],
            construction_kind: ConstructionKind::Normal,
            class_map_bindings: FxHashMap::default(),
            source_span: None,
            motion: None,
            ref_binding: None,
            semantic_bindings: FxHashMap::default(),
            a11y: None,
            drag_binding: None,
            modal: None,
            skeleton: None,
            error_boundary: None,
        })));
        make_test_ir_with_view(comp_name, view)
    }

    /// V5 (spec §3.5): DOM-shaped coordinate names get a migration hint
    /// pointing at the semantic field with the element-local note.
    #[test]
    fn test_invalid_event_field_dom_coord_name_gets_migration_hint() {
        let ir = make_pointer_event_ir("Move", vec!["clientX".into()]);
        let diags = InvalidEventField.check_ir(&ir, &FxHashMap::default());
        assert_eq!(diags.len(), 1, "exactly one diagnostic, got: {diags:?}");
        assert!(
            diags[0].message.contains("可改用 e.x（元素局部坐标）"),
            "hint must suggest e.x with the element-local note: {}",
            diags[0].message
        );
    }

    /// V5 (spec §3.5): DOM-shaped modifier names get a migration hint
    /// pointing at the semantic field.
    #[test]
    fn test_invalid_event_field_dom_modifier_name_gets_migration_hint() {
        for (dom, semantic) in [
            ("altKey", "e.alt"),
            ("ctrlKey", "e.ctrl"),
            ("shiftKey", "e.shift"),
            ("metaKey", "e.meta"),
        ] {
            let ir = make_pointer_event_ir("Move", vec![dom.into()]);
            let diags = InvalidEventField.check_ir(&ir, &FxHashMap::default());
            assert_eq!(
                diags.len(),
                1,
                "{dom}: exactly one diagnostic, got: {diags:?}"
            );
            assert!(
                diags[0].message.contains(&format!("可改用 {}", semantic)),
                "{dom}: hint must suggest {semantic}: {}",
                diags[0].message
            );
        }
    }

    /// V5 (spec §3.5): plain unknown names get no migration hint — the
    /// hint only fires for known DOM-shaped property names.
    #[test]
    fn test_invalid_event_field_no_hint_for_unknown_name() {
        let ir = make_pointer_event_ir("Move", vec!["bogus".into()]);
        let diags = InvalidEventField.check_ir(&ir, &FxHashMap::default());
        assert_eq!(diags.len(), 1, "exactly one diagnostic, got: {diags:?}");
        assert!(
            !diags[0].message.contains("可改用"),
            "no hint for unknown names: {}",
            diags[0].message
        );
    }

    #[test]
    fn test_invalid_event_field_succeeds_on_missing_event_data() {
        let view = Some(ViewTree::Element(Box::new(ElementIR {
            tag: "div".into(),
            props: FxHashMap::default(),
            bindings: FxHashMap::default(),
            events: [(
                "onCustomEvent".into(),
                EventHandlerIR::InlineFn {
                    body: BlockIR { statements: vec![] },
                    params: vec![],
                    event_data: None,
                    used_fields: Some(vec!["whatever".into()]),
                    needs_prevent_default: false,
                    needs_stop_propagation: false,
                },
            )]
            .into_iter()
            .collect(),
            style_classes: None,
            inline_style: None,
            children: vec![],
            construction_kind: ConstructionKind::Normal,
            class_map_bindings: FxHashMap::default(),
            source_span: None,
            motion: None,
            ref_binding: None,
            semantic_bindings: FxHashMap::default(),
            a11y: None,
            drag_binding: None,
            modal: None,
            skeleton: None,
            error_boundary: None,
        })));
        let ir = make_test_ir_with_view("Widget", view);

        let diags = InvalidEventField.check_ir(&ir, &FxHashMap::default());
        assert!(
            diags.is_empty(),
            "Custom events without event_data should not trigger"
        );
    }
}
