//! OmniLang compiler — .omni source -> Intent IR -> JSON / OmniView codegen.
//!
//! Pipeline:
//!   1. Lexer  → token stream
//!   2. Parser → AST
//!   3. Semantic analysis → validated AST + symbol table + type map
//!   4. IR lowering → Intent IR
//!   5. Codegen / JSON serialization

pub mod analyse;
pub mod codegen;
pub mod discovery;
pub mod error;
pub mod ffi;
pub mod ir;
pub mod lexer;
pub mod parser;
pub mod path_mapping;
pub mod semantic;
pub mod sourcemap;
pub mod stdlib;
pub mod validate;

use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// A single symbol from semantic analysis — variable, signal, prop, store, etc.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[ts(export)]
pub struct SymbolInfo {
    pub name: String,
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub type_ann: Option<String>,
    pub mutable: bool,
}

/// Top-level compile result — returned as JSON by WASM/native bindings.
#[derive(Debug, Serialize, Deserialize, TS)]
pub struct CompileResult {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(type = "Record<string, unknown> | null")]
    pub ir: Option<ir::IRProgram>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub errors: Vec<error::Diagnostic>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub symbols: Vec<SymbolInfo>,
    #[serde(default, skip_serializing_if = "FxHashMap::is_empty")]
    pub dependencies: FxHashMap<String, Vec<String>>,
}

/// Compile .omni source -> IR. Core function shared by WASM and native.
#[must_use]
pub fn compile(source: &str) -> CompileResult {
    let output = run_pipeline(source);
    CompileResult {
        success: !output.has_errors,
        ir: Some(output.ir),
        errors: output.diagnostics,
        symbols: output.symbols,
        dependencies: output.dependencies,
    }
}

// ═══════════════════════════════════════════════════════
// Shared Pipeline
// ═══════════════════════════════════════════════════════

/// Pipeline result after lex->parse->semantic->IR lowering.
/// Public so the facade crate can delegate to this single source of truth.
pub struct PipelineOutput {
    pub ir: ir::IRProgram,
    pub diagnostics: Vec<error::Diagnostic>,
    pub has_errors: bool,
    pub symbols: Vec<SymbolInfo>,
    pub dependencies: FxHashMap<String, Vec<String>>,
}

/// Run the shared compilation pipeline: lex -> parse -> semantic -> IR lowering.
/// Always returns a PipelineOutput with all diagnostics (parse errors, semantic
/// errors, etc.) bundled together. Parse errors are included in `diagnostics`.
pub fn run_pipeline(source: &str) -> PipelineOutput {
    let tokens = lexer::tokenize(source);
    let (program, parse_errors) = parser::parse(tokens, source);
    let mut diagnostics: Vec<error::Diagnostic> =
        parse_errors.into_iter().map(|e| e.into()).collect();

    let mut semantics = semantic::analyze(&program);

    // Extract symbols before ir::lower() which borrows semantics mutably
    let symbols: Vec<SymbolInfo> = semantics
        .symbols
        .scopes()
        .iter()
        .flat_map(|scope| scope.symbols.values())
        .map(|sym| SymbolInfo {
            name: sym.name.clone(),
            kind: format!("{:?}", sym.kind).to_lowercase(),
            type_ann: sym.type_ann.clone(),
            mutable: sym.mutable,
        })
        .collect();
    let dependencies = std::mem::take(&mut semantics.dependencies);

    diagnostics.extend(std::mem::take(&mut semantics.diagnostics));
    let has_errors = diagnostics
        .iter()
        .any(|d| matches!(d.severity, crate::error::Severity::Error));
    let mut ir = ir::lower(&program, &mut semantics);
    crate::ir::optimize::optimize(&mut ir);
    PipelineOutput {
        ir,
        diagnostics,
        has_errors,
        symbols,
        dependencies,
    }
}

// ═══════════════════════════════════════════════════════
// OmniView / Style Resolution / Multi-Target Codegen
// ═══════════════════════════════════════════════════════
//
// Moved to individual backend crates and the omnilang facade crate.
// omnilang-core only provides shared types and infrastructure.

