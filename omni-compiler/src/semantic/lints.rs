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
        // Batch 1: Important warnings
        Box::new(rules::MissingReturn),
        Box::new(rules::DeadCode),
        Box::new(rules::UnusedImport),
        // NOTE: Add Batch 2 rules here (ownership, performance, etc.)
    ]
}

// ═══════════════════════════════════════════════════════════
// Rule Implementations
// ═══════════════════════════════════════════════════════════

pub mod rules {
    use super::*;
    use crate::ir::{BlockIR, ExprIR, StatementIR};

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

            // TODO: Batch 2 — detect unused signals, unused functions,
            // unused computed values, unused props.

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

    use crate::parser::ast::{Block, Expr, Stmt, ViewBlock, ViewNode};

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
            ViewNode::SlotDirective(_) => {}
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
}
