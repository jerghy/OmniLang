//! Project lifecycle, file-system, and export commands.

use serde::{Deserialize, Serialize};

use crate::commands::compile::compile_project;
use crate::path::sanitize_path;
use crate::state::ProjectState;
use crate::MAX_SOURCE_SIZE;

#[derive(Debug, Serialize, Deserialize)]
pub struct ExportResult {
    pub success: bool,
    pub output_dir: String,
    pub file_count: u32,
    pub errors: Vec<String>,
}

#[tauri::command]
pub(crate) fn project_export(
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

#[tauri::command]
pub(crate) fn get_project_dir(state: tauri::State<ProjectState>) -> Result<String, String> {
    state
        .project_dir
        .lock()
        .map_err(|e| e.to_string())?
        .clone()
        .ok_or_else(|| "No project open".to_string())
}

// ── Path validation helpers ──

/// Check if a canonicalized path resides within a system directory.
#[cfg(target_os = "windows")]
fn is_in_system_directory(path: &std::path::Path) -> bool {
    // Windows canonicalize may prepend \\?\ for long-path support
    let raw = path.to_string_lossy();
    let norm = raw.strip_prefix(r"\\?\").unwrap_or(&raw).to_lowercase();
    let bad_prefixes: &[&str] = &[
        "c:\\windows",
        "c:\\program files",
        "c:\\program files (x86)",
        "c:\\system32",
    ];
    bad_prefixes
        .iter()
        .any(|prefix| norm == *prefix || norm.starts_with(&format!("{}\\", prefix)))
}

/// Check if a canonicalized path resides within a system directory.
#[cfg(not(target_os = "windows"))]
fn is_in_system_directory(path: &std::path::Path) -> bool {
    let norm = path.to_string_lossy().to_lowercase();
    let bad_prefixes: &[&str] = &["/etc", "/usr", "/bin", "/sbin", "/sys", "/proc", "/dev"];
    bad_prefixes
        .iter()
        .any(|prefix| norm == *prefix || norm.starts_with(&format!("{}/", prefix)))
}

/// Collect allowed project root directories from environment variables.
fn get_allowed_directories() -> Vec<std::path::PathBuf> {
    let mut dirs: Vec<std::path::PathBuf> = Vec::new();

    let mut try_add = |var: &str| {
        if let Ok(val) = std::env::var(var) {
            if val.is_empty() {
                return;
            }
            let p = std::path::Path::new(&val);
            // Try canonicalized version; fall back to raw path
            match p.canonicalize() {
                Ok(c) => {
                    if !dirs.contains(&c) {
                        dirs.push(c);
                    }
                }
                Err(_) => {
                    let buf = p.to_path_buf();
                    if !dirs.contains(&buf) {
                        dirs.push(buf);
                    }
                }
            }
        }
    };

    try_add("HOME");
    #[cfg(target_os = "windows")]
    {
        try_add("APPDATA");
        try_add("USERPROFILE");
        // Per ADR-004 Decision 2: FS全覆盖 is an editor requirement.
        // Capabilities allow D:/** and C:** — align path validation.
        for drive in &["C:\\", "D:\\"] {
            let p = std::path::Path::new(drive);
            if p.exists() && !dirs.contains(&p.to_path_buf()) {
                dirs.push(p.to_path_buf());
            }
        }
    }

    dirs
}

#[tauri::command]
pub(crate) fn open_project(
    path: String,
    state: tauri::State<ProjectState>,
) -> Result<serde_json::Value, String> {
    let p = std::path::Path::new(&path);
    if !p.exists() {
        return Err(format!("Project path not found: {}", path));
    }
    let canonical = p
        .canonicalize()
        .map_err(|e| format!("Failed to resolve project path '{}': {e}", path))?;

    // Security: reject system directories
    if is_in_system_directory(&canonical) {
        return Err("System directories are not allowed as project locations".to_string());
    }

    // Security: require project directory within user's home or appdata
    let allowed = get_allowed_directories();
    if allowed.is_empty() {
        return Err("Project must be within your home directory, but $HOME is not set".to_string());
    }
    if !allowed.iter().any(|dir| canonical.starts_with(dir)) {
        return Err("Project must be within your home directory".to_string());
    }

    let canonical_str = canonical.to_string_lossy().to_string();
    *state.project_dir.lock().map_err(|e| e.to_string())? = Some(canonical_str.clone());
    Ok(serde_json::json!({"success": true, "path": canonical_str}))
}

#[tauri::command]
pub(crate) fn save_project(state: tauri::State<ProjectState>) -> Result<serde_json::Value, String> {
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
pub(crate) fn close_project(
    state: tauri::State<ProjectState>,
) -> Result<serde_json::Value, String> {
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

#[tauri::command]
pub(crate) fn read_file(path: String, state: tauri::State<ProjectState>) -> Result<String, String> {
    let dir = state
        .project_dir
        .lock()
        .map_err(|e| e.to_string())?
        .clone()
        .ok_or_else(|| "No project open".to_string())?;
    let full = sanitize_path(std::path::Path::new(&dir), &path)?;
    std::fs::read_to_string(&full).map_err(|e| format!("Failed to read {}: {}", full.display(), e))
}

#[tauri::command]
pub(crate) fn write_file(
    path: String,
    content: String,
    state: tauri::State<ProjectState>,
) -> Result<serde_json::Value, String> {
    if content.len() > MAX_SOURCE_SIZE {
        return Err(format!(
            "Content exceeds maximum size of {} bytes",
            MAX_SOURCE_SIZE
        ));
    }
    let dir = state
        .project_dir
        .lock()
        .map_err(|e| e.to_string())?
        .clone()
        .ok_or_else(|| "No project open".to_string())?;
    let full = sanitize_path(std::path::Path::new(&dir), &path)?;
    if let Some(parent) = full.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("Failed to create dir: {}", e))?;
    }
    std::fs::write(&full, &content)
        .map_err(|e| format!("Failed to write {}: {}", full.display(), e))?;
    Ok(serde_json::json!({"success": true}))
}
