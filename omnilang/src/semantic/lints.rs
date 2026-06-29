//! Lint rule framework and first batch of critical lint rules.
//!
//! Rules run in two phases:
//!   1. AST-level: checks that operate on the parsed AST before lowering.
//!   2. IR-level: checks that operate on the merged IR after lowering.
//!
//! To add a new rule:
//!   1. Implement `LintRule` for your struct.
//!   2. Register it in `all_rules()`.
//!   3. Add tests in the `tests` module below.

use std::collections::{HashMap, HashSet};

use crate::ir::IRProgram;
use crate::parser::ast::{ImportStmt, Program};

/// Severity of a lint diagnostic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LintSeverity {
    Error,
    Warning,
    Info,
}

/// A single lint finding.
#[derive(Debug, Clone)]
pub struct LintDiagnostic {
    pub rule: String,
    pub severity: LintSeverity,
    pub message: String,
    pub file: String,
    pub line: u32,
    pub col: u32,
}

/// A lint rule can inspect both AST and IR.
///
/// - AST-level: unused-import, naming-convention, dead-code (decl-level)
/// - IR-level: undefined-ref, type-mismatch, missing-return, dead-code (store-level)
pub trait LintRule {
    fn name(&self) -> &str;
    fn category(&self) -> &str;
    fn severity(&self) -> LintSeverity;

    /// Check against the AST (before lowering). Default: no-op.
    fn check_ast(&self, _program: &Program, _file_path: &str) -> Vec<LintDiagnostic> {
        vec![]
    }

    /// Check against the merged IR (after lowering). Default: no-op.
    /// `file_map` maps module keys to their source file paths for diagnostic locations.
    fn check_ir(
        &self,
        _ir: &IRProgram,
        _file_map: &HashMap<String, String>,
    ) -> Vec<LintDiagnostic> {
        vec![]
    }
}

// ═══════════════════════════════════════════════════════════
// Runner
// ═══════════════════════════════════════════════════════════

/// Run all registered lint rules against ASTs and merged IR.
pub fn run_all(programs: &HashMap<String, Program>, ir: &IRProgram) -> Vec<LintDiagnostic> {
    let rules: Vec<Box<dyn LintRule>> = all_rules();
    let mut diags = Vec::new();

    // Build file_map: module_key → source_file_path from IR
    let file_map: HashMap<String, String> = programs
        .keys()
        .map(|path| {
            // Use the path itself as key; lints can match against module names
            (path.clone(), path.clone())
        })
        .collect();

    // AST-level checks — each file gets checked individually
    for (file_path, program) in programs {
        for rule in &rules {
            diags.extend(rule.check_ast(program, file_path));
        }
    }

    // IR-level checks — run against the merged program with file context
    for rule in &rules {
        diags.extend(rule.check_ir(ir, &file_map));
    }

    diags
}

/// Register all lint rules here.
/// Add new rules to this list when implementing them.
fn all_rules() -> Vec<Box<dyn LintRule>> {
    vec![
        // Batch 1: Critical correctness rules (errors)
        Box::new(rules::UndefinedRef),
        Box::new(rules::TypeMismatch),
        // Batch 2: Store/component safety rules (errors)
        Box::new(rules::StoreFieldDirectMutation),
        Box::new(rules::PropImmutability),
        Box::new(rules::ComputedPurity),
        // Batch 1: Important warnings
        Box::new(rules::MissingReturn),
        Box::new(rules::DeadCode),
        Box::new(rules::UnusedImport),
        // ── New event data rules ──
        Box::new(rules::InvalidEventField),
        Box::new(rules::StoreActionEventData),
    ]
}

// ═══════════════════════════════════════════════════════════
// Rule Implementations
// ═══════════════════════════════════════════════════════════

pub mod rules {
    use super::*;
    use crate::ir::{BlockIR, ExprIR, StatementIR, ViewTree, event_data_schema};
    use crate::parser::ast::{
        AssignTarget, BinaryOpKind, Block, ElementProp, EventHandler, Expr, Stmt, TemplatePart,
        ViewBlock, ViewNode,
    };

    // ── UndefinedRef ─────────────────────────────────────
    //
    // Checks that every store reference from a component actually
    // refers to an existing store in the IR program.
    //
    // Category: correctness
    // Python equivalent: e0108_method_not_found.py (partial)

    pub struct UndefinedRef;

    impl LintRule for UndefinedRef {
        fn name(&self) -> &str {
            "undefined-ref"
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
            _file_map: &HashMap<String, String>,
        ) -> Vec<LintDiagnostic> {
            let mut diags = Vec::new();
            for (name, comp) in ir.all_components() {
                for store_ref in &comp.store_refs {
                    if !ir.all_stores().contains_key(store_ref.as_str()) {
                        diags.push(LintDiagnostic {
                            rule: self.name().into(),
                            severity: self.severity(),
                            message: format!(
                                "Component '{}' references undefined store '{}'",
                                name, store_ref
                            ),
                            file: name.to_string(),
                            line: 0,
                            col: 0,
                        });
                    }
                }
            }
            diags
        }
    }

    // ── TypeMismatch ─────────────────────────────────────
    //
    // Checks assignment and return type compatibility.
    //
    // Category: correctness
    // Python equivalent: e0101_type_mismatch.py
    //
    // TODO: Full type inference integration. Currently a placeholder
    // that catches obvious mismatches in IR-level type annotations.

    pub struct TypeMismatch;

    impl LintRule for TypeMismatch {
        fn name(&self) -> &str {
            "type-mismatch"
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
            _file_map: &HashMap<String, String>,
        ) -> Vec<LintDiagnostic> {
            let mut diags = Vec::new();

            for (comp_name, comp) in ir.all_components() {
                // Check signal initializer types
                for state in &comp.states {
                    let type_kind = &state.r#type.kind;
                    let init = &state.initial_value;
                    if !Self::types_compatible(type_kind, init) {
                        diags.push(LintDiagnostic {
                            rule: self.name().into(),
                            severity: self.severity(),
                            message: format!(
                                "Signal '{}' in component '{}': initializer type \
                                 does not match declared type '{}'",
                                state.name, comp_name, type_kind
                            ),
                            file: String::new(),
                            line: 0,
                            col: 0,
                        });
                    }
                }

                // Check function return types against return statements
                for func in &comp.fns {
                    if let Some(ref ret_type) = func.return_type {
                        Self::check_function_returns(
                            &func.body,
                            &ret_type.kind,
                            comp_name,
                            &func.name,
                            &mut diags,
                        );
                    }
                }
            }
            diags
        }
    }

    impl TypeMismatch {
        fn types_compatible(type_ann: &str, value: &ExprIR) -> bool {
            let type_ann = type_ann.to_lowercase();
            match value {
                ExprIR::LiteralExpr { value } => match value {
                    serde_json::Value::Number(_) => {
                        type_ann.contains("i32")
                            || type_ann.contains("i64")
                            || type_ann.contains("f64")
                            || type_ann.contains("f32")
                            || type_ann.contains("int")
                            || type_ann.contains("float")
                            || type_ann.contains("num")
                    }
                    serde_json::Value::String(_) => {
                        type_ann.contains("str") || type_ann.contains("String")
                    }
                    serde_json::Value::Bool(_) => type_ann.contains("bool") || type_ann == "bool",
                    serde_json::Value::Null => true,
                    _ => true,
                },
                _ => true, // Conservative: non-literal = compatible
            }
        }

        fn check_function_returns(
            body: &BlockIR,
            ret_type: &str,
            comp_name: &str,
            func_name: &str,
            diags: &mut Vec<LintDiagnostic>,
        ) {
            let ret_type_lower = ret_type.to_lowercase();
            let is_void =
                ret_type_lower == "void" || ret_type_lower == "()" || ret_type_lower == "unit";

            for stmt in &body.statements {
                match stmt {
                    StatementIR::ReturnStmtIR { value } => {
                        if is_void && value.is_some() {
                            diags.push(LintDiagnostic {
                                rule: "type-mismatch".into(),
                                severity: LintSeverity::Error,
                                message: format!(
                                    "Function '{}.{}' has void return type but returns a value",
                                    comp_name, func_name
                                ),
                                file: String::new(),
                                line: 0,
                                col: 0,
                            });
                        }
                    }
                    // Recurse into nested blocks
                    StatementIR::IfStmtIR {
                        then_body,
                        else_body,
                        ..
                    } => {
                        Self::check_function_returns(
                            then_body, ret_type, comp_name, func_name, diags,
                        );
                        if let Some(ref eb) = else_body {
                            Self::check_function_returns(eb, ret_type, comp_name, func_name, diags);
                        }
                    }
                    StatementIR::WhileStmtIR { body, .. } => {
                        Self::check_function_returns(body, ret_type, comp_name, func_name, diags);
                    }
                    StatementIR::ForInStmtIR { body, .. } => {
                        Self::check_function_returns(body, ret_type, comp_name, func_name, diags);
                    }
                    StatementIR::BlockStmtIR { body } => {
                        Self::check_function_returns(body, ret_type, comp_name, func_name, diags);
                    }
                    _ => {}
                }
            }
        }
    }

