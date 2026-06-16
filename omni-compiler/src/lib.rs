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

use serde::{Deserialize, Serialize};

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

/// Compile .omni source to IR JSON. Returns JSON string.
/// This is the WASM entry point and also used by native FFI.
#[cfg(feature = "wasm")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn compile_to_ir(source: &str) -> String {
    let result = compile(source);
    serde_json::to_string(&result).unwrap_or_else(|e| {
        format!(
            r#"{{"success":false,"ir":null,"errors":[{{"message":"{}"}}]}}"#,
            e
        )
    })
}

/// Compile .omni source -> IR. Core function shared by WASM and native.
pub fn compile(source: &str) -> CompileResult {
    let mut errors: Vec<error::Diagnostic> = Vec::new();

    // 1. Lex
    let tokens = lexer::tokenize(source);

    // 2. Parse
    let program = match parser::parse(&tokens) {
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

    // 3. Semantic analysis — continue even with diagnostics (IR is still useful)
    let semantics = match semantic::analyze(&program) {
        Ok(s) => s,
        Err(diags) => {
            errors.extend(diags);
            // Still lower to IR — semantic issues shouldn't block codegen
            semantic::SemanticResult {
                symbols: semantic::symbols::SymbolTable::new(),
                dependencies: std::collections::HashMap::new(),
            }
        }
    };

    // 4. Lower to IR
    let ir_program = ir::lower(&program, &semantics);

    CompileResult {
        success: errors.is_empty(),  // success=true only if no errors at all
        ir: Some(ir_program),       // always return IR (even with warnings)
        errors,
    }
}

/// Return compiler version.
#[cfg(feature = "wasm")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
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
pub fn compile_omniview(source: &str) -> OmniViewResult {
    let mut errors: Vec<error::Diagnostic> = Vec::new();

    // 1. Lex
    let tokens = lexer::tokenize(source);

    // 2. Parse
    let program = match parser::parse(&tokens) {
        Ok(p) => p,
        Err(e) => {
            errors.push(e.into());
            return OmniViewResult {
                success: false,
                files: None,
                source_map: None,
                errors,
            };
        }
    };

    // 3. Semantic analysis
    let semantics = match semantic::analyze(&program) {
        Ok(s) => s,
        Err(diags) => {
            errors.extend(diags);
            return OmniViewResult {
                success: false,
                files: None,
                source_map: None,
                errors,
            };
        }
    };

    // 4. Lower to IR
    let ir_program = ir::lower(&program, &semantics);

    // 5. Generate OmniView output
    let output = codegen::omniview::generate(&ir_program);

    OmniViewResult {
        success: true,
        files: Some(OmniViewFiles {
            js: output.js,
            html: output.html,
            css: output.css,
        }),
        source_map: Some(output.source_map),
        errors,
    }
}

/// Compile .omni source to OmniView (JS + HTML + CSS). Returns JSON string.
#[cfg(feature = "wasm")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn compile_to_omniview(source: &str) -> String {
    let result = compile_omniview(source);
    serde_json::to_string(&result).unwrap_or_else(|e| {
        format!(
            r#"{{"success":false,"files":null,"source_map":null,"errors":[{{"message":"{}"}}]}}"#,
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
        match parser::parse(&tokens) {
            Ok(program) => {
                programs.insert(path.clone(), program);
            }
            Err(e) => {
                errors.push(format!("{}: parse error: {}", path, e.message));
            }
        }
    }

    if !errors.is_empty() {
        return MultiFileResult { success: false, errors };
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
        errors.push(format!(
            "circular import detected: {}",
            cycle.join(" → ")
        ));
    }

    MultiFileResult { success: errors.is_empty(), errors }
}

/// Check per-file consistency: omniview marker must match file contents.
fn validate_file_consistency(
    file_path: &str,
    program: &Program,
) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();

    let has_omniview_marker = program.imports.iter().any(|imp| {
        matches!(imp, ImportStmt::Lib(name) if name == "omniview")
    });

    let has_ui_declarations = !program.components.is_empty()
        || !program.stores.is_empty()
        || !program.themes.is_empty();

    if has_omniview_marker && !has_ui_declarations {
        errors.push(format!(
            "{}: has 'import omniview;' but no component/store/theme declarations. \
             Either add UI declarations or remove the omniview import.",
            file_path
        ));
    }

    if !has_omniview_marker && has_ui_declarations {
        errors.push(format!(
            "{}: has UI declarations (component/store/theme) but missing 'import omniview;'. \
             Add 'import omniview;' to mark this as a view file.",
            file_path
        ));
    }

    if errors.is_empty() { Ok(()) } else { Err(errors) }
}

#[derive(Debug, Clone, PartialEq)]
enum CrossFileSymbolKind {
    Function,
    Struct,
    TypeAlias,
    Component,
    Store,
    Theme,
    ReExport(String),
}

/// Build pub-visible symbol tables for cross-file validation.
fn build_pub_symbols(
    programs: &HashMap<String, Program>,
) -> HashMap<String, HashMap<String, CrossFileSymbolKind>> {
    let mut pub_symbols: HashMap<String, HashMap<String, CrossFileSymbolKind>> = HashMap::new();

    for (path, program) in programs {
        let mut symbols: HashMap<String, CrossFileSymbolKind> = HashMap::new();

        for func in &program.functions {
            if func.visibility == Visibility::Public {
                symbols.insert(func.name.clone(), CrossFileSymbolKind::Function);
            }
        }
        for strukt in &program.structs {
            if strukt.visibility == Visibility::Public {
                symbols.insert(strukt.name.clone(), CrossFileSymbolKind::Struct);
            }
        }
        for ta in &program.type_aliases {
            if ta.visibility == Visibility::Public {
                symbols.insert(ta.name.clone(), CrossFileSymbolKind::TypeAlias);
            }
        }
        for comp in &program.components {
            symbols.insert(comp.name.clone(), CrossFileSymbolKind::Component);
        }
        for store in &program.stores {
            symbols.insert(store.name.clone(), CrossFileSymbolKind::Store);
        }
        // Re-exports
        for exp in &program.exports {
            for name in &exp.names {
                symbols.insert(name.clone(), CrossFileSymbolKind::ReExport(exp.path.clone()));
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
        for import in &program.imports {
            match import {
                ImportStmt::Named(named) => {
                    if let Some(target) = path_mapping.get(&named.path) {
                        errors.extend(validate_named_import(
                            file_path, &named.names, target, &pub_symbols,
                        ));
                    } else {
                        errors.push(format!(
                            "{}: could not resolve import path '{}'",
                            file_path, named.path
                        ));
                    }
                }
                ImportStmt::Namespace(ns) => {
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
                    if let Some(target) = path_mapping.get(&ti.path) {
                        errors.extend(validate_named_import(
                            file_path, &ti.names, target, &pub_symbols,
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

    // Enforce logic→view boundary
    for (file_path, program) in programs {
        let is_logic = !program.imports.iter().any(|imp| {
            matches!(imp, ImportStmt::Lib(name) if name == "omniview")
        });
        if !is_logic {
            continue;
        }
        for import in &program.imports {
            let target_path: Option<&String> = match import {
                ImportStmt::Named(n) => path_mapping.get(&n.path),
                ImportStmt::Namespace(ns) => path_mapping.get(&ns.path),
                ImportStmt::TypeOnly(t) => path_mapping.get(&t.path),
                ImportStmt::Lib(_) => None,
            };
            if let Some(target) = target_path {
                if let Some(target_prog) = programs.get(target.as_str()) {
                    let target_is_view = target_prog.imports.iter().any(|imp| {
                        matches!(imp, ImportStmt::Lib(name) if name == "omniview")
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

    if errors.is_empty() { Ok(()) } else { Err(errors) }
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

// ═══════════════════════════════════════════════════════
// Native FFI (for Python ctypes/cffi)
// ═══════════════════════════════════════════════════════

use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

fn rust_to_c_string(s: String) -> *mut c_char {
    CString::new(s)
        .unwrap_or_else(|_| CString::new("{}").unwrap())
        .into_raw()
}

/// Compile .omni source to IR JSON. Returns JSON string (caller must free with omni_free_string).
/// # Safety
/// `source` must be a valid, null-terminated C string or null.
#[no_mangle]
pub unsafe extern "C" fn omni_compile_to_ir(source: *const c_char) -> *mut c_char {
    let source_str = unsafe {
        if source.is_null() {
            return rust_to_c_string(
                r#"{"success":false,"ir":null,"errors":[{"message":"null source pointer"}]}"#
                    .into(),
            );
        }
        CStr::from_ptr(source).to_str().unwrap_or("")
    };
    let result = compile(source_str);
    let json = serde_json::to_string(&result).unwrap_or_else(|e| {
        format!(
            r#"{{"success":false,"ir":null,"errors":[{{"message":"{}"}}]}}"#,
            e
        )
    });
    rust_to_c_string(json)
}

/// Free a string previously returned by omni_compile_to_ir or omni_compile_to_omniview.
/// # Safety
/// `ptr` must be a valid pointer previously returned by an omni_* function, or null.
#[no_mangle]
pub unsafe extern "C" fn omni_free_string(ptr: *mut c_char) {
    if ptr.is_null() {
        return;
    }
    unsafe {
        let _ = CString::from_raw(ptr);
    }
}

/// Return compiler version as C string (caller must free).
#[no_mangle]
pub extern "C" fn omni_version() -> *mut c_char {
    rust_to_c_string(env!("CARGO_PKG_VERSION").to_string())
}

/// Compile .omni source to OmniView (JS + HTML + CSS + SourceMap). Returns JSON string (caller must free).
/// # Safety
/// `source` must be a valid, null-terminated C string or null.
#[no_mangle]
pub unsafe extern "C" fn omni_compile_to_omniview(source: *const c_char) -> *mut c_char {
    let source_str = unsafe {
        if source.is_null() {
            return rust_to_c_string(
                r#"{"success":false,"files":null,"source_map":null,"errors":[{"message":"null source pointer"}]}"#
                    .into(),
            );
        }
        CStr::from_ptr(source).to_str().unwrap_or("")
    };
    let result = compile_omniview(source_str);
    let json = serde_json::to_string(&result).unwrap_or_else(|e| {
        format!(
            r#"{{"success":false,"files":null,"source_map":null,"errors":[{{"message":"{}"}}]}}"#,
            e
        )
    });
    rust_to_c_string(json)
}
