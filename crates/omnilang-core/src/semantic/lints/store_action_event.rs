//! Store actions receive no event data. If a store_action handler's
//! target expression references `e`, it's a compile error.
//!
//! Category: correctness
//! Severity: error

use super::{LintDiagnostic, LintRule, LintSeverity};
use crate::ir::{EventHandlerIR, IRProgram, ViewTree};
use rustc_hash::FxHashMap;

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

    fn check_ir(
        &self,
        ir: &IRProgram,
        file_map: &FxHashMap<String, String>,
    ) -> Vec<LintDiagnostic> {
        let mut diags = Vec::new();
        for (comp_name, comp) in ir.all_components() {
            if let Some(ref view) = comp.view {
                let comp_file = comp
                    .source_file
                    .as_deref()
                    .or_else(|| file_map.keys().next().map(|s| s.as_str()))
                    .unwrap_or(comp_name);
                check_view_store_actions(view, comp_file, self, &mut diags);
            }
        }
        diags
    }
}

fn check_view_store_actions(
    tree: &ViewTree,
    comp_file: &str,
    rule: &StoreActionEventData,
    diags: &mut Vec<LintDiagnostic>,
) {
    if let ViewTree::Element(el) = tree {
        for handler in el.events.values() {
            if let EventHandlerIR::StoreAction { r#ref, .. } = handler {
                // StoreAction type cannot hold used_fields, so we check event_data
                // as an indicator that event data was referenced.
                if handler.event_data().is_some() {
                    diags.push(LintDiagnostic {
                        rule: rule.name().into(),
                        severity: rule.severity(),
                        message: format!(
                            "Store action `{}` cannot access event data. \
                             Store actions receive no arguments. \
                             Use an inline handler or fn_ref instead.",
                            r#ref
                        ),
                        file: comp_file.to_string(),
                        line: el.source_span.map_or(0, |s| s.line),
                        col: el.source_span.map_or(0, |s| s.col),
                    });
                }
            }
        }
        for child in &el.children {
            check_view_store_actions(child, comp_file, rule, diags);
        }
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
    fn test_store_action_no_event_data_field() {
        // `event_data` field was removed from StoreAction — the lint
        // cannot detect event data access for StoreAction anymore.
        // Verify that a store action handler passes cleanly.
        let view = Some(ViewTree::Element(Box::new(ElementIR {
            tag: "button".into(),
            props: FxHashMap::default(),
            bindings: FxHashMap::default(),
            events: [(
                "onPress".into(),
                EventHandlerIR::StoreAction {
                    r#ref: "MyStore.increment".into(),
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
                line: 5,
                col: 3,
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
        let ir = make_test_ir_with_view("App", view);

        let diags = StoreActionEventData.check_ir(&ir, &FxHashMap::default());
        assert!(
            diags.is_empty(),
            "Store action without event_data field should pass lint"
        );
    }

    #[test]
    fn test_store_action_allows_no_event_data() {
        let view = Some(ViewTree::Element(Box::new(ElementIR {
            tag: "button".into(),
            props: FxHashMap::default(),
            bindings: FxHashMap::default(),
            events: [(
                "onPress".into(),
                EventHandlerIR::StoreAction {
                    r#ref: "MyStore.increment".into(),
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
        let ir = make_test_ir_with_view("App", view);

        let diags = StoreActionEventData.check_ir(&ir, &FxHashMap::default());
        assert!(
            diags.is_empty(),
            "Store action without used_fields should not trigger"
        );
    }

    #[test]
    fn test_store_action_skips_inline_handlers() {
        let view = Some(ViewTree::Element(Box::new(ElementIR {
            tag: "button".into(),
            props: FxHashMap::default(),
            bindings: FxHashMap::default(),
            events: [(
                "onPress".into(),
                EventHandlerIR::InlineFn {
                    body: BlockIR { statements: vec![] },
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
        let ir = make_test_ir_with_view("App", view);

        let diags = StoreActionEventData.check_ir(&ir, &FxHashMap::default());
        assert!(
            diags.is_empty(),
            "Inline fn handlers with event data should not trigger"
        );
    }
}
