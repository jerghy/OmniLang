//! WASM bindings for OmniLang compiler.
//!
//! Uses `serde_wasm_bindgen` to serialize/deserialize directly to/from `JsValue`,
//! eliminating JSON double-serialization in every compilation call.

use rustc_hash::FxHashMap;
use serde::Serialize;
use std::cell::RefCell;
use wasm_bindgen::prelude::*;

use omnilang::analyse::IncrementalCache;
use omnilang::compile;

#[cfg(feature = "codegen-omniview")]
use omnilang::compile_omniview;

/// Build a JsValue error response for consistent error reporting.
/// Errors use the unified `DiagnosticJson` envelope (all 9 fields present,
/// lowercase severity).
fn js_error_value(message: &str) -> JsValue {
    to_js_value(&ErrorEnvelope {
        success: false,
        errors: vec![omnilang::error::DiagnosticJson::synthetic("E_WASM", message)],
    })
}

// ═══════════════════════════════════════════════════════
// Cross-boundary envelope structs
// ═══════════════════════════════════════════════════════
//
// WASM FFI serialization contract: serde_json::Value is NOT allowed across
// the WASM→JS boundary. serde-wasm-bindgen serializes `Value::Object` as a JS
// Map (or an empty `{}` after wasm-bindgen CLI processing) — downstream JS
// property access (`response.result`) returns undefined. Every exported
// function must return a `#[derive(Serialize)]` struct; nested maps/values
// must be strongly typed as well (no serde_json::Value anywhere in the
// envelope tree).

/// Error-only envelope (js_error_value fallback).
#[derive(Serialize)]
struct ErrorEnvelope {
    success: bool,
    errors: Vec<omnilang::error::DiagnosticJson>,
}

/// `parse_source` / `parse_source_internal` envelope.
#[derive(Serialize)]
struct ParseSourceEnvelope {
    success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    program: Option<omnilang::parser::ast::Program>,
    errors: Vec<omnilang::error::DiagnosticJson>,
}

/// `compile_omniview_component` envelope.
#[derive(Serialize)]
struct ComponentCompileEnvelope {
    success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    component: Option<omnilang::codegen::omniview::ComponentOutput>,
    errors: Vec<omnilang::error::DiagnosticJson>,
}

/// `analyse_multi_cached` envelope.
#[derive(Serialize)]
struct AnalyseEnvelope {
    success: bool,
    errors: Vec<omnilang::error::DiagnosticJson>,
    warnings: Vec<omnilang::error::DiagnosticJson>,
    cache_state: Option<String>,
}

/// `resolve_styles_multi_cached` envelope.
#[derive(Serialize)]
struct StyleResolveEnvelope {
    result: omnilang::codegen::omniview::StyleResolveResult,
    cache_state: Option<String>,
}

/// `compile_multi_to_omniview` envelope.
#[derive(Serialize)]
struct MultiCompileEnvelope {
    result: omnilang::InternalCompileResult,
    cache_state: Option<String>,
}

/// `OmniCompiler::diagnostics` envelope.
#[derive(Serialize)]
struct DiagnosticsEnvelope {
    success: bool,
    errors: Vec<omnilang::error::DiagnosticJson>,
}

/// `OmniCompiler::analyse_dependencies` envelope.
#[derive(Serialize)]
struct DependenciesEnvelope {
    symbols: Vec<omnilang::SymbolInfo>,
    dependencies: FxHashMap<String, Vec<String>>,
    diagnostics: Vec<omnilang::error::DiagnosticJson>,
}

/// Build a boundary-error `Diagnostic` (E_BOUNDARY) for invalid WASM inputs.
fn boundary_diagnostic(message: impl Into<String>) -> omnilang::error::Diagnostic {
    omnilang::error::Diagnostic {
        severity: omnilang::error::Severity::Error,
        kind: omnilang::error::DiagnosticKind::EBoundary,
        message: message.into(),
        file: None,
        line: 0,
        col: 0,
        end_line: None,
        end_col: None,
        suggestion: None,
    }
}

/// Build an error `InternalCompileResult` for invalid WASM inputs.
fn invalid_sources_result(message: impl Into<String>) -> omnilang::InternalCompileResult {
    omnilang::InternalCompileResult {
        success: false,
        files: FxHashMap::default(),
        errors: vec![boundary_diagnostic(message)],
        warnings: Vec::new(),
        components: FxHashMap::default(),
        source_map: Vec::new(),
    }
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

    to_js_value(&ComponentCompileEnvelope {
        success: result.success && component.is_some(),
        component,
        errors: result
            .errors
            .iter()
            .map(omnilang::error::DiagnosticJson::from)
            .collect(),
    })
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
    to_js_value(&omnilang::compile_omniview_full(
        source,
        editor_mode,
        debug_mode,
    ))
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
    to_js_value(&omnilang::resolve_styles(source))
}

