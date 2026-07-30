//! Golden file parse tests — verify the Rust parser successfully parses
//! a representative corpus of .omni files, establishing a baseline for
//! dual-parser (Rust ↔ TypeScript) consistency checking.
//!
//! The file list is sourced from `tests/fixtures/golden/MANIFEST.md`.

use std::fs;
use std::path::PathBuf;

/// Resolve the repo root relative to the test binary's location.
fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("should have parent directory")
        .to_path_buf()
}

/// Read the MANIFEST.md and extract .omni file paths.
fn read_manifest_paths() -> Vec<String> {
    let manifest = repo_root().join("tests/fixtures/golden/MANIFEST.md");
    let content = fs::read_to_string(&manifest)
        .unwrap_or_else(|_| panic!("MANIFEST.md not found at {:?}", manifest));
    content
        .lines()
        .filter(|line| line.starts_with("- `"))
        .filter_map(|line| {
            let start = line.find('`')? + 1;
            let end = line[start..].find('`')?;
            Some(line[start..start + end].to_string())
        })
        .collect()
}

#[test]
fn golden_files_exist() {
    let paths = read_manifest_paths();
    assert!(
        paths.len() >= 20,
        "Expected at least 20 golden files, got {}",
        paths.len()
    );
    for path in &paths {
        let full = repo_root().join(path);
        assert!(
            full.exists(),
            "Golden file not found: {} (resolved to {:?})",
            path,
            full
        );
    }
}

#[test]
fn golden_files_parse_without_errors() {
    let paths = read_manifest_paths();
    let mut errors = Vec::new();

    for path in &paths {
        let full = repo_root().join(path);
        let source = fs::read_to_string(&full).unwrap_or_else(|e| {
            panic!("Failed to read {}: {}", path, e);
        });

        let tokens = omnilang::lexer::tokenize(&source);
        let (_program, parse_errors) = omnilang::parser::parse(tokens, &source);
        for e in &parse_errors {
            errors.push(format!("{} (line {}): {}", path, e.span.line, e.message));
        }
    }

    if !errors.is_empty() {
        panic!(
            "{} golden file(s) failed to parse:\n{}",
            errors.len(),
            errors.join("\n")
        );
    }
}
