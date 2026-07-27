// Tauri plugin commands — replaces Python FastAPI backend
// Routes compilation through the omnilang Rust crate for server-side operations.
// Frontend compilation goes through WASM (rustCompiler.ts); these commands provide
// complementary server-side support (project management, export, analysis).

use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use tauri::Manager;

// ── Compile project ──

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
    fn error_messages(&self) -> Vec<String> {
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

/// Convert an omnilang diagnostic to a JSON Value (useful for FFI or dynamic output).
#[allow(dead_code)]
fn diagnostic_to_json(d: &omnilang::error::Diagnostic) -> serde_json::Value {
    serde_json::json!({
        "severity": match d.severity {
            omnilang::error::Severity::Error => "error",
            omnilang::error::Severity::Warning => "warning",
            omnilang::error::Severity::Info => "info",
        },
        "code": d.kind.code_str(),
        "message": d.message,
        "file": d.file,
        "line": d.line,
        "col": d.col,
        "suggestion": d.suggestion,
    })
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
        Some(c) => {
            omnilang::compile_multi_internal_cached(sources, codegen_target, path_mapping, false, false, None, c)
        }
        None => omnilang::compile_multi_internal(sources, codegen_target, path_mapping, false, false, None),
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
fn compile_project(
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
    let mut sources: FxHashMap<String, String> = FxHashMap::default();

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
    } else if project_dir.is_file()
        && project_dir.extension().is_some_and(|e| e == "omni")
    {
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

// ── Analyse source ──

#[derive(Debug, Serialize, Deserialize)]
pub struct AnalyseResult {
    pub success: bool,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Diagnostic {
    pub severity: String,
    pub code: String,
    pub message: String,
    pub file: String,
    pub line: u32,
    pub col: u32,
}

const MAX_SOURCE_SIZE: usize = 10 * 1024 * 1024; // 10 MB

#[tauri::command]
fn analyse_source(source: String, file_path: String) -> AnalyseResult {
    if source.len() > MAX_SOURCE_SIZE {
        return AnalyseResult {
            success: false,
            diagnostics: vec![Diagnostic {
                severity: "error".to_string(),
                code: "INPUT_TOO_LARGE".to_string(),
                message: format!("Source exceeds maximum size of {} bytes", MAX_SOURCE_SIZE),
                file: file_path,
                line: 0,
                col: 0,
            }],
        };
    }
    let tokens = omnilang::lexer::tokenize(&source);
    let program = match omnilang::parser::parse(tokens, &source) {
        Ok(p) => p,
        Err(e) => {
            return AnalyseResult {
                success: false,
                diagnostics: vec![Diagnostic {
                    severity: "error".to_string(),
                    code: "PARSE_ERROR".to_string(),
                    message: e.message,
                    file: file_path,
                    line: e.span.line,
                    col: e.span.col,
                }],
            };
        }
    };

    let semantics = omnilang::semantic::analyze(&program);
    let diagnostics: Vec<Diagnostic> = semantics
        .diagnostics
        .iter()
        .map(|d| Diagnostic {
            severity: match d.severity {
                omnilang::error::Severity::Error => "error".to_string(),
                omnilang::error::Severity::Warning => "warning".to_string(),
                omnilang::error::Severity::Info => "info".to_string(),
            },
            code: d.kind.code_str().to_string(),
            message: d.message.clone(),
            file: file_path.clone(),
            line: d.line,
            col: d.col,
        })
        .collect();

    let has_errors = diagnostics.iter().any(|d| d.severity == "error");
    AnalyseResult {
        success: !has_errors,
        diagnostics,
    }
}

// ── Project export ──

#[derive(Debug, Serialize, Deserialize)]
pub struct ExportResult {
    pub success: bool,
    pub output_dir: String,
    pub file_count: u32,
    pub errors: Vec<String>,
}

#[tauri::command]
fn project_export(
    project_path: String,
    output_dir: String,
    target: String,
    state: tauri::State<ProjectState>,
) -> ExportResult {
    // Validate output_dir
    if output_dir.is_empty() {
        return ExportResult {
            success: false,
            output_dir,
            file_count: 0,
            errors: vec!["Output directory cannot be empty".into()],
        };
    }
    let out_path_check = std::path::Path::new(&output_dir);
    if out_path_check.is_absolute() {
        return ExportResult {
            success: false,
            output_dir,
            file_count: 0,
            errors: vec!["Absolute paths are not allowed for output directory".into()],
        };
    }
    if output_dir.contains("..") {
        return ExportResult {
            success: false,
            output_dir,
            file_count: 0,
            errors: vec!["Path traversal detected in output directory".into()],
        };
    }

    // First compile the project
    let compile_result = compile_project(project_path.clone(), target.clone(), state);

    if !compile_result.success {
        return ExportResult {
            success: false,
            output_dir: output_dir.clone(),
            file_count: 0,
            errors: compile_result.error_messages(),
        };
    }

    // ── Path traversal protection ──
    // Canonicalize the project directory (compile succeeded, so it exists)
    let project_canonical = match std::path::Path::new(&project_path).canonicalize() {
        Ok(p) => p,
        Err(e) => {
            return ExportResult {
                success: false,
                output_dir: output_dir.clone(),
                file_count: 0,
                errors: vec![format!("Cannot resolve project path: {}", e)],
            };
        }
    };

    // The output directory is relative to the project directory
    let out_absolute = project_canonical.join(&output_dir);

    // Create the output directory before canonicalizing it
    if let Err(e) = std::fs::create_dir_all(&out_absolute) {
        return ExportResult {
            success: false,
            output_dir: output_dir.clone(),
            file_count: 0,
            errors: vec![format!("Cannot create output directory: {}", e)],
        };
    }

    // Canonicalize to resolve any symlinks or path components
    let out_canonical = match out_absolute.canonicalize() {
        Ok(p) => p,
        Err(e) => {
            return ExportResult {
                success: false,
                output_dir: output_dir.clone(),
                file_count: 0,
                errors: vec![format!("Cannot resolve output directory: {}", e)],
            };
        }
    };

    // Verify the output directory stays within the project directory
    if !out_canonical.starts_with(&project_canonical) {
        return ExportResult {
            success: false,
            output_dir: output_dir.clone(),
            file_count: 0,
            errors: vec!["Output directory is outside the project directory".into()],
        };
    }

    // Write compiled files to output directory
    let mut written = 0u32;
    let mut write_errors: Vec<String> = Vec::new();

    for (filename, content) in &compile_result.files {
        // Validate each filename for path traversal
        if filename.contains("..") || std::path::Path::new(filename).is_absolute() {
            write_errors.push(format!("Invalid filename in compile result: {}", filename));
            continue;
        }
        let file_path = out_canonical.join(filename);
        if let Some(parent) = file_path.parent() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                write_errors.push(format!("Cannot create directory {:?}: {}", parent, e));
                continue;
            }
        }
        match std::fs::write(&file_path, content) {
            Ok(_) => written += 1,
            Err(e) => {
                write_errors.push(format!("Cannot write {:?}: {}", file_path, e));
            }
        }
    }

    ExportResult {
        success: write_errors.is_empty(),
        output_dir,
        file_count: written,
        errors: write_errors,
    }
}

// ── Project state management ──

use std::sync::Mutex;
use std::time::SystemTime;

struct CachedFileList {
    sources: FxHashMap<String, String>,
    root_mtime: Option<SystemTime>,
}

struct ProjectState {
    project_dir: Mutex<Option<String>>,
    file_list_cache: Mutex<Option<CachedFileList>>,
    compile_cache: Mutex<omnilang::analyse::IncrementalCache>,
    store_snapshot: Mutex<String>,
    pid_index: Mutex<String>,
}

impl Default for ProjectState {
    fn default() -> Self {
        Self {
            project_dir: Mutex::new(None),
            file_list_cache: Mutex::new(None),
            compile_cache: Mutex::new(omnilang::analyse::IncrementalCache::new()),
            store_snapshot: Mutex::new(String::new()),
            pid_index: Mutex::new("{}".to_string()),
        }
    }
}

// ── Project lifecycle commands ──

#[tauri::command]
fn get_project_dir(state: tauri::State<ProjectState>) -> Result<String, String> {
    state
        .project_dir
        .lock()
        .map_err(|e| e.to_string())?
        .clone()
        .ok_or_else(|| "No project open".to_string())
}

#[tauri::command]
fn open_project(
    path: String,
    state: tauri::State<ProjectState>,
) -> Result<serde_json::Value, String> {
    let p = std::path::Path::new(&path);
    if !p.exists() {
        return Err(format!("Project path not found: {}", path));
    }
    let canonical = p
        .canonicalize()
        .map_err(|e| format!("Failed to resolve project path '{}': {e}", path))?
        .to_string_lossy()
        .to_string();
    *state.project_dir.lock().map_err(|e| e.to_string())? = Some(canonical.clone());
    Ok(serde_json::json!({"success": true, "path": canonical}))
}

#[tauri::command]
fn save_project(state: tauri::State<ProjectState>) -> Result<serde_json::Value, String> {
    let dir = state
        .project_dir
        .lock()
        .map_err(|e| e.to_string())?
        .clone()
        .ok_or_else(|| "No project open".to_string())?;
    let snapshot = state
        .store_snapshot
        .lock()
        .map_err(|e| e.to_string())?
        .clone();
    let project_json_path = std::path::Path::new(&dir).join("project.json");
    std::fs::write(&project_json_path, &snapshot)
        .map_err(|e| format!("Failed to write project.json: {}", e))?;
    Ok(serde_json::json!({"success": true}))
}

#[tauri::command]
fn close_project(state: tauri::State<ProjectState>) -> Result<serde_json::Value, String> {
    *state.project_dir.lock().map_err(|e| e.to_string())? = None;
    *state.file_list_cache.lock().map_err(|e| e.to_string())? = None;
    state
        .compile_cache
        .lock()
        .map_err(|e| e.to_string())?
        .clear();
    *state.store_snapshot.lock().map_err(|e| e.to_string())? = String::new();
    *state.pid_index.lock().map_err(|e| e.to_string())? = "{}".to_string();
    Ok(serde_json::json!({"success": true}))
}

// ── File system commands ──

#[tauri::command]
fn read_file(path: String, state: tauri::State<ProjectState>) -> Result<String, String> {
    let dir = state
        .project_dir
        .lock()
        .map_err(|e| e.to_string())?
        .clone()
        .ok_or_else(|| "No project open".to_string())?;
    // Prevent path traversal via canonicalize + prefix verification.
    let full = sanitize_path(std::path::Path::new(&dir), &path)?;
    std::fs::read_to_string(&full).map_err(|e| format!("Failed to read {}: {}", full.display(), e))
}

/// Validate and canonicalize a user-supplied relative path against a base directory.
/// Returns the canonicalized path if it is within the base directory.
/// Resolves symlinks, `..`, and other path traversal attempts.
///
/// # Safety note (TOCTOU)
/// There is a time-of-check-time-of-use window between path validation and
/// the subsequent file operation. This is acceptable because the project
/// directory is assumed to be under the user's control and not shared with
/// untrusted writers. On production deployments, consider using handle-based
/// operations (e.g. O_NOFOLLOW on Unix) for stronger guarantees.
#[must_use = "sanitized path must be used or path traversal checks are bypassed"]
fn sanitize_path(base: &std::path::Path, user_path: &str) -> Result<std::path::PathBuf, String> {
    // Reject empty paths
    if user_path.is_empty() {
        return Err("Empty path is not allowed".to_string());
    }

    // Reject paths that are already absolute
    let p = std::path::Path::new(user_path);
    if p.is_absolute() {
        return Err("Absolute paths are not allowed".to_string());
    }

    // Join with base and canonicalize (resolves symlinks, .., etc.)
    let full = base.join(p);

    // Canonicalize the base for comparison (must exist)
    let base_canonical = base
        .canonicalize()
        .map_err(|e| format!("Base directory error: {}", e))?;

    // Try canonicalizing the full path first (works for existing files)
    // If the file doesn't exist yet, canonicalize the parent directory
    // and append the filename (handles write_file creating new files)
    let resolved = full.canonicalize().or_else(|_| {
        full.parent()
            .and_then(|parent| parent.canonicalize().ok())
            .map(|parent_canon| {
                parent_canon.join(full.file_name().unwrap_or(std::ffi::OsStr::new("unknown")))
            })
            .ok_or_else(|| format!("Invalid path (cannot resolve): {}", full.display()))
    })?;

    // Verify the resolved path is within the base directory
    if !resolved.starts_with(&base_canonical) {
        return Err("Path traversal detected".to_string());
    }

    Ok(resolved)
}

#[tauri::command]
fn write_file(
    path: String,
    content: String,
    state: tauri::State<ProjectState>,
) -> Result<serde_json::Value, String> {
    if content.len() > MAX_SOURCE_SIZE {
        return Err(format!("Content exceeds maximum size of {} bytes", MAX_SOURCE_SIZE));
    }
    let dir = state
        .project_dir
        .lock()
        .map_err(|e| e.to_string())?
        .clone()
        .ok_or_else(|| "No project open".to_string())?;
    // Prevent path traversal via canonicalize + prefix verification.
    let full = sanitize_path(std::path::Path::new(&dir), &path)?;
    if let Some(parent) = full.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("Failed to create dir: {}", e))?;
    }
    std::fs::write(&full, &content)
        .map_err(|e| format!("Failed to write {}: {}", full.display(), e))?;
    Ok(serde_json::json!({"success": true}))
}