/// Internal compile result struct (avoids JSON round-trip for native consumers).
#[derive(Debug, Serialize, TS)]
pub struct InternalCompileResult {
    pub success: bool,
    #[ts(type = "Record<string, string>")]
    pub files: FxHashMap<String, String>,
    pub errors: Vec<crate::error::Diagnostic>,
    pub warnings: Vec<crate::error::Diagnostic>,
    /// Per-component JS deltas for hot-replace (keyed by component name).
    /// Only populated for OmniView target with codegen-omniview enabled.
    #[serde(default, skip_serializing_if = "FxHashMap::is_empty")]
    #[ts(type = "Record<string, { js: string }>")]
    pub components: FxHashMap<String, serde_json::Value>,
    /// Source map entries mapping generated code back to .omni source positions.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[ts(
        type = "Array<{ node_id: string; omni_span: { line: number; col: number; len: number }; target_spans: Array<{ file: string; line: number; col: number; len: number; kind: string }> }>"
    )]
    pub source_map: Vec<crate::sourcemap::SourceMapEntry>,
}

// ═══════════════════════════════════════════════════════
// Multi-target compile / OmniView + backend orchestration
// ═══════════════════════════════════════════════════════
//
// `compile_to_target_json`, `compile_multi_internal`, `compile_multi_internal_cached`,
// `compile_multi_internal_post`, and `compile_multi_to_target` have been moved to the
// omnilang facade crate. They depend on backend-specific codegen modules that are
// intentionally excluded from omnilang-core.

// ═══════════════════════════════════════════════════════
// TS Export (generates TypeScript type definitions)
// ═══════════════════════════════════════════════════════

// Export all `#[derive(TS)]` types as TypeScript definition files.
// ── Code-block parsing ──────────────────────────────────────

/// Result of parsing code-block source into structured AST statements.
/// NOTE: Code-block parsing returns empty stmts — structured code-block
/// AST parsing is deferred to the Projection Editor (方案 3).
/// Code blocks are currently stored and emitted as raw text via Monaco.
#[derive(Debug, Serialize)]
pub struct ParseStmtsResult {
    pub success: bool,
    pub stmts: Vec<serde_json::Value>,
    pub errors: Vec<error::Diagnostic>,
}

/// Parse a code-block source string into a list of AST statements.
/// Used by WASM/JS `parse_code_block()`.
///
/// Currently returns an empty success result. Structured code-block
/// parsing will be implemented as part of the Projection Editor.
pub fn parse_stmts(_source: &str) -> ParseStmtsResult {
    ParseStmtsResult {
        success: true,
        stmts: vec![],
        errors: vec![],
    }
}

///
/// Generated files go to `poiesis/src/commands/__generated__/`.
/// Dependencies of each root type are exported automatically.
/// Run: `cargo test export_bindings --lib`
#[cfg(test)]
mod type_exports {
    use crate::{error, CompileResult};
    use ts_rs::TS;

    #[test]
    fn export_bindings() {
        // Set the export directory relative to the crate root
        let export_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("poiesis")
            .join("src")
            .join("commands")
            .join("__generated__");
        std::fs::create_dir_all(&export_dir).expect("failed to create __generated__ directory");

        // Export CompileResult and all its dependencies (Diagnostic, Severity)
        CompileResult::export_all_to(&export_dir)
            .expect("failed to export CompileResult TS bindings");

        // Also export Diagnostic directly (in case it's not pulled in transitively)
        error::Diagnostic::export_all_to(&export_dir)
            .expect("failed to export Diagnostic TS bindings");
    }
}

#[cfg(test)]
mod source_map_integration_tests {
    use crate::ir::IRProgram;

    #[test]
    fn ir_program_has_source_map() {
        let ir = IRProgram::default();
        assert!(
            ir.source_map.is_empty(),
            "default IRProgram should have empty source_map"
        );
    }
}
