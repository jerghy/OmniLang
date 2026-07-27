//! Checks that `e.field` references in event handlers refer to fields
//! that exist in the event's EventDataKind schema.
//!
//! Category: correctness
//! Severity: error

use super::{LintDiagnostic, LintRule, LintSeverity};
use crate::ir::{event_data_schema, IRProgram, ViewTree};
use rustc_hash::FxHashMap;
use rustc_hash::FxHashSet;

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
                            diags.push(LintDiagnostic {
                                rule: rule.name().into(),
                                severity: rule.severity(),
                                message: format!(
                                    "`e.{}` is not a field of {:?}. Available fields: {:?}",
                                    field,
                                    kind,
                                    schema.iter().map(|f| f.name.as_str()).collect::<Vec<_>>()
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
                name: comp_name.to_string(),
                generic_params: vec![],
                props: FxHashMap::default(),
                prop_defaults: FxHashMap::default(),
                signals: vec![],
                computeds: vec![],
                effects: vec![],
                lifecycles: vec![],
                fns: vec![],
                store_refs: vec![],
                refs: vec![],
                event_names: vec![],
                view,
                styles: vec![],
                if_mode_default: None,
                hero_defs: vec![],
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
        }
    }

    #[test]
    fn test_invalid_event_field_detects_invalid_field() {
        let view = Some(ViewTree::Element(ElementIR {
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
            test_id: None,
            ref_name: None,
            construction_kind: ConstructionKind::Normal,
            class_map_bindings: FxHashMap::default(),
            source_span: Some(Span {
                start: 0,
                end: 0,
                line: 10,
                col: 5,
            }),
            motion: None,
            scroll: None,
            drag: None,
            hero: None,
        }));
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
        let view = Some(ViewTree::Element(ElementIR {
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
            test_id: None,
            ref_name: None,
            construction_kind: ConstructionKind::Normal,
            class_map_bindings: FxHashMap::default(),
            source_span: None,
            motion: None,
            scroll: None,
            drag: None,
            hero: None,
        }));
        let ir = make_test_ir_with_view("Form", view);

        let diags = InvalidEventField.check_ir(&ir, &FxHashMap::default());
        assert!(
            diags.is_empty(),
            "Valid event field `value` should not trigger diagnostic"
        );
    }

    #[test]
    fn test_invalid_event_field_succeeds_on_missing_event_data() {
        let view = Some(ViewTree::Element(ElementIR {
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
            test_id: None,
            ref_name: None,
            construction_kind: ConstructionKind::Normal,
            class_map_bindings: FxHashMap::default(),
            source_span: None,
            motion: None,
            scroll: None,
            drag: None,
            hero: None,
        }));
        let ir = make_test_ir_with_view("Widget", view);

        let diags = InvalidEventField.check_ir(&ir, &FxHashMap::default());
        assert!(
            diags.is_empty(),
            "Custom events without event_data should not trigger"
        );
    }
}