/// Parse a code-block source string into structured AST statements.
/// Returns a JsValue with `{ success, stmts, errors }`.
#[wasm_bindgen]
pub fn parse_code_block(source: &str) -> JsValue {
    let result = omnilang::parse_stmts(source);
    serde_wasm_bindgen::to_value(&result)
        .unwrap_or_else(|e| js_error_value(&format!("serialization error: {}", e)))
}

/// Internal parse function (not WASM-exported) — lex + parse in one call.
/// Returns the strongly-typed envelope used by the exported `parse_source`.
fn parse_source_internal(source: &str) -> ParseSourceEnvelope {
    const MAX_INPUT_SIZE: usize = 10 * 1024 * 1024; // 10MB
    if source.len() > MAX_INPUT_SIZE {
        return ParseSourceEnvelope {
            success: false,
            program: None,
            errors: vec![omnilang::error::DiagnosticJson::synthetic(
                "E_PARSE",
                "Input exceeds 10MB limit",
            )],
        };
    }
    let tokens = omnilang::lexer::tokenize(source);
    let (program, parse_errors) = omnilang::parser::parse(tokens, source);
    let success = parse_errors.is_empty();
    // Unified DiagnosticJson shape: all 9 fields present, explicit nulls.
    let errors: Vec<omnilang::error::DiagnosticJson> = parse_errors
        .into_iter()
        .map(|err| {
            let span = err.span;
            omnilang::error::DiagnosticJson {
                severity: "error".to_string(),
                code: "E_PARSE".to_string(),
                message: err.message,
                file: None,
                line: span.line,
                col: span.col,
                end_line: Some(span.line),
                end_col: Some(span.col + (span.end.saturating_sub(span.start)) as u32),
                suggestion: None,
            }
        })
        .collect();
    ParseSourceEnvelope {
        success,
        program: Some(program),
        errors,
    }
}

/// Parse complete .omni source text (lex + parse in one call).
///
/// Returns `{ success: bool, program: Program | null, errors: ParseError[] }` as JsValue.
#[wasm_bindgen]
pub fn parse_source(source: &str) -> JsValue {
    to_js_value(&parse_source_internal(source))
}

/// Wrap generated OmniView output in a standalone debug-ready HTML document.
#[cfg(feature = "codegen-omniview")]
#[wasm_bindgen]
pub fn wrap_debug_html(js: &str, css: &str, body_html: &str) -> String {
    omnilang::codegen::omniview::wrap_debug_html(js, css, body_html)
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
            let mut result = omnilang::codegen::omniview::StyleResolveResult {
                success: false,
                design_css: String::new(),
                component_css: FxHashMap::default(),
                element_classes: FxHashMap::default(),
                element_tags: FxHashMap::default(),
                element_tree: Vec::new(),
                nid_map: FxHashMap::default(),
                warnings: Vec::new(),
                errors: Vec::new(),
            };
            result.errors.push(boundary_diagnostic(format!(
                "Failed to parse sources: {}",
                e
            )));
            return to_js_value(&result);
        }
    };
    let mut path_map: FxHashMap<String, String> =
        serde_wasm_bindgen::from_value(path_mapping).unwrap_or_default();
    // Merge auto-derived path mappings from source keys, using caller overrides
    // as priority (or_insert keeps the first value).
    let derived = omnilang::path_mapping::derive_path_mapping(&sources_map);
    for (k, v) in derived {
        path_map.entry(k).or_insert(v);
    }
    let total_size: usize = sources_map.values().map(|s| s.len()).sum();
    if total_size > 10 * 1024 * 1024 {
        return js_error_value("Input exceeds 10MB limit");
    }
    to_js_value(&omnilang::resolve_styles_multi(sources_map, path_map))
}

// Thread-local incremental analysis cache that persists across WASM calls
// within the same module lifetime.  This is the backing store for the
// serialized cache_state string that flows through the JS side.
thread_local! {
    static ANALYSIS_CACHE: RefCell<IncrementalCache> = RefCell::new(IncrementalCache::new());
}

/// Seed the thread-local ANALYSIS_CACHE from a serialized cache_state string.
/// Called on first use or when the thread-local cache is empty (e.g. after
/// WASM re-initialization).  Deserialized entries have invalid (empty)
/// programs, so the first analysis after seeding will re-parse all files.
fn seed_cache_from_state(cache_state: &JsValue) {
    if cache_state.is_null() || cache_state.is_undefined() {
        return;
    }
    let Some(json_str) = cache_state.as_string() else {
        return;
    };
    if json_str.is_empty() {
        return;
    }
    if let Ok(seeded) = serde_json::from_str::<IncrementalCache>(&json_str) {
        ANALYSIS_CACHE.with(|cell| {
            *cell.borrow_mut() = seeded;
        });
    }
}

