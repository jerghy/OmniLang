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

#[cfg(feature = "wasm")]
pub mod wasm;

use serde::{Deserialize, Serialize};
use rustc_hash::FxHashMap;
#[cfg(feature = "codegen-omniview")]
use crate::codegen::omniview::ComponentOutput;

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
#[must_use]
pub fn compile(source: &str) -> CompileResult {
    match run_pipeline(source) {
        Err(errors) => CompileResult {
            success: false,
            ir: None,
            errors,
        },
        Ok(output) => CompileResult {
            success: !output.has_errors,
            ir: Some(output.ir),
            errors: output.diagnostics,
        },
    }
}

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

// ═══════════════════════════════════════════════════════
// OmniView Codegen
// ═══════════════════════════════════════════════════════

/// OmniView compile result — JS + HTML + CSS + SourceMap.
#[cfg(feature = "codegen-omniview")]
#[derive(Debug, Serialize, Deserialize)]
pub struct OmniViewResult {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub files: Option<OmniViewFiles>,
    /// Per-component output for hot-replace (keyed by component name)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub components: Option<FxHashMap<String, ComponentOutput>>,
    #[serde(rename = "sourceMap")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_map: Option<Vec<sourcemap::SourceMapEntry>>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub errors: Vec<error::Diagnostic>,
}

#[cfg(feature = "codegen-omniview")]
#[derive(Debug, Serialize, Deserialize)]
pub struct OmniViewFiles {
    pub js: String,
    pub html: String,
    pub css: String,
}

