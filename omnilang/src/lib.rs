//! OmniLang compiler — .omni source -> Intent IR -> JSON / OmniView codegen.
//!
//! Pipeline:
//!   1. Lexer  → token stream
//!   2. Parser → AST
//!   3. Semantic analysis → validated AST + symbol table + type map
//!   4. IR lowering → Intent IR
//!   5. Codegen / JSON serialization

// ── Facade-specific modules (NOT in omnilang-core) ──
pub mod codegen;
pub mod ffi;

// ── Re-exported from omnilang-core (P35 dedup) ──
pub use omnilang_core::analyse;

// ── Re-exported from omnilang-core (P35 dedup) ──
pub use omnilang_core::discovery;
pub use omnilang_core::error;
pub use omnilang_core::ir;
pub use omnilang_core::lexer;
pub use omnilang_core::parser;
pub use omnilang_core::path_mapping;
pub use omnilang_core::semantic;
pub use omnilang_core::sourcemap;
pub use omnilang_core::stdlib;
pub use omnilang_core::validate;

#[cfg(feature = "codegen-omniview")]
use crate::codegen::omniview::{ComponentOutput, StyleResolveResult};
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

// ── Re-exported from omnilang-core (P35 dedup) ──
pub use omnilang_core::{compile, CompileResult, SymbolInfo};

// ═══════════════════════════════════════════════════════
// Shared Pipeline
// ═══════════════════════════════════════════════════════

/// Delegate to omnilang-core's run_pipeline (single source of truth).
fn run_pipeline(source: &str) -> omnilang_core::PipelineOutput {
    omnilang_core::run_pipeline(source)
}

// ═══════════════════════════════════════════════════════
// OmniView Codegen
// ═══════════════════════════════════════════════════════

/// OmniView compile result — JS + HTML + CSS + SourceMap.
#[cfg(feature = "codegen-omniview")]
#[derive(Debug, Serialize, Deserialize, TS)]
pub struct OmniViewResult {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub files: Option<OmniViewFiles>,
    /// Per-component output for hot-replace (keyed by component name)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(type = "Record<string, { js: string; html: string; css: string }> | null")]
    pub components: Option<FxHashMap<String, ComponentOutput>>,
    #[serde(rename = "sourceMap")]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(type = "Array<{ node_id: string }> | null")]
    pub source_map: Option<Vec<sourcemap::SourceMapEntry>>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub errors: Vec<error::Diagnostic>,
}

#[cfg(feature = "codegen-omniview")]
#[derive(Debug, Serialize, Deserialize, TS)]
pub struct OmniViewFiles {
    pub js: String,
    pub html: String,
    pub css: String,
}

/// Compile .omni → OmniView (JS + HTML + CSS + SourceMap). Core function.
#[cfg(feature = "codegen-omniview")]
#[must_use]
pub fn compile_omniview(source: &str, editor_mode: bool, debug_mode: bool) -> OmniViewResult {
    let pipeline = run_pipeline(source);
    let gen_output =
        codegen::omniview::generate(&pipeline.ir, editor_mode, false, debug_mode);
    OmniViewResult {
        success: !pipeline.has_errors,
        files: Some(OmniViewFiles {
            js: gen_output.js,
            html: gen_output.html,
            css: gen_output.css,
        }),
        components: if gen_output.components.is_empty() {
            None
        } else {
            Some(gen_output.components)
        },
        source_map: Some(gen_output.source_map),
        errors: pipeline.diagnostics,
    }
}

// ═══════════════════════════════════════════════════════
// Combined IR + OmniView Codegen
// ═══════════════════════════════════════════════════════

/// Combined compile result — IR + OmniView (JS + HTML + CSS + SourceMap + Components).
/// Runs the full pipeline in one pass, returning both the IR and codegen output.
#[cfg(feature = "codegen-omniview")]
#[derive(Debug, Serialize, Deserialize, TS)]
pub struct OmniViewFullResult {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(type = "Record<string, unknown> | null")]
    pub ir: Option<ir::IRProgram>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub files: Option<OmniViewFiles>,
    /// Per-component output for hot-replace (keyed by component name)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(type = "Record<string, { js: string; html: string; css: string }> | null")]
    pub components: Option<FxHashMap<String, ComponentOutput>>,
    #[serde(rename = "sourceMap")]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(type = "Array<{ node_id: string }> | null")]
    pub source_map: Option<Vec<sourcemap::SourceMapEntry>>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub errors: Vec<error::Diagnostic>,
}

