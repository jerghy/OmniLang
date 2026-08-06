//! Detects imports whose symbols are never referenced in the file.
//!
//! Category: style
//! Severity: warning

use super::ast_collectors::{
    collect_used_names_in_block, collect_used_names_in_expr, collect_used_names_in_motion_def,
    collect_used_names_in_view, collect_used_names_in_view_node,
};
use super::helpers::import_span_line_col;
use super::{LintDiagnostic, LintRule, LintSeverity};
use crate::parser::ast::{ImportStmt, Program};
use rustc_hash::FxHashMap;
use rustc_hash::FxHashSet;

pub struct UnusedImport;

impl LintRule for UnusedImport {
    fn name(&self) -> &str {
        "unused-import"
    }
    fn category(&self) -> &str {
        "style"
    }
    fn severity(&self) -> LintSeverity {
        LintSeverity::Warning
    }

    fn check_ast(&self, program: &Program, file_path: &str) -> Vec<LintDiagnostic> {
        let mut diags = Vec::new();

        let mut imported: FxHashMap<&str, (&ImportStmt, &[String])> = FxHashMap::default();
        for module in program.modules.values() {
            for import in &module.imports {
                match import {
                    ImportStmt::Named(named) => {
                        for name in &named.names {
                            imported.insert(name.as_str(), (import, &named.names));
                        }
                    }
                    ImportStmt::TypeOnly(ti) => {
                        for name in &ti.names {
                            imported.insert(name.as_str(), (import, &ti.names));
                        }
                    }
                    ImportStmt::Namespace(_) | ImportStmt::Lib(_) => {}
                }
            }
        }

        if imported.is_empty() {
            return diags;
        }

        let mut used: FxHashSet<&str> = FxHashSet::default();
        for module in program.modules.values() {
            for func in &module.functions {
                collect_used_names_in_block(&func.body, &mut used);
            }
            for comp in &module.components {
                used.insert(comp.name.as_str());
                for sig in &comp.signals {
                    collect_used_names_in_expr(&sig.init, &mut used);
                }
                for c in &comp.computeds {
                    collect_used_names_in_expr(&c.body, &mut used);
                }
                for lc in &comp.lifecycles {
                    collect_used_names_in_block(&lc.body, &mut used);
                }
                for func in &comp.fns {
                    collect_used_names_in_block(&func.body, &mut used);
                }
                for action in &comp.actions {
                    collect_used_names_in_block(&action.body, &mut used);
                }
                // `@store X` directives reference module-level stores by name —
                // a real use of the imported store.
                for store_ref in &comp.store_refs {
                    used.insert(store_ref.as_str());
                }
                // Component-level @motion keyframe bodies hold real reference
                // sites (mirrors the dead-code lint's collection).
                for motion in &comp.motions {
                    collect_used_names_in_motion_def(motion, &mut used);
                }
                // Body-level slot fallbacks (ADR-053 D5) may reference
                // signals/imports — collect their names too.
                for slot in &comp.slots {
                    if let Some(ref content) = slot.default_content {
                        for node in content {
                            collect_used_names_in_view_node(node, &mut used);
                        }
                    }
                }
                if let Some(ref view) = comp.view {
                    collect_used_names_in_view(view, &mut used);
                }
            }
            for store in &module.stores {
                used.insert(store.name.as_str());
                for sig in &store.signals {
                    collect_used_names_in_expr(&sig.init, &mut used);
                }
                for getter in &store.getters {
                    collect_used_names_in_expr(&getter.body, &mut used);
                }
                for action in &store.actions {
                    collect_used_names_in_block(&action.body, &mut used);
                }
            }
            // Design token values may hold Phase 2 runtime expressions
            // (`token dynColor: color = DynamicTokenStore.liveColor;`) —
            // real reference sites for store/const imports.
            for design in &module.designs {
                for token in &design.tokens {
                    if let crate::parser::ast::TokenValue::Expression(e) = &token.value {
                        collect_used_names_in_expr(e, &mut used);
                    }
                    for (_, tv) in &token.theme_values {
                        if let crate::parser::ast::TokenValue::Expression(e) = tv {
                            collect_used_names_in_expr(e, &mut used);
                        }
                    }
                }
            }
            for s in &module.structs {
                used.insert(s.name.as_str());
            }
            for ta in &module.type_aliases {
                used.insert(ta.name.as_str());
            }
            for tu in &module.tagged_unions {
                used.insert(tu.name.as_str());
            }
        }

        for (name, &(import, _names)) in &imported {
            if !used.contains(name) {
                let (line, col) = import_span_line_col(import);
                diags.push(LintDiagnostic {
                    rule: self.name().into(),
                    severity: self.severity(),
                    message: format!("Import '{}' is never used", name),
                    file: file_path.to_string(),
                    line,
                    col,
                });
            }
        }

        diags
    }
}