/// Compile .omni → OmniView (JS + HTML + CSS + SourceMap). Core function.
#[cfg(feature = "codegen-omniview")]
#[must_use]
pub fn compile_omniview(source: &str, editor_mode: bool) -> OmniViewResult {
    match run_pipeline(source) {
        Err(errors) => OmniViewResult {
            success: false,
            files: None,
            components: None,
            source_map: None,
            errors,
        },
        Ok(pipeline) => {
            let gen_output = codegen::omniview::generate(&pipeline.ir, editor_mode);
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
    }
}

// ═══════════════════════════════════════════════════════
// Combined IR + OmniView Codegen
// ═══════════════════════════════════════════════════════

/// Combined compile result — IR + OmniView (JS + HTML + CSS + SourceMap + Components).
/// Runs the full pipeline in one pass, returning both the IR and codegen output.
#[cfg(feature = "codegen-omniview")]
#[derive(Debug, Serialize, Deserialize)]
pub struct OmniViewFullResult {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ir: Option<ir::IRProgram>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub files: Option<OmniViewFiles>,
    /// Per-component output for hot-replace (keyed by component name)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub components: Option<FxHashMap<String, ComponentOutput>>,
    #[serde(rename = "sourceMap")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_map: Option<Vec<sourcemap::SourceMapEntry>>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub errors: Vec<error::Diagnostic>,
}

/// Compile .omni -> OmniView + IR in one pass.
/// Eliminates the double lex->parse->semantic->IR when both IR and codegen are needed.
#[cfg(feature = "codegen-omniview")]
#[must_use]
pub fn compile_omniview_full(source: &str, editor_mode: bool) -> OmniViewFullResult {
    match run_pipeline(source) {
        Err(errors) => OmniViewFullResult {
            success: false,
            ir: None,
            files: None,
            components: None,
            source_map: None,
            errors,
        },
        Ok(pipeline) => {
            let gen_output = codegen::omniview::generate(&pipeline.ir, editor_mode);
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
    }
}

// ═══════════════════════════════════════════════════════
// Multi-Target Codegen
// ═══════════════════════════════════════════════════════

/// Compile IR to a specific target, returning JSON with { success, files: {path: content}, errors }.
#[must_use]
pub fn compile_to_target_json(
    ir: &ir::IRProgram,
    target: codegen::Target,
    source_files: &FxHashMap<String, String>,
) -> String {
    let output = codegen::compile(ir, target, source_files);
    let files_map: FxHashMap<String, String> = output
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

/// Extract the inner HTML between `<body>` and `</body>` tags.
/// Works on both multi-line and minified/single-line HTML.
#[cfg(feature = "codegen-omniview")]
fn extract_body_html(html: &str) -> String {
    // Find <body> tag (with optional attributes)
    let body_start = html
        .find("<body")
        .and_then(|i| html[i..].find('>').map(|j| i + j + 1));
    let body_end = html.find("</body>");

    match (body_start, body_end) {
        (Some(start), Some(end)) if start < end => html[start..end].to_string(),
        _ => {
            // Fallback: try line-based extraction for unusual formatting
            html.lines()
                .skip_while(|l| !l.contains("<body"))
                .skip(1)
                .take_while(|l| !l.contains("</body>"))
                .collect::<Vec<_>>()
                .join("\n")
        }
    }
}

/// Internal compile result struct (avoids JSON round-trip for native consumers).
#[derive(Debug, Serialize)]
pub struct InternalCompileResult {
    pub success: bool,
    pub files: FxHashMap<String, String>,
    pub errors: Vec<crate::error::Diagnostic>,
    pub warnings: Vec<crate::error::Diagnostic>,
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
) -> InternalCompileResult {
    let analysis = crate::analyse::analyse_multi(&sources, &path_mapping);
    compile_multi_internal_post(analysis, &sources, target, editor_mode)
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
    cache: &mut crate::analyse::IncrementalCache,
) -> InternalCompileResult {
    let analysis = crate::analyse::analyse_multi_cached(&sources, &path_mapping, cache);
    compile_multi_internal_post(analysis, &sources, target, editor_mode)
}

/// Shared post-analysis codegen + post-processing for compile_multi_* functions.
#[must_use]
#[allow(unused_variables)]
fn compile_multi_internal_post(
    analysis: crate::analyse::AnalyseOutput,
    sources: &FxHashMap<String, String>,
    target: codegen::Target,
    editor_mode: bool,
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
            };
        }
    };

    // Dispatch to target codegen
    let output = codegen::compile(&merged_ir, target, sources);
    #[allow(unused_mut)]
    let mut files_map: FxHashMap<String, String> = output
        .files
        .into_iter()
        .map(|f| (f.path, f.content))
        .collect();

    // Codegen errors are already Diagnostic objects
    diags.extend(output.errors);

    // Post-process: wrap debug HTML for editor mode.
    // Only available when the omniview codegen backend is enabled.
    #[cfg(feature = "codegen-omniview")]
    if editor_mode && matches!(target, codegen::Target::OmniView) {
        for path in files_map.keys().cloned().collect::<Vec<_>>() {
            if path.ends_with(".html") {
                // Resolve JS/CSS by matching the HTML file's base name instead of
                // hardcoding "index.*" — supports multi-page projects.
                let base = path.strip_suffix(".html").unwrap_or(&path);
                let js_key = format!("{}.js", base);
                let css_key = format!("{}.css", base);
                let js = files_map
                    .get(&js_key)
                    .or_else(|| files_map.get("index.js"))
                    .map(|s| s.as_str())
                    .unwrap_or("");
                let css = files_map
                    .get(&css_key)
                    .or_else(|| files_map.get("index.css"))
                    .map(|s| s.as_str())
                    .unwrap_or("");
                if let Some(html_content) = files_map.get(&path) {
                    // Use substring extraction instead of line-based parsing —
                    // works on both multi-line and minified/single-line HTML.
                    let body_html = extract_body_html(html_content);
                    files_map.insert(
                        path,
                        codegen::omniview::wrap_debug_html(js, css, &body_html),
                    );
                }
                break;
            }
        }
    }

    let success = diags.iter().all(|d| d.severity != error::Severity::Error);
    InternalCompileResult {
        success,
        files: files_map,
        errors: diags,
        warnings,
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
) -> String {
    let result = compile_multi_internal(sources, target, path_mapping, editor_mode);
    serde_json::to_string(&serde_json::json!({
        "success": result.success,
        "files": result.files,
        "errors": result.errors,
        "warnings": result.warnings,
    }))
    .unwrap_or_else(|e| {
        format!(
            r#"{{"success":false,"files":{{}},"errors":[{{"severity":"error","code":"E_SERIALIZE","message":"{}","file":null,"line":0,"col":0,"end_line":0,"end_col":0,"suggestion":null}}],"warnings":[]}}"#,
            e
        )
    })
}
