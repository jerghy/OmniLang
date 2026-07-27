//! OmniLang compiler core — .omni source -> Intent IR -> shared codegen infra.
//!
//! Pipeline:
//!   1. Lexer  → token stream
//!   2. Parser → AST
//!   3. Semantic analysis → validated AST + symbol table + type map
//!   4. IR lowering → Intent IR
//!   5. Codegen / JSON serialization (shared infrastructure only;
//!      backend-specific codegen lives in separate crates)

pub mod analyse;
pub mod codegen;
pub mod discovery;
pub mod error;
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

// ═══════════════════════════════════════════════════════
// Shared Pipeline
// ═══════════════════════════════════════════════════════

/// Internal pipeline result after lex->parse->semantic->IR lowering.
struct PipelineOutput {
    ir: ir::IRProgram,
    diagnostics: Vec<error::Diagnostic>,
    has_errors: bool,
}

/// Run the shared compilation pipeline: lex -> parse -> semantic -> IR lowering.
/// Returns `Err` on parse failure (with parse errors), or `Ok` with the pipeline
/// output including all semantic diagnostics.
fn run_pipeline(source: &str) -> Result<PipelineOutput, Vec<error::Diagnostic>> {
    let tokens = lexer::tokenize(source);
    let program = parser::parse(tokens, source).map_err(|e| vec![e.into()])?;
    let mut semantics = semantic::analyze(&program);

    let has_errors = semantics
        .diagnostics
        .iter()
        .any(|d| matches!(d.severity, crate::error::Severity::Error));
    let ir = ir::lower(&program, &mut semantics);
    Ok(PipelineOutput {
        ir,
        diagnostics: semantics.diagnostics,
        has_errors,
    })
}

/// Compile multiple .omni sources to a target language, returning a struct.
/// Same as compile_multi_to_target but returns structured data instead of JSON.
#[must_use]
#[allow(unused_variables)]
pub fn compile_multi_internal(
    sources: FxHashMap<String, String>,
    target: codegen::Target,
    path_mapping: FxHashMap<String, String>,
    editor_mode: bool,
    debug_mode: bool,
    store_mode: Option<&str>,
) -> InternalCompileResult {
    let analysis = crate::analyse::analyse_multi(&sources, &path_mapping);
    compile_multi_internal_post(
        analysis,
        &sources,
        target,
        editor_mode,
        debug_mode,
        store_mode,
    )
}

/// Compile multiple .omni sources with incremental cache support.
///
/// Same as `compile_multi_internal` but uses `analyse_multi_cached` to
/// skip parsing and analysis of unchanged files.  Pass the cache from
/// `ProjectState::compile_cache` for editor/workflow use.
#[must_use]
#[allow(unused_variables)]
pub fn compile_multi_internal_cached(
    sources: FxHashMap<String, String>,
    target: codegen::Target,
    path_mapping: FxHashMap<String, String>,
    editor_mode: bool,
    debug_mode: bool,
    store_mode: Option<&str>,
    cache: &mut crate::analyse::IncrementalCache,
) -> InternalCompileResult {
    let analysis = crate::analyse::analyse_multi_cached(&sources, &path_mapping, cache);
    compile_multi_internal_post(
        analysis,
        &sources,
        target,
        editor_mode,
        debug_mode,
        store_mode,
    )
}

/// Internal compile result struct (avoids JSON round-trip for native consumers).
#[derive(Debug, Serialize)]
pub struct InternalCompileResult {
    pub success: bool,
    pub files: FxHashMap<String, String>,
    pub errors: Vec<crate::error::Diagnostic>,
    pub warnings: Vec<crate::error::Diagnostic>,
    /// Per-component JS deltas for hot-replace (keyed by component name).
    /// Only populated for OmniView target with codegen-omniview enabled.
    #[serde(default, skip_serializing_if = "FxHashMap::is_empty")]
    pub components: FxHashMap<String, serde_json::Value>,
    /// Source map entries mapping generated code back to .omni source positions.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub source_map: Vec<crate::sourcemap::SourceMapEntry>,
}

/// Shared post-analysis codegen + post-processing for compile_multi_* functions.
#[must_use]
#[allow(unused_variables)]
fn compile_multi_internal_post(
    analysis: crate::analyse::AnalyseOutput,
    sources: &FxHashMap<String, String>,
    target: codegen::Target,
    editor_mode: bool,
    debug_mode: bool,
    store_mode: Option<&str>,
) -> InternalCompileResult {
    let mut diags = analysis.diagnostics;
    let warnings = analysis.warnings;

    let merged_ir = match analysis.ir {
        Some(ir) => ir,
        None => {
            return InternalCompileResult {
                success: false,
                files: FxHashMap::default(),
                errors: diags,
                warnings,
                components: FxHashMap::default(),
                source_map: vec![],
            };
        }
    };

    // In the core crate, dispatch to the shared compile function.
    // Backend-specific codegen is handled by the facade crate or individual backend crates.
    let output = codegen::compile(&merged_ir, target, sources, store_mode);

    let files_map: FxHashMap<String, String> = output
        .files
        .into_iter()
        .map(|f| (f.path, f.content))
        .collect();

    // Codegen errors are already Diagnostic objects
    diags.extend(output.errors);

    let success = diags.iter().all(|d| d.severity != error::Severity::Error);
    InternalCompileResult {
        success,
        files: files_map,
        errors: diags,
        warnings,
        components: FxHashMap::default(),
        source_map: output.source_map,
    }
}
