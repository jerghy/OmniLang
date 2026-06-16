//! OmniLang compiler — .omni source -> Intent IR -> JSON / OmniView codegen.
//!
//! Pipeline:
//!   1. Lexer  → token stream
//!   2. Parser → AST
//!   3. Semantic analysis → validated AST + symbol table + type map
//!   4. IR lowering → Intent IR
//!   5. Codegen / JSON serialization

pub mod lexer;
pub mod parser;
pub mod semantic;
pub mod ir;
pub mod codegen;
pub mod sourcemap;
pub mod error;

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

    // 3. Semantic analysis
    let semantics = match semantic::analyze(&program) {
        Ok(s) => s,
        Err(diags) => {
            errors.extend(diags);
            return CompileResult {
                success: false,
                ir: None,
                errors,
            };
        }
    };

    // 4. Lower to IR
    let ir_program = ir::lower(&program, &semantics);

    CompileResult {
        success: true,
        ir: Some(ir_program),
        errors,
    }
}

/// Return compiler version.
#[cfg(feature = "wasm")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}
