//! WASM bindings for OmniLang compiler.
//! Compiled only when the `wasm` feature is enabled.

use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

use crate::{compile, compile_omniview};

#[derive(Debug, Serialize, Deserialize, Default)]
struct CompileOptions {
    #[serde(default)]
    editor_mode: bool,
}

/// Compile .omni source to OmniView (JS + HTML + CSS). Returns JSON string.
/// options_json: {"editor_mode": true}
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
#[wasm_bindgen]
pub fn compile_omniview_component(
    source: &str,
    component_name: &str,
    options_json: &str,
) -> String {
    let opts: CompileOptions = serde_json::from_str(options_json).unwrap_or_default();
    let result = compile_omniview(source, opts.editor_mode);

    let component = result
        .components
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
#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Compile .omni source to IR JSON. Returns JSON string.
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
