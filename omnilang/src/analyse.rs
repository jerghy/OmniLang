//! Shared multi-file analysis pipeline.
//!
//! Unifies parse → validate → semantic → lint into a single function,
//! eliminating duplicate pipeline code across lib.rs, ffi.rs, and api.rs.
//! Also avoids a redundant parse pass by calling `validate_multi_ref()`
//! instead of `validate_multi()`.

use std::collections::HashMap;

use crate::error::{Diagnostic, DiagnosticKind};
use crate::ir::IRProgram;
use crate::parser::ast::Program;

/// Result of multi-file analysis.
pub struct AnalyseOutput {
    pub success: bool,
    pub diagnostics: Vec<Diagnostic>,
    pub ir: Option<IRProgram>,
    pub warnings: Vec<Diagnostic>,
    /// Pre-parsed programs (available to callers that need them for codegen).
    pub programs: HashMap<String, Program>,
}

/// Parse + cross-file validation + semantic analysis + lint for multiple source files.
///
/// Pipeline:
///   1. Parse all sources into ASTs
///   2. Cross-file validation (imports, visibility, cycles)
///   3. Per-file semantic analysis + IR lowering → merged IR
///   4. Lint rules (AST-level and IR-level)
///
/// Returns early on parse failure (ir = None, success = false).
#[must_use]
pub fn analyse_multi(
    sources: &HashMap<String, String>,
    path_mapping: &HashMap<String, String>,
) -> AnalyseOutput {
    let mut diags: Vec<Diagnostic> = Vec::new();
    let mut programs: HashMap<String, Program> = HashMap::new();

    // Phase 1: Parse all sources
    for (path, source) in sources {
        let tokens = crate::lexer::tokenize(source);
        match crate::parser::parse(&tokens, source) {
            Ok(program) => {
                programs.insert(path.clone(), program);
            }
            Err(e) => {
                let d: Diagnostic = e.into();
                diags.push(d.with_file(path));
            }
        }
    }

    if !diags.is_empty() {
        return AnalyseOutput {
            success: false,
            diagnostics: diags,
            ir: None,
            warnings: vec![],
            programs,
        };
    }

    // Phase 2: Cross-file validation using pre-parsed programs
    let validation = crate::validate::validate_multi_ref(&programs, path_mapping);
    diags.extend(validation.errors);

    // Phase 3: Per-file semantic analysis + IR lowering → merged IR
    let mut merged_ir = IRProgram {
        version: "1.0".to_string(),
        modules: HashMap::new(),
        source_map: HashMap::new(),
        stdlib_imports: Vec::new(),
    };

    for (path, program) in &programs {
        let mut semantics = crate::semantic::analyze(program);
        for d in &semantics.diagnostics {
            let mut d = d.clone();
            if d.file.is_none() {
                d.file = Some(path.clone());
            }
            diags.push(d);
        }

        let ir_program = crate::ir::lower(program, &mut semantics);

        // Stamp source file path onto IR nodes for multi-file codegen,
        // then merge modules — combine declarations from multiple files.
        for (mod_name, mut mod_ir) in ir_program.modules {
            for comp in mod_ir.components.values_mut() {
                if comp.source_file.is_none() {
                    comp.source_file = Some(path.clone());
                }
            }
            for store in mod_ir.stores.values_mut() {
                if store.source_file.is_none() {
                    store.source_file = Some(path.clone());
                }
            }

            // Use entry().or_default() + std::mem::take to avoid cloning
            // all collections on the merge path.
            let existing = merged_ir.modules.entry(mod_name).or_default();
            existing
                .components
                .extend(std::mem::take(&mut mod_ir.components));
            existing.stores.extend(std::mem::take(&mut mod_ir.stores));
            existing
                .functions
                .extend(std::mem::take(&mut mod_ir.functions));
            existing.structs.extend(std::mem::take(&mut mod_ir.structs));
            existing
                .type_aliases
                .extend(std::mem::take(&mut mod_ir.type_aliases));
            existing.imports.extend(std::mem::take(&mut mod_ir.imports));
            existing.exports.extend(std::mem::take(&mut mod_ir.exports));
            existing.motions.extend(std::mem::take(&mut mod_ir.motions));
            existing
                .timelines
                .extend(std::mem::take(&mut mod_ir.timelines));
        }

        for (key, entry) in ir_program.source_map {
            merged_ir.source_map.entry(key).or_insert(entry);
        }
    }

    // Phase 4: Run lint rules against ASTs and merged IR
    let lint_diags = crate::semantic::lints::run_all(&programs, &merged_ir);
    let warnings: Vec<Diagnostic> = lint_diags
        .iter()
        .filter(|d| {
            matches!(
                d.severity,
                crate::semantic::lints::LintSeverity::Warning
                    | crate::semantic::lints::LintSeverity::Info
            )
        })
        .map(|d| Diagnostic {
            severity: match d.severity {
                crate::semantic::lints::LintSeverity::Warning => crate::error::Severity::Warning,
                crate::semantic::lints::LintSeverity::Info => crate::error::Severity::Info,
                _ => crate::error::Severity::Warning,
            },
            kind: DiagnosticKind::ELint(d.rule.clone()),
            message: d.message.clone(),
            file: Some(d.file.clone()),
            line: d.line,
            col: d.col,
            end_line: None,
            end_col: None,
            suggestion: None,
        })
        .collect();

    // Lint errors are also included in diagnostics so that all callers
    // (compile_multi_to_target, omni_analyse, analyse_direct) see them.
    for d in &lint_diags {
        if d.severity == crate::semantic::lints::LintSeverity::Error {
            diags.push(Diagnostic {
                severity: crate::error::Severity::Error,
                kind: DiagnosticKind::ELint(d.rule.clone()),
                message: d.message.clone(),
                file: Some(d.file.clone()),
                line: d.line,
                col: d.col,
                end_line: None,
                end_col: None,
                suggestion: None,
            });
        }
    }

    let success = !diags
        .iter()
        .any(|d| d.severity == crate::error::Severity::Error);
    AnalyseOutput {
        success,
        diagnostics: diags,
        ir: Some(merged_ir),
        warnings,
        programs,
    }
}
