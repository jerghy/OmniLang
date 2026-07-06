//! Store actions receive no event data. If a store_action handler's
//! target expression references `e`, it's a compile error.
//!
//! Category: correctness
//! Severity: error

use std::collections::HashMap;

use super::{LintDiagnostic, LintRule, LintSeverity};
use crate::ir::{IRProgram, ViewTree};

pub struct StoreActionEventData;

impl LintRule for StoreActionEventData {
    fn name(&self) -> &str {
        "store-action-no-event-data"
    }
    fn category(&self) -> &str {
        "correctness"
    }
    fn severity(&self) -> LintSeverity {
        LintSeverity::Error
    }

    fn check_ir(&self, ir: &IRProgram, _file_map: &HashMap<String, String>) -> Vec<LintDiagnostic> {
        let mut diags = Vec::new();
        for (comp_name, comp) in ir.all_components() {
            if let Some(ref view) = comp.view {
                check_view_store_actions(view, comp_name, self, &mut diags);
            }
        }
        diags
    }
}

fn check_view_store_actions(
    tree: &ViewTree,
    comp_name: &str,
    rule: &StoreActionEventData,
    diags: &mut Vec<LintDiagnostic>,
) {
    if let ViewTree::Element(el) = tree {
        for handler in el.events.values() {
            if handler.kind == "store_action"
                && handler.used_fields.is_some()
                && !handler
                    .used_fields
                    .as_ref()
                    .expect("used_fields is Some as checked above")
                    .is_empty()
            {
                diags.push(LintDiagnostic {
                    rule: rule.name().into(),
                    severity: rule.severity(),
                    message: format!(
                        "Store action `{}` cannot access event data. \
                             Store actions receive no arguments. \
                             Use an inline handler or fn_ref instead.",
                        handler.r#ref.as_deref().unwrap_or("?")
                    ),
                    file: comp_name.to_string(),
                    line: el.source_line,
                    col: el.source_col,
                });
            }
        }
        for child in &el.children {
            check_view_store_actions(child, comp_name, rule, diags);
        }
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
                ..Default::default()
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
    fn test_store_action_detects_event_data_access() {
        let view = Some(ViewTree::Element(ElementIR {
            tag: "button".into(),
            props: HashMap::new(),
            bindings: HashMap::new(),
            events: [(
                "onClick".into(),
                EventHandlerIR {
                    kind: "store_action".into(),
                    body: None,
                    r#ref: Some("MyStore.increment".into()),
                    params: vec![],
                    event_data: Some(EventDataKind::PointerEvent),
                    used_fields: Some(vec!["x".into()]),
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
            source_line: 5,
            source_col: 3,
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
        let ir = make_test_ir_with_view("App", view);

        let diags = StoreActionEventData.check_ir(&ir, &HashMap::new());
        assert!(
            !diags.is_empty(),
            "Store action with event data access should be detected"
        );
        assert!(
            diags[0].message.contains("MyStore.increment"),
            "Message should mention store action path: {}",
            diags[0].message
        );
        assert_eq!(diags[0].line, 5, "Line should match ElementIR.source_line");
        assert_eq!(diags[0].col, 3, "Col should match ElementIR.source_col");
    }

    #[test]
    fn test_store_action_allows_no_event_data() {
        let view = Some(ViewTree::Element(ElementIR {
            tag: "button".into(),
            props: HashMap::new(),
            bindings: HashMap::new(),
            events: [(
                "onClick".into(),
                EventHandlerIR {
                    kind: "store_action".into(),
                    body: None,
                    r#ref: Some("MyStore.increment".into()),
                    params: vec![],
                    event_data: None,
                    used_fields: None,
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
        let ir = make_test_ir_with_view("App", view);

        let diags = StoreActionEventData.check_ir(&ir, &HashMap::new());
        assert!(
            diags.is_empty(),
            "Store action without used_fields should not trigger"
        );
    }

    #[test]
    fn test_store_action_skips_inline_handlers() {
        let view = Some(ViewTree::Element(ElementIR {
            tag: "button".into(),
            props: HashMap::new(),
            bindings: HashMap::new(),
            events: [(
                "onClick".into(),
                EventHandlerIR {
                    kind: "inline_fn".into(),
                    body: None,
                    r#ref: None,
                    params: vec![],
                    event_data: Some(EventDataKind::PointerEvent),
                    used_fields: Some(vec!["x".into(), "y".into()]),
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
        let ir = make_test_ir_with_view("App", view);

        let diags = StoreActionEventData.check_ir(&ir, &HashMap::new());
        assert!(
            diags.is_empty(),
            "Inline fn handlers with event data should not trigger"
        );
    }
}
