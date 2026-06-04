/// Shared test utilities for OmniLang Rust compiler tests.
/// Fixture paths resolve relative to the workspace root.

use std::path::PathBuf;

/// Get the path to the shared test fixtures (from Python test suite).
pub fn fixtures_dir() -> PathBuf {
    // Try several locations
    let candidates = [
        PathBuf::from("../../../tests/fixtures"),      // from crates/omnilang-core/
        PathBuf::from("../../tests/fixtures"),          // fallback
        PathBuf::from("../tests/fixtures"),             // fallback
        PathBuf::from("tests/fixtures"),                // fallback
    ];

    for candidate in &candidates {
        if candidate.exists() {
            return candidate.clone();
        }
    }

    panic!("Could not find fixtures directory. Tried: {:?}", candidates);
}

/// Get the path to error fixtures
pub fn error_fixtures_dir() -> PathBuf {
    let mut d = fixtures_dir();
    d.push("errors");
    d
}

/// List all .omni fixture files recursively
pub fn list_fixtures() -> Vec<PathBuf> {
    let mut fixtures = vec![];
    let base = fixtures_dir();
    if base.exists() {
        walk_dir(&base, &mut fixtures);
    }
    fixtures
}

fn walk_dir(dir: &PathBuf, fixtures: &mut Vec<PathBuf>) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                // Skip error fixtures for positive tests
                if !path.ends_with("errors") {
                    walk_dir(&path, fixtures);
                }
            } else if path.extension().map_or(false, |e| e == "omni") {
                fixtures.push(path);
            }
        }
    }
}

/// List error fixture files
pub fn list_error_fixtures() -> Vec<PathBuf> {
    let mut fixtures = vec![];
    let error_dir = error_fixtures_dir();
    if error_dir.exists() {
        walk_dir(&error_dir, &mut fixtures);
    }
    fixtures
}
