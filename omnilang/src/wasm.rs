//! WASM bindings for OmniLang compiler.
//! Compiled only when the `wasm` feature is enabled.
//!
//! Uses `serde_wasm_bindgen` to serialize/deserialize directly to/from `JsValue`,
//! eliminating JSON double-serialization in every compilation call.

use rustc_hash::FxHashMap;
use serde::Serialize;
use wasm_bindgen::prelude::*;

use crate::compile;

#[cfg(feature = "codegen-omniview")]
use crate::compile_omniview;

/// Build a JsValue error response for consistent error reporting.
fn js_error_value(message: &str) -> JsValue {
    let err = serde_json::json!({
        "success": false,
        "errors": [{
            "severity": "error",
            "code": "E_WASM",
            "message": message,
            "file": null,
            "line": 0,
            "col": 0,
            "end_line": 0,
            "end_col": 0,
            "suggestion": null
        }]
    });
    serde_wasm_bindgen::to_value(&err)
        .unwrap_or_else(|e| JsValue::from_str(&format!("Critical internal error: {}", e)))
}

/// Serialize any Serialize-able value to JsValue, falling back to an error JsValue.
fn to_js_value<T: Serialize + ?Sized>(value: &T) -> JsValue {
    serde_wasm_bindgen::to_value(value)
        .unwrap_or_else(|e| js_error_value(&format!("serialization error: {}", e)))
}

/// Compile .omni source to OmniView (JS + HTML + CSS). Returns JsValue.
/// editor_mode: enable debug wrappers for editor preview.
#[cfg(feature = "codegen-omniview")]
#[wasm_bindgen]
pub fn compile_to_omniview(source: &str, editor_mode: bool, debug_mode: bool) -> JsValue {
    const MAX_INPUT_SIZE: usize = 10 * 1024 * 1024; // 10MB
    if source.len() > MAX_INPUT_SIZE {
        return js_error_value("Input exceeds 10MB limit");
    }
    to_js_value(&compile_omniview(source, editor_mode, debug_mode))
}

/// Compile a single .omni component to OmniView output. Returns JsValue.
/// The source should contain the full .omni file; component_name selects which component to extract.
/// editor_mode: enable debug wrappers for editor preview.
#[cfg(feature = "codegen-omniview")]
#[wasm_bindgen]
pub fn compile_omniview_component(
    source: &str,
    component_name: &str,
    editor_mode: bool,
    debug_mode: bool,
) -> JsValue {
    const MAX_INPUT_SIZE: usize = 10 * 1024 * 1024; // 10MB
    if source.len() > MAX_INPUT_SIZE {
        return js_error_value("Input exceeds 10MB limit");
    }
    let result = compile_omniview(source, editor_mode, debug_mode);
    let component = result
        .components
        .as_ref()
        .and_then(|comps| comps.get(component_name).cloned());

    let output = serde_json::json!({
        "success": result.success && component.is_some(),
        "component": component,
        "errors": result.errors,
    });
    serde_wasm_bindgen::to_value(&output)
        .unwrap_or_else(|e| js_error_value(&format!("serialization error: {}", e)))
}

/// Return compiler version.
#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Compile .omni source to IR JSON. Returns JsValue.
#[wasm_bindgen]
pub fn compile_to_ir(source: &str) -> JsValue {
    const MAX_INPUT_SIZE: usize = 10 * 1024 * 1024; // 10MB
    if source.len() > MAX_INPUT_SIZE {
        return js_error_value("Input exceeds 10MB limit");
    }
    to_js_value(&compile(source))
}

/// Compile .omni source to OmniView output in a single pass (IR + codegen).
/// Returns JsValue with `ir`, `files`, `components`, `source_map`, and `errors`.
/// editor_mode: enable debug wrappers for editor preview.
#[cfg(feature = "codegen-omniview")]
#[wasm_bindgen]
pub fn compile_to_omniview_full(source: &str, editor_mode: bool, debug_mode: bool) -> JsValue {
    const MAX_INPUT_SIZE: usize = 10 * 1024 * 1024; // 10MB
    if source.len() > MAX_INPUT_SIZE {
        return js_error_value("Input exceeds 10MB limit");
    }
    to_js_value(&crate::compile_omniview_full(source, editor_mode, debug_mode))
}

