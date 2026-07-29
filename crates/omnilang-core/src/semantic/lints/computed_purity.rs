//! Detects side effects (mutations) in computed value bodies.
//! Computed values must be pure — derived from signals only.
//!
//! Category: correctness
//! Severity: error

use super::helpers::expr_contains_side_effect;
use super::{LintDiagnostic, LintRule, LintSeverity};
use crate::parser::ast::Program;
use rustc_hash::FxHashSet;

pub struct ComputedPurity;

impl LintRule for ComputedPurity {
    fn name(&self) -> &str {
        "computed-purity"
    }
    fn category(&self) -> &str {
        "correctness"
    }
    fn severity(&self) -> LintSeverity {
        LintSeverity::Error
    }

    fn check_ast(&self, program: &Program, file_path: &str) -> Vec<LintDiagnostic> {
        let mut diags = Vec::new();

        for module in program.modules.values() {
            for comp in &module.components {
                let signal_names: FxHashSet<&str> =
                    comp.signals.iter().map(|s| s.name.as_str()).collect();

                for computed in &comp.computeds {
                    if expr_contains_side_effect(&computed.body, &signal_names) {
                        diags.push(LintDiagnostic {
                            rule: "computed-purity".into(),
                            severity: LintSeverity::Error,
                            message: format!(
                                "Computed '{}' in component '{}' contains side effects — \
                                 computed values must be pure (no mutations or assignments)",
                                computed.name, comp.name
                            ),
                            file: file_path.to_string(),
                            line: 0,
                            col: 0,
                        });
                    }
                }

                for store in &module.stores {
                    let store_signal_names: FxHashSet<&str> =
                        store.signals.iter().map(|s| s.name.as_str()).collect();

                    for getter in &store.getters {
                        if expr_contains_side_effect(&getter.body, &store_signal_names) {
                            diags.push(LintDiagnostic {
                                rule: "computed-purity".into(),
                                severity: LintSeverity::Error,
                                message: format!(
                                    "Getter '{}' in store '{}' contains side effects — \
                                     getters must be pure",
                                    getter.name, store.name
                                ),
                                file: file_path.to_string(),
                                line: 0,
                                col: 0,
                            });
                        }
                    }
                }
            }
        }

        diags
    }
}

#[cfg(test)]
mod tests {
    use crate::lexer::Span;
    use crate::parser::ast::{
        AssignTarget, BinaryOpKind, Block, ComponentDef, ComputedDecl, Expr, Literal, Module,
        SignalDecl, Stmt, TypeAnnotation,
    };
    use rustc_hash::FxHashMap;

    use super::*;

    #[test]
    fn test_computed_purity_detects_side_effect() {
        let span = Span {
            start: 0,
            end: 10,
            line: 1,
            col: 1,
        };
        let mut modules = FxHashMap::default();
        modules.insert(
            "test.omni".into(),
            Module {
                name: "test".into(),
                functions: vec![],
                structs: vec![],
                tagged_unions: vec![],
                type_aliases: vec![],
                motions: vec![],
                stores: vec![],
                designs: vec![],
                imports: vec![],
                exports: vec![],
                components: vec![ComponentDef {
                    name: "Counter".into(),
                    props: vec![],
                    signals: vec![SignalDecl {
                        name: "count".into(),
                        type_ann: TypeAnnotation::Named {
                            name: "i32".into(),
                            type_args: vec![],
                            span: span.clone(),
                        },
                        init: Expr::Literal(Literal::Int(0), span.clone()),
                    }],
                    computeds: vec![ComputedDecl {
                        name: "badComputed".into(),
                        type_ann: TypeAnnotation::Named {
                            name: "i32".into(),
                            type_args: vec![],
                            span: span.clone(),
                        },
                        body: Expr::Block {
                            body: Block {
                                stmts: vec![Stmt::Assign {
                                    target: AssignTarget::Name("count".into()),
                                    op: BinaryOpKind::Assign,
                                    value: Expr::Literal(Literal::Int(5), span.clone()),
                                    span: span.clone(),
                                }],
                                trailing_expr: Some(Box::new(Expr::VarRef(
                                    "count".into(),
                                    span.clone(),
                                ))),
                                span: span.clone(),
                            },
                            span: span.clone(),
                        },
                    }],
                    events: vec![],
                    refs: vec![],
                    store_refs: vec![],
                    if_mode_default: None,
                    motions: vec![],
                    hero_defs: vec![],
                    lifecycles: vec![],
                    fns: vec![],
                    actions: vec![],
                    getters: vec![],
                    view: None,
                    styles: vec![],
                    span: span.clone(),
                }],
            },
        );
        let program = crate::parser::ast::Program { modules };

        let diags = ComputedPurity.check_ast(&program, "test.omni");
        assert!(!diags.is_empty(), "Should detect side effect in computed");
        assert!(
            diags[0].message.contains("badComputed"),
            "Message should mention computed name: {}",
            diags[0].message
        );
    }

    #[test]
    fn test_computed_purity_allows_pure_computed() {
        let span = Span {
            start: 0,
            end: 10,
            line: 1,
            col: 1,
        };
        let mut modules = FxHashMap::default();
        modules.insert(
            "test.omni".into(),
            Module {
                name: "test".into(),
                functions: vec![],
                structs: vec![],
                tagged_unions: vec![],
                type_aliases: vec![],
                motions: vec![],
                stores: vec![],
                designs: vec![],
                imports: vec![],
                exports: vec![],
                components: vec![ComponentDef {
                    name: "Counter".into(),
                    props: vec![],
                    signals: vec![SignalDecl {
                        name: "count".into(),
                        type_ann: TypeAnnotation::Named {
                            name: "i32".into(),
                            type_args: vec![],
                            span: span.clone(),
                        },
                        init: Expr::Literal(Literal::Int(0), span.clone()),
                    }],
                    computeds: vec![ComputedDecl {
                        name: "doubled".into(),
                        type_ann: TypeAnnotation::Named {
                            name: "i32".into(),
                            type_args: vec![],
                            span: span.clone(),
                        },
                        body: Expr::BinaryOp {
                            left: Box::new(Expr::VarRef("count".into(), span.clone())),
                            op: BinaryOpKind::Mul,
                            right: Box::new(Expr::Literal(Literal::Int(2), span.clone())),
                            span: span.clone(),
                        },
                    }],
                    events: vec![],
                    refs: vec![],
                    store_refs: vec![],
                    if_mode_default: None,
                    motions: vec![],
                    hero_defs: vec![],
                    lifecycles: vec![],
                    fns: vec![],
                    actions: vec![],
                    getters: vec![],
                    view: None,
                    styles: vec![],
                    span: span.clone(),
                }],
            },
        );
        let program = crate::parser::ast::Program { modules };

        let diags = ComputedPurity.check_ast(&program, "test.omni");
        assert!(diags.is_empty(), "Pure computed should not be flagged");
    }
}
