//! Lint rule framework and rule registry.
//!
//! Rules run in two phases:
//!   1. AST-level: checks that operate on the parsed AST before lowering.
//!   2. IR-level: checks that operate on the merged IR after lowering.
//!
//! To add a new rule:
//!   1. Create a new file in this directory implementing `LintRule`.
//!   2. Register it in `all_rules()`.

mod ast_collectors;
mod computed_purity;
mod dead_code;
mod helpers;
mod invalid_event_field;
mod missing_return;
mod prop_immutability;
mod store_action_event;
mod store_field;
mod type_mismatch;
mod undefined_ref;
mod unused_import;

use rustc_hash::FxHashMap;
use crate::ir::IRProgram;
use crate::parser::ast::Program;
use std::sync::Arc;

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
pub trait LintRule {
    fn name(&self) -> &str;
    fn category(&self) -> &str;
    fn severity(&self) -> LintSeverity;

    /// Check against the AST (before lowering). Default: no-op.
    fn check_ast(&self, _program: &Program, _file_path: &str) -> Vec<LintDiagnostic> {
        vec![]
    }

    /// Check against the merged IR (after lowering). Default: no-op.
    fn check_ir(
        &self,
        _ir: &IRProgram,
        _file_map: &FxHashMap<String, String>,
    ) -> Vec<LintDiagnostic> {
        vec![]
    }
}

// ═══════════════════════════════════════════════════════════
// Runner
// ═══════════════════════════════════════════════════════════

/// Run all registered lint rules against ASTs and merged IR.
pub fn run_all(programs: &FxHashMap<String, Arc<Program>>, ir: &IRProgram) -> Vec<LintDiagnostic> {
    let rules: Vec<Box<dyn LintRule>> = all_rules();
    let mut diags = Vec::new();

    let file_map: FxHashMap<String, String> = programs
        .keys()
        .map(|path| (path.clone(), path.clone()))
        .collect();

    for (file_path, program) in programs {
        for rule in &rules {
            diags.extend(rule.check_ast(program, file_path));
        }
    }

    for rule in &rules {
        diags.extend(rule.check_ir(ir, &file_map));
    }

    diags
}

/// Register all lint rules here.
fn all_rules() -> Vec<Box<dyn LintRule>> {
    vec![
        Box::new(undefined_ref::UndefinedRef),
        Box::new(type_mismatch::TypeMismatch),
        Box::new(store_field::StoreFieldDirectMutation),
        Box::new(prop_immutability::PropImmutability),
        Box::new(computed_purity::ComputedPurity),
        Box::new(missing_return::MissingReturn),
        Box::new(dead_code::DeadCode),
        Box::new(unused_import::UnusedImport),
        Box::new(invalid_event_field::InvalidEventField),
        Box::new(store_action_event::StoreActionEventData),
    ]
}

#[cfg(test)]
mod tests {
    use rustc_hash::FxHashMap;
    use crate::ir::IRProgram;
    use crate::parser::ast::Program;
    use std::sync::Arc;

    use super::*;

    #[test]
    fn test_run_all_empty_programs() {
        let programs: FxHashMap<String, Arc<Program>> = FxHashMap::default();
        let ir = IRProgram {
            version: "1.0".into(),
            modules: FxHashMap::default(),
            source_map: FxHashMap::default(),
            stdlib_imports: Vec::new(),
        };

        let diags = run_all(&programs, &ir);
        assert_eq!(diags.len(), 0, "no diagnostics expected for empty input");
    }
}