/// Multi-file analysis with incremental cache.
///
/// `sources`: JS object `{ "file/path.omni": "source content", ... }`
/// `path_mapping`: JS object `{ "import/path": "resolved/file/path.omni", ... }`
/// `cache_state`: serialized cache from previous call (or null/undefined)
///
/// Returns `{ success, errors, warnings, cache_state }`.
#[wasm_bindgen]
pub fn analyse_multi_cached(
    sources: JsValue,
    path_mapping: JsValue,
    cache_state: JsValue,
) -> JsValue {
    let sources_map: FxHashMap<String, String> = match serde_wasm_bindgen::from_value(sources) {
        Ok(s) => s,
        Err(e) => {
            return to_js_value(&AnalyseEnvelope {
                success: false,
                errors: vec![omnilang::error::DiagnosticJson::synthetic(
                    "E_INVALID_SOURCES",
                    format!("Failed to parse sources: {}", e),
                )],
                warnings: Vec::new(),
                cache_state: None,
            });
        }
    };
    let mut path_map: FxHashMap<String, String> =
        serde_wasm_bindgen::from_value(path_mapping).unwrap_or_default();
    let derived = omnilang::path_mapping::derive_path_mapping(&sources_map);
    for (k, v) in derived {
        path_map.entry(k).or_insert(v);
    }

    // Seed the thread-local cache if empty (handles WASM re-initialization)
    ANALYSIS_CACHE.with(|cell| {
        if cell.borrow().entries.is_empty() {
            seed_cache_from_state(&cache_state);
        }
    });

    let analysis = ANALYSIS_CACHE.with(|cell| {
        let mut cache = cell.borrow_mut();
        // Cache lives in WASM memory for the page lifetime. Serialize only on explicit save.
        omnilang::analyse::analyse_multi_cached(&sources_map, &path_map, &mut cache)
    });

    // Diagnostics cross the boundary in the unified DiagnosticJson shape.
    let errors: Vec<omnilang::error::DiagnosticJson> = analysis
        .diagnostics
        .iter()
        .map(omnilang::error::DiagnosticJson::from)
        .collect();
    let warnings: Vec<omnilang::error::DiagnosticJson> = analysis
        .warnings
        .iter()
        .map(omnilang::error::DiagnosticJson::from)
        .collect();

    to_js_value(&AnalyseEnvelope {
        success: analysis.success,
        errors,
        warnings,
        cache_state: None,
    })
}

/// Resolve styles for multiple .omni source files using incremental cache.
///
/// Same interface as `resolve_styles_multi` but accepts `cache_state` and
/// returns `{ result: StyleResolveResult, cache_state: string }`.
#[cfg(feature = "codegen-omniview")]
#[wasm_bindgen]
pub fn resolve_styles_multi_cached(
    sources: JsValue,
    path_mapping: JsValue,
    cache_state: JsValue,
) -> JsValue {
    let sources_map: FxHashMap<String, String> = match serde_wasm_bindgen::from_value(sources) {
        Ok(s) => s,
        Err(e) => {
            let mut result = omnilang::codegen::omniview::StyleResolveResult {
                success: false,
                design_css: String::new(),
                component_css: FxHashMap::default(),
                element_classes: FxHashMap::default(),
                element_tags: FxHashMap::default(),
                element_tree: Vec::new(),
                nid_map: FxHashMap::default(),
                warnings: Vec::new(),
                errors: Vec::new(),
            };
            result.errors.push(boundary_diagnostic(format!(
                "Failed to parse sources: {}",
                e
            )));
            return to_js_value(&StyleResolveEnvelope {
                result,
                cache_state: None,
            });
        }
    };
    let mut path_map: FxHashMap<String, String> =
        serde_wasm_bindgen::from_value(path_mapping).unwrap_or_default();
    let derived = omnilang::path_mapping::derive_path_mapping(&sources_map);
    for (k, v) in derived {
        path_map.entry(k).or_insert(v);
    }
    let total_size: usize = sources_map.values().map(|s| s.len()).sum();
    if total_size > 10 * 1024 * 1024 {
        return js_error_value("Input exceeds 10MB limit");
    }

    // Seed the thread-local cache if empty
    ANALYSIS_CACHE.with(|cell| {
        if cell.borrow().entries.is_empty() {
            seed_cache_from_state(&cache_state);
        }
    });

    let analysis_result = ANALYSIS_CACHE.with(|cell| {
        let mut cache = cell.borrow_mut();
        // Cache lives in WASM memory for the page lifetime. Serialize only on explicit save.
        omnilang::analyse::analyse_multi_cached(&sources_map, &path_map, &mut cache)
    });

    // Diagnostics cross the boundary in the unified Diagnostic shape
    // (Diagnostic's manual Serialize impl emits the same 9-field envelope as
    // DiagnosticJson; optional fields are omitted when None, which the TS
    // side already treats as absent/null).
    let result = match &analysis_result.ir {
        Some(ir) => {
            let output = omnilang::codegen::omniview::generate(ir, true, true, false);
            let success = analysis_result
                .diagnostics
                .iter()
                .all(|d| !matches!(d.severity, omnilang::error::Severity::Error))
                && output.warnings.is_empty();
            omnilang::codegen::omniview::StyleResolveResult {
                success,
                design_css: output.css,
                component_css: output.per_component_css,
                element_classes: output.element_classes,
                element_tags: output.element_tags,
                element_tree: output.element_tree,
                nid_map: output.nid_map,
                warnings: output.warnings,
                errors: analysis_result.diagnostics,
            }
        }
        None => omnilang::codegen::omniview::StyleResolveResult {
            success: false,
            design_css: String::new(),
            component_css: FxHashMap::default(),
            element_classes: FxHashMap::default(),
            element_tags: FxHashMap::default(),
            element_tree: Vec::new(),
            nid_map: FxHashMap::default(),
            warnings: Vec::new(),
            errors: analysis_result.diagnostics,
        },
    };

    to_js_value(&StyleResolveEnvelope {
        result,
        cache_state: None,
    })
}