// ── State sync commands ──

#[tauri::command]
fn sync_store_state(
    store_snapshot: String,
    pid_index: String,
    state: tauri::State<ProjectState>,
) -> Result<(), String> {
    *state.store_snapshot.lock().map_err(|e| e.to_string())? = store_snapshot;
    *state.pid_index.lock().map_err(|e| e.to_string())? = pid_index;
    Ok(())
}

#[tauri::command]
fn get_store_snapshot(state: tauri::State<ProjectState>) -> Result<String, String> {
    let snap = state
        .store_snapshot
        .lock()
        .map_err(|e| e.to_string())?
        .clone();
    if snap.is_empty() {
        // If no state synced yet, return empty JSON object
        Ok("{}".to_string())
    } else {
        Ok(snap)
    }
}

#[tauri::command]
fn get_pid_index(state: tauri::State<ProjectState>) -> Result<String, String> {
    Ok(state.pid_index.lock().map_err(|e| e.to_string())?.clone())
}

// ── PID verification ──

#[derive(Debug, serde::Serialize)]
#[allow(dead_code)]
struct PidRoundtripResult {
    consistent: bool,
    pids: Vec<String>,
}

#[tauri::command]
fn verify_pid_roundtrip(
    file: String,
    state: tauri::State<ProjectState>,
) -> Result<serde_json::Value, String> {
    let dir = state
        .project_dir
        .lock()
        .map_err(|e| e.to_string())?
        .clone()
        .ok_or_else(|| "No project open".to_string())?;
    // Prevent path traversal via canonicalize + prefix verification.
    let full = sanitize_path(std::path::Path::new(&dir), &file)?;
    let source = std::fs::read_to_string(&full)
        .map_err(|e| format!("Failed to read {}: {}", full.display(), e))?;

    // Parse using omnilang
    let tokens = omnilang::lexer::tokenize(&source);
    let program = match omnilang::parser::parse(tokens, &source) {
        Ok(p) => p,
        Err(e) => {
            return Ok(serde_json::json!({
                "consistent": false,
                "pids": [],
                "error": e.message
            }));
        }
    };

    // Extract @pid annotations from comments in source
    let mut pids_from_comments: Vec<String> = Vec::new();
    for line in source.lines() {
        let trimmed = line.trim();
        if let Some(pos) = trimmed.find("// @pid:") {
            let pid = trimmed[pos + 8..].trim().to_string();
            if !pid.is_empty() && !pids_from_comments.contains(&pid) {
                pids_from_comments.push(pid);
            }
        }
    }

    // Count components in the parsed program
    let mut component_count = 0usize;
    for module in program.modules.values() {
        component_count += module.components.len();
    }

    // Consistent if: file parses successfully AND has @pid annotations
    let consistent = !pids_from_comments.is_empty();

    Ok(serde_json::json!({
        "consistent": consistent,
        "pids": pids_from_comments,
        "component_count": component_count,
    }))
}

