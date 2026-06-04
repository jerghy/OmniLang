/// Semantic error tests: verify error fixtures are rejected with expected messages.
/// Mirrors tests/test_semantic.py

use crate::{parser, semantic};
use std::path::PathBuf;

fn errors_dir() -> PathBuf {
    for c in &["../../../tests/fixtures/errors", "../../tests/fixtures/errors", "../tests/fixtures/errors"] {
        let p = PathBuf::from(c);
        if p.exists() { return p; }
    }
    panic!("Errors dir not found");
}

fn compile_should_fail(filename: &str) -> String {
    let path = errors_dir().join(filename);
    let source = std::fs::read_to_string(&path).unwrap_or_default();
    let prog = match parser::parse(&source, &path.to_string_lossy()) {
        Ok(p) => p,
        Err(e) => return e.report(None),
    };
    let lines: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::from([
        (path.to_string_lossy().to_string(), source.lines().map(|l| l.to_string()).collect()),
    ]);
    match semantic::analyze(&prog, Some(&lines)) {
        Ok(_) => String::new(), // Should have failed!
        Err(e) => e.report(Some(&lines)),
    }
}

#[cfg(test)]
mod semantic_error_tests {
    use super::*;

    // (filename, expected_error_substring)
    const ERROR_TESTS: &[(&str, Option<&str>)] = &[
        ("err_var_no_init.omni", None),
        ("err_type_mismatch.omni", Some("Type mismatch")),
        ("err_shadow_same_scope.omni", Some("already declared")),
        ("err_underscore_prefix.omni", None),
        ("err_float_div.omni", Some("forbidden")),
        ("err_float_mod.omni", Some("forbidden")),
        ("err_param_mutate.omni", None),
        ("err_for_mutate_container.omni", None),
        ("err_for_mutate_var.omni", None),
        ("err_main_with_return.omni", None),
        ("err_print_not_string.omni", Some("must be String")),
        ("err_f64_key.omni", None),
        ("err_nested_container.omni", None),
        ("err_break_outside_loop.omni", Some("outside of loop")),
    ];

    #[test]
    fn test_all_semantic_errors() {
        let mut passed = 0;
        let mut failed = vec![];
        for &(filename, expected) in ERROR_TESTS {
            crate::ast::reset_node_ids();
            let result = compile_should_fail(filename);
            if result.is_empty() {
                failed.push((filename, "Should have failed but compiled successfully".into()));
            } else if let Some(expected_msg) = expected {
                if !result.to_lowercase().contains(&expected_msg.to_lowercase()) {
                    failed.push((filename, format!("Expected '{}' in error, got: {}", expected_msg, &result[..result.len().min(200)])));
                } else {
                    passed += 1;
                }
            } else {
                passed += 1; // Any error is acceptable
            }
        }
        if !failed.is_empty() {
            for (f, e) in &failed {
                eprintln!("FAIL {}: {}", f, e);
            }
            panic!("{}/{} error tests failed", failed.len(), ERROR_TESTS.len());
        }
        eprintln!("All {}/{} semantic error tests passed", passed, ERROR_TESTS.len());
    }
}
