//! Path mapping for cross-file import resolution.
//! Shared between the omnic CLI, Poiesis Tauri backend, and WASM API.

use rustc_hash::FxHashMap;
use std::path::Path;

/// Derive import path mapping from source file paths.
///
/// Maps relative import paths to actual file paths for cross-file import resolution.
/// Builds mapping in O(n * depth) instead of O(n^2) by walking each file's parent
/// directory chain to enumerate relative-path variants directly, avoiding pairwise
/// comparison between every pair of files.
#[must_use]
pub fn derive_path_mapping(sources: &FxHashMap<String, String>) -> FxHashMap<String, String> {
    let mut mapping = FxHashMap::default();

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

    // O(n * depth) relative-path resolution: walk each file's parent directory
    // chain to generate all relative-path variants. This replaces the O(n^2)
    // pairwise comparison by exploiting that any valid import target can be
    // expressed as a relative path from one of its ancestor directories.
    //
    // For a file at "src/components/Button.omni", this generates:
    //   ./Button.omni              (from "src/components/")
    //   ./components/Button.omni   (from "src/")
    //   ./src/components/Button.omni (from ".")
    for filepath in sources.keys() {
        let normalized = filepath.replace('\\', "/");
        let path = Path::new(&normalized);

        // Start from the parent of the file, walk up to root
        // For each ancestor, compute relative path from ancestor to the file.
        let mut current = path.parent();
        while let Some(parent) = current {
            if let Ok(rel) = path.strip_prefix(parent) {
                let rel_str = rel.to_string_lossy().replace('\\', "/");
                mapping.insert(format!("./{rel_str}"), filepath.clone());
                mapping
                    .entry(rel_str.clone())
                    .or_insert_with(|| filepath.clone());
                if rel_str.ends_with(".omni") {
                    let no_ext = &rel_str[..rel_str.len() - 5];
                    mapping
                        .entry(format!("./{no_ext}"))
                        .or_insert_with(|| filepath.clone());
                    mapping
                        .entry(no_ext.to_string())
                        .or_insert_with(|| filepath.clone());
                }
            }
            if parent.as_os_str().is_empty() || parent == Path::new(".") || parent == Path::new("/")
            {
                break;
            }
            current = parent.parent();
        }
    }

    // Add normalized entries for `../`-prefixed lookups.
    // E.g., if we have `lib.omni` we also add `../lib.omni` so
    // that imports like `../lib` from subdirectories can resolve.
    // Normalize by collapsing leading `../` where the corresponding
    // ancestor-less path exists.
    let extra: Vec<(String, String)> = mapping
        .iter()
        .filter(|(k, _)| !k.starts_with("./") && !k.starts_with("../") && !k.starts_with('.'))
        .map(|(k, v)| (format!("../{k}"), v.clone()))
        .collect();
    for (k, v) in extra {
        mapping.entry(k).or_insert(v);
    }

    mapping
}
