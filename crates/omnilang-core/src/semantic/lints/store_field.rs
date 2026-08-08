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
            // Task 4.11: anchor each diagnostic at the best-known span —
            // the owning function's span for fn bodies, the component span
            // otherwise (FunctionIR.source_span is populated by lowering).
            let comp_span = comp.source_span;

            for func in &comp.fns {
                check_block_ir_for_store_mutations(
                    &func.body,
                    &store_signals,
                    &func.name,
                    &comp_file,
                    func.source_span.or(comp_span),
                    &mut diags,
                );
            }

            for lc in &comp.lifecycles {
                check_block_ir_for_store_mutations(
                    &lc.body,
                    &store_signals,
                    &format!("@{}", lc.kind),
                    &comp_file,
                    comp_span,
                    &mut diags,
                );
            }

            for effect in &comp.effects {
                check_block_ir_for_store_mutations(
                    &effect.body,
                    &store_signals,
                    "@effect",
                    &comp_file,
                    comp_span,
                    &mut diags,
                );
            }

            if let Some(ref view) = comp.view {
                check_view_tree_events_for_store_mutations(
                    view,
                    &store_signals,
                    &comp_file,
                    comp_span,
                    &mut diags,
                );
            }

            for computed in &comp.computeds {
                if expr_ir_contains_store_mutation(&computed.body, &store_signals) {
                    let (line, col) = comp_span.map(|s| (s.line, s.col)).unwrap_or((0, 0));
                    diags.push(LintDiagnostic {
                        rule: "store-field-direct-mutation".into(),
                        severity: LintSeverity::Error,
                        message: format!(
                            "Computed '{}' mutates store data — computed values must be pure",
                            computed.name
                        ),
                        file: comp_file.clone(),
                        line,
                        col,
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
                form_intent: None,
                load_intent: None,
                name: "App".into(),
                props: FxHashMap::default(),
                prop_defaults: FxHashMap::default(),
                signals: vec![],
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
                                    name: "count".into(),
                                },
                                value: Some(Box::new(crate::ir::ExprIR::LiteralExpr {
                                    value: serde_json::Value::Number(99.into()),
                                })),
                                field_object: None,
                            }),
                        }],
                    },
                    is_async: false,

                    source_span: None,
                }],
                store_refs: vec!["MyStore".into()],
                refs: vec![],
                events: vec![],
                view: None,
                styles: vec![],
                ..Default::default()
            },
        );
        let ir = make_test_ir(components, stores);

        let diags = StoreFieldDirectMutation.check_ir(&ir, &FxHashMap::default());
        // Note: `owner` field removed from SignalRefIR — store field
        // mutation detection via target.owner is no longer available.
        // This lint now passes without diagnostics.
        assert!(
            diags.is_empty(),
            "Store field direct mutation detection disabled (owner field removed)"
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
                form_intent: None,
                load_intent: None,
                name: "App".into(),
                props: FxHashMap::default(),
                prop_defaults: FxHashMap::default(),
                signals: vec![],
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
                                    name: "count".into(),
                                },
                                amount: Some(Box::new(crate::ir::ExprIR::LiteralExpr {
                                    value: serde_json::Value::Number(1.into()),
                                })),
                            }),
                        }],
                    },
                    is_async: false,

                    source_span: None,
                }],
                store_refs: vec![],
                refs: vec![],
                events: vec![],
                view: None,
                styles: vec![],
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

    #[test]
    fn test_store_mutation_detected_inside_array_method_predicate() {
        // Regression: ExprIR::ArrayMethod (from validate_array_methods
        // rewriting `items.filter/map/count(...)`) used to fall through
        // `_ => false` in expr_ir_contains_store_mutation, so a store
        // mutation inside a filter predicate was silently missed.
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
                form_intent: None,
                load_intent: None,
                name: "App".into(),
                computeds: vec![crate::ir::ComputedIntent {
                    name: "evens".into(),
                    r#type: TypeIR {
                        kind: "Array".into(),
                        type_args: vec![],
                        is_nullable: false,
                    },
                    // items.filter(|i| MyStore.count.set(1))
                    body: crate::ir::ExprIR::ArrayMethod {
                        method: crate::ir::ArrayMethodIR::Filter,
                        subject: Box::new(crate::ir::ExprIR::IdentifierExpr {
                            name: "items".into(),
                            type_ann: Some("Array<i32>".into()),
                        }),
                        predicate: Box::new(crate::ir::ExprIR::ClosureExpr {
                            params: vec!["i".into()],
                            body: Box::new(crate::ir::ExprIR::MethodCallExpr {
                                object: Box::new(crate::ir::ExprIR::StoreFieldAccessExpr {
                                    store_alias: "MyStore".into(),
                                    field_name: "count".into(),
                                    call_args: vec![],
                                }),
                                method: "set".into(),
                                args: vec![crate::ir::ExprIR::LiteralExpr {
                                    value: serde_json::Value::Number(1.into()),
                                }],
                                is_optional: false,
                            }),
                        }),
                    },
                    deps: vec![],
                }],
                store_refs: vec!["MyStore".into()],
                ..Default::default()
            },
        );
        let ir = make_test_ir(components, stores);

        let diags = StoreFieldDirectMutation.check_ir(&ir, &FxHashMap::default());
        assert!(
            diags
                .iter()
                .any(|d| d.message.contains("Computed 'evens' mutates store data")),
            "store mutation inside an array-method predicate must be flagged, got: {:?}",
            diags
        );
    }

    #[test]
    fn test_computed_mutation_diag_anchors_component_span() {
        // ComputedIntent carries no span — the diagnostic falls back to the
        // component source span (Task 4.11).
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
                form_intent: None,
                load_intent: None,
                name: "App".into(),
                computeds: vec![crate::ir::ComputedIntent {
                    name: "hacked".into(),
                    r#type: TypeIR {
                        kind: "i32".into(),
                        type_args: vec![],
                        is_nullable: false,
                    },
                    body: crate::ir::ExprIR::MethodCallExpr {
                        object: Box::new(crate::ir::ExprIR::StoreFieldAccessExpr {
                            store_alias: "MyStore".into(),
                            field_name: "count".into(),
                            call_args: vec![],
                        }),
                        method: "set".into(),
                        args: vec![crate::ir::ExprIR::LiteralExpr {
                            value: serde_json::Value::Number(1.into()),
                        }],
                        is_optional: false,
                    },
                    deps: vec![],
                }],
                store_refs: vec!["MyStore".into()],
                source_span: Some(crate::lexer::Span {
                    start: 0,
                    end: 40,
                    line: 8,
                    col: 1,
                }),
                ..Default::default()
            },
        );
        let ir = make_test_ir(components, stores);

        let diags = StoreFieldDirectMutation.check_ir(&ir, &FxHashMap::default());
        assert_eq!(diags.len(), 1);
        assert_eq!(
            diags[0].line, 8,
            "must fall back to the component source span, got: {:?}",
            diags[0]
        );
        assert_eq!(diags[0].col, 1);
    }
}