/// Compile .omni -> OmniView + IR in one pass.
/// Eliminates the double lex->parse->semantic->IR when both IR and codegen are needed.
#[cfg(feature = "codegen-omniview")]
#[must_use]
pub fn compile_omniview_full(
    source: &str,
    editor_mode: bool,
    debug_mode: bool,
) -> OmniViewFullResult {
    let pipeline = run_pipeline(source);
    let gen_output =
        codegen::omniview::generate(&pipeline.ir, editor_mode, false, debug_mode);
    OmniViewFullResult {
        success: !pipeline.has_errors,
        ir: Some(pipeline.ir),
        files: Some(OmniViewFiles {
            js: gen_output.js,
            html: gen_output.html,
            css: gen_output.css,
        }),
        components: if gen_output.components.is_empty() {
            None
        } else {
            Some(gen_output.components)
        },
        source_map: Some(gen_output.source_map),
        errors: pipeline.diagnostics,
    }
}

// ═══════════════════════════════════════════════════════
// Style Resolution for Editor Canvas
// ═══════════════════════════════════════════════════════

/// Resolve per-element computed styles for editor canvas rendering.
///
/// Runs the full `codegen::omniview::generate()` pipeline in css_only mode,
/// reusing the authoritative CSS generation path (design blocks, component
/// styles, variant expansion) rather than a parallel ad-hoc IR walker.
///
/// Returns compiled CSS strings + per-element class/tag mappings that
/// Poiesis uses for canvas rendering.
#[cfg(feature = "codegen-omniview")]
#[must_use]
pub fn resolve_styles(source: &str) -> StyleResolveResult {
    let pipeline = run_pipeline(source);
    let gen_output =
        codegen::omniview::generate(&pipeline.ir, true, /* css_only= */ true, false);

    StyleResolveResult {
        success: !pipeline.has_errors && gen_output.warnings.is_empty(),
        design_css: gen_output.css,
        component_css: gen_output.per_component_css,
        element_classes: gen_output.element_classes,
        element_tags: gen_output.element_tags,
        element_tree: gen_output.element_tree,
        nid_map: gen_output.nid_map,
        warnings: gen_output.warnings,
        errors: pipeline.diagnostics,
    }
}

// ═══════════════════════════════════════════════════════
// Multi-Target Codegen
// ═══════════════════════════════════════════════════════

/// Resolve styles for multiple .omni source files (cross-file design imports).
/// Uses the full multi-file analysis pipeline so that `import { DesignName }`
/// statements resolve correctly and design class validation works.
#[cfg(feature = "codegen-omniview")]
#[must_use]
pub fn resolve_styles_multi(
    sources: FxHashMap<String, String>,
    path_mapping: FxHashMap<String, String>,
) -> StyleResolveResult {
    let analysis = crate::analyse::analyse_multi(&sources, &path_mapping);
    let errors = analysis.diagnostics;

    match analysis.ir {
        Some(ir) => {
            let output = codegen::omniview::generate(&ir, true, /* css_only= */ true, false);
            StyleResolveResult {
                success: errors
                    .iter()
                    .all(|d| d.severity != crate::error::Severity::Error)
                    && output.warnings.is_empty(),
                design_css: output.css,
                component_css: output.per_component_css,
                element_classes: output.element_classes,
                element_tags: output.element_tags,
                element_tree: output.element_tree,
                nid_map: output.nid_map,
                warnings: output.warnings,
                errors,
            }
        }
        None => StyleResolveResult {
            success: false,
            design_css: String::new(),
            component_css: FxHashMap::default(),
            element_classes: FxHashMap::default(),
            element_tags: FxHashMap::default(),
            element_tree: Vec::new(),
            nid_map: FxHashMap::default(),
            warnings: Vec::new(),
            errors,
        },
    }
}

/// Compile IR to a specific target, returning JSON with { success, files: {path: content}, errors }.
#[must_use]
pub fn compile_to_target_json(
    ir: &ir::IRProgram,
    target: codegen::Target,
    source_files: &FxHashMap<String, String>,
    store_mode: Option<&str>,
) -> String {
    let output = codegen::compile(ir, target, source_files, store_mode);
    let files_map: FxHashMap<String, String> = output
        .files
        .into_iter()
        .map(|f| (f.path, f.content))
        .collect();
    serde_json::to_string(&serde_json::json!({
        "success": output.errors.is_empty(),
        "files": files_map,
        "errors": output.errors,
        "sourceMap": output.source_map,
    }))
    .unwrap_or_else(|e| {
        format!(
            r#"{{"success":false,"files":{{}},"errors":["serialization error: {}"]}}"#,
            e
        )
    })
}

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

