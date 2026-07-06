//! Path mapping for cross-file import resolution.
//! Shared between the omnic CLI, Poiesis Tauri backend, and WASM API.

use std::collections::HashMap;
use std::path::Path;

/// Derive import path mapping from source file paths.
///
/// Maps relative import paths to actual file paths for cross-file import resolution.
/// For each pair of source files, computes relative-path keys (e.g. `"./Component.omni"`,
/// `"Component.omni"`, `"./Component"`) so that import statements can be resolved
/// regardless of how the importer refers to the target.
pub fn derive_path_mapping(sources: &HashMap<String, String>) -> HashMap<String, String> {
    let mut mapping = HashMap::new();

    for filepath in sources.keys() {
        // Direct path → self
        mapping.insert(filepath.clone(), filepath.clone());

        // Basename (e.g. "Counter.omni")
        if let Some(basename) = Path::new(filepath).file_name() {
            let basename_str = basename.to_string_lossy().to_string();
            mapping
                .entry(basename_str.clone())
                .or_insert_with(|| filepath.clone());

            // Stem (e.g. "Counter")
            if let Some(stem) = Path::new(&basename_str).file_stem() {
                let stem_str = stem.to_string_lossy().to_string();
                mapping.entry(stem_str).or_insert_with(|| filepath.clone());
            }
        }
    }

    // For every pair of files, compute relative-path keys
    let paths: Vec<&String> = sources.keys().collect();
    for import_path in &paths {
        let importer_dir = Path::new(import_path).parent().unwrap_or(Path::new("."));
        for target_path in &paths {
            if *import_path == *target_path {
                continue;
            }

            // Compute relative path
            if let Some(rel_simple) = pathdiff::diff_paths(Path::new(target_path), importer_dir) {
                let rel_path = rel_simple.to_string_lossy().replace('\\', "/");

                // Map "./Component.omni" → full path
                mapping.insert(format!("./{rel_path}"), (*target_path).clone());
                // Map "Component.omni" (without ./ prefix)
                mapping.insert(rel_path.clone(), (*target_path).clone());

                // Map "./Component" (without .omni extension) → full path
                if rel_path.ends_with(".omni") {
                    let no_ext = &rel_path[..rel_path.len() - 5];
                    mapping.insert(format!("./{no_ext}"), (*target_path).clone());
                    mapping.insert(no_ext.to_string(), (*target_path).clone());
                }
            }
        }
    }

    mapping
}