/// Compile multiple .omni source files to OmniView with incremental cache.
///
/// `sources`: JS object mapping file path to source content, e.g. {"path": "source content", ...}
/// `editor_mode`: enable debug wrappers for editor preview.
/// `cache_state`: serialized incremental cache from previous call (or null/undefined).
///
/// Returns `{ result: InternalCompileResult, cache_state: string }`.
#[wasm_bindgen]
pub fn compile_multi_to_omniview(
    sources: JsValue,
    editor_mode: bool,
    debug_mode: bool,
    cache_state: JsValue,
) -> JsValue {
    let sources_map: FxHashMap<String, String> = match serde_wasm_bindgen::from_value(sources) {
        Ok(s) => s,
        Err(e) => {
            return to_js_value(&MultiCompileEnvelope {
                result: invalid_sources_result(format!("Failed to parse sources: {}", e)),
                cache_state: None,
            });
        }
    };
    const MAX_INPUT_SIZE: usize = 10 * 1024 * 1024; // 10MB
    let total_size: usize = sources_map.values().map(|s| s.len()).sum();
    if total_size > MAX_INPUT_SIZE {
        return to_js_value(&MultiCompileEnvelope {
            result: invalid_sources_result("Input exceeds 10MB limit"),
            cache_state: None,
        });
    }
    // Build path_mapping from source keys so cross-file imports resolve.
    let path_mapping = omnilang::path_mapping::derive_path_mapping(&sources_map);

    // Seed the thread-local cache if empty (handles WASM re-initialization)
    ANALYSIS_CACHE.with(|cell| {
        if cell.borrow().entries.is_empty() {
            seed_cache_from_state(&cache_state);
        }
    });

    // Compile using cached analysis to skip unchanged files
    let result = ANALYSIS_CACHE.with(|cell| {
        let mut cache = cell.borrow_mut();
        omnilang::compile_multi_internal_cached(
            sources_map,
            omnilang::codegen::Target::OmniView,
            path_mapping,
            editor_mode,
            debug_mode,
            None,
            &mut cache,
        )
    });

    // The cache is NOT serialized per call — it lives in WASM memory for the
    // page lifetime (see `save_cache_state` for explicit persistence). The
    // JS side passes `cache_state` back on the next call and the seed logic
    // no-ops on null.
    to_js_value(&MultiCompileEnvelope {
        result,
        cache_state: None,
    })
}

