//! WASM bindings for OmniLang compiler.
//! Compiled only when the `wasm` feature is enabled.

use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

use crate::{compile, compile_omniview};

#[derive(Debug, Serialize, Deserialize, Default)]
struct CompileOptions {
    #[serde(default)]
    editor_mode: bool,
}

/// Parse options JSON, returning a structured error JSON string on failure.
fn parse_options_or_error(options_json: &str) -> Result<CompileOptions, String> {
    serde_json::from_str::<CompileOptions>(options_json).map_err(|e| {
        serde_json::json!({
            "success": false,
            "files": null,
            "errors": [{
                "severity": "error",
                "code": "E_INVALID_OPTIONS",
                "message": format!("Failed to parse options JSON: {}", e),
                "line": 0, "col": 0, "end_line": 0, "end_col": 0, "suggestion": null
            }]
        })
        .to_string()
    })
}

/// Compile .omni source to OmniView (JS + HTML + CSS). Returns JSON string.
/// options_json: {"editor_mode": true}
#[wasm_bindgen]
pub fn compile_to_omniview(source: &str, options_json: &str) -> String {
    let opts = match parse_options_or_error(options_json) {
        Ok(o) => o,
        Err(error_json) => return error_json,
    };
    let result = compile_omniview(source, opts.editor_mode);
    serde_json::to_string(&result).unwrap_or_else(|e| {
        serde_json::json!({
            "success": false,
            "files": null,
            "components": null,
            "sourceMap": null,
            "errors": [{
                "severity": "error",
                "code": "E_SERIALIZE",
                "message": e.to_string(),
                "line": 0,
                "col": 0,
                "end_line": 0,
                "end_col": 0,
                "suggestion": null
            }]
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
    let opts = match parse_options_or_error(options_json) {
        Ok(o) => o,
        Err(error_json) => return error_json,
    };
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
            "errors": [{
                "severity": "error",
                "code": "E_SERIALIZE",
                "message": e.to_string(),
                "line": 0,
                "col": 0,
                "end_line": 0,
                "end_col": 0,
                "suggestion": null
            }]
        })
        .to_string()
    })
}

/// Compile .omni source to OmniView output in a single pass (IR + codegen).
/// Returns JSON with `ir`, `files`, `components`, `source_map`, and `errors`.
/// options_json: {"editor_mode": true}
#[wasm_bindgen]
pub fn compile_to_omniview_full(source: &str, options_json: &str) -> String {
    let opts = match parse_options_or_error(options_json) {
        Ok(o) => o,
        Err(error_json) => return error_json,
    };
    let result = crate::compile_omniview_full(source, opts.editor_mode);
    serde_json::to_string(&result).unwrap_or_else(|e| {
        serde_json::json!({
            "success": false,
            "ir": null,
            "files": null,
            "components": null,
            "sourceMap": null,
            "errors": [{
                "severity": "error",
                "code": "E_SERIALIZE",
                "message": e.to_string(),
                "line": 0,
                "col": 0,
                "end_line": 0,
                "end_col": 0,
                "suggestion": null
            }]
        })
        .to_string()
    })
}