    // ── StoreFieldDirectMutation ─────────────────────────
    //
    // Detects direct mutation of store signals from component code
    // outside of the store's own action bodies.
    //
    // Category: correctness
    // Error: "Store signal 'X.Y' modified outside of store action"

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
            _file_map: &HashMap<String, String>,
        ) -> Vec<LintDiagnostic> {
            let mut diags = Vec::new();

            // Build map: store_name -> set of signal names
            let store_signals: HashMap<&str, HashSet<&str>> = ir
                .all_stores()
                .iter()
                .map(|(name, store)| {
                    (*name, store.signals.keys().map(|s| s.as_str()).collect())
                })
                .collect();

            if store_signals.is_empty() {
                return diags;
            }

            for (_comp_name, comp) in ir.all_components() {
                // Check functions and actions
                for func in &comp.fns {
                    check_block_ir_for_store_mutations(
                        &func.body,
                        &store_signals,
                        &func.name,
                        &mut diags,
                    );
                }

                // Check lifecycles
                for lc in &comp.lifecycles {
                    check_block_ir_for_store_mutations(
                        &lc.body,
                        &store_signals,
                        &format!("@{}", lc.kind),
                        &mut diags,
                    );
                }

                // Check effects
                for effect in &comp.effects {
                    check_block_ir_for_store_mutations(
                        &effect.body,
                        &store_signals,
                        "@effect",
                        &mut diags,
                    );
                }

                // Check inline event handlers in view tree
                if let Some(ref view) = comp.view {
                    check_view_tree_events_for_store_mutations(
                        view,
                        &store_signals,
                        &mut diags,
                    );
                }

                // Check computed bodies (walk expressions for mutations)
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

    // ── PropImmutability ─────────────────────────────────
    //
    // Detects reassignment of component props.
    // Props are read-only — they flow from parent to child.
    //
    // Category: correctness
    // Error: "Cannot assign to prop 'X' — component props are read-only"

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

        fn check_ast(
            &self,
            program: &Program,
            _file_path: &str,
        ) -> Vec<LintDiagnostic> {
            let mut diags = Vec::new();

            for module in program.modules.values() {
                for comp in &module.components {
                    let prop_names: HashSet<&str> =
                        comp.props.iter().map(|p| p.name.as_str()).collect();

                    if prop_names.is_empty() {
                        continue;
                    }

                    // Check component functions
                    for func in &comp.fns {
                        check_block_for_prop_assign(
                            &func.body,
                            &prop_names,
                            &comp.name,
                            &func.name,
                            &mut diags,
                        );
                    }

                    // Check component actions
                    for action in &comp.actions {
                        check_block_for_prop_assign(
                            &action.body,
                            &prop_names,
                            &comp.name,
                            &action.name,
                            &mut diags,
                        );
                    }

                    // Check lifecycle hooks
                    for lc in &comp.lifecycles {
                        check_block_for_prop_assign(
                            &lc.body,
                            &prop_names,
                            &comp.name,
                            &format!("@{:?}", lc.kind),
                            &mut diags,
                        );
                    }

                    // Check inline event handlers in view
                    if let Some(ref view) = comp.view {
                        check_view_for_prop_assign(
                            &view.children,
                            &prop_names,
                            &comp.name,
                            &mut diags,
                        );
                    }
                }
            }

            diags
        }
    }

    // ── ComputedPurity ───────────────────────────────────
    //
    // Detects side effects (mutations) in computed value bodies.
    // Computed values must be pure — derived from signals only.
    //
    // Category: correctness
    // Error: "Computed 'X' contains side effects — computed values must be pure"

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

        fn check_ast(
            &self,
            program: &Program,
            _file_path: &str,
        ) -> Vec<LintDiagnostic> {
            let mut diags = Vec::new();

            for module in program.modules.values() {
                for comp in &module.components {
                    let signal_names: HashSet<&str> =
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
                                file: String::new(),
                                line: 0,
                                col: 0,
                            });
                        }
                    }

                    // Also check store getters for purity
                    for store in &module.stores {
                        let store_signal_names: HashSet<&str> =
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
                                    file: String::new(),
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

    // ── MissingReturn ────────────────────────────────────
    //
    // Flags functions with a non-void return type that can fall
    // off the end without a return statement.
    //
    // Category: correctness
    // Python equivalent: (combined with type checking)

    pub struct MissingReturn;

    impl LintRule for MissingReturn {
        fn name(&self) -> &str {
            "missing-return"
        }
        fn category(&self) -> &str {
            "correctness"
        }
        fn severity(&self) -> LintSeverity {
            LintSeverity::Warning
        }

        fn check_ir(
            &self,
            ir: &IRProgram,
            _file_map: &HashMap<String, String>,
        ) -> Vec<LintDiagnostic> {
            let mut diags = Vec::new();
            for (comp_name, comp) in ir.all_components() {
                for func in &comp.fns {
                    if let Some(ref ret_type) = func.return_type {
                        let kind = ret_type.kind.as_str();
                        if kind != "void" && kind != "()" && kind != "Unit" {
                            let has_return =
                                statements_contain_return(&func.body.statements, false);
                            if !has_return {
                                diags.push(LintDiagnostic {
                                    rule: self.name().into(),
                                    severity: self.severity(),
                                    message: format!(
                                        "Function '{}' in component '{}' \
                                         has return type '{}' but no return statement",
                                        func.name, comp_name, ret_type.kind
                                    ),
                                    file: String::new(),
                                    line: 0,
                                    col: 0,
                                });
                            }
                        }
                    }
                }
            }
            // Also check top-level functions (stored in a synthetic component
            // during lowering, or we can check the AST directly).
            // Currently IR only has component fns — expand in Batch 2.
            diags
        }
    }

    // ── DeadCode ─────────────────────────────────────────
    //
    // Detects unused components, stores, signals, and functions.
    //
    // Category: style
    // Python equivalent: s0103_dead_code.py, s0106_mutable_not_mutated.py

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

        fn check_ast(
            &self,
            program: &Program,
            _file_path: &str,
        ) -> Vec<LintDiagnostic> {
            let mut diags = Vec::new();

            for module in program.modules.values() {
                for comp in &module.components {
                    // Collect all declared names
                    let signal_names: HashSet<&str> =
                        comp.signals.iter().map(|s| s.name.as_str()).collect();
                    let computed_names: HashSet<&str> =
                        comp.computeds.iter().map(|c| c.name.as_str()).collect();
                    let fn_names: HashSet<&str> =
                        comp.fns.iter().map(|f| f.name.as_str()).collect();

                    // Collect all used names in the component
                    let mut used: HashSet<&str> = HashSet::new();

                    // Walk function bodies
                    for func in &comp.fns {
                        collect_used_names_in_block(&func.body, &mut used);
                    }

                    // Walk action bodies
                    for action in &comp.actions {
                        collect_used_names_in_block(&action.body, &mut used);
                    }

                    // Walk lifecycle bodies
                    for lc in &comp.lifecycles {
                        collect_used_names_in_block(&lc.body, &mut used);
                        for dep in &lc.deps {
                            used.insert(dep.as_str());
                        }
                    }

                    // Walk computed bodies
                    for c in &comp.computeds {
                        collect_used_names_in_expr(&c.body, &mut used);
                    }

                    // Walk signal initializers
                    for sig in &comp.signals {
                        collect_used_names_in_expr(&sig.init, &mut used);
                    }

                    // Walk view tree
                    if let Some(ref view) = comp.view {
                        for child in &view.children {
                            collect_used_names_in_view_node(child, &mut used);
                        }
                    }

                    // Report unused signals
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

                    // Report unused computeds
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

                    // Report unused functions
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

        fn check_ir(
            &self,
            ir: &IRProgram,
            _file_map: &HashMap<String, String>,
        ) -> Vec<LintDiagnostic> {
            let mut diags = Vec::new();

            // Collect all stores referenced by any component
            let mut used_stores: HashSet<&str> = HashSet::new();
            for (_name, comp) in ir.all_components() {
                for store_ref in &comp.store_refs {
                    used_stores.insert(store_ref.as_str());
                }
            }

            // Warn about stores that are never used
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

    // ── UnusedImport ─────────────────────────────────────
    //
    // Detects imports whose symbols are never referenced in the file.
    //
    // Category: style
    // Python equivalent: s0102_unused_import.py

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

        fn check_ast(&self, program: &Program, _file_path: &str) -> Vec<LintDiagnostic> {
            let mut diags = Vec::new();

            // Collect all imported names across all modules
            let mut imported: HashMap<&str, (&ImportStmt, &[String])> = HashMap::new();
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
                        // Namespace imports and lib markers are not name-level
                        ImportStmt::Namespace(_) | ImportStmt::Lib(_) => {}
                    }
                }
            }

            if imported.is_empty() {
                return diags;
            }

            // Collect all name references across all modules
            let mut used: HashSet<&str> = HashSet::new();
            for module in program.modules.values() {
                for func in &module.functions {
                    collect_used_names_in_block(&func.body, &mut used);
                }
                for comp in &module.components {
                    // Component name itself
                    used.insert(comp.name.as_str());
                    // Signal initializers, computed bodies, lifecycle bodies
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
                    // Actions are also functions:
                    for action in &comp.actions {
                        collect_used_names_in_block(&action.body, &mut used);
                    }
                    // View blocks reference component names
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

            // Check imports
            for (name, &(import, _names)) in &imported {
                if !used.contains(name) {
                    // Get source location from the import
                    let (line, col) = import_span_line_col(import);
                    diags.push(LintDiagnostic {
                        rule: self.name().into(),
                        severity: self.severity(),
                        message: format!("Import '{}' is never used", name),
                        file: String::new(),
                        line,
                        col,
                    });
                }
            }

            diags
        }
    }

    // ── InvalidEventField ────────────────────────────────
    //
    // Checks that `e.field` references in event handlers refer to fields
    // that exist in the event's EventDataKind schema.
    //
    // Category: correctness

    pub struct InvalidEventField;

    impl LintRule for InvalidEventField {
        fn name(&self) -> &str {
            "invalid-event-field"
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
            _file_map: &HashMap<String, String>,
        ) -> Vec<LintDiagnostic> {
            let mut diags = Vec::new();
            for (_comp_name, comp) in ir.all_components() {
                if let Some(ref view) = comp.view {
                    check_view_invalid_fields(view, &comp.name, self, &mut diags);
                }
            }
            diags
        }
    }

    fn check_view_invalid_fields(
        tree: &ViewTree,
        comp_name: &str,
        rule: &InvalidEventField,
        diags: &mut Vec<LintDiagnostic>,
    ) {
        match tree {
            ViewTree::Element(el) => {
                for (_ev_name, handler) in &el.events {
                    if let (Some(ref kind), Some(ref used)) =
                        (&handler.event_data, &handler.used_fields)
                    {
                        let schema = event_data_schema(kind);
                        let valid: HashSet<&str> =
                            schema.iter().map(|f| f.name.as_str()).collect();
                        for field in used {
                            if !valid.contains(field.as_str()) {
                                diags.push(LintDiagnostic {
                                    rule: rule.name().into(),
                                    severity: rule.severity(),
                                    message: format!(
                                        "`e.{}` is not a field of {:?}. Available fields: {:?}",
                                        field,
                                        kind,
                                        schema
                                            .iter()
                                            .map(|f| f.name.as_str())
                                            .collect::<Vec<_>>()
                                    ),
                                    file: comp_name.to_string(),
                                    line: el.source_line,
                                    col: el.source_col,
                                });
                            }
                        }
                    }
                }
                for child in &el.children {
                    check_view_invalid_fields(child, comp_name, rule, diags);
                }
            }
            ViewTree::Conditional(cond) => {
                for branch in &cond.branches {
                    if let Some(ref body) = branch.body {
                        check_view_invalid_fields(body, comp_name, rule, diags);
                    }
                }
                if let Some(ref fallback) = cond.fallback {
                    check_view_invalid_fields(fallback, comp_name, rule, diags);
                }
            }
            ViewTree::ListRendering(list) => {
                if let Some(ref body) = list.body {
                    check_view_invalid_fields(body, comp_name, rule, diags);
                }
                if let Some(ref empty) = list.empty_body {
                    check_view_invalid_fields(empty, comp_name, rule, diags);
                }
            }
            _ => {}
        }
    }

    // ── StoreActionEventData ─────────────────────────────
    //
    // Store actions receive no event data. If a store_action handler's
    // target expression references `e`, it's a compile error.
    //
    // Category: correctness

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
            _file_map: &HashMap<String, String>,
        ) -> Vec<LintDiagnostic> {
            let mut diags = Vec::new();
            for (comp_name, comp) in ir.all_components() {
                if let Some(ref view) = comp.view {
                    check_view_store_actions(view, comp_name, self, &mut diags);
                }
            }
            diags
        }
    }

    fn check_view_store_actions(
        tree: &ViewTree,
        comp_name: &str,
        rule: &StoreActionEventData,
        diags: &mut Vec<LintDiagnostic>,
    ) {
        match tree {
            ViewTree::Element(el) => {
                for (_ev_name, handler) in &el.events {
                    if handler.kind == "store_action" {
                        // used_fields should be None for store_actions
                        if handler.used_fields.is_some()
                            && !handler.used_fields.as_ref().unwrap().is_empty()
                        {
                            diags.push(LintDiagnostic {
                                rule: rule.name().into(),
                                severity: rule.severity(),
                                message: format!(
                                    "Store action `{}` cannot access event data. \
                                     Store actions receive no arguments. \
                                     Use an inline handler or fn_ref instead.",
                                    handler.r#ref.as_deref().unwrap_or("?")
                                ),
                                file: comp_name.to_string(),
                                line: el.source_line,
                                col: el.source_col,
                            });
                        }
                    }
                }
                for child in &el.children {
                    check_view_store_actions(child, comp_name, rule, diags);
                }
            }
            _ => {}
        }
    }

    // ── Helpers ──────────────────────────────────────────

    /// Recursively check if a block's statement tree contains a return.
    pub fn statements_contain_return(stmts: &[StatementIR], _in_loop: bool) -> bool {
        for stmt in stmts {
            match stmt {
                StatementIR::ReturnStmtIR { .. } => return true,
                StatementIR::IfStmtIR {
                    then_body,
                    else_body,
                    ..
                } => {
                    if statements_contain_return(&then_body.statements, _in_loop) {
                        return true;
                    }
                    if let Some(ref else_body) = else_body {
                        if statements_contain_return(&else_body.statements, _in_loop) {
                            return true;
                        }
                    }
                }
                StatementIR::BlockStmtIR { body } => {
                    if statements_contain_return(&body.statements, _in_loop) {
                        return true;
                    }
                }
                StatementIR::WhileStmtIR { body, .. } => {
                    if statements_contain_return(&body.statements, true) {
                        return true;
                    }
                }
                StatementIR::ForInStmtIR { body, .. } => {
                    if statements_contain_return(&body.statements, true) {
                        return true;
                    }
                }
                StatementIR::LetStmtIR { .. }
                | StatementIR::MutationStmt { .. }
                | StatementIR::EventEmitStmtIR { .. }
                | StatementIR::ExprStmtIR { .. }
                | StatementIR::BreakStmtIR
                | StatementIR::ContinueStmtIR
                | StatementIR::ThrowStmtIR { .. } => {}
                StatementIR::TryCatchStmtIR {
                    try_body,
                    catch_body,
                    finally_body,
                    ..
                } => {
                    if statements_contain_return(&try_body.statements, _in_loop) {
                        if let Some(ref cb) = catch_body {
                            if statements_contain_return(&cb.statements, _in_loop) {
                                return true;
                            }
                        }
                    }
                    if let Some(ref fb) = finally_body {
                        if statements_contain_return(&fb.statements, _in_loop) {
                            return true;
                        }
                    }
                }
            }
        }
        false
    }

    /// Get source line/col from an ImportStmt for diagnostic reporting.
    fn import_span_line_col(import: &ImportStmt) -> (u32, u32) {
        match import {
            ImportStmt::Named(n) => (n.span.line, n.span.col),
            ImportStmt::Namespace(ns) => (ns.span.line, ns.span.col),
            ImportStmt::TypeOnly(t) => (t.span.line, t.span.col),
            ImportStmt::Lib(_) => (0, 0),
        }
    }

    // ── AST name reference collectors ────────────────────

    fn collect_used_names_in_block<'a>(block: &'a Block, used: &mut HashSet<&'a str>) {
        for stmt in &block.stmts {
            match stmt {
                crate::parser::ast::Stmt::VarDecl { init, .. } => {
                    if let Some(ref e) = init {
                        collect_used_names_in_expr(e, used);
                    }
                }
                crate::parser::ast::Stmt::Assign { value, .. } => {
                    collect_used_names_in_expr(value, used);
                }
                Stmt::ExprStmt { expr, .. } => {
                    collect_used_names_in_expr(expr, used);
                }
                crate::parser::ast::Stmt::Return { value: Some(e), .. } => {
                    collect_used_names_in_expr(e, used);
                }
                crate::parser::ast::Stmt::Return { .. } => {}
                crate::parser::ast::Stmt::If {
                    condition,
                    then_body,
                    else_ifs,
                    else_body,
                    ..
                } => {
                    collect_used_names_in_expr(condition, used);
                    collect_used_names_in_block(then_body, used);
                    for (cond, body) in else_ifs {
                        collect_used_names_in_expr(cond, used);
                        collect_used_names_in_block(body, used);
                    }
                    if let Some(ref body) = else_body {
                        collect_used_names_in_block(body, used);
                    }
                }
                crate::parser::ast::Stmt::While {
                    condition, body, ..
                } => {
                    collect_used_names_in_expr(condition, used);
                    collect_used_names_in_block(body, used);
                }
                crate::parser::ast::Stmt::ForIn { iterable, body, .. } => {
                    collect_used_names_in_expr(iterable, used);
                    collect_used_names_in_block(body, used);
                }
                crate::parser::ast::Stmt::Print { arg, .. } => {
                    collect_used_names_in_expr(arg, used);
                }
                crate::parser::ast::Stmt::Break(_) | crate::parser::ast::Stmt::Continue(_) => {}
                crate::parser::ast::Stmt::EventEmit { args, .. } => {
                    for arg in args {
                        collect_used_names_in_expr(arg, used);
                    }
                }
                crate::parser::ast::Stmt::Throw { expr, .. } => {
                    collect_used_names_in_expr(expr, used);
                }
                crate::parser::ast::Stmt::TryCatch {
                    try_body,
                    catch_body,
                    finally_body,
                    ..
                } => {
                    collect_used_names_in_block(try_body, used);
                    if let Some(ref b) = catch_body {
                        collect_used_names_in_block(b, used);
                    }
                    if let Some(ref b) = finally_body {
                        collect_used_names_in_block(b, used);
                    }
                }
            }
        }
        if let Some(ref expr) = block.trailing_expr {
            collect_used_names_in_expr(expr, used);
        }
    }

    fn collect_used_names_in_expr<'a>(expr: &'a Expr, used: &mut HashSet<&'a str>) {
        match expr {
            Expr::VarRef(name, _) => {
                used.insert(name.as_str());
            }
            Expr::Literal(..) => {}
            Expr::BinaryOp { left, right, .. } => {
                collect_used_names_in_expr(left, used);
                collect_used_names_in_expr(right, used);
            }
            Expr::UnaryOp { operand, .. } => {
                collect_used_names_in_expr(operand, used);
            }
            Expr::FuncCall { func, args, .. } => {
                collect_used_names_in_expr(func, used);
                for a in args {
                    collect_used_names_in_expr(a, used);
                }
            }
            Expr::MethodCall { object, args, .. } => {
                collect_used_names_in_expr(object, used);
                for a in args {
                    collect_used_names_in_expr(a, used);
                }
            }
            Expr::FieldAccess { object, .. } | Expr::OptionalChain { object, .. } => {
                collect_used_names_in_expr(object, used);
            }
            Expr::IndexGet { object, index, .. } => {
                collect_used_names_in_expr(object, used);
                collect_used_names_in_expr(index, used);
            }
            Expr::ArrayLiteral { elements, .. } => {
                for e in elements {
                    collect_used_names_in_expr(e, used);
                }
            }
            Expr::MapLiteral { entries, .. } => {
                for (k, v) in entries {
                    collect_used_names_in_expr(k, used);
                    collect_used_names_in_expr(v, used);
                }
            }
            Expr::IfExpr {
                condition,
                then_expr,
                else_expr,
                ..
            }
            | Expr::TernaryExpr {
                condition,
                then_expr,
                else_expr,
                ..
            } => {
                collect_used_names_in_expr(condition, used);
                collect_used_names_in_expr(then_expr, used);
                collect_used_names_in_expr(else_expr, used);
            }
            Expr::TemplateString { parts, .. } => {
                for part in parts {
                    if let crate::parser::ast::TemplatePart::Expr(e) = part {
                        collect_used_names_in_expr(e, used);
                    }
                }
            }
            Expr::StructLiteral { fields, .. } => {
                for (_, val) in fields {
                    collect_used_names_in_expr(val, used);
                }
            }
            Expr::StoreFieldAccess { .. } => {
                // Store field access doesn't necessarily use a named import
                // in the current file — it's a direct reference to a store
            }
            Expr::ClosureExpr { body, .. } => {
                collect_used_names_in_expr(body, used);
            }
            Expr::AwaitExpr { operand, .. } => {
                collect_used_names_in_expr(operand, used);
            }
            Expr::MatchExpr { matched, arms, .. } => {
                collect_used_names_in_expr(matched, used);
                for arm in arms {
                    collect_used_names_in_block(&arm.body, used);
                }
            }
            Expr::Block { .. } => {}
            Expr::NotYetImplemented { .. } => {}
        }
    }

    fn collect_used_names_in_view<'a>(view: &'a ViewBlock, used: &mut HashSet<&'a str>) {
        for node in &view.children {
            collect_used_names_in_view_node(node, used);
        }
    }

    fn collect_used_names_in_view_node<'a>(node: &'a ViewNode, used: &mut HashSet<&'a str>) {
        match node {
            ViewNode::Element(el) => {
                for prop in &el.props {
                    match prop {
                        crate::parser::ast::ElementProp::Static { value, .. } => {
                            collect_used_names_in_expr(value, used);
                        }
                        crate::parser::ast::ElementProp::Bind { signal_path, .. } => {
                            // Signal paths may reference component-scoped names
                            if let Some(dot) = signal_path.find('.') {
                                used.insert(&signal_path[..dot]);
                            } else {
                                used.insert(signal_path.as_str());
                            }
                        }
                        crate::parser::ast::ElementProp::Event { handler, .. } => match handler {
                            crate::parser::ast::EventHandler::InlineFn { body, .. } => {
                                collect_used_names_in_block(body, used);
                            }
                            crate::parser::ast::EventHandler::FnRef(name) => {
                                used.insert(name.as_str());
                            }
                            crate::parser::ast::EventHandler::StoreAction {
                                store, action, ..
                            } => {
                                used.insert(store.as_str());
                                used.insert(action.as_str());
                            }
                        },
                        _ => {}
                    }
                }
                for child in &el.children {
                    collect_used_names_in_view_node(child, used);
                }
            }
            ViewNode::Text(_) => {}
            ViewNode::Interpolation(expr) => {
                collect_used_names_in_expr(expr, used);
            }
            ViewNode::IfDirective(if_dir) => {
                collect_used_names_in_expr(&if_dir.condition, used);
                for node in &if_dir.body {
                    collect_used_names_in_view_node(node, used);
                }
                for (cond, body) in &if_dir.else_ifs {
                    collect_used_names_in_expr(cond, used);
                    for node in body {
                        collect_used_names_in_view_node(node, used);
                    }
                }
                if let Some(ref body) = if_dir.else_body {
                    for node in body {
                        collect_used_names_in_view_node(node, used);
                    }
                }
            }
            ViewNode::ForDirective(for_dir) => {
                collect_used_names_in_expr(&for_dir.iterable, used);
                for node in &for_dir.body {
                    collect_used_names_in_view_node(node, used);
                }
                if let Some(ref empty_body) = for_dir.empty_body {
                    for node in empty_body {
                        collect_used_names_in_view_node(node, used);
                    }
                }
            }
            ViewNode::SlotDirective { .. } => {}
        }
    }

    // ── Store / Prop / Computed validation helpers ──────

    /// Recursively walk a BlockIR looking for store signal mutations.
    fn check_block_ir_for_store_mutations(
        block: &BlockIR,
        store_signals: &HashMap<&str, HashSet<&str>>,
        context_name: &str,
        diags: &mut Vec<LintDiagnostic>,
    ) {
        for stmt in &block.statements {
            match stmt {
                StatementIR::MutationStmt { intent } => {
                    if let Some(ref mutation) = intent {
                        if let Some(ref owner) = mutation.target.owner {
                            if let Some(signals) = store_signals.get(owner.as_str()) {
                                if signals.contains(mutation.target.name.as_str()) {
                                    diags.push(LintDiagnostic {
                                        rule: "store-field-direct-mutation".into(),
                                        severity: LintSeverity::Error,
                                        message: format!(
                                            "Store signal '{}.{}' modified in '{}' — \
                                             store data must only be changed through store actions",
                                            owner, mutation.target.name, context_name
                                        ),
                                        file: String::new(),
                                        line: 0,
                                        col: 0,
                                    });
                                }
                            }
                        }
                    }
                }
                StatementIR::IfStmtIR {
                    then_body,
                    else_ifs,
                    else_body,
                    ..
                } => {
                    check_block_ir_for_store_mutations(
                        then_body, store_signals, context_name, diags,
                    );
                    for (_, body) in else_ifs {
                        check_block_ir_for_store_mutations(
                            body, store_signals, context_name, diags,
                        );
                    }
                    if let Some(ref body) = else_body {
                        check_block_ir_for_store_mutations(
                            body, store_signals, context_name, diags,
                        );
                    }
                }
                StatementIR::WhileStmtIR { body, .. }
                | StatementIR::ForInStmtIR { body, .. } => {
                    check_block_ir_for_store_mutations(
                        body, store_signals, context_name, diags,
                    );
                }
                StatementIR::BlockStmtIR { body } => {
                    check_block_ir_for_store_mutations(
                        body, store_signals, context_name, diags,
                    );
                }
                StatementIR::TryCatchStmtIR {
                    try_body,
                    catch_body,
                    finally_body,
                    ..
                } => {
                    check_block_ir_for_store_mutations(
                        try_body, store_signals, context_name, diags,
                    );
                    if let Some(ref body) = catch_body {
                        check_block_ir_for_store_mutations(
                            body, store_signals, context_name, diags,
                        );
                    }
                    if let Some(ref body) = finally_body {
                        check_block_ir_for_store_mutations(
                            body, store_signals, context_name, diags,
                        );
                    }
                }
                // ExprStmtIR may contain method calls that mutate store data,
                // but those are harder to detect at IR level — focus on explicit
                // MutationStmt which covers Stmt::Assign patterns.
                StatementIR::ExprStmtIR { expr } => {
                    if expr_ir_contains_store_mutation(expr, store_signals) {
                        diags.push(LintDiagnostic {
                            rule: "store-field-direct-mutation".into(),
                            severity: LintSeverity::Error,
                            message: format!(
                                "Expression in '{}' mutates store data — \
                                 store data must only be changed through store actions",
                                context_name
                            ),
                            file: String::new(),
                            line: 0,
                            col: 0,
                        });
                    }
                }
                _ => {}
            }
        }
    }

    /// Check if an ExprIR contains a store mutation (method calls like .set/.push on store fields).
    fn expr_ir_contains_store_mutation(
        expr: &ExprIR,
        store_signals: &HashMap<&str, HashSet<&str>>,
    ) -> bool {
        match expr {
            ExprIR::StoreFieldAccessExpr {
                store_alias,
                call_args,
                ..
            } => {
                // store.field(args) — if args are non-empty, it's likely a mutation call
                if !call_args.is_empty() {
                    return store_signals.contains_key(store_alias.as_str());
                }
                false
            }
            ExprIR::MethodCallExpr { object, method, .. } => {
                if matches!(method.as_str(), "set" | "push" | "pop" | "inc" | "increment" | "dec" | "decrement") {
                    if let ExprIR::StoreFieldAccessExpr { store_alias, .. } = object.as_ref() {
                        return store_signals.contains_key(store_alias.as_str());
                    }
                    if let ExprIR::FieldAccessExpr { object: inner, .. } = object.as_ref() {
                        if let ExprIR::IdentifierExpr { name } = inner.as_ref() {
                            return store_signals.contains_key(name.as_str());
                        }
                    }
                }
                expr_ir_contains_store_mutation(object, store_signals)
            }
            ExprIR::BinaryOpExpr { left, right, op, .. } => {
                if matches!(op.as_str(), "assign" | "addassign" | "subassign" | "mulassign" | "divassign") {
                    if let ExprIR::StoreFieldAccessExpr { store_alias, .. } = left.as_ref() {
                        return store_signals.contains_key(store_alias.as_str());
                    }
                }
                expr_ir_contains_store_mutation(left, store_signals)
                    || expr_ir_contains_store_mutation(right, store_signals)
            }
            ExprIR::CallExpr { args, .. } => {
                args.iter().any(|a| expr_ir_contains_store_mutation(a, store_signals))
            }
            ExprIR::FieldAccessExpr { object, .. }
            | ExprIR::OptionalChainExpr { object, .. }
            | ExprIR::UnaryOpExpr { operand: object, .. }
            | ExprIR::AwaitExpr { operand: object, .. } => {
                expr_ir_contains_store_mutation(object, store_signals)
            }
            ExprIR::ArrayExpr { elements } => {
                elements.iter().any(|e| expr_ir_contains_store_mutation(e, store_signals))
            }
            ExprIR::StructExpr { fields, .. } => {
                fields.iter().any(|(_, v)| expr_ir_contains_store_mutation(v, store_signals))
            }
            ExprIR::TernaryIR { cond, then_expr, else_expr } => {
                expr_ir_contains_store_mutation(cond, store_signals)
                    || expr_ir_contains_store_mutation(then_expr, store_signals)
                    || expr_ir_contains_store_mutation(else_expr, store_signals)
            }
            ExprIR::IndexExpr { object, index } => {
                expr_ir_contains_store_mutation(object, store_signals)
                    || expr_ir_contains_store_mutation(index, store_signals)
            }
            ExprIR::ClosureExpr { body, .. } => {
                expr_ir_contains_store_mutation(body, store_signals)
            }
            ExprIR::BlockExpr { statements, tail } => {
                statements.iter().any(|s| {
                    if let StatementIR::MutationStmt { intent } = s {
                        if let Some(ref m) = intent {
                            if let Some(ref owner) = m.target.owner {
                                return store_signals.contains_key(owner.as_str());
                            }
                        }
                    }
                    false
                }) || expr_ir_contains_store_mutation(tail, store_signals)
            }
            ExprIR::MatchExpr { matched, arms } => {
                expr_ir_contains_store_mutation(matched, store_signals)
                    || arms.iter().any(|arm| {
                        arm.body.statements.iter().any(|s| {
                            if let StatementIR::MutationStmt { intent } = s {
                                if let Some(ref m) = intent {
                                    if let Some(ref owner) = m.target.owner {
                                        return store_signals.contains_key(owner.as_str());
                                    }
                                }
                            }
                            false
                        })
                    })
            }
            ExprIR::VariantLiteral { fields, .. } => {
                fields.iter().any(|(_, v)| expr_ir_contains_store_mutation(v, store_signals))
            }
            _ => false,
        }
    }

    /// Walk a ViewTree recursively and check event handler bodies for store mutations.
    fn check_view_tree_events_for_store_mutations(
        tree: &ViewTree,
        store_signals: &HashMap<&str, HashSet<&str>>,
        diags: &mut Vec<LintDiagnostic>,
    ) {
        match tree {
            ViewTree::Element(el) => {
                for (_event_name, handler) in &el.events {
                    if let Some(ref body) = handler.body {
                        check_block_ir_for_store_mutations(
                            body,
                            store_signals,
                            "inline event handler",
                            diags,
                        );
                    }
                }
                for child in &el.children {
                    check_view_tree_events_for_store_mutations(child, store_signals, diags);
                }
            }
            ViewTree::Conditional(cond) => {
                for branch in &cond.branches {
                    if let Some(ref body) = branch.body {
                        check_view_tree_events_for_store_mutations(body, store_signals, diags);
                    }
                }
                if let Some(ref fallback) = cond.fallback {
                    check_view_tree_events_for_store_mutations(fallback, store_signals, diags);
                }
            }
            ViewTree::ListRendering(list) => {
                if let Some(ref body) = list.body {
                    check_view_tree_events_for_store_mutations(body, store_signals, diags);
                }
                if let Some(ref empty) = list.empty_body {
                    check_view_tree_events_for_store_mutations(empty, store_signals, diags);
                }
            }
            _ => {}
        }
    }

    /// Check a Block for assignments to component props.
    fn check_block_for_prop_assign(
        block: &Block,
        prop_names: &HashSet<&str>,
        comp_name: &str,
        context_name: &str,
        diags: &mut Vec<LintDiagnostic>,
    ) {
        for stmt in &block.stmts {
            match stmt {
                Stmt::Assign {
                    target: AssignTarget::Name(name),
                    ..
                } => {
                    if prop_names.contains(name.as_str()) {
                        diags.push(LintDiagnostic {
                            rule: "prop-immutability".into(),
                            severity: LintSeverity::Error,
                            message: format!(
                                "Cannot assign to prop '{}' in '{}' of component '{}' — \
                                 component props are read-only",
                                name, context_name, comp_name
                            ),
                            file: String::new(),
                            line: 0,
                            col: 0,
                        });
                    }
                }
                Stmt::If {
                    then_body,
                    else_ifs,
                    else_body,
                    ..
                } => {
                    check_block_for_prop_assign(
                        then_body, prop_names, comp_name, context_name, diags,
                    );
                    for (_, body) in else_ifs {
                        check_block_for_prop_assign(
                            body, prop_names, comp_name, context_name, diags,
                        );
                    }
                    if let Some(ref body) = else_body {
                        check_block_for_prop_assign(
                            body, prop_names, comp_name, context_name, diags,
                        );
                    }
                }
                Stmt::While { body, .. } | Stmt::ForIn { body, .. } => {
                    check_block_for_prop_assign(
                        body, prop_names, comp_name, context_name, diags,
                    );
                }
                Stmt::TryCatch {
                    try_body,
                    catch_body,
                    finally_body,
                    ..
                } => {
                    check_block_for_prop_assign(
                        try_body, prop_names, comp_name, context_name, diags,
                    );
                    if let Some(ref body) = catch_body {
                        check_block_for_prop_assign(
                            body, prop_names, comp_name, context_name, diags,
                        );
                    }
                    if let Some(ref body) = finally_body {
                        check_block_for_prop_assign(
                            body, prop_names, comp_name, context_name, diags,
                        );
                    }
                }
                _ => {}
            }
        }
        // Also check trailing expression for prop assignments
        if let Some(ref expr) = block.trailing_expr {
            if expr_contains_prop_assign(expr, prop_names) {
                diags.push(LintDiagnostic {
                    rule: "prop-immutability".into(),
                    severity: LintSeverity::Error,
                    message: format!(
                        "Cannot assign to prop in '{}' of component '{}' — \
                         component props are read-only",
                        context_name, comp_name
                    ),
                    file: String::new(),
                    line: 0,
                    col: 0,
                });
            }
        }
    }

    /// Check if an expression assigns to a prop.
    fn expr_contains_prop_assign(expr: &Expr, prop_names: &HashSet<&str>) -> bool {
        match expr {
            Expr::BinaryOp { left, op, right, .. } => {
                if matches!(
                    op,
                    BinaryOpKind::Assign
                        | BinaryOpKind::AddAssign
                        | BinaryOpKind::SubAssign
                        | BinaryOpKind::MulAssign
                        | BinaryOpKind::DivAssign
                        | BinaryOpKind::ModAssign
                ) {
                    if let Expr::VarRef(name, _) = left.as_ref() {
                        return prop_names.contains(name.as_str());
                    }
                }
                expr_contains_prop_assign(left, prop_names)
                    || expr_contains_prop_assign(right, prop_names)
            }
            Expr::Block { body, .. } => {
                body.stmts.iter().any(|s| {
                    if let Stmt::Assign {
                        target: AssignTarget::Name(name),
                        ..
                    } = s
                    {
                        return prop_names.contains(name.as_str());
                    }
                    false
                })
            }
            _ => false,
        }
    }

    /// Walk view nodes to find prop assignments in inline event handlers.
    fn check_view_for_prop_assign(
        nodes: &[ViewNode],
        prop_names: &HashSet<&str>,
        comp_name: &str,
        diags: &mut Vec<LintDiagnostic>,
    ) {
        for node in nodes {
            match node {
                ViewNode::Element(el) => {
                    for prop in &el.props {
                        if let ElementProp::Event { handler, .. } = prop {
                            if let EventHandler::InlineFn { body, .. } = handler {
                                check_block_for_prop_assign(
                                    body,
                                    prop_names,
                                    comp_name,
                                    "inline event handler",
                                    diags,
                                );
                            }
                        }
                    }
                    check_view_for_prop_assign(&el.children, prop_names, comp_name, diags);
                }
                ViewNode::IfDirective(if_dir) => {
                    check_view_for_prop_assign(
                        &if_dir.body, prop_names, comp_name, diags,
                    );
                    for (_, body) in &if_dir.else_ifs {
                        check_view_for_prop_assign(body, prop_names, comp_name, diags);
                    }
                    if let Some(ref else_body) = if_dir.else_body {
                        check_view_for_prop_assign(else_body, prop_names, comp_name, diags);
                    }
                }
                ViewNode::ForDirective(for_dir) => {
                    check_view_for_prop_assign(
                        &for_dir.body, prop_names, comp_name, diags,
                    );
                    if let Some(ref empty_body) = for_dir.empty_body {
                        check_view_for_prop_assign(empty_body, prop_names, comp_name, diags);
                    }
                }
                _ => {}
            }
        }
    }

    /// Check if an AST expression contains side effects (mutations).
    /// Used by ComputedPurity to ensure computed values are pure.
    fn expr_contains_side_effect(expr: &Expr, signal_names: &HashSet<&str>) -> bool {
        match expr {
            // Assignment operators on signals
            Expr::BinaryOp { left, op, right, .. } => {
                if matches!(
                    op,
                    BinaryOpKind::Assign
                        | BinaryOpKind::AddAssign
                        | BinaryOpKind::SubAssign
                        | BinaryOpKind::MulAssign
                        | BinaryOpKind::DivAssign
                        | BinaryOpKind::ModAssign
                ) {
                    if let Expr::VarRef(name, _) = left.as_ref() {
                        if signal_names.contains(name.as_str()) {
                            return true;
                        }
                    }
                }
                expr_contains_side_effect(left, signal_names)
                    || expr_contains_side_effect(right, signal_names)
            }
            // Mutation method calls: signal.set(), signal.push(), signal.pop(), etc.
            Expr::MethodCall { object, method, .. } => {
                if matches!(
                    method.as_str(),
                    "set" | "setValue" | "push" | "pop" | "inc" | "increment" | "dec" | "decrement"
                ) {
                    if let Expr::VarRef(name, _) = object.as_ref() {
                        if signal_names.contains(name.as_str()) {
                            return true;
                        }
                    }
                }
                expr_contains_side_effect(object, signal_names)
            }
            // Function call with FieldAccess: signal.set(value)
            Expr::FuncCall { func, args, .. } => {
                if let Expr::FieldAccess { object, field, .. } = func.as_ref() {
                    if matches!(
                        field.as_str(),
                        "set" | "setValue" | "push" | "pop" | "inc" | "increment" | "dec"
                            | "decrement"
                    ) {
                        if let Expr::VarRef(name, _) = object.as_ref() {
                            if signal_names.contains(name.as_str()) {
                                return true;
                            }
                        }
                    }
                }
                expr_contains_side_effect(func, signal_names)
                    || args.iter().any(|a| expr_contains_side_effect(a, signal_names))
            }
            // Block expressions: check all statements for assignments
            Expr::Block { body, .. } => {
                for stmt in &body.stmts {
                    match stmt {
                        Stmt::Assign {
                            target: AssignTarget::Name(name),
                            ..
                        } => {
                            if signal_names.contains(name.as_str()) {
                                return true;
                            }
                        }
                        Stmt::ExprStmt { expr: e, .. } => {
                            if expr_contains_side_effect(e, signal_names) {
                                return true;
                            }
                        }
                        Stmt::Return { value: Some(e), .. }
                        | Stmt::Print { arg: e, .. } => {
                            if expr_contains_side_effect(e, signal_names) {
                                return true;
                            }
                        }
                        _ => {}
                    }
                }
                if let Some(ref trailing) = body.trailing_expr {
                    expr_contains_side_effect(trailing, signal_names)
                } else {
                    false
                }
            }
            // Recursively check sub-expressions
            Expr::UnaryOp { operand, .. } => expr_contains_side_effect(operand, signal_names),
            Expr::FieldAccess { object, .. }
            | Expr::OptionalChain { object, .. }
            | Expr::IndexGet { object, .. } => expr_contains_side_effect(object, signal_names),
            Expr::ArrayLiteral { elements, .. } => {
                elements.iter().any(|e| expr_contains_side_effect(e, signal_names))
            }
            Expr::MapLiteral { entries, .. } => entries
                .iter()
                .any(|(k, v)| expr_contains_side_effect(k, signal_names) || expr_contains_side_effect(v, signal_names)),
            Expr::IfExpr {
                condition,
                then_expr,
                else_expr,
                ..
            }
            | Expr::TernaryExpr {
                condition,
                then_expr,
                else_expr,
                ..
            } => {
                expr_contains_side_effect(condition, signal_names)
                    || expr_contains_side_effect(then_expr, signal_names)
                    || expr_contains_side_effect(else_expr, signal_names)
            }
            Expr::StructLiteral { fields, .. } => fields
                .iter()
                .any(|(_, v)| expr_contains_side_effect(v, signal_names)),
            Expr::TemplateString { parts, .. } => parts.iter().any(|p| {
                if let TemplatePart::Expr(e) = p {
                    expr_contains_side_effect(e, signal_names)
                } else {
                    false
                }
            }),
            Expr::ClosureExpr { body, .. } => expr_contains_side_effect(body, signal_names),
            Expr::AwaitExpr { operand, .. } => expr_contains_side_effect(operand, signal_names),
            Expr::MatchExpr { matched, arms, .. } => {
                expr_contains_side_effect(matched, signal_names)
                    || arms.iter().any(|arm| {
                        arm.body.stmts.iter().any(|s| {
                            if let Stmt::Assign {
                                target: AssignTarget::Name(name),
                                ..
                            } = s
                            {
                                return signal_names.contains(name.as_str());
                            }
                            false
                        }) || arm.body.trailing_expr.as_ref().map_or(false, |e| {
                            expr_contains_side_effect(e, signal_names)
                        })
                    })
            }
            Expr::NotYetImplemented { .. } => false,
            // StoreFieldAccess is read-only access, not a side effect
            Expr::StoreFieldAccess { .. } => false,
            // Literal and VarRef are pure
            Expr::Literal(..) | Expr::VarRef(..) => false,
        }
    }
}