#[cfg(test)]
mod tests {
    use crate::lexer::Span;
    use crate::parser::ast::{
        Block, Expr, FunctionDef, Literal, Module, NamedImport, Program, Stmt, Visibility,
    };
    use rustc_hash::FxHashMap;

    use super::*;

    #[test]
    fn test_unused_import_detects_unused_named_import() {
        let mut modules = FxHashMap::default();
        modules.insert(
            "test.omni".to_string(),
            Module {
                name: "test".into(),
                functions: vec![],
                structs: vec![],
                tagged_unions: vec![],
                type_aliases: vec![],
                components: vec![],
                motions: vec![],
                stores: vec![],
                designs: vec![],
                imports: vec![ImportStmt::Named(NamedImport {
                    names: vec!["unusedSymbol".into()],
                    path: "./utils.omni".into(),
                    span: Span {
                        start: 0,
                        end: 30,
                        line: 1,
                        col: 1,
                    },
                })],
                exports: vec![],
                consts: vec![],
            },
        );
        let program = Program { modules };

        let diags = UnusedImport.check_ast(&program, "test.omni");
        assert_eq!(diags.len(), 1, "expected 1 diagnostic for unused import");
        assert!(
            diags[0].message.contains("unusedSymbol"),
            "message should mention the unused symbol: {}",
            diags[0].message
        );
    }

    #[test]
    fn test_struct_literal_construction_counts_as_usage() {
        let mut used: FxHashSet<&str> = FxHashSet::default();
        let st = Expr::StructLiteral {
            type_name: "Settings".into(),
            fields: vec![(
                "name".into(),
                Expr::Literal(
                    Literal::String("x".into()),
                    Span {
                        start: 20,
                        end: 25,
                        line: 2,
                        col: 21,
                    },
                ),
            )],
            span: Span {
                start: 10,
                end: 30,
                line: 2,
                col: 11,
            },
        };
        collect_used_names_in_expr(&st, &mut used);
        assert!(
            used.contains("Settings"),
            "struct name at a construction site must count as usage \
             (else imported structs are falsely flagged unused-import)"
        );
    }

    #[test]
    fn test_unused_import_skips_used_import() {
        let mut modules = FxHashMap::default();
        modules.insert(
            "test.omni".to_string(),
            Module {
                name: "test".into(),
                functions: vec![FunctionDef {
                    visibility: Visibility::Private,
                    name: "greet".into(),
                    params: vec![],
                    return_type: None,
                    is_async: false,
                    body: Block {
                        stmts: vec![Stmt::ExprStmt {
                            expr: Expr::VarRef(
                                "usedSymbol".into(),
                                Span {
                                    start: 0,
                                    end: 10,
                                    line: 2,
                                    col: 1,
                                },
                            ),
                            span: Span {
                                start: 0,
                                end: 10,
                                line: 2,
                                col: 1,
                            },
                        }],
                        trailing_expr: None,
                        span: Span {
                            start: 0,
                            end: 10,
                            line: 2,
                            col: 1,
                        },
                    },
                    span: Span {
                        start: 0,
                        end: 10,
                        line: 2,
                        col: 1,
                    },
                }],
                structs: vec![],
                tagged_unions: vec![],
                type_aliases: vec![],
                components: vec![],
                motions: vec![],
                stores: vec![],
                designs: vec![],
                imports: vec![ImportStmt::Named(NamedImport {
                    names: vec!["usedSymbol".into()],
                    path: "./utils.omni".into(),
                    span: Span {
                        start: 0,
                        end: 30,
                        line: 1,
                        col: 1,
                    },
                })],
                exports: vec![],
                consts: vec![],
            },
        );
        let program = Program { modules };

        let diags = UnusedImport.check_ast(&program, "test.omni");
        assert_eq!(
            diags.len(),
            0,
            "used imports should not trigger unused-import"
        );
    }
}
