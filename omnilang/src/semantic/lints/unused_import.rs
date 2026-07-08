//! Detects imports whose symbols are never referenced in the file.
//!
//! Category: style
//! Severity: warning

use rustc_hash::FxHashMap;
use rustc_hash::FxHashSet;
use super::ast_collectors::{
    collect_used_names_in_block, collect_used_names_in_expr, collect_used_names_in_view,
};
use super::helpers::import_span_line_col;
use super::{LintDiagnostic, LintRule, LintSeverity};
use crate::parser::ast::{ImportStmt, Program};

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
    use rustc_hash::FxHashMap;
    use crate::lexer::Span;
    use crate::parser::ast::{
        Block, Expr, FunctionDef, Module, NamedImport, Program, Stmt, Visibility,
    };

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
                stores: vec![],
                themes: vec![],
                designs: vec![],
                motions: vec![],
                timelines: vec![],
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
                stores: vec![],
                themes: vec![],
                designs: vec![],
                motions: vec![],
                timelines: vec![],
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
