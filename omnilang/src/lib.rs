//! OmniLang compiler — .omni source -> Intent IR -> JSON / OmniView codegen.
//!
//! Pipeline:
//!   1. Lexer  → token stream
//!   2. Parser → AST
//!   3. Semantic analysis → validated AST + symbol table + type map
//!   4. IR lowering → Intent IR
//!   5. Codegen / JSON serialization

pub mod codegen;
pub mod error;
pub mod ffi;
pub mod ir;
pub mod lexer;
pub mod parser;
pub mod semantic;
pub mod sourcemap;
pub mod stdlib;
pub mod validate;

#[cfg(feature = "wasm")]
pub mod wasm;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::codegen::omniview::ComponentOutput;
use crate::parser::ast::{ImportStmt, Program};

/// Top-level compile result — returned as JSON by WASM/native bindings.
#[derive(Debug, Serialize, Deserialize)]
pub struct CompileResult {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ir: Option<ir::IRProgram>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub errors: Vec<error::Diagnostic>,
}

/// Compile .omni source -> IR. Core function shared by WASM and native.
pub fn compile(source: &str) -> CompileResult {
    let mut errors: Vec<error::Diagnostic> = Vec::new();

    // 1. Lex
    let tokens = lexer::tokenize(source);

    // 2. Parse
    let program = match parser::parse(&tokens, source) {
        Ok(p) => p,
        Err(e) => {
            errors.push(e.into());
            return CompileResult {
                success: false,
                ir: None,
                errors,
            };
        }
    };

    // 3. Semantic analysis — accumulate ALL diagnostics, always return partial result
    let semantics = semantic::analyze(&program);
    let has_semantic_errors = semantics
        .diagnostics
        .iter()
        .any(|d| matches!(d.severity, crate::error::Severity::Error));
    // Collect ALL diagnostics (errors, warnings, info) into the output
    errors.extend(semantics.diagnostics.clone());

    // 4. Lower to IR
    let ir_program = ir::lower(&program, &semantics);

    CompileResult {
        // success=true only if no Error-severity diagnostics
        success: !has_semantic_errors,
        ir: Some(ir_program), // always return IR (even with warnings)
        errors,
    }
}

// ═══════════════════════════════════════════════════════
// OmniView Codegen
// ═══════════════════════════════════════════════════════

/// OmniView compile result — JS + HTML + CSS + SourceMap.
#[derive(Debug, Serialize, Deserialize)]
pub struct OmniViewResult {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub files: Option<OmniViewFiles>,
    /// Per-component output for hot-replace (keyed by component name)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub components: Option<HashMap<String, ComponentOutput>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_map: Option<Vec<sourcemap::SourceMapEntry>>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub errors: Vec<error::Diagnostic>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OmniViewFiles {
    pub js: String,
    pub html: String,
    pub css: String,
}

/// Compile .omni → OmniView (JS + HTML + CSS + SourceMap). Core function.
pub fn compile_omniview(source: &str, editor_mode: bool) -> OmniViewResult {
    let mut errors: Vec<error::Diagnostic> = Vec::new();

    // 1. Lex
    let tokens = lexer::tokenize(source);

    // 2. Parse
    let program = match parser::parse(&tokens, source) {
        Ok(p) => p,
        Err(e) => {
            errors.push(e.into());
            return OmniViewResult {
                success: false,
                files: None,
                components: None,
                source_map: None,
                errors,
            };
        }
    };

    // 3. Semantic analysis — accumulate ALL diagnostics, always proceed to lowering
    let semantics = semantic::analyze(&program);
    let has_errors = semantics
        .diagnostics
        .iter()
        .any(|d| matches!(d.severity, crate::error::Severity::Error));
    // Collect ALL diagnostics (errors, warnings, info) into the output
    for d in &semantics.diagnostics {
        errors.push(d.clone());
    }

    // 4. Lower to IR
    let ir_program = ir::lower(&program, &semantics);

    // 5. Generate OmniView output
    let output = codegen::omniview::generate(&ir_program, editor_mode);

    OmniViewResult {
        success: !has_errors,
        files: Some(OmniViewFiles {
            js: output.js,
            html: output.html,
            css: output.css,
        }),
        components: if output.components.is_empty() {
            None
        } else {
            Some(output.components)
        },
        source_map: Some(output.source_map),
        errors,
    }
}

// ═══════════════════════════════════════════════════════
// Multi-Target Codegen
// ═══════════════════════════════════════════════════════

