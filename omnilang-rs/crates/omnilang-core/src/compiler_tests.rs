/// Compiler snapshot tests: verify generated code patterns.
/// Mirrors tests/test_compiler.py

use crate::{parser, semantic, codegen};
use std::path::PathBuf;

fn fixtures_dir() -> PathBuf {
    for c in &["../../../tests/fixtures", "../../tests/fixtures", "../tests/fixtures"] {
        let p = PathBuf::from(c);
        if p.exists() { return p; }
    }
    panic!("Fixtures dir not found");
}

fn generate(fixture: &str, target: &str) -> String {
    crate::ast::reset_node_ids();
    let path = fixtures_dir().join(fixture);
    let source = std::fs::read_to_string(&path).unwrap();
    let prog = parser::parse(&source, &path.to_string_lossy()).expect("parse");
    let lines: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::from([
        (path.to_string_lossy().to_string(), source.lines().map(|l| l.to_string()).collect()),
    ]);
    let (tm, rm) = semantic::analyze(&prog, Some(&lines)).expect("analyze");
    codegen::generate(&prog, target, &rm, &tm).expect("codegen")
}

#[cfg(test)]
mod compiler_snapshot_tests {
    use super::*;

    #[test]
    fn test_python_main_guard() {
        let code = generate("01_hello_world.omni", "python");
        assert!(code.contains("__name__ == \"__main__\""), "Python output should have main guard");
    }

    #[test]
    fn test_python_no_idiv_when_not_needed() {
        let code = generate("20_no_idiv.omni", "python");
        assert!(!code.contains("_idiv"), "Should not contain _idiv");
        assert!(!code.contains("_irem"), "Should not contain _irem");
    }

    #[test]
    fn test_js_helpers_when_needed() {
        let code = generate("03_arithmetic.omni", "js");
        assert!(code.contains("_idiv"), "JS output should contain _idiv helper when division is used");
    }

    #[test]
    fn test_js_helpers_when_not_needed() {
        let code = generate("20_no_idiv.omni", "js");
        assert!(!code.contains("_idiv"), "JS output should not contain _idiv when no division");
    }

    #[test]
    fn test_js_main_call() {
        let code = generate("01_hello_world.omni", "js");
        assert!(code.contains("main();"), "JS output should call main() at the end");
    }

    #[test]
    fn test_js_const_for_let() {
        let code = generate("02_variables.omni", "js");
        assert!(code.contains("const x = 10"), "JS let should be const for immutable");
    }

    #[test]
    fn test_dart_import_io_when_print() {
        let code = generate("01_hello_world.omni", "dart");
        assert!(code.contains("import 'dart:io';"), "Dart output should import dart:io when printing");
    }

    #[test]
    fn test_dart_final_for_let() {
        let code = generate("02_variables.omni", "dart");
        assert!(code.contains("final"), "Dart let should be final for immutable");
    }

    #[test]
    fn test_dart_len_property() {
        let code = generate("05_arrays.omni", "dart");
        assert!(code.contains(".length"), "Dart len() should use .length property");
    }

    #[test]
    fn test_rust_string_param() {
        let code = generate("08_functions.omni", "rust");
        assert!(code.contains("name: &str"), "Rust String param should be &str");
    }

    #[test]
    fn test_rust_format_concat() {
        let code = generate("04_strings.omni", "rust");
        assert!(code.contains("format!"), "Rust string concat should use format!");
    }

    #[test]
    fn test_rust_main_no_return_type() {
        let code = generate("01_hello_world.omni", "rust");
        assert!(code.contains("fn main() {"), "Rust main should not have return type");
    }

    #[test]
    fn test_rust_clone_for_container() {
        let code = generate("10_ownership.omni", "rust");
        assert!(code.contains(".clone()"), "Rust container assignment should use .clone()");
    }

    #[test]
    fn test_rust_hashmap_import() {
        let code = generate("06_maps.omni", "rust");
        assert!(code.contains("use std::collections::HashMap"), "Rust should import HashMap when Map is used");
    }

    #[test]
    fn test_js_arrlen_helper() {
        let code = generate("05_arrays.omni", "js");
        assert!(code.contains("_arrlen"), "JS should use _arrlen helper for array length");
    }
}
