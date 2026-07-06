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
pub mod path_mapping;
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
use crate::parser::ast::Program;

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
    let mut diags: Vec<error::Diagnostic> = Vec::new();
    let mut programs: HashMap<String, Program> = HashMap::new();

    // Phase 1: Parse all sources
    for (path, source) in &sources {
        let tokens = lexer::tokenize(source);
        match parser::parse(&tokens, source) {
            Ok(program) => {
                programs.insert(path.clone(), program);
            }
            Err(e) => {
                let d: error::Diagnostic = e.into();
                diags.push(d.with_file(path));
            }
        }
    }

    if !diags.is_empty() {
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

    // Phase 2: Cross-file validation
    let validation_result = validate::validate_multi(sources.clone(), path_mapping);
    diags.extend(validation_result.errors);

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
            let mut d = d.clone();
            if d.file.is_none() {
                d.file = Some(path.clone());
            }
            diags.push(d);
        }

        let ir_program = ir::lower(program, &semantics);

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
    let warnings: Vec<error::Diagnostic> = lint_diags
        .iter()
        .filter(|d| {
            matches!(
                d.severity,
                crate::semantic::lints::LintSeverity::Warning
                    | crate::semantic::lints::LintSeverity::Info
            )
        })
        .map(|d| error::Diagnostic {
            severity: match d.severity {
                crate::semantic::lints::LintSeverity::Warning => error::Severity::Warning,
                crate::semantic::lints::LintSeverity::Info => error::Severity::Info,
                _ => error::Severity::Warning,
            },
            code: d.rule.clone(),
            message: d.message.clone(),
            file: Some(d.file.clone()),
            line: d.line,
            col: d.col,
            end_line: d.line,
            end_col: d.col.saturating_add(d.message.len() as u32),
            suggestion: None,
        })
        .collect();

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
