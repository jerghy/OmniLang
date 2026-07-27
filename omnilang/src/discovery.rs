//! OmniLang source file discovery.
//! Shared between omnic CLI, Poiesis Tauri backend, and WASM API.

use rustc_hash::FxHashMap;
use std::path::Path;

/// How to discover .omni source files.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiscoveryMode {
    /// Walk the entire directory tree for `**/*.omni`.
    Directory,
    /// Single file + sibling .omni files in the same tree (for import resolution).
    SingleFileWithSiblings,
}

/// Discover .omni source files and return them as (relative_path → content).
///
/// For Directory mode, paths are relative to `root`.
/// For SingleFileWithSiblings mode, the target file is keyed by its basename,
/// and sibling files are keyed by paths relative to the target's parent directory.
///
/// # Errors
///
/// Returns `Err(String)` if the root path does not exist or no .omni files are found.
#[must_use]
pub fn discover_omni_sources(
    root: &Path,
    mode: DiscoveryMode,
) -> Result<FxHashMap<String, String>, String> {
    match mode {
        DiscoveryMode::Directory => discover_directory(root),
        DiscoveryMode::SingleFileWithSiblings => discover_single_with_siblings(root),
    }
}

fn discover_directory(project_dir: &Path) -> Result<FxHashMap<String, String>, String> {
    if !project_dir.is_dir() {
        return Err(format!(
            "Project directory not found: {}",
            project_dir.display()
        ));
    }

    let mut sources: FxHashMap<String, String> = FxHashMap::default();
    let mut errors: Vec<String> = Vec::new();

    for entry in walkdir::WalkDir::new(project_dir)
        .follow_links(false)
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
                    .replace('\\', "/");
                sources.insert(rel_path, content);
            }
            Err(e) => {
                errors.push(format!("Cannot read {:?}: {}", path, e));
            }
        }
    }

    if sources.is_empty() {
        let mut msg = String::from("No .omni source files found.");
        for err in &errors {
            msg.push_str(&format!("\n  - {err}"));
        }
        return Err(msg);
    }

    Ok(sources)
}

fn discover_single_with_siblings(file_path: &Path) -> Result<FxHashMap<String, String>, String> {
    if !file_path.is_file() {
        return Err(format!("File not found: {}", file_path.display()));
    }

    if file_path.extension().map_or(true, |e| e != "omni") {
        return Err(format!("Not a .omni file: {}", file_path.display()));
    }

    let mut sources: FxHashMap<String, String> = FxHashMap::default();

    // Add the target file itself
    let name = file_path
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .ok_or_else(|| "Invalid file path: no filename".to_string())?;
    let content = std::fs::read_to_string(file_path)
        .map_err(|e| format!("Cannot read {:?}: {}", file_path, e))?;
    sources.insert(name.clone(), content);

    // Walk sibling directories for import resolution
    let parent = file_path.parent().unwrap_or_else(|| Path::new("."));
    let mut errors: Vec<String> = Vec::new();
    for entry in walkdir::WalkDir::new(parent)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| match e {
            Ok(entry) => Some(entry),
            Err(err) => {
                eprintln!("[omni] warning: skipping inaccessible entry: {}", err);
                None
            }
        })
        .filter(|e| e.file_name().to_string_lossy().ends_with(".omni") && e.path() != file_path)
    {
        let path = entry.path().to_owned();
        match std::fs::read_to_string(&path) {
            Ok(content) => {
                let rel_path = path
                    .strip_prefix(parent)
                    .unwrap_or(&path)
                    .to_string_lossy()
                    .replace('\\', "/");
                sources.insert(rel_path, content);
            }
            Err(e) => {
                errors.push(format!("Cannot read {:?}: {}", path, e));
            }
        }
    }

    if !errors.is_empty() {
        eprintln!(
            "[omni] warning: {} sibling file(s) could not be read:",
            errors.len()
        );
        for err in &errors {
            eprintln!("  - {}", err);
        }
    }

    Ok(sources)
}
