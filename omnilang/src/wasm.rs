//! WASM bindings for OmniLang compiler.
//! Compiled only when the `wasm` feature is enabled.

use std::collections::HashMap;

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

/// Compile .omni source to OmniView output in a single pass (IR + codegen).
/// Returns JSON with `ir`, `files`, `components`, `source_map`, and `errors`.
/// options_json: {"editor_mode": true}
#[wasm_bindgen]
pub fn compile_to_omniview_full(source: &str, options_json: &str) -> String {
    let opts: CompileOptions = serde_json::from_str(options_json).unwrap_or_default();
    let result = crate::compile_omniview_full(source, opts.editor_mode);
    serde_json::to_string(&result).unwrap_or_else(|e| {
        serde_json::json!({
            "success": false,
            "ir": null,
            "files": null,
            "components": null,
            "source_map": null,
            "errors": [{ "message": e.to_string() }]
        })
        .to_string()
    })
}

/// Compile multiple .omni source files to OmniView.
/// sources_json: {"path": "source content", ...}
/// Returns JSON with `success`, `files`, and `errors`.
#[wasm_bindgen]
pub fn compile_multi_to_omniview(sources_json: &str, options_json: &str) -> String {
    let _opts: CompileOptions = serde_json::from_str(options_json).unwrap_or_default();
    let sources: HashMap<String, String> = match serde_json::from_str(sources_json) {
        Ok(s) => s,
        Err(e) => {
            return serde_json::json!({
                "success": false,
                "files": {},
                "errors": [format!("Failed to parse sources JSON: {}", e)]
            })
            .to_string();
        }
    };
    crate::compile_multi_to_target(sources, crate::codegen::Target::OmniView, HashMap::new())
}

/// Stateful compiler instance that caches sources and compilation results across calls.
#[wasm_bindgen]
pub struct OmniCompiler {
    sources: HashMap<String, String>,
    cached_compile_result: Option<(String, String)>,  // (options_json, result)
    cached_diags_result: Option<String>,
}

#[wasm_bindgen]
impl OmniCompiler {
    /// Create a new OmniCompiler with no source files registered.
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            sources: HashMap::new(),
            cached_compile_result: None,
            cached_diags_result: None,
        }
    }

    /// Add or update a source file. Invalidates cached compilation results.
    pub fn upsert_source(&mut self, path: String, source: String) {
        self.sources.insert(path, source);
        self.cached_compile_result = None;
        self.cached_diags_result = None;
    }

    /// Remove a source file. Invalidates cached compilation results.
    pub fn remove_source(&mut self, path: String) {
        self.sources.remove(&path);
        self.cached_compile_result = None;
        self.cached_diags_result = None;
    }

    /// Compile all registered sources to OmniView. Uses cached result when available.
    /// options_json: {"editor_mode": true}
    pub fn compile_to_omniview(&mut self, options_json: &str) -> String {
        if self.sources.is_empty() {
            return serde_json::json!({
                "success": false,
                "files": null,
                "components": null,
                "source_map": null,
                "errors": [{"message": "no source files added to compiler"}]
            })
            .to_string();
        }
        if let Some((ref cached_opts, ref cached)) = self.cached_compile_result {
            if cached_opts == options_json {
                return cached.clone();
            }
        }
        let result = if self.sources.len() == 1 {
            // Single file: use compile_omniview_full for full result with IR
            let source = self.sources.values().next().unwrap();
            let opts: CompileOptions = serde_json::from_str(options_json).unwrap_or_default();
            let full = crate::compile_omniview_full(source, opts.editor_mode);
            serde_json::to_string(&full).unwrap_or_else(|e| {
                serde_json::json!({
                    "success": false,
                    "files": null,
                    "errors": [{"message": e.to_string()}]
                })
                .to_string()
            })
        } else {
            // Multi-file: use compile_multi_to_target (returns JSON string).
            // NOTE: editor_mode is not yet supported for multi-file compilation
            // because compile_multi_to_target does not accept an editor_mode flag.
            // TODO(wasm): extend compile_multi_to_target to support editor_mode.
            crate::compile_multi_to_target(
                self.sources.clone(),
                crate::codegen::Target::OmniView,
                HashMap::new(),
            )
        };
        self.cached_compile_result = Some((options_json.to_string(), result.clone()));
        self.cached_diags_result = None;
        result
    }

    /// Run diagnostics on all registered sources without compiling to output.
    /// Returns JSON with `success` and `errors` arrays.
    pub fn diagnostics(&mut self) -> String {
        if let Some(ref cached) = self.cached_diags_result {
            return cached.clone();
        }
        let mut all_errors: Vec<serde_json::Value> = Vec::new();
        let mut all_success = true;
        for source in self.sources.values() {
            let result = crate::compile(source);
            if !result.success {
                all_success = false;
            }
            for err in &result.errors {
                all_errors.push(serde_json::to_value(err).unwrap_or_default());
            }
        }
        let result = serde_json::json!({
            "success": all_success,
            "errors": all_errors,
        })
        .to_string();
        self.cached_diags_result = Some(result.clone());
        result
    }

    /// Return the compiler version string.
    pub fn version(&self) -> String {
        env!("CARGO_PKG_VERSION").to_string()
    }
}
