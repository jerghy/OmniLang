//! Detects unused components, stores, signals, and functions.
//!
//! Category: style
//! Severity: warning

use super::ast_collectors::{
    collect_used_names_in_block, collect_used_names_in_expr, collect_used_names_in_view_node,
};
use super::{LintDiagnostic, LintRule, LintSeverity};
use crate::ir::IRProgram;
use crate::parser::ast::Program;
use rustc_hash::FxHashMap;
use rustc_hash::FxHashSet;

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

    fn check_ast(&self, program: &Program, file_path: &str) -> Vec<LintDiagnostic> {
        let mut diags = Vec::new();

        for module in program.modules.values() {
            for comp in &module.components {
                let signal_names: FxHashSet<&str> =
                    comp.signals.iter().map(|s| s.name.as_str()).collect();
                let computed_names: FxHashSet<&str> =
                    comp.computeds.iter().map(|c| c.name.as_str()).collect();
                let fn_names: FxHashSet<&str> = comp.fns.iter().map(|f| f.name.as_str()).collect();

                let mut used: FxHashSet<&str> = FxHashSet::default();

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

                // Body-level slot fallbacks (ADR-053 D5) may reference
                // signals/computeds — collect their names too.
                for slot in &comp.slots {
                    if let Some(ref content) = slot.default_content {
                        for node in content {
                            collect_used_names_in_view_node(node, &mut used);
                        }
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
                            file: file_path.to_string(),
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
                            file: file_path.to_string(),
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
                            file: file_path.to_string(),
                            line: 0,
                            col: 0,
                        });
                    }
                }
            }
        }

        diags
    }

    fn check_ir(
        &self,
        ir: &IRProgram,
        file_map: &FxHashMap<String, String>,
    ) -> Vec<LintDiagnostic> {
        let mut diags = Vec::new();

        let mut used_stores: FxHashSet<&str> = FxHashSet::default();
        for (_name, comp) in ir.all_components() {
            for store_ref in &comp.store_refs {
                used_stores.insert(store_ref.as_str());
            }
        }

        // Determine the best file path to use: first component's source_file, or first entry from file_map
        let default_file = ir
            .all_components()
            .values()
            .find_map(|c| c.source_file.as_deref())
            .or_else(|| file_map.keys().next().map(|s| s.as_str()))
            .unwrap_or("")
            .to_string();

        for store_name in ir.all_stores().keys() {
            if !used_stores.contains(store_name) {
                let store_file = ir
                    .all_stores()
                    .get(store_name)
                    .and_then(|s| s.source_file.as_deref())
                    .unwrap_or(&default_file);
                diags.push(LintDiagnostic {
                    rule: self.name().into(),
                    severity: self.severity(),
                    message: format!("Store '{}' is never used by any component", store_name),
                    file: store_file.to_string(),
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
    use crate::ir::{ComponentIR, StoreIR};
    use crate::lexer::Span;
    use crate::parser::ast::{
        Block, ComponentDef, Expr, FunctionDef, Literal, Module, SignalDecl, TypeAnnotation,
        ViewBlock, ViewNode, Visibility,
    };
    use rustc_hash::FxHashMap;

    use super::super::helpers::test_helpers::make_test_ir;
    use super::*;

    #[test]
    fn test_dead_code_detects_unused_store() {
        let mut stores = FxHashMap::default();
        stores.insert(
            "UnusedStore".into(),
            StoreIR {
                name: "UnusedStore".into(),
                signals: FxHashMap::default(),
                signal_defaults: FxHashMap::default(),
                getters: FxHashMap::default(),
                actions: vec![],
                ..Default::default()
            },
        );
        let ir = make_test_ir(FxHashMap::default(), stores);

        let diags = DeadCode.check_ir(&ir, &FxHashMap::default());
        assert_eq!(diags.len(), 1, "expected 1 diagnostic for unused store");
        assert!(
            diags[0].message.contains("UnusedStore"),
            "message should mention the unused store: {}",
            diags[0].message
        );
    }

    #[test]
    fn test_dead_code_skips_used_store() {
        let mut stores = FxHashMap::default();
        stores.insert(
            "UsedStore".into(),
            StoreIR {
                name: "UsedStore".into(),
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
                props: FxHashMap::default(),
                prop_defaults: FxHashMap::default(),
                signals: vec![],
                computeds: vec![],
                effects: vec![],
                lifecycles: vec![],
                fns: vec![],
                store_refs: vec!["UsedStore".into()],
                refs: vec![],
                events: vec![],
                view: None,
                styles: vec![],
                ..Default::default()
            },
        );
        let ir = make_test_ir(components, stores);

        let diags = DeadCode.check_ir(&ir, &FxHashMap::default());
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
        let mut modules = FxHashMap::default();
        modules.insert(
            "test.omni".into(),
            Module {
                name: "test".into(),
                functions: vec![],
                structs: vec![],
                tagged_unions: vec![],
                type_aliases: vec![],
                stores: vec![],
                motions: vec![],
                designs: vec![],
                imports: vec![],
                exports: vec![],
                consts: vec![],
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
                    motions: vec![],
                    lifecycles: vec![],
                    fns: vec![],
                    actions: vec![],
                    getters: vec![],
                    slots: vec![],
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
        let mut modules = FxHashMap::default();
        modules.insert(
            "test.omni".into(),
            Module {
                name: "test".into(),
                functions: vec![],
                structs: vec![],
                tagged_unions: vec![],
                type_aliases: vec![],
                stores: vec![],
                motions: vec![],
                designs: vec![],
                imports: vec![],
                exports: vec![],
                consts: vec![],
                components: vec![ComponentDef {
                    name: "Comp".into(),
                    props: vec![],
                    signals: vec![],
                    computeds: vec![],
                    events: vec![],
                    refs: vec![],
                    store_refs: vec![],
                    motions: vec![],
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
                    slots: vec![],
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

    // ── Slot content reference sites (ADR-053 D5) ──────────

    /// Parse source and run the dead-code lint against it.
    fn lint_source(source: &str) -> Vec<LintDiagnostic> {
        let tokens = crate::lexer::tokenize(source);
        let (program, _) = crate::parser::parse(tokens, source);
        DeadCode.check_ast(&program, "test.omni")
    }

    #[test]
    fn test_signal_used_only_in_consumer_slot_projection_not_reported() {
        // ADR-053 D5: <Modal header={<div>{count}</div>} /> — the projected
        // content is a real reference site for the dead-code lint.
        let diags = lint_source(
            "component App { signal count: i32 = 0;              view { <Modal header={<div>{count}</div>} /> } }",
        );
        assert!(
            !diags
                .iter()
                .any(|d| d.message.contains("'count'") && d.message.contains("never used")),
            "signal referenced from a consumer slot projection must not be reported unused, got: {:?}",
            diags
        );
    }

    #[test]
    fn test_signal_used_only_in_body_slot_fallback_not_reported() {
        let diags = lint_source(
            "component Card { signal title: String = \"x\";              @slot body { <p>{title}</p> } view { <div/> } }",
        );
        assert!(
            !diags
                .iter()
                .any(|d| d.message.contains("'title'") && d.message.contains("never used")),
            "signal referenced from a body-slot fallback must not be reported unused, got: {:?}",
            diags
        );
    }

    // ── Inline style binding reference sites (deferral #20) ─────────

    #[test]
    fn test_signal_used_only_in_inline_style_not_reported() {
        // style: { ... } entries are real reference sites — a bare signal
        // reference as a style value must count as a read.
        let diags = lint_source(
            "component App { signal accentColor: String = \"red\";              view { <div style: { color: accentColor }/> } }",
        );
        assert!(
            !diags
                .iter()
                .any(|d| d.message.contains("'accentColor'") && d.message.contains("never used")),
            "signal referenced from an inline style binding must not be reported unused, got: {:?}",
            diags
        );
    }

    #[test]
    fn test_signal_used_only_in_composite_inline_style_not_reported() {
        // Style values can be arbitrary expressions (e.g. string concat),
        // mirroring CssVarsDemo's `border: "2px solid " + accentColor`.
        let diags = lint_source(
            "component App { signal accentColor: String = \"red\";              view { <div style: { border: \"2px solid \" + accentColor }/> } }",
        );
        assert!(
            !diags
                .iter()
                .any(|d| d.message.contains("'accentColor'") && d.message.contains("never used")),
            "signal referenced from a composite inline style binding must not be reported unused, got: {:?}",
            diags
        );
    }

    #[test]
    fn test_unused_signal_still_reported_alongside_style_references() {
        let diags = lint_source(
            "component App { signal usedColor: String = \"red\"; signal unusedColor: String = \"blue\";              view { <div style: { color: usedColor }/> } }",
        );
        assert!(
            !diags
                .iter()
                .any(|d| d.message.contains("'usedColor'") && d.message.contains("never used")),
            "style-referenced signal must not be reported unused, got: {:?}",
            diags
        );
        assert!(
            diags
                .iter()
                .any(|d| d.message.contains("'unusedColor'") && d.message.contains("never used")),
            "genuinely unused signal must still be reported, got: {:?}",
            diags
        );
    }

    // ── Dynamic class map reference sites (same-kind gap as style) ──

    #[test]
    fn test_signal_used_only_in_class_map_not_reported() {
        // `class: { key: expr }` — ClassValue::ClassMap entry values are real
        // reference sites, mirroring Modal.omni's `class: { visible: isOpen }`.
        let diags = lint_source(
            "component App { signal isOpen: bool = false;              view { <div class: { active: isOpen }/> } }",
        );
        assert!(
            !diags
                .iter()
                .any(|d| d.message.contains("'isOpen'") && d.message.contains("never used")),
            "signal referenced from a dynamic class map must not be reported unused, got: {:?}",
            diags
        );
    }

    #[test]
    fn test_unused_signal_still_reported_alongside_class_map_references() {
        let diags = lint_source(
            "component App { signal usedFlag: bool = false; signal unusedFlag: bool = true;              view { <div class: { active: usedFlag }/> } }",
        );
        assert!(
            !diags
                .iter()
                .any(|d| d.message.contains("'usedFlag'") && d.message.contains("never used")),
            "class-map-referenced signal must not be reported unused, got: {:?}",
            diags
        );
        assert!(
            diags
                .iter()
                .any(|d| d.message.contains("'unusedFlag'") && d.message.contains("never used")),
            "genuinely unused signal must still be reported, got: {:?}",
            diags
        );
    }
}
