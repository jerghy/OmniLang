//! Detects direct mutation of store signals from component code
//! outside of the store's own action bodies.
//!
//! Category: correctness
//! Severity: error

use std::collections::{HashMap, HashSet};

use super::helpers::{
    check_block_ir_for_store_mutations, check_view_tree_events_for_store_mutations,
    expr_ir_contains_store_mutation,
};
use super::{LintDiagnostic, LintRule, LintSeverity};
use crate::ir::IRProgram;

pub struct StoreFieldDirectMutation;

impl LintRule for StoreFieldDirectMutation {
    fn name(&self) -> &str {
        "store-field-direct-mutation"
    }
    fn category(&self) -> &str {
        "correctness"
    }
    fn severity(&self) -> LintSeverity {
        LintSeverity::Error
    }

    fn check_ir(&self, ir: &IRProgram, _file_map: &HashMap<String, String>) -> Vec<LintDiagnostic> {
        let mut diags = Vec::new();

        // Build map: store_name -> set of signal names
        let store_signals: HashMap<&str, HashSet<&str>> = ir
            .all_stores()
            .iter()
            .map(|(name, store)| (*name, store.signals.keys().map(|s| s.as_str()).collect()))
            .collect();

        if store_signals.is_empty() {
            return diags;
        }

        for (_comp_name, comp) in ir.all_components() {
            for func in &comp.fns {
                check_block_ir_for_store_mutations(
                    &func.body,
                    &store_signals,
                    &func.name,
                    &mut diags,
                );
            }

            for lc in &comp.lifecycles {
                check_block_ir_for_store_mutations(
                    &lc.body,
                    &store_signals,
                    &format!("@{}", lc.kind),
                    &mut diags,
                );
            }

            for effect in &comp.effects {
                check_block_ir_for_store_mutations(
                    &effect.body,
                    &store_signals,
                    "@effect",
                    &mut diags,
                );
            }

            if let Some(ref view) = comp.view {
                check_view_tree_events_for_store_mutations(view, &store_signals, &mut diags);
            }

            for computed in &comp.computeds {
                if expr_ir_contains_store_mutation(&computed.body, &store_signals) {
                    diags.push(LintDiagnostic {
                        rule: "store-field-direct-mutation".into(),
                        severity: LintSeverity::Error,
                        message: format!(
                            "Computed '{}' mutates store data — computed values must be pure",
                            computed.name
                        ),
                        file: String::new(),
                        line: 0,
                        col: 0,
                    });
                }
            }
        }

        diags
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::ir::{BlockIR, ComponentIR, FunctionIR, StatementIR, StoreIR, TypeIR};

    use super::super::helpers::test_helpers::make_test_ir;
    use super::*;

    #[test]
    fn test_store_mutation_detects_direct_assign() {
        let mut stores = HashMap::new();
        stores.insert(
            "MyStore".into(),
            StoreIR {
                name: "MyStore".into(),
                signals: [(
                    "count".into(),
                    TypeIR {
                        kind: "i32".into(),
                        type_args: vec![],
                        is_nullable: false,
                        needs_clone: false,
                        needs_partial_eq: false,
                    },
                )]
                .into_iter()
                .collect(),
                signal_defaults: HashMap::new(),
                getters: HashMap::new(),
                actions: vec![],
                ..Default::default()
            },
        );
        let mut components = HashMap::new();
        components.insert(
            "App".into(),
            ComponentIR {
                name: "App".into(),
                generic_params: vec![],
                props: HashMap::new(),
                prop_defaults: HashMap::new(),
                states: vec![],
                computeds: vec![],
                effects: vec![],
                lifecycles: vec![],
                fns: vec![FunctionIR {
                    name: "hack".into(),
                    visibility: "private".into(),
                    params: vec![],
                    return_type: None,
                    body: BlockIR {
                        statements: vec![StatementIR::MutationStmt {
                            intent: Some(crate::ir::MutationIntent {
                                target: crate::ir::SignalRefIR {
                                    owner: Some("MyStore".into()),
                                    name: "count".into(),
                                },
                                operation: "direct_assign".into(),
                                amount: Some(crate::ir::ExprIR::LiteralExpr {
                                    value: serde_json::Value::Number(99.into()),
                                }),
                                item: None,
                            }),
                        }],
                        execution_mode: "sync".into(),
                    },
                    is_async: false,
                }],
                store_refs: vec!["MyStore".into()],
                refs: vec![],
                event_names: vec![],
                view: None,
                styles: vec![],
                motions: vec![],
                if_mode_default: None,
                hero_defs: vec![],
                timelines: vec![],
                ..Default::default()
            },
        );
        let ir = make_test_ir(components, stores);

        let diags = StoreFieldDirectMutation.check_ir(&ir, &HashMap::new());
        assert!(
            !diags.is_empty(),
            "Should detect store field direct mutation"
        );
        assert!(
            diags[0].message.contains("MyStore.count"),
            "Message should mention store.signal: {}",
            diags[0].message
        );
    }

    #[test]
    fn test_store_mutation_allows_non_store_mutations() {
        let mut stores = HashMap::new();
        stores.insert(
            "MyStore".into(),
            StoreIR {
                name: "MyStore".into(),
                signals: HashMap::new(),
                signal_defaults: HashMap::new(),
                getters: HashMap::new(),
                actions: vec![],
                ..Default::default()
            },
        );
        let mut components = HashMap::new();
        components.insert(
            "App".into(),
            ComponentIR {
                name: "App".into(),
                generic_params: vec![],
                props: HashMap::new(),
                prop_defaults: HashMap::new(),
                states: vec![],
                computeds: vec![],
                effects: vec![],
                lifecycles: vec![],
                fns: vec![FunctionIR {
                    name: "increment".into(),
                    visibility: "private".into(),
                    params: vec![],
                    return_type: None,
                    body: BlockIR {
                        statements: vec![StatementIR::MutationStmt {
                            intent: Some(crate::ir::MutationIntent {
                                target: crate::ir::SignalRefIR {
                                    owner: None,
                                    name: "count".into(),
                                },
                                operation: "increment".into(),
                                amount: Some(crate::ir::ExprIR::LiteralExpr {
                                    value: serde_json::Value::Number(1.into()),
                                }),
                                item: None,
                            }),
                        }],
                        execution_mode: "sync".into(),
                    },
                    is_async: false,
                }],
                store_refs: vec![],
                refs: vec![],
                event_names: vec![],
                view: None,
                styles: vec![],
                motions: vec![],
                if_mode_default: None,
                hero_defs: vec![],
                timelines: vec![],
                ..Default::default()
            },
        );
        let ir = make_test_ir(components, stores);

        let diags = StoreFieldDirectMutation.check_ir(&ir, &HashMap::new());
        assert!(
            diags.is_empty(),
            "Local signal mutations should not be flagged"
        );
    }
}