// ── Debug window commands ──
// P18(Sprint 5): move to commands/debug.rs

/// Open the debug window. Creates it if it doesn't exist yet.
#[tauri::command]
fn open_debug_window(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("debug") {
        window.show().map_err(|e| e.to_string())?;
        window.set_focus().map_err(|e| e.to_string())?;
        return Ok(());
    }
    // Window was not pre-created — create it now
    tauri::WebviewWindowBuilder::new(
        &app,
        "debug",
        tauri::WebviewUrl::App("debug-window/index.html".into()),
    )
    .title("Poiesis Debug")
    .inner_size(1400.0, 900.0)
    .resizable(true)
    .visible(true)
    .build()
    .map_err(|e| format!("Failed to create debug window: {e}"))?;
    Ok(())
}

/// Hide the debug window (never destroy — recreate blocks IPC).
#[tauri::command]
fn close_debug_window(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("debug") {
        window.hide().map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Append a line to the debug log file for frontend debugging.
#[tauri::command]
fn dw_log(message: String) -> Result<(), String> {
    use std::io::Write;
    use std::time::{SystemTime, UNIX_EPOCH};
    let path = std::env::temp_dir().join("poiesis-debug.log");
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|e| format!("Failed to open log file: {e}"))?;
    let ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let secs = (ms / 1000) % 86400;
    let h = secs / 3600;
    let m = (secs % 3600) / 60;
    let s = secs % 60;
    let millis = ms % 1000;
    writeln!(file, "[{h:02}:{m:02}:{s:02}.{millis:03}] {message}")
        .map_err(|e| format!("Failed to write log: {e}"))?;
    Ok(())
}
// P18(Sprint 5): move to commands/debug.rs

// ── Tauri app entry ──

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(
            tauri_plugin_window_state::Builder::default()
                .with_denylist(&["debug"])
                .build(),
        )
        .plugin(tauri_plugin_store::Builder::default().build())
        .manage(ProjectState::default())
        .setup(|app| {
            // Pre-create the debug window hidden so open_debug_window can just show it.
            // WebviewWindowBuilder::build() blocks IPC responses on Windows,
            // so we create it here during setup (no IPC involved).
            // If it fails, open_debug_window will create it on demand.
            if let Err(e) = tauri::WebviewWindowBuilder::new(
                app,
                "debug",
                tauri::WebviewUrl::App("debug-window/index.html".into()),
            )
            .title("Poiesis Debug")
            .inner_size(1400.0, 900.0)
            .resizable(true)
            .visible(false)
            .build()
            {
                eprintln!("[Poiesis] Failed to pre-create debug window: {e}");
            }
            Ok(())
        })
        // Never destroy the debug window — hide it instead.
        // Recreating via WebviewWindowBuilder::build() blocks IPC responses
        // on Windows, which freezes the main window.
        .on_window_event(|window, event| {
            if window.label() == "debug" {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    window.hide().ok();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            compile_project,
            analyse_source,
            project_export,
            get_project_dir,
            open_project,
            save_project,
            close_project,
            read_file,
            write_file,
            sync_store_state,
            get_store_snapshot,
            get_pid_index,
            verify_pid_roundtrip,
            open_debug_window,
            close_debug_window,
            dw_log,
        ]);

    #[cfg(debug_assertions)]
    let app = app.plugin(
        victauri_plugin::VictauriBuilder::new()
            .auth_disabled()
            .build()
            .expect("Failed to build Victauri plugin"),
    );

    let app = app
        .build(tauri::generate_context!())
        .expect("error while building Poiesis");
    app.run(|_app_handle, _event| {});
}
