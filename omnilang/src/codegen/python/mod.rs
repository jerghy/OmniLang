//! Python codegen: IR -> Python.
//!
//! Generates Python source files from Intent IR. Each module becomes
//! a .py file with:
//! - Type aliases (e.g., `UserId = int`)
//! - Structs as `@dataclass` classes
//! - Enums as `Enum` classes
//! - Standalone functions (`def` without `self`)
//! - Stores as standalone classes
//! - Components as classes with methods

pub mod expr;
pub mod stmt;
mod visitor;

#[cfg(test)]
mod tests;

use super::{CodeWriter, Codegen, CompileOutput, CompiledFile};
use crate::codegen::shared::types::{FormatType, PythonTypeFormatter};
use crate::codegen::shared::visitor::CodegenVisitor;
use crate::error::{Diagnostic, DiagnosticKind, Severity};
use crate::ir::*;
use rustc_hash::FxHashMap;
use std::cell::RefCell;
use std::rc::Rc;

pub struct PythonCodegen;

impl Codegen for PythonCodegen {
    fn generate(&self, ir: &IRProgram, _source_files: &FxHashMap<String, String>) -> CompileOutput {
        let mut files = Vec::new();
        // Codegen-message channel: expression formatters report unsupported
        // constructs (e.g. statement-bearing block expressions) here; each
        // message becomes an E_CODEGEN error in the output.
        let diagnostics = Rc::new(RefCell::new(Vec::<String>::new()));

        for (module_key, module) in &ir.modules {
            // Strip .omni suffix if present to derive module name
            let module_name = module_key.strip_suffix(".omni").unwrap_or(module_key);
            let source_path = format!("{}.omni", module_name);

            let mut w = CodeWriter::new();
            w.emit_fmt(format_args!("# Generated from {}", source_path));
            w.emit("");

            let mut visitor = visitor::PythonVisitor {
                diagnostics: Rc::clone(&diagnostics),
            };
            visitor.generate_module(&mut w, module);

            let out_path = format!("{}.py", module_name);
            files.push(CompiledFile {
                path: out_path,
                content: w.into_string(),
            });
        }

        let errors = diagnostics
            .borrow()
            .iter()
            .map(|message| Diagnostic {
                severity: Severity::Error,
                kind: DiagnosticKind::ECodegen,
                message: message.clone(),
                file: None,
                line: 0,
                col: 0,
                end_line: None,
                end_col: None,
                suggestion: None,
            })
            .collect();
        CompileOutput {
            files,
            errors,
            source_map: vec![],
            warnings: vec![],
        }
    }
}

// ═══════════════════════════════════════════════════════════
// Module-level helpers
// ═══════════════════════════════════════════════════════════

/// Generate import statements needed by logic constructs.
fn generate_module_imports(w: &mut CodeWriter, module: &ModuleIR) {
    let mut imports: Vec<&str> = Vec::new();
    if !module.structs.is_empty() {
        imports.push("from dataclasses import dataclass");
    }
    if !module.enums.is_empty() {
        imports.push("from enum import Enum");
    }
    for imp in &imports {
        w.emit(imp);
    }
    if !imports.is_empty() {
        w.emit("");
    }
}

/// Map OMNI type to Python type annotation string.
pub fn py_type(kind: &str, type_args: &[TypeIR]) -> String {
    PythonTypeFormatter.format_type(kind, type_args)
}