/// Stateful compiler instance that caches sources and compilation results across calls.
#[wasm_bindgen]
#[derive(Default)]
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
            return to_js_value(&omnilang::OmniViewFullResult {
                success: false,
                ir: None,
                files: None,
                components: None,
                source_map: None,
                errors: vec![boundary_diagnostic("no source files added to compiler")],
                warnings: Vec::new(),
            });
        }
        if let Some((ref cached_editor, ref cached_debug, ref cached)) = self.cached_compile_result
        {
            if *cached_editor == editor_mode && *cached_debug == debug_mode {
                return cached.clone();
            }
        }
        let result_js = if self.sources.len() == 1 {
            // Single file: use compile_omniview_full for full result with IR
            let source = match self.sources.values().next() {
                Some(s) => s,
                None => {
                    return to_js_value(&omnilang::OmniViewFullResult {
                        success: false,
                        ir: None,
                        files: None,
                        components: None,
                        source_map: None,
                        errors: vec![boundary_diagnostic("no source files in compiler")],
                        warnings: Vec::new(),
                    });
                }
            };
            let full = omnilang::compile_omniview_full(source, editor_mode, debug_mode);
            to_js_value(&full)
        } else {
            // Multi-file: use compile_multi_internal with structured result
            let result = omnilang::compile_multi_internal(
                self.sources.clone(),
                omnilang::codegen::Target::OmniView,
                omnilang::path_mapping::derive_path_mapping(&self.sources),
                editor_mode,
                debug_mode,
                None,
            );
            to_js_value(&result)
        };
        self.cached_compile_result = Some((editor_mode, debug_mode, result_js.clone()));
        self.cached_diags_result = None;
        result_js
    }

    /// Run diagnostics on all registered sources without compiling to output.
    /// Returns JsValue with `success` and `errors` arrays (both error- and
    /// warning-severity diagnostics, matching the pre-refactor single-file
    /// path where lint warnings folded into `compile().errors`).
    ///
    /// Routes through the multi-file analysis path (`analyse_multi_cached`)
    /// so diagnostics are cross-file aware (import errors, cross-file type
    /// mismatches) instead of per-file isolated compiles. Diagnostics cross
    /// the boundary in the unified `DiagnosticJson` shape.
    pub fn diagnostics(&mut self) -> JsValue {
        if let Some(ref cached) = self.cached_diags_result {
            return cached.clone();
        }
        let sources_map = self.sources.clone();
        let path_mapping = omnilang::path_mapping::derive_path_mapping(&sources_map);
        let analysis = ANALYSIS_CACHE.with(|cell| {
            let mut cache = cell.borrow_mut();
            omnilang::analyse::analyse_multi_cached(&sources_map, &path_mapping, &mut cache)
        });
        // Lint/merge warnings live in `analysis.warnings` (the multi-file
        // path separates them from `diagnostics`); fold them back in so
        // editor warning markers keep surfacing (Phase 0 "warnings 三网关
        // 可见" gate).
        let errors: Vec<omnilang::error::DiagnosticJson> = analysis
            .diagnostics
            .iter()
            .chain(analysis.warnings.iter())
            .map(omnilang::error::DiagnosticJson::from)
            .collect();
        let result_value = to_js_value(&DiagnosticsEnvelope {
            success: analysis.success,
            errors,
        });
        self.cached_diags_result = Some(result_value.clone());
        result_value
    }

    /// Run semantic analysis on all registered sources and return
    /// symbols + dependencies in addition to diagnostics.
    /// Returns JsValue with { symbols, dependencies, diagnostics }.
    ///
    /// Routes through the multi-file analysis path (cross-file aware
    /// diagnostics). Symbols/dependencies are extracted from the pre-parsed
    /// programs via a per-file semantic pass — parsing, cross-file
    /// validation, IR lowering and codegen all run once in the shared
    /// multi-file path, not per-file in isolation. Lint/merge warnings are
    /// folded into `diagnostics` like the pre-refactor single-file path.
    pub fn analyse_dependencies(&mut self) -> JsValue {
        let sources_map = self.sources.clone();
        let path_mapping = omnilang::path_mapping::derive_path_mapping(&sources_map);
        let analysis = ANALYSIS_CACHE.with(|cell| {
            let mut cache = cell.borrow_mut();
            omnilang::analyse::analyse_multi_cached(&sources_map, &path_mapping, &mut cache)
        });

        let mut all_symbols: Vec<omnilang::SymbolInfo> = Vec::new();
        let mut all_deps: FxHashMap<String, Vec<String>> = FxHashMap::default();
        for program in analysis.programs.values() {
            let semantics = omnilang::semantic::analyze(program);
            for sym in semantics
                .symbols
                .scopes()
                .iter()
                .flat_map(|scope| scope.symbols.values())
            {
                all_symbols.push(omnilang::SymbolInfo {
                    name: sym.name.clone(),
                    kind: format!("{:?}", sym.kind).to_lowercase(),
                    type_ann: sym.type_ann.clone(),
                    mutable: sym.mutable,
                });
            }
            for (k, v) in &semantics.dependencies {
                all_deps.entry(k.clone()).or_default().extend(v.clone());
            }
        }
        let diagnostics: Vec<omnilang::error::DiagnosticJson> = analysis
            .diagnostics
            .iter()
            .chain(analysis.warnings.iter())
            .map(omnilang::error::DiagnosticJson::from)
            .collect();

        to_js_value(&DependenciesEnvelope {
            symbols: all_symbols,
            dependencies: all_deps,
            diagnostics,
        })
    }

    /// Return the compiler version string.
    pub fn version(&self) -> String {
        env!("CARGO_PKG_VERSION").to_string()
    }
}

