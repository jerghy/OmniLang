//! Detects reassignment of component props.
//! Props are read-only — they flow from parent to child.
//!
//! Category: correctness
//! Severity: error

use super::helpers::{check_block_for_prop_assign, check_view_for_prop_assign};
use super::{LintDiagnostic, LintRule, LintSeverity};
use crate::parser::ast::Program;
use rustc_hash::FxHashSet;

pub struct PropImmutability;

impl LintRule for PropImmutability {
    fn name(&self) -> &str {
        "prop-immutability"
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
                let prop_names: FxHashSet<&str> =
                    comp.props.iter().map(|p| p.name.as_str()).collect();

                if prop_names.is_empty() {
                    continue;
                }

                for func in &comp.fns {
                    check_block_for_prop_assign(
                        &func.body,
                        &prop_names,
                        &comp.name,
                        &func.name,
                        file_path,
                        &mut diags,
                    );
                }

                for action in &comp.actions {
                    check_block_for_prop_assign(
                        &action.body,
                        &prop_names,
                        &comp.name,
                        &action.name,
                        file_path,
                        &mut diags,
                    );
                }

                for lc in &comp.lifecycles {
                    check_block_for_prop_assign(
                        &lc.body,
                        &prop_names,
                        &comp.name,
                        &format!("@{:?}", lc.kind),
                        file_path,
                        &mut diags,
                    );
                }

                if let Some(ref view) = comp.view {
                    check_view_for_prop_assign(
                        &view.children,
                        &prop_names,
                        &comp.name,
                        file_path,
                        &mut diags,
                    );
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
        AssignTarget, BinaryOpKind, Block, ComponentDef, Expr, FunctionDef, Literal, Module,
        PropDecl, Stmt, TypeAnnotation, Visibility,
    };
    use rustc_hash::FxHashMap;

    use super::*;

    #[test]
    fn test_prop_immutability_detects_reassignment() {
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
                consts: vec![],
                components: vec![ComponentDef {
                    name: "Card".into(),
                    props: vec![PropDecl {
                        name: "title".into(),
                        type_ann: TypeAnnotation::Named {
                            name: "String".into(),
                            type_args: vec![],
                            span: span.clone(),
                        },
                        default: None,
                    }],
                    signals: vec![],
                    computeds: vec![],
                    events: vec![],
                    refs: vec![],
                    store_refs: vec![],
                    inline_stores: vec![],
                    inline_store_defs: vec![],
                    motions: vec![],
                    lifecycles: vec![],
                    fns: vec![FunctionDef {
                        visibility: Visibility::Private,
                        name: "badFn".into(),
                        params: vec![],
                        return_type: None,
                        is_async: false,
                        body: Block {
                            stmts: vec![Stmt::Assign {
                                target: AssignTarget::Name("title".into()),
                                op: BinaryOpKind::Assign,
                                value: Expr::Literal(
                                    Literal::String("hacked".into()),
                                    span.clone(),
                                ),
                                span: span.clone(),
                            }],
                            trailing_expr: None,
                            span: span.clone(),
                        },
                        span: span.clone(),
                    }],
                    actions: vec![],
                    getters: vec![],
                    slots: vec![],
                    view: None,
                    styles: vec![],
                    span: span.clone(),
                }],
            },
        );
        let program = crate::parser::ast::Program { modules };

        let diags = PropImmutability.check_ast(&program, "test.omni");
        assert!(!diags.is_empty(), "Should detect prop reassignment");
        assert!(
            diags[0].message.contains("title"),
            "Message should mention prop name: {}",
            diags[0].message
        );
        assert_eq!(
            diags[0].line, 1,
            "must anchor at the assignment statement (Stmt::Assign span)"
        );
        assert_eq!(diags[0].col, 1);
    }
}
