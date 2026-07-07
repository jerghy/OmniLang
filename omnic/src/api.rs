//! Shared API utilities for the `omnic` CLI.
//!
//! Mirrors Python `omnilang/api.py` and `omnilang/rust_bridge.py`.

use std::path::Path;

use rustc_hash::FxHashMap;

/// Collect .omni source files from the given paths.
///
/// Mirrors Python `_collect_sources()` in cli.py:73.
/// - Empty list: auto-discover `src/**/*.omni` then `./*.omni`
/// - Directories: glob `**/*.omni` inside them
/// - Files: added directly
pub fn collect_sources(paths: &[String]) -> anyhow::Result<FxHashMap<String, String>> {
    let mut source_files: Vec<std::path::PathBuf> = Vec::new();

    if paths.is_empty() {
        for dir in &["src", "."] {
            let pattern = format!("{dir}/**/*.omni");
            let mut found: Vec<_> = match glob::glob(&pattern) {
                Ok(entries) => entries
                    .filter_map(|r| match r {
                        Ok(path) => Some(path),
                        Err(e) => {
                            eprintln!("warning: glob read error for '{}': {}", pattern, e);
                            None
                        }
                    })
                    .collect(),
                Err(e) => {
                    eprintln!("warning: invalid glob pattern '{}': {}", pattern, e);
                    Vec::new()
                }
            };
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
                let found: Vec<_> = match glob::glob(&pattern) {
                    Ok(entries) => entries
                        .filter_map(|r| match r {
                            Ok(path) => Some(path),
                            Err(e) => {
                                eprintln!("warning: glob read error for '{}': {}", pattern, e);
                                None
                            }
                        })
                        .collect(),
                    Err(e) => {
                        eprintln!("warning: invalid glob pattern '{}': {}", pattern, e);
                        Vec::new()
                    }
                };
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

    let mut sources = FxHashMap::default();
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
pub fn detect_target(sources: &FxHashMap<String, String>, preferred: Option<&str>) -> String {
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
    let sources: FxHashMap<String, String> = match request["sources"].as_object() {
        Some(s) => s
            .iter()
            .map(|(k, v)| (k.clone(), v.as_str().unwrap_or("").to_string()))
            .collect(),
        None => return serde_json::json!({ "success": false, "diagnostics": [] }),
    };

    let path_mapping: FxHashMap<String, String> =
        match request.get("path_mapping").and_then(|v| v.as_object()) {
            Some(obj) => obj
                .iter()
                .map(|(k, v)| (k.clone(), v.as_str().unwrap_or("").to_string()))
                .collect(),
            None => FxHashMap::default(),
        };

    // Run the unified analysis pipeline
    let analysis = omnilang::analyse::analyse_multi(&sources, &path_mapping);

    // Convert Diagnostics to JSON Value objects matching CLI output format
    let mut diagnostics: Vec<serde_json::Value> = Vec::new();

    for d in &analysis.diagnostics {
        let severity = match d.severity {
            omnilang::error::Severity::Error => "error",
            omnilang::error::Severity::Warning => "warning",
            omnilang::error::Severity::Info => "info",
        };
        diagnostics.push(serde_json::json!({
            "severity": severity,
            "code": d.kind.code_str(),
            "message": d.message,
            "file": d.file,
            "line": d.line,
            "col": d.col,
            "suggestion": d.suggestion,
        }));
    }

    for w in &analysis.warnings {
        diagnostics.push(serde_json::json!({
            "severity": "warning",
            "code": w.kind.code_str(),
            "message": w.message,
            "file": w.file,
            "line": w.line,
            "col": w.col,
            "suggestion": null,
        }));
    }

    let has_errors = diagnostics.iter().any(|d| d["severity"] == "error");

    serde_json::json!({
        "success": !has_errors,
        "diagnostics": diagnostics,
    })
}
