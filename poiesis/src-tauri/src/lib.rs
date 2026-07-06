// Tauri plugin commands — replaces Python FastAPI backend
// Routes compilation through the omnilang Rust crate for server-side operations.
// Frontend compilation goes through WASM (rustCompiler.ts); these commands provide
// complementary server-side support (project management, export, analysis).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
    pub files: HashMap<String, String>,
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
            files: HashMap::new(),
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

    /// Create an error result from multiple message strings.
    fn from_errors(errors: Vec<String>, code: &str) -> Self {
        CompileResult {
            success: false,
            files: HashMap::new(),
            errors: errors
                .into_iter()
                .map(|msg| CompileDiagnostic {
                    severity: "error".to_string(),
                    code: code.to_string(),
                    message: msg,
                    file: None,
                    line: 0,
                    col: 0,
                    end_line: 0,
                    end_col: 0,
                    suggestion: None,
                })
                .collect(),
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

#[tauri::command]
fn compile_project(project_path: String, target: String) -> CompileResult {
    // Convert target string to codegen::Target
    let codegen_target = match target.as_str() {
        "python" => omnilang::codegen::Target::Python,
        "js" | "javascript" => omnilang::codegen::Target::JavaScript,
        "ts" | "typescript" => omnilang::codegen::Target::TypeScript,
        "rust" | "rs" => omnilang::codegen::Target::Rust,
        "dart" => omnilang::codegen::Target::Dart,
        "react" => omnilang::codegen::Target::React,
        "vue" => omnilang::codegen::Target::Vue,
        "dioxus" => omnilang::codegen::Target::Dioxus,
        "flutter" => omnilang::codegen::Target::Flutter,
        "omniview" => omnilang::codegen::Target::OmniView,
        _ => {
            return CompileResult {
                success: false,
                files: HashMap::new(),
                errors: vec![CompileDiagnostic {
                    severity: "error".to_string(),
                    code: "E_TARGET".to_string(),
                    message: format!("Unknown target: {}", target),
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
    };

    // Discover .omni files in project directory
    let project_dir = std::path::Path::new(&project_path);
    let mut sources: HashMap<String, String> = HashMap::new();
    let mut discover_errors: Vec<String> = Vec::new();

    if project_dir.is_dir() {
        for entry in walkdir::WalkDir::new(project_dir)
            .into_iter()
            .filter_map(|e| match e {
                Ok(entry) => Some(entry),
                Err(err) => {
                    eprintln!("[omni] warning: skipping inaccessible entry: {}", err);
                    None
                }
            })
            .filter(|e| e.file_name().to_string_lossy().ends_with(".omni"))
        {
            let path = entry.path().to_owned();
            match std::fs::read_to_string(&path) {
                Ok(content) => {
                    let rel_path = path
                        .strip_prefix(project_dir)
                        .unwrap_or(&path)
                        .to_string_lossy()
                        .to_string();
                    sources.insert(rel_path, content);
                }
                Err(e) => {
                    discover_errors.push(format!("Cannot read {:?}: {}", path, e));
                }
            }
        }
    } else if project_dir.is_file() && project_dir.extension().is_some_and(|e| e == "omni") {
        // Also discover sibling .omni files for import resolution
        let _file_stem = project_dir
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
        let name = project_dir
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
        if name.is_empty() {
            return CompileResult::from_error("Invalid file path: no filename", "E_IO");
        }
        match std::fs::read_to_string(project_dir) {
            Ok(content) => {
                sources.insert(name, content);
            }
            Err(e) => {
                return CompileResult::from_error(
                    &format!("Cannot read {:?}: {}", project_dir, e),
                    "E_IO",
                );
            }
        }
        // Walk sibling directories to resolve imports
        if let Some(parent) = project_dir.parent() {
            let parent = if parent.as_os_str().is_empty() {
                std::path::Path::new(".")
            } else {
                parent
            };
            for entry in walkdir::WalkDir::new(parent)
                .into_iter()
                .filter_map(|e| match e {
                    Ok(entry) => Some(entry),
                    Err(err) => {
                        eprintln!("[omni] warning: skipping inaccessible entry: {}", err);
                        None
                    }
                })
                .filter(|e| {
                    e.file_name().to_string_lossy().ends_with(".omni") && e.path() != project_dir
                })
            {
                let path = entry.path().to_owned();
                if let Ok(content) = std::fs::read_to_string(&path) {
                    let rel_path = path
                        .strip_prefix(parent)
                        .unwrap_or(&path)
                        .to_string_lossy()
                        .to_string();
                    sources.insert(rel_path, content);
                }
            }
        }
    } else {
        return CompileResult::from_error(
            &format!("Project path not found: {}", project_path),
            "E_NOT_FOUND",
        );
    }

    if sources.is_empty() {
        let mut all_errors = discover_errors;
        all_errors.push("No .omni source files found.".to_string());
        return CompileResult::from_errors(all_errors, "E_IO");
    }

    // Build path mapping for cross-file imports
    let path_mapping = omnilang::path_mapping::derive_path_mapping(&sources);

    // Compile via omnilang crate
    let result_json =
        omnilang::compile_multi_to_target(sources, codegen_target, path_mapping, false);

    // Parse result JSON
    match serde_json::from_str::<CompileResult>(&result_json) {
        Ok(result) => result,
        Err(e) => {
            CompileResult::from_error(&format!("Failed to parse compile result: {}", e), "E_PARSE")
        }
    }
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

#[tauri::command]
fn analyse_source(source: String, file_path: String) -> AnalyseResult {
    let tokens = omnilang::lexer::tokenize(&source);
    let program = match omnilang::parser::parse(&tokens, &source) {
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
            code: d.code.clone(),
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
fn project_export(project_path: String, output_dir: String, target: String) -> ExportResult {
    // First compile the project
    let compile_result = compile_project(project_path.clone(), target.clone());

    if !compile_result.success {
        return ExportResult {
            success: false,
            output_dir: output_dir.clone(),
            file_count: 0,
            errors: compile_result.error_messages(),
        };
    }

    // Write compiled files to output directory
    let out_path = std::path::Path::new(&output_dir);
    if let Err(e) = std::fs::create_dir_all(out_path) {
        return ExportResult {
            success: false,
            output_dir,
            file_count: 0,
            errors: vec![format!("Cannot create output directory: {}", e)],
        };
    }

    let mut written = 0u32;
    let mut write_errors: Vec<String> = Vec::new();

    for (filename, content) in &compile_result.files {
        let file_path = out_path.join(filename);
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

struct ProjectState {
    project_dir: Mutex<Option<String>>,
    store_snapshot: Mutex<String>,
    pid_index: Mutex<String>,
}

impl Default for ProjectState {
    fn default() -> Self {
        Self {
            project_dir: Mutex::new(None),
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
        .unwrap_or_else(|_| p.to_path_buf())
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
    let program = match omnilang::parser::parse(&tokens, &source) {
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

// ── Tauri app entry ──

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .plugin(tauri_plugin_store::Builder::default().build())
        .manage(ProjectState::default())
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
        ])
        .plugin(victauri_plugin::init())
        .run(tauri::generate_context!())
        .expect("error while running Poiesis");
}
