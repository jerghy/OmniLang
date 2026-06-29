//! Checks that `e.field` references in event handlers refer to fields
//! that exist in the event's EventDataKind schema.
//!
//! Category: correctness
//! Severity: error

use std::collections::{HashMap, HashSet};

use super::{LintDiagnostic, LintRule, LintSeverity};
use crate::ir::{event_data_schema, IRProgram, ViewTree};

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

    fn check_ir(&self, ir: &IRProgram, _file_map: &HashMap<String, String>) -> Vec<LintDiagnostic> {
        let mut diags = Vec::new();
        for (_comp_name, comp) in ir.all_components() {
            if let Some(ref view) = comp.view {
                check_view_invalid_fields(view, &comp.name, self, &mut diags);
            }
        }
        diags
    }
}

fn check_view_invalid_fields(
    tree: &ViewTree,
    comp_name: &str,
    rule: &InvalidEventField,
    diags: &mut Vec<LintDiagnostic>,
) {
    match tree {
        ViewTree::Element(el) => {
            for (_ev_name, handler) in &el.events {
                if let (Some(ref kind), Some(ref used)) =
                    (&handler.event_data, &handler.used_fields)
                {
                    let schema = event_data_schema(kind);
                    let valid: HashSet<&str> = schema.iter().map(|f| f.name.as_str()).collect();
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
                                file: comp_name.to_string(),
                                line: el.source_line,
                                col: el.source_col,
                            });
                        }
                    }
                }
            }
            for child in &el.children {
                check_view_invalid_fields(child, comp_name, rule, diags);
            }
        }
        ViewTree::Conditional(cond) => {
            for branch in &cond.branches {
                if let Some(ref body) = branch.body {
                    check_view_invalid_fields(body, comp_name, rule, diags);
                }
            }
            if let Some(ref fallback) = cond.fallback {
                check_view_invalid_fields(fallback, comp_name, rule, diags);
            }
        }
        ViewTree::ListRendering(list) => {
            if let Some(ref body) = list.body {
                check_view_invalid_fields(body, comp_name, rule, diags);
            }
            if let Some(ref empty) = list.empty_body {
                check_view_invalid_fields(empty, comp_name, rule, diags);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::ir::{
        ComponentIR, ElementIR, EventDataKind, EventHandlerIR, IRProgram, ModuleIR, ViewTree,
    };

    use super::*;

    fn make_test_ir_with_view(comp_name: &str, view: Option<ViewTree>) -> IRProgram {
        let mut components = HashMap::new();
        components.insert(
            comp_name.to_string(),
            ComponentIR {
                name: comp_name.to_string(),
                generic_params: vec![],
                props: HashMap::new(),
                prop_defaults: HashMap::new(),
                states: vec![],
                computeds: vec![],
                effects: vec![],
                lifecycles: vec![],
                fns: vec![],
                store_refs: vec![],
                refs: vec![],
                event_names: vec![],
                view,
                styles: vec![],
                motions: vec![],
                if_mode_default: None,
                hero_defs: vec![],
                timelines: vec![],
            },
        );
        let mut modules = HashMap::new();
        modules.insert(
            "main".to_string(),
            ModuleIR {
                name: "main".to_string(),
                components,
                stores: HashMap::new(),
                ..Default::default()
            },
        );
        IRProgram {
            version: "1.0".into(),
            modules,
            source_map: HashMap::new(),
            stdlib_imports: Vec::new(),
        }
    }

    #[test]
    fn test_invalid_event_field_detects_invalid_field() {
        let view = Some(ViewTree::Element(ElementIR {
            tag: "input".into(),
            props: HashMap::new(),
            bindings: HashMap::new(),
            events: [(
                "onChange".into(),
                EventHandlerIR {
                    kind: "inline_fn".into(),
                    body: None,
                    r#ref: None,
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
            construction_kind: "normal".into(),
            class_map_bindings: HashMap::new(),
            source_line: 10,
            source_col: 5,
            motion_triggers: vec![],
            states: None,
            paused_override: None,
            driven_by: None,
            hover_reverse: None,
            in_view_once: None,
            in_view_repeat: None,
            in_view_threshold: None,
            scroll_start: None,
            scroll_end: None,
            scroll_scrub: None,
            drag_constraints: None,
            drag_elastic: None,
            drag_scrub: None,
            drag_momentum: None,
            drag_threshold: None,
            drag_velocity_max: None,
            hero_tag: None,
            hero_transition: None,
        }));
        let ir = make_test_ir_with_view("Form", view);

        let diags = InvalidEventField.check_ir(&ir, &HashMap::new());
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
        assert_eq!(diags[0].line, 10, "Line should match ElementIR.source_line");
        assert_eq!(diags[0].col, 5, "Col should match ElementIR.source_col");
    }

    #[test]
    fn test_invalid_event_field_allows_valid_field() {
        let view = Some(ViewTree::Element(ElementIR {
            tag: "input".into(),
            props: HashMap::new(),
            bindings: HashMap::new(),
            events: [(
                "onChange".into(),
                EventHandlerIR {
                    kind: "inline_fn".into(),
                    body: None,
                    r#ref: None,
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
            construction_kind: "normal".into(),
            class_map_bindings: HashMap::new(),
            source_line: 0,
            source_col: 0,
            motion_triggers: vec![],
            states: None,
            paused_override: None,
            driven_by: None,
            hover_reverse: None,
            in_view_once: None,
            in_view_repeat: None,
            in_view_threshold: None,
            scroll_start: None,
            scroll_end: None,
            scroll_scrub: None,
            drag_constraints: None,
            drag_elastic: None,
            drag_scrub: None,
            drag_momentum: None,
            drag_threshold: None,
            drag_velocity_max: None,
            hero_tag: None,
            hero_transition: None,
        }));
        let ir = make_test_ir_with_view("Form", view);

        let diags = InvalidEventField.check_ir(&ir, &HashMap::new());
        assert!(
            diags.is_empty(),
            "Valid event field `value` should not trigger diagnostic"
        );
    }

    #[test]
    fn test_invalid_event_field_succeeds_on_missing_event_data() {
        let view = Some(ViewTree::Element(ElementIR {
            tag: "div".into(),
            props: HashMap::new(),
            bindings: HashMap::new(),
            events: [(
                "onCustomEvent".into(),
                EventHandlerIR {
                    kind: "inline_fn".into(),
                    body: None,
                    r#ref: None,
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
            construction_kind: "normal".into(),
            class_map_bindings: HashMap::new(),
            source_line: 0,
            source_col: 0,
            motion_triggers: vec![],
            states: None,
            paused_override: None,
            driven_by: None,
            hover_reverse: None,
            in_view_once: None,
            in_view_repeat: None,
            in_view_threshold: None,
            scroll_start: None,
            scroll_end: None,
            scroll_scrub: None,
            drag_constraints: None,
            drag_elastic: None,
            drag_scrub: None,
            drag_momentum: None,
            drag_threshold: None,
            drag_velocity_max: None,
            hero_tag: None,
            hero_transition: None,
        }));
        let ir = make_test_ir_with_view("Widget", view);

        let diags = InvalidEventField.check_ir(&ir, &HashMap::new());
        assert!(
            diags.is_empty(),
            "Custom events without event_data should not trigger"
        );
    }
}