/// Serialize the thread-local analysis cache to a JSON string for explicit persistence.
///
/// Call this from JS on `beforeunload` to preserve the incremental cache across
/// page navigations. Returns the full cached state as a JSON string, or empty
/// string on serialization failure.
///
/// The cache is NOT serialized automatically on every analysis call — it lives
/// in WASM memory for the page lifetime. Use this function only when you need
/// to persist the cache externally (session storage, beforeunload, etc.).
#[wasm_bindgen]
pub fn save_cache_state() -> JsValue {
    ANALYSIS_CACHE.with(|cell| {
        let cache = cell.borrow();
        JsValue::from_str(&serde_json::to_string(&*cache).unwrap_or_default())
    })
}

#[cfg(all(test, target_arch = "wasm32"))]
mod tests {
    use super::*;
    use wasm_bindgen_test::wasm_bindgen_test;

    #[cfg(feature = "codegen-omniview")]
    #[wasm_bindgen_test]
    fn test_compile_to_omniview_full_success() {
        let result = compile_to_omniview_full(
            "component App { view { Text { \"hello\" } } }",
            false,
            false,
        );
        let parsed: serde_json::Value = serde_wasm_bindgen::from_value(result).unwrap();
        assert!(parsed["success"].as_bool().unwrap());
        assert!(parsed["files"]["js"].as_str().unwrap().len() > 0);
        assert!(parsed["files"]["html"].as_str().unwrap().len() > 0);
        // Should have IR
        assert!(parsed["ir"].is_object());
    }

    #[cfg(feature = "codegen-omniview")]
    #[wasm_bindgen_test]
    fn test_compile_to_omniview_full_parse_error() {
        let result = compile_to_omniview_full("this is invalid @@@", false, false);
        let parsed: serde_json::Value = serde_wasm_bindgen::from_value(result).unwrap();
        assert!(!parsed["success"].as_bool().unwrap());
        // The full pipeline always returns ir + files (parse-error recovery);
        // success=false + populated errors is the error contract.
        assert!(parsed["ir"].is_object());
        assert!(parsed["errors"].as_array().unwrap().len() > 0);
    }

