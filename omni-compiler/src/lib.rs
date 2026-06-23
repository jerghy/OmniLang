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
pub mod ir;
pub mod lexer;
pub mod parser;
pub mod semantic;
pub mod sourcemap;
pub mod stdlib;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[cfg(feature = "wasm")]
use wasm_bindgen::prelude::*;

use crate::codegen::omniview::ComponentOutput;
use crate::parser::ast::{ImportStmt, Program, Visibility};

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
// WASM Bindings
// ═══════════════════════════════════════════════════════

#[cfg(feature = "wasm")]
#[derive(Debug, Serialize, Deserialize, Default)]
struct CompileOptions {
    #[serde(default)]
    editor_mode: bool,
}

/// Compile .omni source to OmniView (JS + HTML + CSS). Returns JSON string.
/// options_json: {"editor_mode": true}
#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn compile_to_omniview(source: &str, options_json: &str) -> String {
    let opts: CompileOptions = serde_json::from_str(options_json).unwrap_or_default();
    let result = compile_omniview(source, opts.editor_mode);
    serde_json::to_string(&result).unwrap_or_else(|e| {
        serde_json::json!({
            "success": false,
            "files": null,
            "components": null,
            "source_map": null,
            "errors": [{ "message": e.to_string() }]
        })
        .to_string()
    })
}

/// Compile a single .omni component to OmniView output. Returns JSON string.
/// The source should contain the full .omni file; component_name selects which component to extract.
/// options_json: {"editor_mode": true}
#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn compile_omniview_component(source: &str, component_name: &str, options_json: &str) -> String {
    let opts: CompileOptions = serde_json::from_str(options_json).unwrap_or_default();
    let result = compile_omniview(source, opts.editor_mode);

    let component = result.components
        .as_ref()
        .and_then(|comps| comps.get(component_name).cloned());

    serde_json::json!({
        "success": result.success && component.is_some(),
        "component": component,
        "errors": result.errors,
    })
    .to_string()
}

/// Return compiler version.
#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Compile .omni source to IR JSON. Returns JSON string.
#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn compile_to_ir(source: &str) -> String {
    let result = compile(source);
    serde_json::to_string(&result).unwrap_or_else(|e| {
        serde_json::json!({
            "success": false,
            "ir": null,
            "errors": [{ "message": e.to_string() }]
        })
        .to_string()
    })
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

// ═══════════════════════════════════════════════════════
// Multi-File Validation
// ═══════════════════════════════════════════════════════

/// Result of multi-file validation.
#[derive(Debug, Serialize, Deserialize)]
pub struct MultiFileResult {
    pub success: bool,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub errors: Vec<String>,
}

/// Validate multiple .omni source files with cross-file checks.
///
/// Python does all file I/O and path resolution, then passes:
///   - `sources`: HashMap<ResolvedAbsolutePath, SourceText>
///   - `path_mapping`: HashMap<OriginalImportPath, ResolvedAbsolutePath>
///
/// The compiler parses all files, resolves cross-file imports using the mapping,
/// validates visibility and cycles. Codegen is done separately in Python.
pub fn validate_multi(
    sources: HashMap<String, String>,
    path_mapping: HashMap<String, String>,
) -> MultiFileResult {
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
        return MultiFileResult {
            success: false,
            errors,
        };
    }

    // Phase 2: Per-file consistency checks (omniview double-check)
    for (path, program) in &programs {
        if let Err(file_errors) = validate_file_consistency(path, program) {
            errors.extend(file_errors);
        }
    }

    // Phase 3: Cross-file import validation
    if let Err(more_errors) = validate_cross_file(&programs, &path_mapping) {
        errors.extend(more_errors);
    }

    // Phase 4: Cycle detection
    let cycles = semantic::cycles::detect_cycles(&programs, &path_mapping);
    for cycle in &cycles {
        errors.push(format!("circular import detected: {}", cycle.join(" → ")));
    }

    MultiFileResult {
        success: errors.is_empty(),
        errors,
    }
}

