//! Detects unused components, stores, signals, and functions.
//!
//! Category: style
//! Severity: warning

use std::collections::{HashMap, HashSet};

use super::ast_collectors::{
    collect_used_names_in_block, collect_used_names_in_expr, collect_used_names_in_view_node,
};
use super::{LintDiagnostic, LintRule, LintSeverity};
use crate::ir::IRProgram;
use crate::parser::ast::Program;

pub struct DeadCode;

impl LintRule for DeadCode {
    fn name(&self) -> &str {
        "dead-code"
    }
    fn category(&self) -> &str {
        "style"
    }
    fn severity(&self) -> LintSeverity {
        LintSeverity::Warning
    }

    fn check_ast(&self, program: &Program, _file_path: &str) -> Vec<LintDiagnostic> {
        let mut diags = Vec::new();

        for module in program.modules.values() {
            for comp in &module.components {
                let signal_names: HashSet<&str> =
                    comp.signals.iter().map(|s| s.name.as_str()).collect();
                let computed_names: HashSet<&str> =
                    comp.computeds.iter().map(|c| c.name.as_str()).collect();
                let fn_names: HashSet<&str> = comp.fns.iter().map(|f| f.name.as_str()).collect();

                let mut used: HashSet<&str> = HashSet::new();

                for func in &comp.fns {
                    collect_used_names_in_block(&func.body, &mut used);
                }

                for action in &comp.actions {
                    collect_used_names_in_block(&action.body, &mut used);
                }

                for lc in &comp.lifecycles {
                    collect_used_names_in_block(&lc.body, &mut used);
                    for dep in &lc.deps {
                        used.insert(dep.as_str());
                    }
                }

                for c in &comp.computeds {
                    collect_used_names_in_expr(&c.body, &mut used);
                }

                for sig in &comp.signals {
                    collect_used_names_in_expr(&sig.init, &mut used);
                }

                if let Some(ref view) = comp.view {
                    for child in &view.children {
                        collect_used_names_in_view_node(child, &mut used);
                    }
                }

                for name in &signal_names {
                    if !used.contains(name) {
                        diags.push(LintDiagnostic {
                            rule: "dead-code".into(),
                            severity: LintSeverity::Warning,
                            message: format!(
                                "Signal '{}' in component '{}' is never used",
                                name, comp.name
                            ),
                            file: String::new(),
                            line: 0,
                            col: 0,
                        });
                    }
                }

                for name in &computed_names {
                    if !used.contains(name) {
                        diags.push(LintDiagnostic {
                            rule: "dead-code".into(),
                            severity: LintSeverity::Warning,
                            message: format!(
                                "Computed '{}' in component '{}' is never used",
                                name, comp.name
                            ),
                            file: String::new(),
                            line: 0,
                            col: 0,
                        });
                    }
                }

                for name in &fn_names {
                    if !used.contains(name) {
                        diags.push(LintDiagnostic {
                            rule: "dead-code".into(),
                            severity: LintSeverity::Warning,
                            message: format!(
                                "Function '{}' in component '{}' is never called",
                                name, comp.name
                            ),
                            file: String::new(),
                            line: 0,
                            col: 0,
                        });
                    }
                }
            }
        }

        diags
    }

