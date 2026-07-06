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
use std::collections::HashMap;

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
    let program = parser::parse(&tokens, source).map_err(|e| vec![e.into()])?;
    let semantics = semantic::analyze(&program);
    let has_errors = semantics
        .diagnostics
        .iter()
        .any(|d| matches!(d.severity, crate::error::Severity::Error));
    let ir = ir::lower(&program, &semantics);
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
#[derive(Debug, Serialize, Deserialize)]
pub struct OmniViewResult {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub files: Option<OmniViewFiles>,
    /// Per-component output for hot-replace (keyed by component name)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub components: Option<HashMap<String, ComponentOutput>>,
    #[serde(rename = "sourceMap")]
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
#[derive(Debug, Serialize, Deserialize)]
pub struct OmniViewFullResult {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ir: Option<ir::IRProgram>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub files: Option<OmniViewFiles>,
    /// Per-component output for hot-replace (keyed by component name)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub components: Option<HashMap<String, ComponentOutput>>,
    #[serde(rename = "sourceMap")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_map: Option<Vec<sourcemap::SourceMapEntry>>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub errors: Vec<error::Diagnostic>,
}

/// Compile .omni -> OmniView + IR in one pass.
/// Eliminates the double lex->parse->semantic->IR when both IR and codegen are needed.
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

/// Extract the inner HTML between `<body>` and `</body>` tags.
/// Works on both multi-line and minified/single-line HTML.
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

/// Compile multiple .omni sources to a target language.
///
/// Pipeline: parse all sources → cross-file validation → per-file semantic analysis
/// → IR lowering → merged IR → target codegen.
///
/// Returns JSON with structured Diagnostic errors (severity, code, file, line, col, …)
/// plus a flat `warnings` array from lint rules.
pub fn compile_multi_to_target(
    sources: HashMap<String, String>,
    target: codegen::Target,
    path_mapping: HashMap<String, String>,
    editor_mode: bool,
) -> String {
    // Run the unified analysis pipeline (parse → validate → semantic → lint)
    let analysis = crate::analyse::analyse_multi(&sources, &path_mapping);
    let mut diags = analysis.diagnostics;
    let warnings = analysis.warnings;

    let merged_ir = match analysis.ir {
        Some(ir) => ir,
        None => {
            // Parse failed — return errors early
            return serde_json::to_string(&serde_json::json!({
                "success": false,
                "files": {},
                "errors": diags,
                "warnings": [],
            }))
            .unwrap_or_else(|e| {
                format!(
                    r#"{{"success":false,"files":{{}},"errors":[{{"severity":"error","code":"E_SERIALIZE","message":"{}","file":null,"line":0,"col":0,"end_line":0,"end_col":0,"suggestion":null}}],"warnings":[]}}"#,
                    e
                )
            });
        }
    };

    // Phase 5: Dispatch to target codegen
    let output = codegen::compile(&merged_ir, target, &sources);
    let mut files_map: HashMap<String, String> = output
        .files
        .into_iter()
        .map(|f| (f.path, f.content))
        .collect();

    // Convert codegen errors (plain strings) to Diagnostic objects
    let codegen_diags: Vec<error::Diagnostic> = output
        .errors
        .into_iter()
        .map(|msg| error::Diagnostic {
            severity: error::Severity::Error,
            code: "E_CODEGEN".to_string(),
            message: msg,
            file: None,
            line: 0,
            col: 0,
            end_line: 0,
            end_col: 0,
            suggestion: None,
        })
        .collect();
    diags.extend(codegen_diags);

    // Post-process: wrap debug HTML for editor mode
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
    serde_json::to_string(&serde_json::json!({
        "success": success,
        "files": files_map,
        "errors": diags,
        "warnings": warnings,
    }))
    .unwrap_or_else(|e| {
        format!(
            r#"{{"success":false,"files":{{}},"errors":[{{"severity":"error","code":"E_SERIALIZE","message":"{}","file":null,"line":0,"col":0,"end_line":0,"end_col":0,"suggestion":null}}],"warnings":[]}}"#,
            e
        )
    })
}