/// Check per-file consistency: omniview marker must match file contents.
fn validate_file_consistency(file_path: &str, program: &Program) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();

    let has_omniview_marker = program.modules.values().any(|module| {
        module
            .imports
            .iter()
            .any(|imp| matches!(imp, ImportStmt::Lib(name) if name == "omniview"))
    });

    let has_ui_declarations = program.modules.values().any(|module| {
        !module.components.is_empty() || !module.stores.is_empty() || !module.themes.is_empty()
    });

    if !has_omniview_marker && has_ui_declarations {
        errors.push(format!(
            "{}: has UI declarations (component/store/theme) but missing 'import omniview;'. \
             Add 'import omniview;' to mark this as a view file.",
            file_path
        ));
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)]
enum CrossFileSymbolKind {
    Function,
    Struct,
    TypeAlias,
    Component,
    Store,
    Theme, // reserved for future theme support
    ReExport(String),
}

/// Build pub-visible symbol tables for cross-file validation.
fn build_pub_symbols(
    programs: &HashMap<String, Program>,
) -> HashMap<String, HashMap<String, CrossFileSymbolKind>> {
    let mut pub_symbols: HashMap<String, HashMap<String, CrossFileSymbolKind>> = HashMap::new();

    for (path, program) in programs {
        let mut symbols: HashMap<String, CrossFileSymbolKind> = HashMap::new();

        for module in program.modules.values() {
            for func in &module.functions {
                if func.visibility == Visibility::Public {
                    symbols.insert(func.name.clone(), CrossFileSymbolKind::Function);
                }
            }
            for strukt in &module.structs {
                if strukt.visibility == Visibility::Public {
                    symbols.insert(strukt.name.clone(), CrossFileSymbolKind::Struct);
                }
            }
            for ta in &module.type_aliases {
                if ta.visibility == Visibility::Public {
                    symbols.insert(ta.name.clone(), CrossFileSymbolKind::TypeAlias);
                }
            }
            for comp in &module.components {
                symbols.insert(comp.name.clone(), CrossFileSymbolKind::Component);
            }
            for store in &module.stores {
                symbols.insert(store.name.clone(), CrossFileSymbolKind::Store);
            }
            // Re-exports
            for exp in &module.exports {
                for name in &exp.names {
                    symbols.insert(
                        name.clone(),
                        CrossFileSymbolKind::ReExport(exp.path.clone()),
                    );
                }
            }
        }

        pub_symbols.insert(path.clone(), symbols);
    }

    pub_symbols
}

