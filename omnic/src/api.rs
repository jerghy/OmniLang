//! Shared API utilities for the `omnic` CLI.
//!
//! Mirrors Python `omnilang/api.py` and `omnilang/rust_bridge.py`.

use std::collections::HashMap;
use std::path::Path;

/// Collect .omni source files from the given paths.
///
/// Mirrors Python `_collect_sources()` in cli.py:73.
/// - Empty list: auto-discover `src/**/*.omni` then `./*.omni`
/// - Directories: glob `**/*.omni` inside them
/// - Files: added directly
pub fn collect_sources(paths: &[String]) -> anyhow::Result<HashMap<String, String>> {
    let mut source_files: Vec<std::path::PathBuf> = Vec::new();

    if paths.is_empty() {
        for dir in &["src", "."] {
            let pattern = format!("{dir}/**/*.omni");
            let mut found: Vec<_> = glob::glob(&pattern)
                .ok()
                .into_iter()
                .flatten()
                .filter_map(|r| r.ok())
                .collect();
            found.sort();
            if !found.is_empty() {
                source_files = found;
                break;
            }
        }
    } else {
        for p in paths {
            let path = Path::new(p);
            if path.is_dir() {
                let pattern = format!("{p}/**/*.omni");
                let found: Vec<_> = glob::glob(&pattern)
                    .ok()
                    .into_iter()
                    .flatten()
                    .filter_map(|r| r.ok())
                    .collect();
                source_files.extend(found);
            } else if path.exists() {
                source_files.push(path.to_path_buf());
            } else {
                anyhow::bail!("File not found: {p}");
            }
        }
    }

    if source_files.is_empty() {
        anyhow::bail!("No .omni files found. Specify file(s) or run from a project directory.");
    }

    let mut sources = HashMap::new();
    for f in &source_files {
        let content = std::fs::read_to_string(f)
            .map_err(|e| anyhow::anyhow!("cannot read {}: {e}", f.display()))?;
        let normalized = f.to_string_lossy().replace('\\', "/");
        sources.insert(normalized, content);
    }

    Ok(sources)
}

/// Auto-detect target from source content.
///
/// Mirrors Python `_detect_target()` in cli.py:120.
/// Priority: --target flag > auto-detect (UI or logic).
pub fn detect_target(sources: &HashMap<String, String>, preferred: Option<&str>) -> String {
    if let Some(t) = preferred {
        return t.to_string();
    }

    // Auto-detect: if any source imports omniview, default to react
    for src in sources.values() {
        if src.contains("import omniview") || src.trim_start().starts_with("component") {
            return "react".to_string();
        }
    }

    "python".to_string()
}

/// Run the full 3-phase analysis pipeline (parse → semantic → lint).
///
/// Mirrors the Rust FFI `omni_analyse` function in `omnilang/src/lib.rs:747`
/// and `rust_bridge.py`'s `RustBridge.analyse()`.
///
/// Accepts the same JSON shape:
///   { "sources": { path: source, ... }, "path_mapping": { ... } }
pub fn analyse_direct(request: serde_json::Value) -> serde_json::Value {
    let sources: HashMap<String, String> = match request["sources"].as_object() {
        Some(s) => s
            .iter()
            .map(|(k, v)| (k.clone(), v.as_str().unwrap_or("").to_string()))
            .collect(),
        None => return serde_json::json!({ "success": false, "diagnostics": [] }),
    };

    let path_mapping: HashMap<String, String> =
        match request.get("path_mapping").and_then(|v| v.as_object()) {
            Some(obj) => obj
                .iter()
                .map(|(k, v)| (k.clone(), v.as_str().unwrap_or("").to_string()))
                .collect(),
            None => HashMap::new(),
        };

    // Run the unified analysis pipeline
    let analysis = omnilang::analyse::analyse_multi(&sources, &path_mapping);

    // Use unified diagnostic serialization from omnilang::error
    let mut diagnostics = omnilang::error::diagnostics_to_json(&analysis.diagnostics);
    let warning_jsons = omnilang::error::diagnostics_to_json(&analysis.warnings);
    diagnostics.extend(warning_jsons);

    let has_errors = diagnostics.iter().any(|d| d["severity"] == "error");

    serde_json::json!({
        "success": !has_errors,
        "diagnostics": diagnostics,
    })
}
