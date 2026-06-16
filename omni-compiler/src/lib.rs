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
// Native FFI (for Python ctypes/cffi)
// ═══════════════════════════════════════════════════════

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