    #[cfg(feature = "codegen-omniview")]
    #[wasm_bindgen_test]
    fn test_compile_to_omniview_full_editor_mode() {
        let result1 =
            compile_to_omniview_full("component App { view { Text { \"hi\" } } }", false, false);
        let result2 =
            compile_to_omniview_full("component App { view { Text { \"hi\" } } }", true, false);
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

    #[cfg(feature = "codegen-omniview")]
    #[wasm_bindgen_test]
    fn test_resolve_styles_multi_cached_envelope_shape() {
        // FFI contract: the cached style response must be a strongly-typed
        // envelope { result: StyleResolveResult, cache_state } — never a
        // serde_json::Value wrapper (which serializes to {} / Map and breaks
        // `response.result` on the JS side).
        let sources = serde_wasm_bindgen::to_value(&serde_json::json!({
            "main.omni": "import omniview;\ncomponent App { view { <div>hi</div> } }"
        }))
        .unwrap();
        let path_mapping =
            serde_wasm_bindgen::to_value(&serde_json::json!({})).unwrap();
        let result =
            resolve_styles_multi_cached(sources, path_mapping, JsValue::null());
        let parsed: serde_json::Value = serde_wasm_bindgen::from_value(result).unwrap();
        assert!(
            parsed["result"].is_object(),
            "envelope must have a `result` object, got: {parsed}"
        );
        assert!(parsed["result"]["element_tree"].is_array());
        assert!(parsed["cache_state"].is_null());
    }

    #[wasm_bindgen_test]
    fn test_compile_multi_to_omniview_single_file() {
        let sources = serde_wasm_bindgen::to_value(&serde_json::json!({
            "main.omni": "component App { view { Text { \"multi\" } } }"
        }))
        .unwrap();
        let result = compile_multi_to_omniview(sources, false, false, JsValue::null());
        let parsed: serde_json::Value = serde_wasm_bindgen::from_value(result).unwrap();
        // Result is wrapped: { result: ..., cache_state: ... }
        let inner = &parsed["result"];
        let has_output = inner["files"]
            .as_object()
            .map(|o| !o.is_empty())
            .unwrap_or(false);
        let has_errors = inner["errors"]
            .as_array()
            .map(|a| !a.is_empty())
            .unwrap_or(false);
        assert!(
            has_output || has_errors,
            "Expected either files or errors in multi compile result"
        );
        // Cache is NOT serialized per call anymore — explicit save_cache_state() API.
        assert!(
            parsed["cache_state"].is_null(),
            "Expected null cache_state (no per-call cache serialization)"
        );
    }

    #[wasm_bindgen_test]
    fn test_compile_multi_error_envelope_unified_shape() {
        // Synthetic boundary errors use the unified Diagnostic shape: the
        // 5 mandatory fields (severity/code/message/line/col) present,
        // lowercase severity; optional fields omitted when None.
        let sources = serde_wasm_bindgen::to_value("not a map").unwrap();
        let result = compile_multi_to_omniview(sources, false, false, JsValue::null());
        let parsed: serde_json::Value = serde_wasm_bindgen::from_value(result).unwrap();
        let errors = parsed["result"]["errors"].as_array().unwrap();
        assert!(!errors.is_empty());
        for e in errors {
            assert_diagnostic_shape(e);
        }
        assert_eq!(errors[0]["severity"], "error");
        assert_eq!(errors[0]["code"], "E_BOUNDARY");
    }

    #[wasm_bindgen_test]
    fn test_analyse_multi_cached_error_envelope_unified_shape() {
        let sources = serde_wasm_bindgen::to_value(&serde_json::json!({
            "main.omni": "this is invalid @@@"
        }))
        .unwrap();
        let path_mapping = serde_wasm_bindgen::to_value(&serde_json::json!({})).unwrap();
        let result = analyse_multi_cached(sources, path_mapping, JsValue::null());
        let parsed: serde_json::Value = serde_wasm_bindgen::from_value(result).unwrap();
        assert!(!parsed["success"].as_bool().unwrap());
        let errors = parsed["errors"].as_array().unwrap();
        assert!(!errors.is_empty());
        for e in errors {
            assert_diagnostic_json_shape(e);
        }
        // Parse diagnostics carry file attribution + spans.
        assert!(errors[0]["file"].is_string());
        assert!(errors[0]["end_line"].is_number());
        // Cache state stays null (no per-call serialization).
        assert!(parsed["cache_state"].is_null());
    }

    #[wasm_bindgen_test]
    fn test_omni_compiler_diagnostics_unified_shape() {
        let mut c = OmniCompiler::new();
        c.upsert_source(
            "app.omni".into(),
            "component App { view { Text { \"ok\" } } }".into(),
        );
        let d = c.diagnostics();
        let parsed: serde_json::Value = serde_wasm_bindgen::from_value(d).unwrap();
        assert!(parsed["success"].as_bool().unwrap());
        assert!(parsed["errors"].is_array());

        // Invalid source → errors carry the unified DiagnosticJson shape.
        let mut c2 = OmniCompiler::new();
        c2.upsert_source("bad.omni".into(), "not valid @@@".into());
        let d2 = c2.diagnostics();
        let parsed2: serde_json::Value = serde_wasm_bindgen::from_value(d2).unwrap();
        let errors = parsed2["errors"].as_array().unwrap();
        assert!(!errors.is_empty());
        for e in errors {
            assert_diagnostic_json_shape(e);
        }
    }

    #[wasm_bindgen_test]
    fn test_omni_compiler_diagnostics_includes_warnings() {
        // Lint warnings (unused import) must surface through diagnostics()
        // with their severity preserved — regression: routing through the
        // multi-file analysis path dropped `analysis.warnings` (the old
        // single-file path folded lint warnings into compile().errors, so
        // poiesis lost Monaco warning markers after the refactor).
        let mut c = OmniCompiler::new();
        c.upsert_source(
            "app.omni".into(),
            "import { array_length } from \"std/array\";\ncomponent App { view { <div>hi</div> } }"
                .into(),
        );
        let d = c.diagnostics();
        let parsed: serde_json::Value = serde_wasm_bindgen::from_value(d).unwrap();
        let errors = parsed["errors"].as_array().unwrap();
        assert!(
            errors.iter().any(|e| e["severity"] == "warning"),
            "expected a lint warning diagnostic through diagnostics(); got: {errors:?}"
        );
        for e in errors {
            assert_diagnostic_json_shape(e);
        }
    }

    #[wasm_bindgen_test]
    fn test_compile_multi_to_omniview_invalid_sources() {
        let sources = serde_wasm_bindgen::to_value("not a map").unwrap();
        let result = compile_multi_to_omniview(sources, false, false, JsValue::null());
        let parsed: serde_json::Value = serde_wasm_bindgen::from_value(result).unwrap();
        assert!(!parsed["result"]["success"].as_bool().unwrap());
    }

    #[wasm_bindgen_test]
    fn test_omni_compiler_new() {
        let c = OmniCompiler::new();
        assert_eq!(c.version(), env!("CARGO_PKG_VERSION"));
    }

    #[cfg(feature = "codegen-omniview")]
    #[wasm_bindgen_test]
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
    #[wasm_bindgen_test]
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
    #[wasm_bindgen_test]
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

    #[wasm_bindgen_test]
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

    // ── parse_source tests ─────────────────────────────────

    #[wasm_bindgen_test]
    fn test_parse_source_component() {
        let result = parse_source_internal("component App { view { Text { \"hello\" } } }");
        let v = serde_json::to_value(&result).unwrap();
        assert!(v["success"].as_bool().unwrap());
        let program = &v["program"];
        assert!(program.is_object());
        assert!(program["modules"].is_object());
    }

    #[wasm_bindgen_test]
    fn test_parse_source_function() {
        let result = parse_source_internal("pub fn add(a: i32, b: i32) -> i32 { return a + b; }");
        let v = serde_json::to_value(&result).unwrap();
        assert!(v["success"].as_bool().unwrap());
        assert!(v["program"]["modules"].is_object());
    }

    #[wasm_bindgen_test]
    fn test_parse_source_invalid_syntax() {
        let result = parse_source_internal("this is definitely not valid omni @@@");
        let v = serde_json::to_value(&result).unwrap();
        assert!(!v["success"].as_bool().unwrap());
        // The parser performs error recovery and returns a partial program —
        // success=false + populated errors is the error contract.
        let errors = v["errors"].as_array().unwrap();
        assert!(!errors.is_empty());
    }

    #[wasm_bindgen_test]
    fn test_parse_source_empty() {
        let result = parse_source_internal("");
        let v = serde_json::to_value(&result).unwrap();
        // Empty source is valid — produces empty module
        assert!(v["success"].as_bool().unwrap());
        assert!(v["program"]["modules"].is_object());
    }

    #[wasm_bindgen_test]
    fn test_parse_source_store() {
        let result = parse_source_internal("store Counter { signal count: i32 = 0 }");
        let v = serde_json::to_value(&result).unwrap();
        assert!(v["success"].as_bool().unwrap());
    }

    // ── Unified Diagnostic shapes ───────────────────────────

    /// Assert a diagnostic object has the unified 9-field envelope:
    /// severity (lowercase), code, message, file, line, col, end_line,
    /// end_col, suggestion — with explicit nulls for missing optionals.
    fn assert_diagnostic_json_shape(e: &serde_json::Value) {
        for field in [
            "severity",
            "code",
            "message",
            "file",
            "line",
            "col",
            "end_line",
            "end_col",
            "suggestion",
        ] {
            assert!(
                e.get(field).is_some(),
                "diagnostic missing field {field:?}: {e}"
            );
        }
        let sev = e["severity"].as_str().unwrap();
        assert!(
            sev == "error" || sev == "warning" || sev == "info",
            "severity must be lowercase, got {sev:?}: {e}"
        );
        assert_eq!(e["message"].as_str().unwrap().len() > 0, true);
    }

    #[wasm_bindgen_test]
    fn test_parse_source_error_unified_shape() {
        let result = parse_source_internal("this is definitely not valid omni @@@");
        let v = serde_json::to_value(&result).unwrap();
        assert!(!v["success"].as_bool().unwrap());
        let errors = v["errors"].as_array().unwrap();
        assert!(!errors.is_empty());
        for e in errors {
            assert_diagnostic_json_shape(e);
        }
        // E_PARSE synthetic + real parse errors both carry the full shape.
        assert_eq!(errors[0]["code"], "E_PARSE");
    }

    /// Assert a diagnostic has the mandatory 5-field shape used by
    /// `Diagnostic` (optional file/end_line/end_col/suggestion omitted when
    /// None): severity (lowercase), code, message, line, col.
    fn assert_diagnostic_shape(e: &serde_json::Value) {
        for field in ["severity", "code", "message", "line", "col"] {
            assert!(
                e.get(field).is_some(),
                "diagnostic missing field {field:?}: {e}"
            );
        }
        let sev = e["severity"].as_str().unwrap();
        assert!(
            sev == "error" || sev == "warning" || sev == "info",
            "severity must be lowercase, got {sev:?}: {e}"
        );
        assert_eq!(e["message"].as_str().unwrap().len() > 0, true);
    }

    #[wasm_bindgen_test]
    fn test_js_error_value_unified_shape() {
        let result = js_error_value("boom");
        let parsed: serde_json::Value = serde_wasm_bindgen::from_value(result).unwrap();
        assert!(!parsed["success"].as_bool().unwrap());
        let errors = parsed["errors"].as_array().unwrap();
        assert_eq!(errors.len(), 1);
        assert_diagnostic_json_shape(&errors[0]);
        assert_eq!(errors[0]["code"], "E_WASM");
        assert_eq!(errors[0]["message"], "boom");
    }
}