    fn check_ir(&self, ir: &IRProgram, _file_map: &HashMap<String, String>) -> Vec<LintDiagnostic> {
        let mut diags = Vec::new();

        let mut used_stores: HashSet<&str> = HashSet::new();
        for (_name, comp) in ir.all_components() {
            for store_ref in &comp.store_refs {
                used_stores.insert(store_ref.as_str());
            }
        }

        for store_name in ir.all_stores().keys() {
            if !used_stores.contains(store_name) {
                diags.push(LintDiagnostic {
                    rule: self.name().into(),
                    severity: self.severity(),
                    message: format!("Store '{}' is never used by any component", store_name),
                    file: String::new(),
                    line: 0,
                    col: 0,
                });
            }
        }

        diags
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::ir::{ComponentIR, StoreIR};
    use crate::lexer::Span;
    use crate::parser::ast::{
        Block, ComponentDef, Expr, FunctionDef, Literal, Module, SignalDecl, TypeAnnotation,
        ViewBlock, ViewNode, Visibility,
    };

    use super::super::helpers::test_helpers::make_test_ir;
    use super::*;

    #[test]
    fn test_dead_code_detects_unused_store() {
        let mut stores = HashMap::new();
        stores.insert(
            "UnusedStore".into(),
            StoreIR {
                name: "UnusedStore".into(),
                signals: HashMap::new(),
                signal_defaults: HashMap::new(),
                getters: HashMap::new(),
                actions: vec![],
                ..Default::default()
            },
        );
        let ir = make_test_ir(HashMap::new(), stores);

        let diags = DeadCode.check_ir(&ir, &HashMap::new());
        assert_eq!(diags.len(), 1, "expected 1 diagnostic for unused store");
        assert!(
            diags[0].message.contains("UnusedStore"),
            "message should mention the unused store: {}",
            diags[0].message
        );
    }

    #[test]
    fn test_dead_code_skips_used_store() {
        let mut stores = HashMap::new();
        stores.insert(
            "UsedStore".into(),
            StoreIR {
                name: "UsedStore".into(),
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
                fns: vec![],
                store_refs: vec!["UsedStore".into()],
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

        let diags = DeadCode.check_ir(&ir, &HashMap::new());
        assert_eq!(diags.len(), 0, "used stores should not trigger dead-code");
    }

    #[test]
    fn test_dead_code_detects_unused_signal() {
        let span = Span {
            start: 0,
            end: 10,
            line: 1,
            col: 1,
        };
        let mut modules = HashMap::new();
        modules.insert(
            "test.omni".into(),
            Module {
                name: "test".into(),
                functions: vec![],
                structs: vec![],
                tagged_unions: vec![],
                type_aliases: vec![],
                stores: vec![],
                themes: vec![],
                motions: vec![],
                timelines: vec![],
                imports: vec![],
                exports: vec![],
                components: vec![ComponentDef {
                    name: "Comp".into(),
                    props: vec![],
                    signals: vec![SignalDecl {
                        name: "unusedSignal".into(),
                        type_ann: TypeAnnotation::Named {
                            name: "i32".into(),
                            type_args: vec![],
                            span: span.clone(),
                        },
                        init: Expr::Literal(Literal::Int(0), span.clone()),
                    }],
                    computeds: vec![],
                    events: vec![],
                    refs: vec![],
                    store_refs: vec![],
                    if_mode_default: None,
                    hero_defs: vec![],
                    lifecycles: vec![],
                    fns: vec![],
                    actions: vec![],
                    getters: vec![],
                    motions: vec![],
                    timelines: vec![],
                    view: Some(ViewBlock {
                        children: vec![ViewNode::Text("hello".into())],
                    }),
                    styles: vec![],
                    span: span.clone(),
                }],
            },
        );
        let program = crate::parser::ast::Program { modules };

        let diags = DeadCode.check_ast(&program, "test.omni");
        let signal_diags: Vec<_> = diags
            .iter()
            .filter(|d| d.message.contains("Signal"))
            .collect();
        assert!(!signal_diags.is_empty(), "Should detect unused signal");
        assert!(
            signal_diags[0].message.contains("unusedSignal"),
            "Message should mention unused signal: {}",
            signal_diags[0].message
        );
    }

    #[test]
    fn test_dead_code_detects_unused_function() {
        let span = Span {
            start: 0,
            end: 10,
            line: 1,
            col: 1,
        };
        let mut modules = HashMap::new();
        modules.insert(
            "test.omni".into(),
            Module {
                name: "test".into(),
                functions: vec![],
                structs: vec![],
                tagged_unions: vec![],
                type_aliases: vec![],
                stores: vec![],
                themes: vec![],
                motions: vec![],
                timelines: vec![],
                imports: vec![],
                exports: vec![],
                components: vec![ComponentDef {
                    name: "Comp".into(),
                    props: vec![],
                    signals: vec![],
                    computeds: vec![],
                    events: vec![],
                    refs: vec![],
                    store_refs: vec![],
                    if_mode_default: None,
                    hero_defs: vec![],
                    lifecycles: vec![],
                    fns: vec![FunctionDef {
                        visibility: Visibility::Private,
                        name: "unusedFn".into(),
                        params: vec![],
                        return_type: None,
                        is_async: false,
                        body: Block {
                            stmts: vec![],
                            trailing_expr: None,
                            span: span.clone(),
                        },
                        span: span.clone(),
                    }],
                    actions: vec![],
                    getters: vec![],
                    motions: vec![],
                    timelines: vec![],
                    view: Some(ViewBlock {
                        children: vec![ViewNode::Text("hello".into())],
                    }),
                    styles: vec![],
                    span: span.clone(),
                }],
            },
        );
        let program = crate::parser::ast::Program { modules };

        let diags = DeadCode.check_ast(&program, "test.omni");
        let fn_diags: Vec<_> = diags
            .iter()
            .filter(|d| d.message.contains("Function"))
            .collect();
        assert!(!fn_diags.is_empty(), "Should detect unused function");
        assert!(
            fn_diags[0].message.contains("unusedFn"),
            "Message should mention unused function: {}",
            fn_diags[0].message
        );
    }
}