fn validate_cross_file(
    programs: &HashMap<String, Program>,
    path_mapping: &HashMap<String, String>,
) -> Result<(), Vec<String>> {
    let mut errors: Vec<String> = Vec::new();
    let pub_symbols = build_pub_symbols(programs);

    for (file_path, program) in programs {
        for module in program.modules.values() {
            for import in &module.imports {
                match import {
                    ImportStmt::Named(named) => {
                        // stdlib imports (std/http, std/json, etc.) are resolved
                        // by the IR/stdlib layer, not via file path mapping
                        if named.path.starts_with("std/") {
                            continue;
                        }
                        if let Some(target) = path_mapping.get(&named.path) {
                            errors.extend(validate_named_import(
                                file_path,
                                &named.names,
                                target,
                                &pub_symbols,
                            ));
                        } else {
                            errors.push(format!(
                                "{}: could not resolve import path '{}'",
                                file_path, named.path
                            ));
                        }
                    }
                    ImportStmt::Namespace(ns) => {
                        if ns.path.starts_with("std/") {
                            continue;
                        }
                        if let Some(target) = path_mapping.get(&ns.path) {
                            if !programs.contains_key(target.as_str()) {
                                errors.push(format!(
                                    "{}: imported file '{}' (resolved to '{}') not found in compilation",
                                    file_path, ns.path, target
                                ));
                            }
                        } else {
                            errors.push(format!(
                                "{}: could not resolve import path '{}'",
                                file_path, ns.path
                            ));
                        }
                    }
                    ImportStmt::TypeOnly(ti) => {
                        if ti.path.starts_with("std/") {
                            continue;
                        }
                        if let Some(target) = path_mapping.get(&ti.path) {
                            errors.extend(validate_named_import(
                                file_path,
                                &ti.names,
                                target,
                                &pub_symbols,
                            ));
                        } else {
                            errors.push(format!(
                                "{}: could not resolve import path '{}'",
                                file_path, ti.path
                            ));
                        }
                    }
                    ImportStmt::Lib(_) => {
                        // 'import omniview;' — validated in validate_file_consistency
                    }
                }
            }
        }
    }

    // Enforce logic→view boundary
    for (file_path, program) in programs {
        for module in program.modules.values() {
            let is_logic = !module
                .imports
                .iter()
                .any(|imp| matches!(imp, ImportStmt::Lib(name) if name == "omniview"));
            if !is_logic {
                continue;
            }
            for import in &module.imports {
                let target_path: Option<&String> = match import {
                    ImportStmt::Named(n) => path_mapping.get(&n.path),
                    ImportStmt::Namespace(ns) => path_mapping.get(&ns.path),
                    ImportStmt::TypeOnly(t) => path_mapping.get(&t.path),
                    ImportStmt::Lib(_) => None,
                };
                if let Some(target) = target_path {
                    if let Some(target_prog) = programs.get(target.as_str()) {
                        let target_is_view = target_prog.modules.values().any(|tm| {
                            tm.imports.iter().any(
                                |imp| matches!(imp, ImportStmt::Lib(name) if name == "omniview"),
                            )
                        });
                        if target_is_view {
                            errors.push(format!(
                                "{}: logic file cannot import view file '{}'",
                                file_path, target
                            ));
                        }
                    }
                }
            }
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

fn validate_named_import(
    importing_file: &str,
    names: &[String],
    target_path: &str,
    pub_symbols: &HashMap<String, HashMap<String, CrossFileSymbolKind>>,
) -> Vec<String> {
    let mut errors = Vec::new();

    let target_symbols = match pub_symbols.get(target_path) {
        Some(s) => s,
        None => {
            errors.push(format!(
                "{}: imported file '{}' not found in compilation",
                importing_file, target_path
            ));
            return errors;
        }
    };

    for name in names {
        if !target_symbols.contains_key(name) {
            errors.push(format!(
                "{}: '{}' is not pub-visible in '{}'",
                importing_file, name, target_path
            ));
        }
    }

    errors
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
    let validation_result = validate_multi(sources.clone(), path_mapping);
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

        let ir_program = ir::lower(program, &semantics);

        // Merge modules — combine declarations from multiple files
        for (mod_name, mod_ir) in ir_program.modules {
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

// ═══════════════════════════════════════════════════════
// Native FFI (for Python ctypes/cffi)
// ═══════════════════════════════════════════════════════

use std::ffi::CString;
use std::os::raw::c_char;

fn rust_to_c_string(s: String) -> *mut c_char {
    // If the string contains an interior NUL byte, replace it
    // with a placeholder rather than silently returning "{}"
    let sanitized = s.replace('\0', "\\0");
    CString::new(sanitized)
        .unwrap_or_else(|_| CString::new("{\"error\":\"serialization failed\"}").unwrap())
        .into_raw()
}

/// Free a string previously returned by an omni_* function.
/// # Safety
/// `ptr` must be a valid pointer previously returned by an omni_* function, or null.
#[no_mangle]
pub unsafe extern "C" fn omni_free_string(ptr: *mut c_char) {
    if ptr.is_null() {
        return;
    }
    // SAFETY: `ptr` is non-null (checked above) and was produced by
    // CString::into_raw() from a prior omni_* call. We take ownership
    // back via from_raw() so the CString is dropped and its memory freed.
    unsafe {
        let _ = CString::from_raw(ptr);
    }
}

/// Return compiler version as C string (caller must free).
#[no_mangle]
pub extern "C" fn omni_version() -> *mut c_char {
    rust_to_c_string(env!("CARGO_PKG_VERSION").to_string())
}

/// Analyze multiple .omni sources and return diagnostics (errors, warnings, lint results).
///
/// Takes JSON: `{ "sources": { "path": "source", ... }, "path_mapping": { "orig": "resolved", ... } }`
/// Returns JSON: `{ "success": bool, "diagnostics": [...] }`
///
/// # Safety
/// `request_json_ptr` must be valid for `request_json_len` bytes, or null.
#[no_mangle]
pub unsafe extern "C" fn omni_analyse(
    request_json_ptr: *const u8,
    request_json_len: u32,
) -> *mut c_char {
    let request_str = if request_json_ptr.is_null() {
        return rust_to_c_string(
            r#"{"success":false,"diagnostics":[{"message":"null pointer","severity":"error"}]}"#
                .into(),
        );
    } else {
        // SAFETY: `request_json_ptr` is non-null (checked above) and the
        // caller guarantees it points to `request_json_len` bytes of valid
        // UTF-8 data for the duration of this call.
        let slice =
            unsafe { std::slice::from_raw_parts(request_json_ptr, request_json_len as usize) };
        std::str::from_utf8(slice).unwrap_or("{}")
    };

    #[derive(Deserialize)]
    struct AnalyseRequest {
        sources: HashMap<String, String>,
        #[serde(default)]
        #[allow(dead_code)]
        path_mapping: HashMap<String, String>,
    }

    let input: AnalyseRequest = match serde_json::from_str(request_str) {
        Ok(req) => req,
        Err(e) => {
            let err_json = serde_json::json!({
                "diagnostics": [{
                    "severity": "Error",
                    "code": "E0000",
                    "message": format!("Failed to parse request JSON: {}", e),
                    "line": 0,
                    "col": 0
                }],
                "success": false
            });
            return rust_to_c_string(err_json.to_string());
        }
    };

    let mut diagnostics: Vec<serde_json::Value> = Vec::new();
    let mut programs: HashMap<String, Program> = HashMap::new();

    // Phase 1: Parse all sources
    for (path, source) in &input.sources {
        let tokens = crate::lexer::tokenize(source);
        match crate::parser::parse(&tokens, source) {
            Ok(program) => {
                programs.insert(path.clone(), program);
            }
            Err(e) => {
                diagnostics.push(serde_json::json!({
                    "severity": "error",
                    "code": "PARSE_ERROR",
                    "message": e.message,
                    "file": path,
                    "line": e.span.line,
                    "col": e.span.col,
                    "suggestion": null,
                }));
            }
        }
    }

    // Phase 2: Semantic analysis + IR lowering
    let mut merged_ir = ir::IRProgram {
        version: "1.0".to_string(),
        modules: HashMap::new(),
        source_map: HashMap::new(),
        stdlib_imports: Vec::new(),
    };

    for (path, program) in &programs {
        let semantics = crate::semantic::analyze(program);
        for diag in &semantics.diagnostics {
            diagnostics.push(serde_json::json!({
                "severity": match diag.severity {
                    crate::error::Severity::Error => "error",
                    crate::error::Severity::Warning => "warning",
                    crate::error::Severity::Info => "info",
                },
                "code": diag.code,
                "message": diag.message,
                "file": path,
                "line": diag.line,
                "col": diag.col,
                "suggestion": diag.suggestion,
            }));
        }
        let ir_program = ir::lower(program, &semantics);
        for (mod_name, mod_ir) in ir_program.modules {
            merged_ir.modules.entry(mod_name).or_insert(mod_ir);
        }
        for (key, entry) in ir_program.source_map {
            merged_ir.source_map.entry(key).or_insert(entry);
        }
    }

    // Phase 3: Run lint rules
    let lint_diags = crate::semantic::lints::run_all(&programs, &merged_ir);
    for lint in &lint_diags {
        diagnostics.push(serde_json::json!({
            "severity": match lint.severity {
                crate::semantic::lints::LintSeverity::Error => "error",
                crate::semantic::lints::LintSeverity::Warning => "warning",
                crate::semantic::lints::LintSeverity::Info => "info",
            },
            "code": lint.rule,
            "message": lint.message,
            "file": lint.file,
            "line": lint.line,
            "col": lint.col,
            "suggestion": null,
        }));
    }

    let response = serde_json::json!({
        "success": diagnostics.iter().all(|d| d["severity"] != "error"),
        "diagnostics": diagnostics,
    });

    rust_to_c_string(response.to_string())
}

/// Compile multiple .omni sources to a target language via C FFI.
///
/// # Parameters (ptr + len convention for all string inputs)
/// - `sources_json_ptr/len`: JSON object mapping file paths to source text
/// - `target_ptr/len`: target language string (e.g. "python", "js", "rust", "dart")
/// - `path_mapping_ptr/len`: JSON object mapping original import paths to resolved paths
///
/// Returns a JSON string (caller must free with omni_free_string).
/// # Safety
/// All pointers must be valid for the given length, or null.
#[no_mangle]
pub unsafe extern "C" fn omni_compile_multi_to_target(
    sources_json_ptr: *const u8,
    sources_json_len: u32,
    target_ptr: *const u8,
    target_len: u32,
    path_mapping_ptr: *const u8,
    path_mapping_len: u32,
) -> *mut c_char {
    // Convert ptr+len to string slices
    let sources_str = if sources_json_ptr.is_null() {
        return rust_to_c_string(
            r#"{"success":false,"files":{},"errors":["null sources pointer"]}"#.into(),
        );
    } else {
        // SAFETY: `sources_json_ptr` is non-null (checked above) and the
        // caller guarantees it points to `sources_json_len` bytes of valid
        // UTF-8 data for the duration of this call.
        let slice =
            unsafe { std::slice::from_raw_parts(sources_json_ptr, sources_json_len as usize) };
        std::str::from_utf8(slice).unwrap_or("{}")
    };

    let target_str = if target_ptr.is_null() {
        return rust_to_c_string(
            r#"{"success":false,"files":{},"errors":["null target pointer"]}"#.into(),
        );
    } else {
        // SAFETY: `target_ptr` is non-null (checked above) and the caller
        // guarantees it points to `target_len` bytes of valid UTF-8 data.
        let slice = unsafe { std::slice::from_raw_parts(target_ptr, target_len as usize) };
        std::str::from_utf8(slice).unwrap_or("")
    };

    let path_mapping_str = if path_mapping_ptr.is_null() {
        "{}"
    } else {
        // SAFETY: `path_mapping_ptr` is non-null (implicit from else branch)
        // and the caller guarantees it points to `path_mapping_len` bytes.
        let slice =
            unsafe { std::slice::from_raw_parts(path_mapping_ptr, path_mapping_len as usize) };
        std::str::from_utf8(slice).unwrap_or("{}")
    };

    // Parse sources
    let sources: HashMap<String, String> = match serde_json::from_str(sources_str) {
        Ok(s) => s,
        Err(e) => {
            return rust_to_c_string(
                serde_json::json!({
                    "success": false,
                    "files": {},
                    "errors": [format!("Failed to parse sources JSON: {}", e)]
                })
                .to_string(),
            );
        }
    };

    // Parse target
    let target: codegen::Target = match target_str.parse() {
        Ok(t) => t,
        Err(e) => {
            return rust_to_c_string(
                serde_json::to_string(&serde_json::json!({
                    "success": false,
                    "files": {},
                    "errors": [e]
                }))
                .unwrap_or_else(|_| {
                    r#"{"success":false,"files":{},"errors":["serialization error"]}"#.into()
                }),
            );
        }
    };

    // Parse path_mapping (optional — defaults to empty on parse failure)
    let path_mapping: HashMap<String, String> = serde_json::from_str(path_mapping_str)
        .unwrap_or_else(|e| {
            // Log a diagnostic but continue — path_mapping is non-critical
            eprintln!("[omni] warning: failed to parse path_mapping JSON: {}", e);
            HashMap::new()
        });

    let result = compile_multi_to_target(sources, target, path_mapping);
    rust_to_c_string(result)
}