/// Compile IR to a specific target, returning JSON with { success, files: {path: content}, errors }.
pub fn compile_to_target_json(
    ir: &ir::IRProgram,
    target: codegen::Target,
    source_files: &HashMap<String, String>,
) -> String {
    let output = codegen::compile(ir, target, source_files);
    let files_map: HashMap<String, String> = output
        .files
        .into_iter()
        .map(|f| (f.path, f.content))
        .collect();
    serde_json::to_string(&serde_json::json!({
        "success": output.errors.is_empty(),
        "files": files_map,
        "errors": output.errors,
    }))
    .unwrap_or_else(|e| {
        format!(
            r#"{{"success":false,"files":{{}},"errors":["serialization error: {}"]}}"#,
            e
        )
    })
}

/// Compile multiple .omni sources to a target language.
///
/// Pipeline: parse all sources → cross-file validation → per-file semantic analysis
/// → IR lowering → merged IR → target codegen.
///
/// Returns JSON: { success: bool, files: { path: content }, errors: [...] }
pub fn compile_multi_to_target(
    sources: HashMap<String, String>,
    target: codegen::Target,
    path_mapping: HashMap<String, String>,
) -> String {
    let mut errors: Vec<String> = Vec::new();
    let mut programs: HashMap<String, Program> = HashMap::new();

    // Phase 1: Parse all sources
    for (path, source) in &sources {
        let tokens = lexer::tokenize(source);
        match parser::parse(&tokens, source) {
            Ok(program) => {
                programs.insert(path.clone(), program);
            }
            Err(e) => {
                errors.push(format!("{}: parse error: {}", path, e.message));
            }
        }
    }

    if !errors.is_empty() {
        return serde_json::to_string(&serde_json::json!({
            "success": false,
            "files": {},
            "errors": errors,
        }))
        .unwrap_or_else(|e| {
            format!(
                r#"{{"success":false,"files":{{}},"errors":["serialization error: {}"]}}"#,
                e
            )
        });
    }

    // Phase 2: Cross-file validation
    let validation_result = validate::validate_multi(sources.clone(), path_mapping);
    errors.extend(validation_result.errors);

    // Phase 3: Per-file semantic analysis + IR lowering → merged IR
    let mut merged_ir = ir::IRProgram {
        version: "1.0".to_string(),
        modules: HashMap::new(),
        source_map: HashMap::new(),
        stdlib_imports: Vec::new(),
    };

    for (path, program) in &programs {
        let semantics = semantic::analyze(program);
        for d in &semantics.diagnostics {
            errors.push(format!("{}: {}", path, d.message));
        }

        let mut ir_program = ir::lower(program, &semantics);

        // Annotate components and stores with their source file path,
        // then merge modules — combine declarations from multiple files.
        for (mod_name, mut mod_ir) in ir_program.modules {
            // Stamp source file path onto each component and store
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

            merged_ir
                .modules
                .entry(mod_name)
                .and_modify(|existing| {
                    existing.components.extend(mod_ir.components.clone());
                    existing.stores.extend(mod_ir.stores.clone());
                    existing.functions.extend(mod_ir.functions.clone());
                    existing.structs.extend(mod_ir.structs.clone());
                    existing.type_aliases.extend(mod_ir.type_aliases.clone());
                    existing.imports.extend(mod_ir.imports.clone());
                    existing.exports.extend(mod_ir.exports.clone());
                    existing.motions.extend(mod_ir.motions.clone());
                    existing.timelines.extend(mod_ir.timelines.clone());
                })
                .or_insert(mod_ir);
        }
        // Merge source maps
        for (key, entry) in ir_program.source_map {
            merged_ir.source_map.entry(key).or_insert(entry);
        }
    }

    // Phase 4: Run lint rules against ASTs and merged IR
    let lint_diags = crate::semantic::lints::run_all(&programs, &merged_ir);
    let warnings: Vec<String> = lint_diags
        .iter()
        .filter(|d| {
            matches!(
                d.severity,
                crate::semantic::lints::LintSeverity::Warning
                    | crate::semantic::lints::LintSeverity::Info
            )
        })
        .map(|d| {
            format!(
                "[{}] {}:{}: {} (rule: {})",
                d.file, d.line, d.col, d.message, d.rule
            )
        })
        .collect();

    // Phase 5: Dispatch to target codegen
    let output = codegen::compile(&merged_ir, target, &sources);
    let files_map: HashMap<String, String> = output
        .files
        .into_iter()
        .map(|f| (f.path, f.content))
        .collect();

    // Combine codegen errors with earlier errors
    let all_errors: Vec<String> = errors.into_iter().chain(output.errors).collect();

    serde_json::to_string(&serde_json::json!({
        "success": all_errors.is_empty(),
        "files": files_map,
        "errors": all_errors,
        "warnings": warnings,
    }))
    .unwrap_or_else(|e| {
        format!(
            r#"{{"success":false,"files":{{}},"errors":["serialization error: {}"]}}"#,
            e
        )
    })
}