/// Compile multiple .omni source files to OmniView.
/// sources_json: {"path": "source content", ...}
/// Returns JSON with `success`, `files`, and `errors`.
#[wasm_bindgen]
pub fn compile_multi_to_omniview(sources_json: &str, options_json: &str) -> String {
    let opts = match parse_options_or_error(options_json) {
        Ok(o) => o,
        Err(error_json) => return error_json,
    };
    let sources: FxHashMap<String, String> = match serde_json::from_str(sources_json) {
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
    crate::compile_multi_to_target(
        sources,
        crate::codegen::Target::OmniView,
        FxHashMap::default(),
        opts.editor_mode,
    )
}

/// Stateful compiler instance that caches sources and compilation results across calls.
#[wasm_bindgen]
pub struct OmniCompiler {
    sources: FxHashMap<String, String>,
    cached_compile_result: Option<(String, String)>, // (options_json, result)
    cached_diags_result: Option<String>,
}

#[wasm_bindgen]
impl OmniCompiler {
    /// Create a new OmniCompiler with no source files registered.
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            sources: FxHashMap::default(),
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
            let source = self.sources.values().next().expect("len checked above");
            let opts = match parse_options_or_error(options_json) {
                Ok(o) => o,
                Err(error_json) => return error_json,
            };
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
            // Multi-file: use compile_multi_to_target with editor_mode support.
            let opts = match parse_options_or_error(options_json) {
                Ok(o) => o,
                Err(error_json) => return error_json,
            };
            crate::compile_multi_to_target(
                self.sources.clone(),
                crate::codegen::Target::OmniView,
                FxHashMap::default(),
                opts.editor_mode,
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
                all_errors.push(err.to_json_value());
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compile_to_omniview_full_success() {
        let result =
            compile_to_omniview_full("component App { view { Text { \"hello\" } } }", "{}");
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert!(parsed["success"].as_bool().unwrap());
        assert!(parsed["files"]["js"].as_str().unwrap().len() > 0);
        assert!(parsed["files"]["html"].as_str().unwrap().len() > 0);
        // Should have IR
        assert!(parsed["ir"].is_object());
    }

    #[test]
    fn test_compile_to_omniview_full_parse_error() {
        let result = compile_to_omniview_full("this is invalid @@@", "{}");
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert!(!parsed["success"].as_bool().unwrap());
        assert!(parsed["ir"].is_null());
        assert!(parsed["files"].is_null());
        assert!(parsed["errors"].as_array().unwrap().len() > 0);
    }

    #[test]
    fn test_compile_to_omniview_full_editor_mode() {
        let result1 = compile_to_omniview_full("component App { view { Text { \"hi\" } } }", "{}");
        let result2 = compile_to_omniview_full(
            "component App { view { Text { \"hi\" } } }",
            "{\"editor_mode\": true}",
        );
        let p1: serde_json::Value = serde_json::from_str(&result1).unwrap();
        let p2: serde_json::Value = serde_json::from_str(&result2).unwrap();
        // Both should compile successfully
        assert!(p1["success"].as_bool().unwrap());
        assert!(p2["success"].as_bool().unwrap());
        // Editor mode should produce different JS (adds debug wrappers)
        assert_ne!(
            p1["files"]["js"].as_str().unwrap(),
            p2["files"]["js"].as_str().unwrap()
        );
    }

    #[test]
    fn test_compile_multi_to_omniview_single_file() {
        let sources = serde_json::json!({
            "main.omni": "component App { view { Text { \"multi\" } } }"
        })
        .to_string();
        let result = compile_multi_to_omniview(&sources, "{}");
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        // Multi-file compile succeeds or has diagnostic errors (both valid outcomes)
        let has_output = parsed["files"]
            .as_object()
            .map(|o| !o.is_empty())
            .unwrap_or(false);
        let has_errors = parsed["errors"]
            .as_array()
            .map(|a| !a.is_empty())
            .unwrap_or(false);
        assert!(
            has_output || has_errors,
            "Expected either files or errors in multi compile result"
        );
    }

    #[test]
    fn test_compile_multi_to_omniview_invalid_json() {
        let result = compile_multi_to_omniview("not json", "{}");
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert!(!parsed["success"].as_bool().unwrap());
    }

    #[test]
    fn test_omni_compiler_new() {
        let c = OmniCompiler::new();
        assert_eq!(c.version(), env!("CARGO_PKG_VERSION"));
    }

    #[test]
    fn test_omni_compiler_empty_sources() {
        let mut c = OmniCompiler::new();
        let result = c.compile_to_omniview("{}");
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert!(!parsed["success"].as_bool().unwrap());
        assert!(parsed["errors"]
            .as_array()
            .unwrap()
            .iter()
            .any(|e| e["message"].as_str().unwrap().contains("no source")));
    }

    #[test]
    fn test_omni_compiler_single_file_compile() {
        let mut c = OmniCompiler::new();
        c.upsert_source(
            "app.omni".into(),
            "component App { view { Text { \"hello\" } } }".into(),
        );
        let result = c.compile_to_omniview("{}");
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert!(parsed["success"].as_bool().unwrap());
        assert!(parsed["files"]["js"].as_str().unwrap().len() > 0);
    }

    #[test]
    fn test_omni_compiler_cache_invalidation() {
        let mut c = OmniCompiler::new();
        c.upsert_source(
            "app.omni".into(),
            "component App { view { Text { \"v1\" } } }".into(),
        );
        let r1 = c.compile_to_omniview("{}");
        // Same source, same options — should hit cache
        let r2 = c.compile_to_omniview("{}");
        assert_eq!(r1, r2);
        // Change source — should invalidate
        c.upsert_source(
            "app.omni".into(),
            "component App { view { Text { \"v2\" } } }".into(),
        );
        let r3 = c.compile_to_omniview("{}");
        assert_ne!(r1, r3);
    }

    #[test]
    fn test_omni_compiler_diagnostics() {
        let mut c = OmniCompiler::new();
        c.upsert_source(
            "app.omni".into(),
            "component App { view { Text { \"ok\" } } }".into(),
        );
        let d = c.diagnostics();
        let parsed: serde_json::Value = serde_json::from_str(&d).unwrap();
        assert!(parsed["success"].as_bool().unwrap());
        assert!(parsed["errors"].is_array());
    }
}