/// Compile multiple .omni sources to a target language, returning a struct.
/// Same as compile_multi_to_target but returns structured data instead of JSON.
#[must_use]
pub fn compile_multi_internal(
    sources: FxHashMap<String, String>,
    target: codegen::Target,
    path_mapping: FxHashMap<String, String>,
    #[allow(unused_variables)] editor_mode: bool,
    #[allow(unused_variables)] debug_mode: bool,
    #[allow(unused_variables)] store_mode: Option<&str>,
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
pub fn compile_multi_internal_cached(
    sources: FxHashMap<String, String>,
    target: codegen::Target,
    path_mapping: FxHashMap<String, String>,
    #[allow(unused_variables)] editor_mode: bool,
    #[allow(unused_variables)] debug_mode: bool,
    #[allow(unused_variables)] store_mode: Option<&str>,
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

/// Shared post-analysis codegen + post-processing for compile_multi_* functions.
#[must_use]
fn compile_multi_internal_post(
    analysis: crate::analyse::AnalyseOutput,
    sources: &FxHashMap<String, String>,
    target: codegen::Target,
    #[allow(unused_variables)] editor_mode: bool,
    #[allow(unused_variables)] debug_mode: bool,
    #[allow(unused_variables)] store_mode: Option<&str>,
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

    // Dispatch to target codegen
    // For OmniView, call compile_files directly to forward editor_mode/debug_mode.
    // Also extract per-component output for hot-replace support.
    #[cfg(feature = "codegen-omniview")]
    let (output, components) = {
        if matches!(target, codegen::Target::OmniView) {
            let gen_output =
                codegen::omniview::generate(&merged_ir, editor_mode, false, debug_mode);
            let files = codegen::omniview::build_file_output(&gen_output);
            let comps: FxHashMap<String, serde_json::Value> = gen_output
                .components
                .into_iter()
                .map(|(name, c)| (name, serde_json::json!({ "js": c.js })))
                .collect();
            (files, comps)
        } else {
            (
                codegen::compile(&merged_ir, target, sources, store_mode),
                FxHashMap::default(),
            )
        }
    };
    #[cfg(not(feature = "codegen-omniview"))]
    let (output, components): (_, FxHashMap<String, serde_json::Value>) = (
        codegen::compile(&merged_ir, target, sources, store_mode),
        FxHashMap::default(),
    );

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
        components,
        source_map: output.source_map,
    }
}

/// Compile multiple .omni sources to a target language, returning JSON.
///
/// Pipeline: parse all sources → cross-file validation → per-file semantic analysis
/// → IR lowering → merged IR → target codegen.
///
/// Returns JSON with structured Diagnostic errors (severity, code, file, line, col, …)
/// plus a flat `warnings` array from lint rules.
#[must_use]
pub fn compile_multi_to_target(
    sources: FxHashMap<String, String>,
    target: codegen::Target,
    path_mapping: FxHashMap<String, String>,
    editor_mode: bool,
    debug_mode: bool,
    store_mode: Option<&str>,
) -> String {
    let result = compile_multi_internal(
        sources,
        target,
        path_mapping,
        editor_mode,
        debug_mode,
        store_mode,
    );
    serde_json::to_string(&serde_json::json!({
        "success": result.success,
        "files": result.files,
        "errors": result.errors,
        "warnings": result.warnings,
        "sourceMap": result.source_map,
    }))
    .unwrap_or_else(|e| {
        format!(
            r#"{{"success":false,"files":{{}},"errors":[{{"severity":"error","code":"E_SERIALIZE","message":"{}","file":null,"line":0,"col":0,"end_line":0,"end_col":0,"suggestion":null}}],"warnings":[]}}"#,
            e
        )
    })
}

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
    #[cfg(feature = "codegen-omniview")]
    use crate::{OmniViewFiles, OmniViewFullResult, OmniViewResult};
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

        // Export feature-gated types (only when codegen-omniview is enabled)
        #[cfg(feature = "codegen-omniview")]
        {
            OmniViewResult::export_all_to(&export_dir)
                .expect("failed to export OmniViewResult TS bindings");
            OmniViewFullResult::export_all_to(&export_dir)
                .expect("failed to export OmniViewFullResult TS bindings");
            OmniViewFiles::export_all_to(&export_dir)
                .expect("failed to export OmniViewFiles TS bindings");
        }

        // Also export Diagnostic directly (in case it's not pulled in transitively)
        error::Diagnostic::export_all_to(&export_dir)
            .expect("failed to export Diagnostic TS bindings");
    }
}

#[cfg(test)]
mod source_map_integration_tests {
    use crate::codegen::Target;
    use rustc_hash::FxHashMap;

    #[test]
    fn compile_to_target_json_contains_source_map() {
        let ir = crate::ir::IRProgram::default();
        let json = crate::compile_to_target_json(&ir, Target::Python, &FxHashMap::default(), None);
        assert!(
            json.contains("\"sourceMap\""),
            "compile_to_target_json output should contain sourceMap key.\nGot: {}",
            json
        );
    }
}
