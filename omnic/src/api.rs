//! Shared API utilities for the `omnic` CLI.
//!
//! Mirrors Python `omnilang/api.py` and `omnilang/rust_bridge.py`.

use rustc_hash::FxHashMap;

/// Collect .omni source files from the given paths.
///
/// - Empty list: auto-discover `src/**/*.omni` then `./*.omni`
/// - Directories: discover `.omni` files inside them
/// - Files: added directly with sibling discovery for import resolution
pub fn collect_sources(paths: &[String]) -> anyhow::Result<FxHashMap<String, String>> {
    use omnilang::discovery::{discover_omni_sources, DiscoveryMode};
    use std::path::Path;

    if paths.is_empty() {
        // Auto-discover: try src/ first, then .
        for dir in &["src", "."] {
            let p = Path::new(dir);
            if p.is_dir() {
                match discover_omni_sources(p, DiscoveryMode::Directory) {
                    Ok(sources) => return Ok(sources),
                    Err(_) => continue,
                }
            }
        }
        anyhow::bail!("No .omni files found. Specify file(s) or run from a project directory.");
    }

    let mut sources: FxHashMap<String, String> = FxHashMap::default();
    for p in paths {
        let path = Path::new(p);
        if path.is_dir() {
            let discovered = discover_omni_sources(path, DiscoveryMode::Directory)
                .map_err(|e| anyhow::anyhow!("{p}: {e}"))?;
            sources.extend(discovered);
        } else if path.exists() {
            let discovered = discover_omni_sources(path, DiscoveryMode::SingleFileWithSiblings)
                .map_err(|e| anyhow::anyhow!("{p}: {e}"))?;
            sources.extend(discovered);
        } else {
            anyhow::bail!("File not found: {p}");
        }
    }

    if sources.is_empty() {
        anyhow::bail!("No .omni files found. Specify file(s) or run from a project directory.");
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
