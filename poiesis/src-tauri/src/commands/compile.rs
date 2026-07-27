//! Compile-project and compile-source commands.

use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};

use crate::state::{CachedFileList, ProjectState};

/// Structured diagnostic from the omnilang compiler (matches omnilang::error::Diagnostic JSON).
#[derive(Debug, Serialize, Deserialize)]
pub struct CompileDiagnostic {
    #[serde(default)]
    pub severity: String,
    #[serde(default)]
    pub code: String,
    pub message: String,
    #[serde(default)]
    pub file: Option<String>,
    #[serde(default)]
    pub line: u32,
    #[serde(default)]
    pub col: u32,
    #[serde(default)]
    pub end_line: u32,
    #[serde(default)]
    pub end_col: u32,
    #[serde(default)]
    pub suggestion: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CompileResult {
    pub success: bool,
    pub files: FxHashMap<String, String>,
    #[serde(default)]
    pub errors: Vec<CompileDiagnostic>,
    #[serde(default)]
    pub warnings: Vec<CompileDiagnostic>,
}

impl CompileResult {
    /// Create an error result from a single message string and error code.
    fn from_error(msg: &str, code: &str) -> Self {
        CompileResult {
            success: false,
            files: FxHashMap::default(),
            errors: vec![CompileDiagnostic {
                severity: "error".to_string(),
                code: code.to_string(),
                message: msg.to_string(),
                file: None,
                line: 0,
                col: 0,
                end_line: 0,
                end_col: 0,
                suggestion: None,
            }],
            warnings: vec![],
        }
    }

    /// Extract user-facing error messages from structured diagnostics.
    pub(crate) fn error_messages(&self) -> Vec<String> {
        self.errors
            .iter()
            .map(|d| {
                if let Some(ref file) = d.file {
                    format!("{}:{}: {}: {}", file, d.line, d.col, d.message)
                } else {
                    d.message.clone()
                }
            })
            .collect()
    }
}

/// Convert an omnilang diagnostic to the CompileDiagnostic struct.
fn omnilang_diagnostic_into_compile(d: omnilang::error::Diagnostic) -> CompileDiagnostic {
    CompileDiagnostic {
        severity: match d.severity {
            omnilang::error::Severity::Error => "error",
            omnilang::error::Severity::Warning => "warning",
            omnilang::error::Severity::Info => "info",
        }
        .to_string(),
        code: d.kind.code_str().to_string(),
        message: d.message,
        file: d.file,
        line: d.line,
        col: d.col,
        end_line: d.end_line.unwrap_or(0),
        end_col: d.end_col.unwrap_or(0),
        suggestion: d.suggestion,
    }
}

fn do_compile(
    sources: FxHashMap<String, String>,
    codegen_target: omnilang::codegen::Target,
    cache: Option<&mut omnilang::analyse::IncrementalCache>,
) -> CompileResult {
    // Check total input size
    let total_bytes: usize = sources.values().map(|s| s.len()).sum();
    const MAX_TOTAL_SIZE: usize = 10 * 1024 * 1024; // 10 MB
    if total_bytes > MAX_TOTAL_SIZE {
        return CompileResult {
            success: false,
            files: FxHashMap::default(),
            errors: vec![CompileDiagnostic {
                severity: "error".into(),
                code: "INPUT_TOO_LARGE".into(),
                message: format!("Total source size {} exceeds maximum of 10 MB", total_bytes),
                file: None,
                line: 0,
                col: 0,
                end_line: 0,
                end_col: 0,
                suggestion: None,
            }],
            warnings: vec![],
        };
    }

    let path_mapping = omnilang::path_mapping::derive_path_mapping(&sources);
    let output = match cache {
        Some(c) => omnilang::compile_multi_internal_cached(
            sources,
            codegen_target,
            path_mapping,
            false,
            false,
            None,
            c,
        ),
        None => omnilang::compile_multi_internal(
            sources,
            codegen_target,
            path_mapping,
            false,
            false,
            None,
        ),
    };
    CompileResult {
        success: output.success,
        files: output.files,
        errors: output
            .errors
            .into_iter()
            .map(omnilang_diagnostic_into_compile)
            .collect(),
        warnings: output
            .warnings
            .into_iter()
            .map(omnilang_diagnostic_into_compile)
            .collect(),
    }
}

#[tauri::command]
pub(crate) fn compile_project(
    project_path: String,
    target: String,
    state: tauri::State<ProjectState>,
) -> CompileResult {
    // Convert target string to codegen::Target
    let codegen_target: omnilang::codegen::Target = match target.parse() {
        Ok(t) => t,
        Err(e) => {
            return CompileResult::from_error(
                &format!("Unknown target: {} ({})", target, e),
                "E_TARGET",
            );
        }
    };

    // Discover .omni files in project directory
    let project_dir = std::path::Path::new(&project_path);
    let sources: FxHashMap<String, String>;

    if project_dir.is_dir() {
        // Check file list cache
        let root_mtime = std::fs::metadata(project_dir)
            .ok()
            .and_then(|m| m.modified().ok());
        if let Ok(cache) = state.file_list_cache.lock() {
            if let Some(ref cached) = *cache {
                if cached.root_mtime == root_mtime {
                    let mut compile_cache = state
                        .compile_cache
                        .lock()
                        .unwrap_or_else(|e| e.into_inner());
                    return do_compile(
                        cached.sources.clone(),
                        codegen_target,
                        Some(&mut *compile_cache),
                    );
                }
            }
        }

        match omnilang::discovery::discover_omni_sources(
            project_dir,
            omnilang::discovery::DiscoveryMode::Directory,
        ) {
            Ok(s) => sources = s,
            Err(e) => {
                return CompileResult::from_error(&e, "E_DISCOVERY");
            }
        }

        // Update file list cache
        if let Ok(mut cache) = state.file_list_cache.lock() {
            *cache = Some(CachedFileList {
                sources: sources.clone(),
                root_mtime,
            });
        }
    } else if project_dir.is_file() && project_dir.extension().is_some_and(|e| e == "omni") {
        match omnilang::discovery::discover_omni_sources(
            project_dir,
            omnilang::discovery::DiscoveryMode::SingleFileWithSiblings,
        ) {
            Ok(s) => sources = s,
            Err(e) => {
                return CompileResult::from_error(&e, "E_DISCOVERY");
            }
        }
    } else {
        return CompileResult::from_error(
            &format!("Project path not found: {}", project_path),
            "E_NOT_FOUND",
        );
    }

    if sources.is_empty() {
        return CompileResult::from_error("No .omni source files found.", "E_IO");
    }

    let mut compile_cache = state
        .compile_cache
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    do_compile(sources, codegen_target, Some(&mut *compile_cache))
}

/// Compile a single .omni source string via Tauri IPC (fallback for WASM).
/// Used when the WASM compiler module is unavailable in the Tauri webview.
#[tauri::command]
pub(crate) fn compile_source(source: String, target: Option<String>) -> CompileResult {
    let target_str = target.unwrap_or_else(|| "react".to_string());
    let codegen_target: omnilang::codegen::Target = match target_str.parse() {
        Ok(t) => t,
        Err(e) => {
            return CompileResult::from_error(
                &format!("Unknown target: {} ({})", target_str, e),
                "E_TARGET",
            );
        }
    };

    let sources = FxHashMap::from_iter([("main.omni".to_string(), source)]);
    do_compile(sources, codegen_target, None)
}
