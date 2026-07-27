//! Cross-target correctness tests — verify that the same .omni source
//! compiles without errors across all P0 backends (OmniView, React, Flutter).
//!
//! Each Golden fixture is compiled to all 3 backends. The primary invariant:
//! if a file parses successfully, ALL backends should produce error-free output.

use std::fs;
use std::path::PathBuf;

use rustc_hash::FxHashMap;

use omnilang::codegen::{compile, Target};
use omnilang::ir::lower;
use omnilang::lexer::tokenize;
use omnilang::parser;
use omnilang::semantic::SemanticResult;

/// Resolve the repo root.
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

/// Compile source text to a specific target.
fn compile_source(source: &str, target: Target) -> omnilang::codegen::CompileOutput {
    let tokens = tokenize(source);
    let program = match parser::parse(tokens, source) {
        Ok(p) => p,
        Err(e) => {
            return omnilang::codegen::CompileOutput {
                files: vec![],
                errors: vec![omnilang::error::Diagnostic {
                    severity: omnilang::error::Severity::Error,
                    kind: omnilang::error::DiagnosticKind::ECodegen,
                    message: format!("Parse error: {}", e.message),
                    file: None,
                    line: e.span.line,
                    col: e.span.col,
                    end_line: None,
                    end_col: None,
                    suggestion: None,
                }],
                source_map: vec![],
                warnings: vec![],
            };
        }
    };

    let mut semantics = SemanticResult {
        symbols: omnilang::semantic::symbols::SymbolTable::new(),
        dependencies: FxHashMap::default(),
        diagnostics: vec![],
    };
    let ir = lower(&program, &mut semantics);
    let source_files: FxHashMap<String, String> =
        FxHashMap::from_iter([("input.omni".to_string(), source.to_string())]);

    compile(&ir, target, &source_files, None)
}

const P0_BACKENDS: &[(Target, &str)] = &[
    (Target::OmniView, "OmniView"),
    (Target::React, "React"),
    (Target::Flutter, "Flutter"),
];

/// Primary cross-target test: every Golden fixture must compile error-free
/// on all 3 P0 backends.
#[test]
fn cross_target_no_errors_on_all_backends() {
    let paths = read_manifest_paths();
    assert!(
        paths.len() >= 20,
        "Expected at least 20 golden files, got {}",
        paths.len()
    );

    let repo = repo_root();
    let mut failures: Vec<String> = Vec::new();
    let mut total = 0usize;
    let mut ok = 0usize;

    for rel_path in &paths {
        let abs_path = repo.join(rel_path);
        let source = match fs::read_to_string(&abs_path) {
            Ok(s) => s,
            Err(e) => {
                // File not found is not a backend failure — skip
                eprintln!("SKIP {}: cannot read — {}", rel_path, e);
                continue;
            }
        };

        for &(target, target_name) in P0_BACKENDS {
            total += 1;
            let output = compile_source(&source, target);

            if !output.errors.is_empty() {
                failures.push(format!(
                    "{} → {}: {} error(s) — {}",
                    rel_path,
                    target_name,
                    output.errors.len(),
                    output
                        .errors
                        .first()
                        .map(|e| e.message.as_str())
                        .unwrap_or("unknown"),
                ));
                continue;
            }

            ok += 1;
        }
    }

    if !failures.is_empty() {
        eprintln!(
            "\n=== Cross-target errors ({}/{}) ===\n{}",
            failures.len(),
            total,
            failures.join("\n"),
        );
    }

    eprintln!(
        "\nCross-target: {}/{} passed ({} files × {} backends)",
        ok,
        total,
        paths.len(),
        P0_BACKENDS.len(),
    );

    assert!(
        failures.is_empty(),
        "{} cross-target compilation errors",
        failures.len()
    );
}

/// Verify that component files produce output on all backends.
/// Pure-library and design-only files are excluded as they legitimately
/// produce empty output on some backends.
#[test]
fn cross_target_component_files_produce_output() {
    // Component-containing files that should produce non-empty output
    let component_files = [
        "test/projects/learn-omniview/src/components/Counter.omni",
        "test/projects/learn-omniview/src/components/AddTodo.omni",
        "test/projects/learn-omniview/src/components/StatsPanel.omni",
        "test/projects/learn-omniview/src/components/DataTable.omni",
        "test/projects/learn-omniview/src/stores/TodoStore.omni",
        "test/projects/learn-omniview/src/pages/HomePage.omni",
        "test/projects/learn-omniview/src/pages/Dashboard.omni",
        "test/projects/learn-omniview/src/pages/MotionTriggersDemo.omni",
    ];

    let repo = repo_root();
    let mut failures: Vec<String> = Vec::new();

    for rel_path in &component_files {
        let abs_path = repo.join(rel_path);
        let source = fs::read_to_string(&abs_path)
            .unwrap_or_else(|e| panic!("Cannot read {}: {}", rel_path, e));

        for &(target, target_name) in P0_BACKENDS {
            let output = compile_source(&source, target);

            if !output.errors.is_empty() {
                failures.push(format!(
                    "{} → {}: errors — {}",
                    rel_path,
                    target_name,
                    output
                        .errors
                        .first()
                        .map(|e| e.message.as_str())
                        .unwrap_or("unknown"),
                ));
                continue;
            }

            if output.files.is_empty() {
                failures.push(format!(
                    "{} → {}: expected output files, got none",
                    rel_path, target_name,
                ));
            }
        }
    }

    assert!(
        failures.is_empty(),
        "{} component file output failures:\n{}",
        failures.len(),
        failures.join("\n"),
    );
}
