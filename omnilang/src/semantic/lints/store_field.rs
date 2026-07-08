//! Detects direct mutation of store signals from component code
//! outside of the store's own action bodies.
//!
//! Category: correctness
//! Severity: error

use super::helpers::{
    check_block_ir_for_store_mutations, check_view_tree_events_for_store_mutations,
    expr_ir_contains_store_mutation,
};
use super::{LintDiagnostic, LintRule, LintSeverity};
use crate::ir::IRProgram;
use rustc_hash::FxHashMap;
use rustc_hash::FxHashSet;

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

    fn check_ir(
        &self,
        ir: &IRProgram,
        file_map: &FxHashMap<String, String>,
    ) -> Vec<LintDiagnostic> {
        let mut diags = Vec::new();

        // Build map: store_name -> set of signal names
        let store_signals: FxHashMap<&str, FxHashSet<&str>> = ir
            .all_stores()
            .iter()
            .map(|(name, store)| (*name, store.signals.keys().map(|s| s.as_str()).collect()))
            .collect();

        if store_signals.is_empty() {
            return diags;
        }

        for (_comp_name, comp) in ir.all_components() {
            let comp_file = comp
                .source_file
                .as_deref()
                .or_else(|| file_map.keys().next().map(|s| s.as_str()))
                .unwrap_or("")
                .to_string();

            for func in &comp.fns {
                check_block_ir_for_store_mutations(
                    &func.body,
                    &store_signals,
                    &func.name,
                    &comp_file,
                    &mut diags,
                );
            }

            for lc in &comp.lifecycles {
                check_block_ir_for_store_mutations(
                    &lc.body,
                    &store_signals,
                    &format!("@{}", lc.kind),
                    &comp_file,
                    &mut diags,
                );
            }

            for effect in &comp.effects {
                check_block_ir_for_store_mutations(
                    &effect.body,
                    &store_signals,
                    "@effect",
                    &comp_file,
                    &mut diags,
                );
            }

            if let Some(ref view) = comp.view {
                check_view_tree_events_for_store_mutations(
                    view,
                    &store_signals,
                    &comp_file,
                    &mut diags,
                );
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
                        file: comp_file.clone(),
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
    use crate::ir::{BlockIR, ComponentIR, FunctionIR, StatementIR, StoreIR, TypeIR, Visibility};
    use rustc_hash::FxHashMap;

    use super::super::helpers::test_helpers::make_test_ir;
    use super::*;

    #[test]
    fn test_store_mutation_detects_direct_assign() {
        let mut stores = FxHashMap::default();
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
                    },
                )]
                .into_iter()
                .collect(),
                signal_defaults: FxHashMap::default(),
                getters: FxHashMap::default(),
                actions: vec![],
                ..Default::default()
            },
        );
        let mut components = FxHashMap::default();
        components.insert(
            "App".into(),
            ComponentIR {
                name: "App".into(),
                generic_params: vec![],
                props: FxHashMap::default(),
                prop_defaults: FxHashMap::default(),
                states: vec![],
                computeds: vec![],
                effects: vec![],
                lifecycles: vec![],
                fns: vec![FunctionIR {
                    name: "hack".into(),
                    visibility: Visibility::Private,
                    params: vec![],
                    return_type: None,
                    body: BlockIR {
                        statements: vec![StatementIR::MutationStmt {
                            intent: Some(crate::ir::MutationIntent::DirectAssign {
                                target: crate::ir::SignalRefIR {
                                    owner: Some("MyStore".into()),
                                    name: "count".into(),
                                },
                                value: Some(Box::new(crate::ir::ExprIR::LiteralExpr {
                                    value: serde_json::Value::Number(99.into()),
                                })),
                            }),
                        }],
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

        let diags = StoreFieldDirectMutation.check_ir(&ir, &FxHashMap::default());
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
        let mut stores = FxHashMap::default();
        stores.insert(
            "MyStore".into(),
            StoreIR {
                name: "MyStore".into(),
                signals: FxHashMap::default(),
                signal_defaults: FxHashMap::default(),
                getters: FxHashMap::default(),
                actions: vec![],
                ..Default::default()
            },
        );
        let mut components = FxHashMap::default();
        components.insert(
            "App".into(),
            ComponentIR {
                name: "App".into(),
                generic_params: vec![],
                props: FxHashMap::default(),
                prop_defaults: FxHashMap::default(),
                states: vec![],
                computeds: vec![],
                effects: vec![],
                lifecycles: vec![],
                fns: vec![FunctionIR {
                    name: "increment".into(),
                    visibility: Visibility::Private,
                    params: vec![],
                    return_type: None,
                    body: BlockIR {
                        statements: vec![StatementIR::MutationStmt {
                            intent: Some(crate::ir::MutationIntent::Increment {
                                target: crate::ir::SignalRefIR {
                                    owner: None,
                                    name: "count".into(),
                                },
                                amount: Some(Box::new(crate::ir::ExprIR::LiteralExpr {
                                    value: serde_json::Value::Number(1.into()),
                                })),
                            }),
                        }],
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

        let diags = StoreFieldDirectMutation.check_ir(&ir, &FxHashMap::default());
        assert!(
            diags.is_empty(),
            "Local signal mutations should not be flagged"
        );
    }
}