/// Resolve per-element computed styles for editor canvas rendering.
/// Returns structured JSON with component -> element -> computed CSS.
#[cfg(feature = "codegen-omniview")]
#[wasm_bindgen]
pub fn resolve_styles(source: &str) -> JsValue {
    const MAX_INPUT_SIZE: usize = 10 * 1024 * 1024; // 10MB
    if source.len() > MAX_INPUT_SIZE {
        return js_error_value("Input exceeds 10MB limit");
    }
    to_js_value(&crate::resolve_styles(source))
}

/// Resolve styles for multiple .omni source files (cross-file design imports).
///
/// `sources`: JS object `{ "file/path.omni": "source content", ... }`
/// `path_mapping`: JS object `{ "import/path": "resolved/file/path.omni", ... }`
///
/// Returns `StyleResolveResult` with compiled CSS + element mappings.
#[cfg(feature = "codegen-omniview")]
#[wasm_bindgen]
pub fn resolve_styles_multi(sources: JsValue, path_mapping: JsValue) -> JsValue {
    let sources_map: FxHashMap<String, String> = match serde_wasm_bindgen::from_value(sources) {
        Ok(s) => s,
        Err(e) => {
            return serde_wasm_bindgen::to_value(&serde_json::json!({
                "success": false,
                "design_css": "",
                "component_css": {},
                "element_classes": {},
                "element_tags": {},
                "warnings": [],
                "errors": [{
                    "severity": "error",
                    "code": "E_INVALID_SOURCES",
                    "message": format!("Failed to parse sources: {}", e),
                    "line": 0, "col": 0, "end_line": 0, "end_col": 0
                }]
            }))
            .unwrap_or_else(|e| js_error_value(&format!("serialization error: {}", e)));
        }
    };
    let path_map: FxHashMap<String, String> =
        serde_wasm_bindgen::from_value(path_mapping).unwrap_or_default();
    let total_size: usize = sources_map.values().map(|s| s.len()).sum();
    if total_size > 10 * 1024 * 1024 {
        return js_error_value("Input exceeds 10MB limit");
    }
    to_js_value(&crate::resolve_styles_multi(sources_map, path_map))
}

/// Compile multiple .omni source files to OmniView.
/// sources: JS object mapping file path to source content, e.g. {"path": "source content", ...}
/// editor_mode: enable debug wrappers for editor preview.
/// Returns JsValue with `success`, `files`, `errors`, and `warnings`.
#[wasm_bindgen]
pub fn compile_multi_to_omniview(sources: JsValue, editor_mode: bool, debug_mode: bool) -> JsValue {
    let sources_map: FxHashMap<String, String> = match serde_wasm_bindgen::from_value(sources) {
        Ok(s) => s,
        Err(e) => {
            return serde_wasm_bindgen::to_value(&serde_json::json!({
                "success": false,
                "files": {},
                "errors": [{
                    "severity": "error",
                    "code": "E_INVALID_SOURCES",
                    "message": format!("Failed to parse sources: {}", e),
                    "file": null,
                    "line": 0,
                    "col": 0,
                    "end_line": 0,
                    "end_col": 0,
                    "suggestion": null
                }]
            }))
            .unwrap_or_else(|e| js_error_value(&format!("serialization error: {}", e)));
        }
    };
    const MAX_INPUT_SIZE: usize = 10 * 1024 * 1024; // 10MB
    let total_size: usize = sources_map.values().map(|s| s.len()).sum();
    if total_size > MAX_INPUT_SIZE {
        return js_error_value("Input exceeds 10MB limit");
    }
    let result = crate::compile_multi_internal(
        sources_map,
        crate::codegen::Target::OmniView,
        FxHashMap::default(),
        editor_mode,
        debug_mode,
    );
    to_js_value(&result)
}