// ═══════════════════════════════════════════════════════════
// Tests
// ═══════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::ModuleIR;
    use crate::lexer::Span;
    use crate::parser::ast::{Block, Expr, FunctionDef, Module, NamedImport, Stmt, Visibility};

    /// Helper to construct an IRProgram with a single module for testing.
    fn make_test_ir(
        components: HashMap<String, crate::ir::ComponentIR>,
        stores: HashMap<String, crate::ir::StoreIR>,
    ) -> IRProgram {
        let mut modules = HashMap::new();
        modules.insert(
            "main".to_string(),
            ModuleIR {
                name: "main".to_string(),
                components,
                stores,
                ..Default::default()
            },
        );
        IRProgram {
            version: "1.0".into(),
            modules,
            source_map: HashMap::new(),
            stdlib_imports: Vec::new(),
        }
    }

    // ── UndefinedRef tests ───────────────────────────────

    #[test]
    fn test_undefined_ref_detects_missing_store() {
        let mut components = HashMap::new();
        components.insert(
            "App".into(),
            crate::ir::ComponentIR {
                name: "App".into(),
                generic_params: vec![],
                props: HashMap::new(),
                prop_defaults: HashMap::new(),
                states: vec![],
                computeds: vec![],
                effects: vec![],
                lifecycles: vec![],
                fns: vec![],
                store_refs: vec!["MissingStore".into()],
                refs: vec![],
                event_names: vec![],
                view: None,
                styles: vec![],
                motions: vec![],
                if_mode_default: None,
                hero_defs: vec![],
                timelines: vec![],
            },
        );
        let ir = make_test_ir(components, HashMap::new());

        let diags = rules::UndefinedRef.check_ir(&ir, &HashMap::new());
        assert_eq!(diags.len(), 1, "expected 1 diagnostic for missing store");
        assert!(
            diags[0].message.contains("MissingStore"),
            "message should mention the missing store: {}",
            diags[0].message
        );
    }

    #[test]
    fn test_undefined_ref_skips_existing_store() {
        let mut stores = HashMap::new();
        stores.insert(
            "CounterStore".into(),
            crate::ir::StoreIR {
                name: "CounterStore".into(),
                signals: HashMap::new(),
                signal_defaults: HashMap::new(),
                getters: HashMap::new(),
                actions: vec![],
            },
        );
        let mut components = HashMap::new();
        components.insert(
            "App".into(),
            crate::ir::ComponentIR {
                name: "App".into(),
                generic_params: vec![],
                props: HashMap::new(),
                prop_defaults: HashMap::new(),
                states: vec![],
                computeds: vec![],
                effects: vec![],
                lifecycles: vec![],
                fns: vec![],
                store_refs: vec!["CounterStore".into()],
                refs: vec![],
                event_names: vec![],
                view: None,
                styles: vec![],
                motions: vec![],
                if_mode_default: None,
                hero_defs: vec![],
                timelines: vec![],
            },
        );
        let ir = make_test_ir(components, stores);

        let diags = rules::UndefinedRef.check_ir(&ir, &HashMap::new());
        assert_eq!(diags.len(), 0, "no diagnostics expected when store exists");
    }

    // ── MissingReturn tests ──────────────────────────────

    #[test]
    fn test_missing_return_detects_missing_return() {
        let mut components = HashMap::new();
        components.insert(
            "App".into(),
            crate::ir::ComponentIR {
                name: "App".into(),
                generic_params: vec![],
                props: HashMap::new(),
                prop_defaults: HashMap::new(),
                states: vec![],
                computeds: vec![],
                effects: vec![],
                lifecycles: vec![],
                fns: vec![crate::ir::FunctionIR {
                    name: "getCount".into(),
                    visibility: "private".to_string(),
                    params: vec![],
                    return_type: Some(crate::ir::TypeIR {
                        kind: "Int".into(),
                        type_args: vec![],
                        is_nullable: false,
                        needs_clone: false,
                        needs_partial_eq: false,
                    }),
                    body: crate::ir::BlockIR {
                        statements: vec![crate::ir::StatementIR::LetStmtIR {
                            name: "x".into(),
                            value: None,
                            r#type: None,
                        }],
                        execution_mode: "client-only".into(),
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
            },
        );
        let ir = make_test_ir(components, HashMap::new());

        let diags = rules::MissingReturn.check_ir(&ir, &HashMap::new());
        assert_eq!(diags.len(), 1, "expected 1 diagnostic for missing return");
        assert!(
            diags[0].message.contains("getCount"),
            "message should mention the function: {}",
            diags[0].message
        );
    }

    #[test]
    fn test_missing_return_skips_void_functions() {
        let mut components = HashMap::new();
        components.insert(
            "App".into(),
            crate::ir::ComponentIR {
                name: "App".into(),
                generic_params: vec![],
                props: HashMap::new(),
                prop_defaults: HashMap::new(),
                states: vec![],
                computeds: vec![],
                effects: vec![],
                lifecycles: vec![],
                fns: vec![crate::ir::FunctionIR {
                    name: "log".into(),
                    visibility: "private".to_string(),
                    params: vec![],
                    return_type: Some(crate::ir::TypeIR {
                        kind: "void".into(),
                        type_args: vec![],
                        is_nullable: false,
                        needs_clone: false,
                        needs_partial_eq: false,
                    }),
                    body: crate::ir::BlockIR {
                        statements: vec![],
                        execution_mode: "client-only".into(),
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
            },
        );
        let ir = make_test_ir(components, HashMap::new());

        let diags = rules::MissingReturn.check_ir(&ir, &HashMap::new());
        assert_eq!(
            diags.len(),
            0,
            "void functions should not trigger missing-return"
        );
    }

    #[test]
    fn test_missing_return_skips_when_return_present() {
        let mut components = HashMap::new();
        components.insert(
            "App".into(),
            crate::ir::ComponentIR {
                name: "App".into(),
                generic_params: vec![],
                props: HashMap::new(),
                prop_defaults: HashMap::new(),
                states: vec![],
                computeds: vec![],
                effects: vec![],
                lifecycles: vec![],
                fns: vec![crate::ir::FunctionIR {
                    name: "getCount".into(),
                    visibility: "private".to_string(),
                    params: vec![],
                    return_type: Some(crate::ir::TypeIR {
                        kind: "Int".into(),
                        type_args: vec![],
                        is_nullable: false,
                        needs_clone: false,
                        needs_partial_eq: false,
                    }),
                    body: crate::ir::BlockIR {
                        statements: vec![crate::ir::StatementIR::ReturnStmtIR {
                            value: Some(crate::ir::ExprIR::LiteralExpr {
                                value: serde_json::Value::Number(42.into()),
                            }),
                        }],
                        execution_mode: "client-only".into(),
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
            },
        );
        let ir = make_test_ir(components, HashMap::new());

        let diags = rules::MissingReturn.check_ir(&ir, &HashMap::new());
        assert_eq!(diags.len(), 0, "functions with return should not trigger");
    }

    // ── DeadCode tests ───────────────────────────────────

    #[test]
    fn test_dead_code_detects_unused_store() {
        let mut stores = HashMap::new();
        stores.insert(
            "UnusedStore".into(),
            crate::ir::StoreIR {
                name: "UnusedStore".into(),
                signals: HashMap::new(),
                signal_defaults: HashMap::new(),
                getters: HashMap::new(),
                actions: vec![],
            },
        );
        let ir = make_test_ir(HashMap::new(), stores);

        let diags = rules::DeadCode.check_ir(&ir, &HashMap::new());
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
            crate::ir::StoreIR {
                name: "UsedStore".into(),
                signals: HashMap::new(),
                signal_defaults: HashMap::new(),
                getters: HashMap::new(),
                actions: vec![],
            },
        );
        let mut components = HashMap::new();
        components.insert(
            "App".into(),
            crate::ir::ComponentIR {
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
            },
        );
        let ir = make_test_ir(components, stores);

        let diags = rules::DeadCode.check_ir(&ir, &HashMap::new());
        assert_eq!(diags.len(), 0, "used stores should not trigger dead-code");
    }

    // ── UnusedImport tests ───────────────────────────────

    #[test]
    fn test_unused_import_detects_unused_named_import() {
        let mut modules = HashMap::new();
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

        let diags = rules::UnusedImport.check_ast(&program, "test.omni");
        assert_eq!(diags.len(), 1, "expected 1 diagnostic for unused import");
        assert!(
            diags[0].message.contains("unusedSymbol"),
            "message should mention the unused symbol: {}",
            diags[0].message
        );
    }

    #[test]
    fn test_unused_import_skips_used_import() {
        let mut modules = HashMap::new();
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

        let diags = rules::UnusedImport.check_ast(&program, "test.omni");
        assert_eq!(
            diags.len(),
            0,
            "used imports should not trigger unused-import"
        );
    }

    // ── run_all integration test ─────────────────────────

    #[test]
    fn test_run_all_empty_programs() {
        let programs: HashMap<String, Program> = HashMap::new();
        let ir = make_test_ir(HashMap::new(), HashMap::new());

        let diags = run_all(&programs, &ir);
        assert_eq!(diags.len(), 0, "no diagnostics expected for empty input");
    }

    #[test]
    fn test_type_mismatch_void_return_with_value() {
        let mut components = HashMap::new();
        components.insert(
            "Test".into(),
            crate::ir::ComponentIR {
                name: "Test".into(),
                generic_params: vec![],
                props: HashMap::new(),
                prop_defaults: HashMap::new(),
                states: vec![],
                computeds: vec![],
                effects: vec![],
                lifecycles: vec![],
                fns: vec![crate::ir::FunctionIR {
                    name: "returns_void".into(),
                    visibility: "private".into(),
                    params: vec![],
                    return_type: Some(crate::ir::TypeIR {
                        kind: "void".into(),
                        type_args: vec![],
                        is_nullable: false,
                        needs_clone: false,
                        needs_partial_eq: false,
                    }),
                    body: crate::ir::BlockIR {
                        statements: vec![crate::ir::StatementIR::ReturnStmtIR {
                            value: Some(crate::ir::ExprIR::LiteralExpr {
                                value: serde_json::Value::Number(42.into()),
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
            },
        );
        let ir = make_test_ir(components, HashMap::new());

        let rule = rules::TypeMismatch;
        let diags = rule.check_ir(&ir, &HashMap::new());
        assert!(
            !diags.is_empty(),
            "Should detect void function returning a value"
        );
        assert!(
            diags[0].message.contains("void"),
            "Message should mention void: {}",
            diags[0].message
        );
    }

    // ── StoreFieldDirectMutation tests ───────────────────

    #[test]
    fn test_store_mutation_detects_direct_assign() {
        let mut stores = HashMap::new();
        stores.insert(
            "MyStore".into(),
            crate::ir::StoreIR {
                name: "MyStore".into(),
                signals: [("count".into(), crate::ir::TypeIR {
                    kind: "i32".into(), type_args: vec![],
                    is_nullable: false, needs_clone: false, needs_partial_eq: false,
                })].into_iter().collect(),
                signal_defaults: HashMap::new(),
                getters: HashMap::new(),
                actions: vec![],
            },
        );
        let mut components = HashMap::new();
        components.insert(
            "App".into(),
            crate::ir::ComponentIR {
                name: "App".into(),
                generic_params: vec![],
                props: HashMap::new(),
                prop_defaults: HashMap::new(),
                states: vec![],
                computeds: vec![],
                effects: vec![],
                lifecycles: vec![],
                fns: vec![crate::ir::FunctionIR {
                    name: "hack".into(),
                    visibility: "private".into(),
                    params: vec![],
                    return_type: None,
                    body: crate::ir::BlockIR {
                        statements: vec![crate::ir::StatementIR::MutationStmt {
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
            },
        );
        let ir = make_test_ir(components, stores);

        let diags = rules::StoreFieldDirectMutation.check_ir(&ir, &HashMap::new());
        assert!(!diags.is_empty(), "Should detect store field direct mutation");
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
            crate::ir::StoreIR {
                name: "MyStore".into(),
                signals: HashMap::new(),
                signal_defaults: HashMap::new(),
                getters: HashMap::new(),
                actions: vec![],
            },
        );
        let mut components = HashMap::new();
        components.insert(
            "App".into(),
            crate::ir::ComponentIR {
                name: "App".into(),
                generic_params: vec![],
                props: HashMap::new(),
                prop_defaults: HashMap::new(),
                states: vec![],
                computeds: vec![],
                effects: vec![],
                lifecycles: vec![],
                fns: vec![crate::ir::FunctionIR {
                    name: "increment".into(),
                    visibility: "private".into(),
                    params: vec![],
                    return_type: None,
                    body: crate::ir::BlockIR {
                        // Local signal mutation (no owner) — should NOT be flagged
                        statements: vec![crate::ir::StatementIR::MutationStmt {
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
            },
        );
        let ir = make_test_ir(components, stores);

        let diags = rules::StoreFieldDirectMutation.check_ir(&ir, &HashMap::new());
        assert!(diags.is_empty(), "Local signal mutations should not be flagged");
    }

    // ── PropImmutability tests ───────────────────────────

    #[test]
    fn test_prop_immutability_detects_reassignment() {
        let span = crate::lexer::Span { start: 0, end: 10, line: 1, col: 1 };
        let mut modules = HashMap::new();
        modules.insert(
            "test.omni".into(),
            crate::parser::ast::Module {
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
                components: vec![crate::parser::ast::ComponentDef {
                    name: "Card".into(),
                    props: vec![crate::parser::ast::PropDecl {
                        name: "title".into(),
                        type_ann: crate::parser::ast::TypeAnnotation::Named {
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
                    if_mode_default: None,
                    hero_defs: vec![],
                    lifecycles: vec![],
                    fns: vec![crate::parser::ast::FunctionDef {
                        visibility: crate::parser::ast::Visibility::Private,
                        name: "badFn".into(),
                        params: vec![],
                        return_type: None,
                        is_async: false,
                        body: crate::parser::ast::Block {
                            stmts: vec![crate::parser::ast::Stmt::Assign {
                                target: crate::parser::ast::AssignTarget::Name("title".into()),
                                op: crate::parser::ast::BinaryOpKind::Assign,
                                value: crate::parser::ast::Expr::Literal(
                                    crate::parser::ast::Literal::String("hacked".into()),
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
                    motions: vec![],
                    timelines: vec![],
                    view: None,
                    styles: vec![],
                    span: span.clone(),
                }],
            },
        );
        let program = crate::parser::ast::Program { modules };

        let diags = rules::PropImmutability.check_ast(&program, "test.omni");
        assert!(!diags.is_empty(), "Should detect prop reassignment");
        assert!(
            diags[0].message.contains("title"),
            "Message should mention prop name: {}",
            diags[0].message
        );
    }

    // ── ComputedPurity tests ─────────────────────────────

    #[test]
    fn test_computed_purity_detects_side_effect() {
        let span = crate::lexer::Span { start: 0, end: 10, line: 1, col: 1 };
        let mut modules = HashMap::new();
        modules.insert(
            "test.omni".into(),
            crate::parser::ast::Module {
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
                components: vec![crate::parser::ast::ComponentDef {
                    name: "Counter".into(),
                    props: vec![],
                    signals: vec![crate::parser::ast::SignalDecl {
                        name: "count".into(),
                        type_ann: crate::parser::ast::TypeAnnotation::Named {
                            name: "i32".into(),
                            type_args: vec![],
                            span: span.clone(),
                        },
                        init: crate::parser::ast::Expr::Literal(
                            crate::parser::ast::Literal::Int(0),
                            span.clone(),
                        ),
                    }],
                    computeds: vec![crate::parser::ast::ComputedDecl {
                        name: "badComputed".into(),
                        type_ann: crate::parser::ast::TypeAnnotation::Named {
                            name: "i32".into(),
                            type_args: vec![],
                            span: span.clone(),
                        },
                        // Computed that assigns to count — should be flagged
                        body: crate::parser::ast::Expr::Block {
                            body: crate::parser::ast::Block {
                                stmts: vec![crate::parser::ast::Stmt::Assign {
                                    target: crate::parser::ast::AssignTarget::Name("count".into()),
                                    op: crate::parser::ast::BinaryOpKind::Assign,
                                    value: crate::parser::ast::Expr::Literal(
                                        crate::parser::ast::Literal::Int(5),
                                        span.clone(),
                                    ),
                                    span: span.clone(),
                                }],
                                trailing_expr: Some(Box::new(crate::parser::ast::Expr::VarRef(
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
                    hero_defs: vec![],
                    lifecycles: vec![],
                    fns: vec![],
                    actions: vec![],
                    getters: vec![],
                    motions: vec![],
                    timelines: vec![],
                    view: None,
                    styles: vec![],
                    span: span.clone(),
                }],
            },
        );
        let program = crate::parser::ast::Program { modules };

        let diags = rules::ComputedPurity.check_ast(&program, "test.omni");
        assert!(!diags.is_empty(), "Should detect side effect in computed");
        assert!(
            diags[0].message.contains("badComputed"),
            "Message should mention computed name: {}",
            diags[0].message
        );
    }

    #[test]
    fn test_computed_purity_allows_pure_computed() {
        let span = crate::lexer::Span { start: 0, end: 10, line: 1, col: 1 };
        let mut modules = HashMap::new();
        modules.insert(
            "test.omni".into(),
            crate::parser::ast::Module {
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
                components: vec![crate::parser::ast::ComponentDef {
                    name: "Counter".into(),
                    props: vec![],
                    signals: vec![crate::parser::ast::SignalDecl {
                        name: "count".into(),
                        type_ann: crate::parser::ast::TypeAnnotation::Named {
                            name: "i32".into(),
                            type_args: vec![],
                            span: span.clone(),
                        },
                        init: crate::parser::ast::Expr::Literal(
                            crate::parser::ast::Literal::Int(0),
                            span.clone(),
                        ),
                    }],
                    computeds: vec![crate::parser::ast::ComputedDecl {
                        name: "doubled".into(),
                        type_ann: crate::parser::ast::TypeAnnotation::Named {
                            name: "i32".into(),
                            type_args: vec![],
                            span: span.clone(),
                        },
                        // Pure computed: just reads count
                        body: crate::parser::ast::Expr::BinaryOp {
                            left: Box::new(crate::parser::ast::Expr::VarRef(
                                "count".into(), span.clone(),
                            )),
                            op: crate::parser::ast::BinaryOpKind::Mul,
                            right: Box::new(crate::parser::ast::Expr::Literal(
                                crate::parser::ast::Literal::Int(2),
                                span.clone(),
                            )),
                            span: span.clone(),
                        },
                    }],
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
                    view: None,
                    styles: vec![],
                    span: span.clone(),
                }],
            },
        );
        let program = crate::parser::ast::Program { modules };

        let diags = rules::ComputedPurity.check_ast(&program, "test.omni");
        assert!(diags.is_empty(), "Pure computed should not be flagged");
    }

    // ── DeadCode enhanced tests ──────────────────────────

    #[test]
    fn test_dead_code_detects_unused_signal() {
        let span = crate::lexer::Span { start: 0, end: 10, line: 1, col: 1 };
        let mut modules = HashMap::new();
        modules.insert(
            "test.omni".into(),
            crate::parser::ast::Module {
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
                components: vec![crate::parser::ast::ComponentDef {
                    name: "Comp".into(),
                    props: vec![],
                    signals: vec![crate::parser::ast::SignalDecl {
                        name: "unusedSignal".into(),
                        type_ann: crate::parser::ast::TypeAnnotation::Named {
                            name: "i32".into(),
                            type_args: vec![],
                            span: span.clone(),
                        },
                        init: crate::parser::ast::Expr::Literal(
                            crate::parser::ast::Literal::Int(0),
                            span.clone(),
                        ),
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
                    view: Some(crate::parser::ast::ViewBlock {
                        children: vec![crate::parser::ast::ViewNode::Text("hello".into())],
                    }),
                    styles: vec![],
                    span: span.clone(),
                }],
            },
        );
        let program = crate::parser::ast::Program { modules };

        let diags = rules::DeadCode.check_ast(&program, "test.omni");
        let signal_diags: Vec<_> = diags.iter().filter(|d| d.message.contains("Signal")).collect();
        assert!(!signal_diags.is_empty(), "Should detect unused signal");
        assert!(
            signal_diags[0].message.contains("unusedSignal"),
            "Message should mention unused signal: {}",
            signal_diags[0].message
        );
    }

    #[test]
    fn test_dead_code_detects_unused_function() {
        let span = crate::lexer::Span { start: 0, end: 10, line: 1, col: 1 };
        let mut modules = HashMap::new();
        modules.insert(
            "test.omni".into(),
            crate::parser::ast::Module {
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
                components: vec![crate::parser::ast::ComponentDef {
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
                    fns: vec![crate::parser::ast::FunctionDef {
                        visibility: crate::parser::ast::Visibility::Private,
                        name: "unusedFn".into(),
                        params: vec![],
                        return_type: None,
                        is_async: false,
                        body: crate::parser::ast::Block {
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
                    view: Some(crate::parser::ast::ViewBlock {
                        children: vec![crate::parser::ast::ViewNode::Text("hello".into())],
                    }),
                    styles: vec![],
                    span: span.clone(),
                }],
            },
        );
        let program = crate::parser::ast::Program { modules };

        let diags = rules::DeadCode.check_ast(&program, "test.omni");
        let fn_diags: Vec<_> = diags.iter().filter(|d| d.message.contains("Function")).collect();
        assert!(!fn_diags.is_empty(), "Should detect unused function");
        assert!(
            fn_diags[0].message.contains("unusedFn"),
            "Message should mention unused function: {}",
            fn_diags[0].message
        );
    }

    // ── InvalidEventField tests ──────────────────────────

    #[test]
    fn test_invalid_event_field_detects_invalid_field() {
        let mut components = HashMap::new();
        components.insert(
            "Form".into(),
            crate::ir::ComponentIR {
                name: "Form".into(),
                generic_params: vec![],
                props: HashMap::new(),
                prop_defaults: HashMap::new(),
                states: vec![],
                computeds: vec![],
                effects: vec![],
                lifecycles: vec![],
                fns: vec![],
                store_refs: vec![],
                refs: vec![],
                event_names: vec![],
                view: Some(crate::ir::ViewTree::Element(crate::ir::ElementIR {
                    tag: "input".into(),
                    props: HashMap::new(),
                    bindings: HashMap::new(),
                    events: [(
                        "onChange".into(),
                        crate::ir::EventHandlerIR {
                            kind: "inline_fn".into(),
                            body: None,
                            r#ref: None,
                            params: vec![],
                            event_data: Some(crate::ir::EventDataKind::InputEvent),
                            used_fields: Some(vec!["nonexistent_field".into()]),
                            needs_prevent_default: false,
                            needs_stop_propagation: false,
                        },
                    )]
                    .into_iter()
                    .collect(),
                    style_classes: None,
                    inline_style: None,
                    children: vec![],
                    test_id: None,
                    ref_name: None,
                    construction_kind: "normal".into(),
                    class_map_bindings: HashMap::new(),
                    source_line: 10,
                    source_col: 5,
                    motion_triggers: vec![],
                    states: None,
                    paused_override: None,
                    driven_by: None,
                    hover_reverse: None,
                    in_view_once: None,
                    in_view_repeat: None,
                    in_view_threshold: None,
                    scroll_start: None,
                    scroll_end: None,
                    scroll_scrub: None,
                    drag_constraints: None,
                    drag_elastic: None,
                    drag_scrub: None,
                    drag_momentum: None,
                    drag_threshold: None,
                    drag_velocity_max: None,
                    hero_tag: None,
                    hero_transition: None,
                })),
                styles: vec![],
                motions: vec![],
                if_mode_default: None,
                hero_defs: vec![],
                timelines: vec![],
            },
        );
        let ir = make_test_ir(components, HashMap::new());

        let diags = rules::InvalidEventField.check_ir(&ir, &HashMap::new());
        assert!(
            !diags.is_empty(),
            "Should detect invalid event field reference"
        );
        assert!(
            diags[0].message.contains("nonexistent_field"),
            "Message should mention the invalid field: {}",
            diags[0].message
        );
        assert!(
            diags[0].message.contains("InputEvent"),
            "Message should mention the event data kind: {}",
            diags[0].message
        );
        assert_eq!(diags[0].line, 10, "Line should match ElementIR.source_line");
        assert_eq!(diags[0].col, 5, "Col should match ElementIR.source_col");
    }

    #[test]
    fn test_invalid_event_field_allows_valid_field() {
        let mut components = HashMap::new();
        components.insert(
            "Form".into(),
            crate::ir::ComponentIR {
                name: "Form".into(),
                generic_params: vec![],
                props: HashMap::new(),
                prop_defaults: HashMap::new(),
                states: vec![],
                computeds: vec![],
                effects: vec![],
                lifecycles: vec![],
                fns: vec![],
                store_refs: vec![],
                refs: vec![],
                event_names: vec![],
                view: Some(crate::ir::ViewTree::Element(crate::ir::ElementIR {
                    tag: "input".into(),
                    props: HashMap::new(),
                    bindings: HashMap::new(),
                    events: [(
                        "onChange".into(),
                        crate::ir::EventHandlerIR {
                            kind: "inline_fn".into(),
                            body: None,
                            r#ref: None,
                            params: vec![],
                            event_data: Some(crate::ir::EventDataKind::InputEvent),
                            used_fields: Some(vec!["value".into()]),
                            needs_prevent_default: false,
                            needs_stop_propagation: false,
                        },
                    )]
                    .into_iter()
                    .collect(),
                    style_classes: None,
                    inline_style: None,
                    children: vec![],
                    test_id: None,
                    ref_name: None,
                    construction_kind: "normal".into(),
                    class_map_bindings: HashMap::new(),
                    source_line: 0,
                    source_col: 0,
                    motion_triggers: vec![],
                    states: None,
                    paused_override: None,
                    driven_by: None,
                    hover_reverse: None,
                    in_view_once: None,
                    in_view_repeat: None,
                    in_view_threshold: None,
                    scroll_start: None,
                    scroll_end: None,
                    scroll_scrub: None,
                    drag_constraints: None,
                    drag_elastic: None,
                    drag_scrub: None,
                    drag_momentum: None,
                    drag_threshold: None,
                    drag_velocity_max: None,
                    hero_tag: None,
                    hero_transition: None,
                })),
                styles: vec![],
                motions: vec![],
                if_mode_default: None,
                hero_defs: vec![],
                timelines: vec![],
            },
        );
        let ir = make_test_ir(components, HashMap::new());

        let diags = rules::InvalidEventField.check_ir(&ir, &HashMap::new());
        assert!(
            diags.is_empty(),
            "Valid event field `value` should not trigger diagnostic"
        );
    }

    #[test]
    fn test_invalid_event_field_succeeds_on_missing_event_data() {
        // Handler with no event_data (e.g., custom events) should not trigger
        let mut components = HashMap::new();
        components.insert(
            "Widget".into(),
            crate::ir::ComponentIR {
                name: "Widget".into(),
                generic_params: vec![],
                props: HashMap::new(),
                prop_defaults: HashMap::new(),
                states: vec![],
                computeds: vec![],
                effects: vec![],
                lifecycles: vec![],
                fns: vec![],
                store_refs: vec![],
                refs: vec![],
                event_names: vec![],
                view: Some(crate::ir::ViewTree::Element(crate::ir::ElementIR {
                    tag: "div".into(),
                    props: HashMap::new(),
                    bindings: HashMap::new(),
                    events: [(
                        "onCustomEvent".into(),
                        crate::ir::EventHandlerIR {
                            kind: "inline_fn".into(),
                            body: None,
                            r#ref: None,
                            params: vec![],
                            event_data: None,
                            used_fields: Some(vec!["whatever".into()]),
                            needs_prevent_default: false,
                            needs_stop_propagation: false,
                        },
                    )]
                    .into_iter()
                    .collect(),
                    style_classes: None,
                    inline_style: None,
                    children: vec![],
                    test_id: None,
                    ref_name: None,
                    construction_kind: "normal".into(),
                    class_map_bindings: HashMap::new(),
                    source_line: 0,
                    source_col: 0,
                    motion_triggers: vec![],
                    states: None,
                    paused_override: None,
                    driven_by: None,
                    hover_reverse: None,
                    in_view_once: None,
                    in_view_repeat: None,
                    in_view_threshold: None,
                    scroll_start: None,
                    scroll_end: None,
                    scroll_scrub: None,
                    drag_constraints: None,
                    drag_elastic: None,
                    drag_scrub: None,
                    drag_momentum: None,
                    drag_threshold: None,
                    drag_velocity_max: None,
                    hero_tag: None,
                    hero_transition: None,
                })),
                styles: vec![],
                motions: vec![],
                if_mode_default: None,
                hero_defs: vec![],
                timelines: vec![],
            },
        );
        let ir = make_test_ir(components, HashMap::new());

        let diags = rules::InvalidEventField.check_ir(&ir, &HashMap::new());
        assert!(
            diags.is_empty(),
            "Custom events without event_data should not trigger"
        );
    }

    // ── StoreActionEventData tests ──────────────────────

    #[test]
    fn test_store_action_detects_event_data_access() {
        let mut components = HashMap::new();
        components.insert(
            "App".into(),
            crate::ir::ComponentIR {
                name: "App".into(),
                generic_params: vec![],
                props: HashMap::new(),
                prop_defaults: HashMap::new(),
                states: vec![],
                computeds: vec![],
                effects: vec![],
                lifecycles: vec![],
                fns: vec![],
                store_refs: vec![],
                refs: vec![],
                event_names: vec![],
                view: Some(crate::ir::ViewTree::Element(crate::ir::ElementIR {
                    tag: "button".into(),
                    props: HashMap::new(),
                    bindings: HashMap::new(),
                    events: [(
                        "onClick".into(),
                        crate::ir::EventHandlerIR {
                            kind: "store_action".into(),
                            body: None,
                            r#ref: Some("MyStore.increment".into()),
                            params: vec![],
                            event_data: Some(crate::ir::EventDataKind::PointerEvent),
                            used_fields: Some(vec!["x".into()]),
                            needs_prevent_default: false,
                            needs_stop_propagation: false,
                        },
                    )]
                    .into_iter()
                    .collect(),
                    style_classes: None,
                    inline_style: None,
                    children: vec![],
                    test_id: None,
                    ref_name: None,
                    construction_kind: "normal".into(),
                    class_map_bindings: HashMap::new(),
                    source_line: 5,
                    source_col: 3,
                    motion_triggers: vec![],
                    states: None,
                    paused_override: None,
                    driven_by: None,
                    hover_reverse: None,
                    in_view_once: None,
                    in_view_repeat: None,
                    in_view_threshold: None,
                    scroll_start: None,
                    scroll_end: None,
                    scroll_scrub: None,
                    drag_constraints: None,
                    drag_elastic: None,
                    drag_scrub: None,
                    drag_momentum: None,
                    drag_threshold: None,
                    drag_velocity_max: None,
                    hero_tag: None,
                    hero_transition: None,
                })),
                styles: vec![],
                motions: vec![],
                if_mode_default: None,
                hero_defs: vec![],
                timelines: vec![],
            },
        );
        let ir = make_test_ir(components, HashMap::new());

        let diags = rules::StoreActionEventData.check_ir(&ir, &HashMap::new());
        assert!(
            !diags.is_empty(),
            "Store action with event data access should be detected"
        );
        assert!(
            diags[0].message.contains("MyStore.increment"),
            "Message should mention store action path: {}",
            diags[0].message
        );
        assert_eq!(diags[0].line, 5, "Line should match ElementIR.source_line");
        assert_eq!(diags[0].col, 3, "Col should match ElementIR.source_col");
    }

    #[test]
    fn test_store_action_allows_no_event_data() {
        // Store action with no used_fields should not trigger
        let mut components = HashMap::new();
        components.insert(
            "App".into(),
            crate::ir::ComponentIR {
                name: "App".into(),
                generic_params: vec![],
                props: HashMap::new(),
                prop_defaults: HashMap::new(),
                states: vec![],
                computeds: vec![],
                effects: vec![],
                lifecycles: vec![],
                fns: vec![],
                store_refs: vec![],
                refs: vec![],
                event_names: vec![],
                view: Some(crate::ir::ViewTree::Element(crate::ir::ElementIR {
                    tag: "button".into(),
                    props: HashMap::new(),
                    bindings: HashMap::new(),
                    events: [(
                        "onClick".into(),
                        crate::ir::EventHandlerIR {
                            kind: "store_action".into(),
                            body: None,
                            r#ref: Some("MyStore.increment".into()),
                            params: vec![],
                            event_data: None,
                            used_fields: None,
                            needs_prevent_default: false,
                            needs_stop_propagation: false,
                        },
                    )]
                    .into_iter()
                    .collect(),
                    style_classes: None,
                    inline_style: None,
                    children: vec![],
                    test_id: None,
                    ref_name: None,
                    construction_kind: "normal".into(),
                    class_map_bindings: HashMap::new(),
                    source_line: 0,
                    source_col: 0,
                    motion_triggers: vec![],
                    states: None,
                    paused_override: None,
                    driven_by: None,
                    hover_reverse: None,
                    in_view_once: None,
                    in_view_repeat: None,
                    in_view_threshold: None,
                    scroll_start: None,
                    scroll_end: None,
                    scroll_scrub: None,
                    drag_constraints: None,
                    drag_elastic: None,
                    drag_scrub: None,
                    drag_momentum: None,
                    drag_threshold: None,
                    drag_velocity_max: None,
                    hero_tag: None,
                    hero_transition: None,
                })),
                styles: vec![],
                motions: vec![],
                if_mode_default: None,
                hero_defs: vec![],
                timelines: vec![],
            },
        );
        let ir = make_test_ir(components, HashMap::new());

        let diags = rules::StoreActionEventData.check_ir(&ir, &HashMap::new());
        assert!(
            diags.is_empty(),
            "Store action without used_fields should not trigger"
        );
    }

    #[test]
    fn test_store_action_skips_inline_handlers() {
        // Inline fn handlers are allowed to use event data
        let mut components = HashMap::new();
        components.insert(
            "App".into(),
            crate::ir::ComponentIR {
                name: "App".into(),
                generic_params: vec![],
                props: HashMap::new(),
                prop_defaults: HashMap::new(),
                states: vec![],
                computeds: vec![],
                effects: vec![],
                lifecycles: vec![],
                fns: vec![],
                store_refs: vec![],
                refs: vec![],
                event_names: vec![],
                view: Some(crate::ir::ViewTree::Element(crate::ir::ElementIR {
                    tag: "button".into(),
                    props: HashMap::new(),
                    bindings: HashMap::new(),
                    events: [(
                        "onClick".into(),
                        crate::ir::EventHandlerIR {
                            kind: "inline_fn".into(),
                            body: None,
                            r#ref: None,
                            params: vec![],
                            event_data: Some(crate::ir::EventDataKind::PointerEvent),
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
                    test_id: None,
                    ref_name: None,
                    construction_kind: "normal".into(),
                    class_map_bindings: HashMap::new(),
                    source_line: 0,
                    source_col: 0,
                    motion_triggers: vec![],
                    states: None,
                    paused_override: None,
                    driven_by: None,
                    hover_reverse: None,
                    in_view_once: None,
                    in_view_repeat: None,
                    in_view_threshold: None,
                    scroll_start: None,
                    scroll_end: None,
                    scroll_scrub: None,
                    drag_constraints: None,
                    drag_elastic: None,
                    drag_scrub: None,
                    drag_momentum: None,
                    drag_threshold: None,
                    drag_velocity_max: None,
                    hero_tag: None,
                    hero_transition: None,
                })),
                styles: vec![],
                motions: vec![],
                if_mode_default: None,
                hero_defs: vec![],
                timelines: vec![],
            },
        );
        let ir = make_test_ir(components, HashMap::new());

        let diags = rules::StoreActionEventData.check_ir(&ir, &HashMap::new());
        assert!(
            diags.is_empty(),
            "Inline fn handlers with event data should not trigger"
        );
    }
}