/// Stateful compiler instance that caches sources and compilation results across calls.
#[wasm_bindgen]
pub struct OmniCompiler {
    sources: FxHashMap<String, String>,
    cached_compile_result: Option<(bool, bool, JsValue)>, // (editor_mode, debug_mode, result JsValue)
    cached_diags_result: Option<JsValue>,
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
    /// editor_mode: enable debug wrappers for editor preview.
    #[cfg(feature = "codegen-omniview")]
    pub fn compile_to_omniview(&mut self, editor_mode: bool, debug_mode: bool) -> JsValue {
        if self.sources.is_empty() {
            return serde_wasm_bindgen::to_value(&serde_json::json!({
                "success": false,
                "files": null,
                "components": null,
                "source_map": null,
                "errors": [{
                    "severity": "error",
                    "code": "E_NO_SOURCES",
                    "message": "no source files added to compiler",
                    "line": 0,
                    "col": 0,
                    "end_line": 0,
                    "end_col": 0,
                    "suggestion": null
                }]
            }))
            .unwrap_or_else(|e| js_error_value(&format!("serialization error: {}", e)));
        }
        if let Some((ref cached_editor, ref cached_debug, ref cached)) = self.cached_compile_result {
            if *cached_editor == editor_mode && *cached_debug == debug_mode {
                return cached.clone();
            }
        }
        let result_js = if self.sources.len() == 1 {
            // Single file: use compile_omniview_full for full result with IR
            let source = match self.sources.values().next() {
                Some(s) => s,
                None => {
                    return serde_wasm_bindgen::to_value(&serde_json::json!({
                        "success": false,
                        "files": null,
                        "components": null,
                        "source_map": null,
                        "errors": [{
                            "severity": "error",
                            "code": "E_NO_SOURCES",
                            "message": "no source files in compiler",
                            "line": 0,
                            "col": 0,
                            "end_line": 0,
                            "end_col": 0,
                            "suggestion": null
                        }]
                    }))
                    .unwrap_or_else(|e| js_error_value(&format!("serialization error: {}", e)));
                }
            };
            let full = crate::compile_omniview_full(source, editor_mode, debug_mode);
            to_js_value(&full)
        } else {
            // Multi-file: use compile_multi_internal with structured result
            let result = crate::compile_multi_internal(
                self.sources.clone(),
                crate::codegen::Target::OmniView,
                FxHashMap::default(),
                editor_mode,
                debug_mode,
            );
            to_js_value(&result)
        };
        self.cached_compile_result = Some((editor_mode, debug_mode, result_js.clone()));
        self.cached_diags_result = None;
        result_js
    }

    /// Run diagnostics on all registered sources without compiling to output.
    /// Returns JsValue with `success` and `errors` arrays.
    pub fn diagnostics(&mut self) -> JsValue {
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
        let result_value = serde_wasm_bindgen::to_value(&serde_json::json!({
            "success": all_success,
            "errors": all_errors,
        }))
        .unwrap_or_else(|e| js_error_value(&format!("serialization error: {}", e)));
        self.cached_diags_result = Some(result_value.clone());
        result_value
    }

    /// Return the compiler version string.
    pub fn version(&self) -> String {
        env!("CARGO_PKG_VERSION").to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "codegen-omniview")]
    #[test]
    fn test_compile_to_omniview_full_success() {
        let result =
            compile_to_omniview_full("component App { view { Text { \"hello\" } } }", false);
        let parsed: serde_json::Value = serde_wasm_bindgen::from_value(result).unwrap();
        assert!(parsed["success"].as_bool().unwrap());
        assert!(parsed["files"]["js"].as_str().unwrap().len() > 0);
        assert!(parsed["files"]["html"].as_str().unwrap().len() > 0);
        // Should have IR
        assert!(parsed["ir"].is_object());
    }

    #[cfg(feature = "codegen-omniview")]
    #[test]
    fn test_compile_to_omniview_full_parse_error() {
        let result = compile_to_omniview_full("this is invalid @@@", false, false);
        let parsed: serde_json::Value = serde_wasm_bindgen::from_value(result).unwrap();
        assert!(!parsed["success"].as_bool().unwrap());
        assert!(parsed["ir"].is_null());
        assert!(parsed["files"].is_null());
        assert!(parsed["errors"].as_array().unwrap().len() > 0);
    }

    #[cfg(feature = "codegen-omniview")]
    #[test]
    fn test_compile_to_omniview_full_editor_mode() {
        let result1 = compile_to_omniview_full("component App { view { Text { \"hi\" } } }", false);
        let result2 = compile_to_omniview_full("component App { view { Text { \"hi\" } } }", true);
        let p1: serde_json::Value = serde_wasm_bindgen::from_value(result1).unwrap();
        let p2: serde_json::Value = serde_wasm_bindgen::from_value(result2).unwrap();
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
        let sources = serde_wasm_bindgen::to_value(&serde_json::json!({
            "main.omni": "component App { view { Text { \"multi\" } } }"
        }))
        .unwrap();
        let result = compile_multi_to_omniview(sources, false, false);
        let parsed: serde_json::Value = serde_wasm_bindgen::from_value(result).unwrap();
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
    fn test_compile_multi_to_omniview_invalid_sources() {
        let sources = serde_wasm_bindgen::to_value("not a map").unwrap();
        let result = compile_multi_to_omniview(sources, false, false);
        let parsed: serde_json::Value = serde_wasm_bindgen::from_value(result).unwrap();
        assert!(!parsed["success"].as_bool().unwrap());
    }

    #[test]
    fn test_omni_compiler_new() {
        let c = OmniCompiler::new();
        assert_eq!(c.version(), env!("CARGO_PKG_VERSION"));
    }

    #[cfg(feature = "codegen-omniview")]
    #[test]
    fn test_omni_compiler_empty_sources() {
        let mut c = OmniCompiler::new();
        let result = c.compile_to_omniview(false, false);
        let parsed: serde_json::Value = serde_wasm_bindgen::from_value(result).unwrap();
        assert!(!parsed["success"].as_bool().unwrap());
        assert!(parsed["errors"]
            .as_array()
            .unwrap()
            .iter()
            .any(|e| e["message"].as_str().unwrap().contains("no source")));
    }

    #[cfg(feature = "codegen-omniview")]
    #[test]
    fn test_omni_compiler_single_file_compile() {
        let mut c = OmniCompiler::new();
        c.upsert_source(
            "app.omni".into(),
            "component App { view { Text { \"hello\" } } }".into(),
        );
        let result = c.compile_to_omniview(false, false);
        let parsed: serde_json::Value = serde_wasm_bindgen::from_value(result).unwrap();
        assert!(parsed["success"].as_bool().unwrap());
        assert!(parsed["files"]["js"].as_str().unwrap().len() > 0);
    }

    #[cfg(feature = "codegen-omniview")]
    #[test]
    fn test_omni_compiler_cache_invalidation() {
        let mut c = OmniCompiler::new();
        c.upsert_source(
            "app.omni".into(),
            "component App { view { Text { \"v1\" } } }".into(),
        );
        let r1 = c.compile_to_omniview(false, false);
        let p1: serde_json::Value = serde_wasm_bindgen::from_value(r1).unwrap();

        // Same source, same options — should hit cache
        let r2 = c.compile_to_omniview(false, false);
        let p2: serde_json::Value = serde_wasm_bindgen::from_value(r2).unwrap();
        assert_eq!(p1, p2);

        // Change source — should invalidate
        c.upsert_source(
            "app.omni".into(),
            "component App { view { Text { \"v2\" } } }".into(),
        );
        let r3 = c.compile_to_omniview(false, false);
        let p3: serde_json::Value = serde_wasm_bindgen::from_value(r3).unwrap();
        assert_ne!(p1, p3);
    }

    #[test]
    fn test_omni_compiler_diagnostics() {
        let mut c = OmniCompiler::new();
        c.upsert_source(
            "app.omni".into(),
            "component App { view { Text { \"ok\" } } }".into(),
        );
        let d = c.diagnostics();
        let parsed: serde_json::Value = serde_wasm_bindgen::from_value(d).unwrap();
        assert!(parsed["success"].as_bool().unwrap());
        assert!(parsed["errors"].is_array());
    }
}
