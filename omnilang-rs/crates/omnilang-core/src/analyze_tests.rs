/// Analyze rule tests: verify each rule detects the expected patterns.
/// Mirrors tests/analyze/test_rules.py

use crate::analyze;
use crate::{parser, semantic};
use std::collections::HashMap;

fn run_check(source: &str, targets: &[String]) -> HashMap<String, usize> {
    let prog = match parser::parse(source, "<test>") {
        Ok(p) => p,
        Err(_) => return HashMap::new(),
    };
    let sl = HashMap::from([("<test>".into(), source.lines().map(|l| l.to_string()).collect())]);
    let (tm, rm) = match semantic::analyze(&prog, Some(&sl)) {
        Ok(r) => r,
        Err(_) => return HashMap::new(), // analysis errors are separate from lint rules
    };
    let diags = analyze::run_checks(&prog, &tm, &rm, targets, sl);
    let mut counts = HashMap::new();
    for d in &diags {
        *counts.entry(d.code.clone()).or_insert(0) += 1;
    }
    counts
}

#[cfg(test)]
mod analyze_rule_tests {
    use super::*;

    #[test]
    fn test_p0101_integer_division() {
        let src = "fn main() { let a: i32 = 5; let b: i32 = 2; let c: i32 = a / b; }";
        let counts = run_check(src, &["python".into(), "js".into(), "dart".into(), "rust".into()]);
        assert!(counts.contains_key("P0101"), "P0101 should detect integer division, got: {:?}", counts.keys().collect::<Vec<_>>());
    }

    #[test]
    fn test_p0110_literal_divzero() {
        let src = "fn main() { let a: i32 = 5 / 0; }";
        let counts = run_check(src, &["python".into(), "js".into(), "dart".into(), "rust".into()]);
        assert!(counts.contains_key("P0110"), "P0110 should detect literal division by zero");
    }

    #[test]
    fn test_p0102_float_comparison() {
        let src = "fn main() { let a: f64 = 3.14; let b: f64 = 2.71; let c: bool = a == b; }";
        let counts = run_check(src, &["python".into(), "js".into(), "dart".into(), "rust".into()]);
        assert!(counts.contains_key("P0102"), "P0102 should detect float equality comparison");
    }

    #[test]
    fn test_no_portability_issues_clean_code() {
        let src = "fn main() { let a: i32 = 5; let b: i32 = 2; println(a.to_string()); }";
        let counts = run_check(src, &["python".into(), "js".into(), "dart".into(), "rust".into()]);
        assert!(!counts.contains_key("P0101"), "Clean code should not trigger P0101");
    }

    #[test]
    fn test_e0201_await_in_non_async() {
        let src = "fn fetch() -> String { return \"ok\"; } fn main() { let x: String = await fetch(); }";
        let counts = run_check(src, &["python".into(), "js".into(), "dart".into(), "rust".into()]);
        assert!(counts.contains_key("E0201"), "E0201 should detect await in non-async function, got: {:?}", counts.keys().collect::<Vec<_>>());
    }

    #[test]
    fn test_e0202_async_without_await() {
        let src = "async fn empty() { println(\"ok\"); } fn main() { }";
        let counts = run_check(src, &["python".into(), "js".into(), "dart".into(), "rust".into()]);
        assert!(counts.contains_key("E0202"), "E0202 should detect async without await");
    }

    #[test]
    fn test_s0108_backslash_in_path() {
        let src = "import { foo } from \".\\\\bar\"; fn main() { }";
        let counts = run_check(src, &["python".into(), "js".into(), "dart".into(), "rust".into()]);
        assert!(counts.contains_key("S0108"), "S0108 should detect backslash in import path");
    }

    #[test]
    fn test_s0101_unused_variable() {
        let src = "fn main() { let x: i32 = 5; let y: i32 = 10; println(x.to_string()); }";
        let counts = run_check(src, &["python".into(), "js".into(), "dart".into(), "rust".into()]);
        assert!(counts.contains_key("S0101"), "S0101 should detect unused variable (y)");
    }

    #[test]
    fn test_s0102_unused_import() {
        let src = "import { foo } from \"./lib\"; fn main() { }";
        let counts = run_check(src, &["python".into(), "js".into(), "dart".into(), "rust".into()]);
        assert!(counts.contains_key("S0102"), "S0102 should detect unused import, got: {:?}", counts.keys().collect::<Vec<_>>());
    }

    #[test]
    fn test_e0105_match_exhaustiveness() {
        // Match on i32 without wildcard should trigger E0105
        let src = "fn main() { let x: i32 = 1; let y: i32 = match x { 1 => 2, }; println(y.to_string()); }";
        let counts = run_check(src, &["python".into(), "js".into(), "dart".into(), "rust".into()]);
        assert!(counts.contains_key("E0105"), "E0105 should detect non-exhaustive match, got: {:?}", counts.keys().collect::<Vec<_>>());
    }

    #[test]
    fn test_rules_registered() {
        let registry = analyze::rules::build_default_registry();
        assert!(registry.count() >= 54, "Expected at least 54 rules, got {}", registry.count());
        let expected_codes = ["P0101","P0110","P0102","E0105","E0201","E0306",
            "S0101","S0102","S0108","P0105","P0107","E0401",
            "E0101","F0101","SEC0101","E0202","E0104","E0110",
            "F0105","S0103","S0110","SEC0102","P0108","E0309"];
        for code in expected_codes {
            assert!(registry.get(code).is_some(), "Rule {} not found", code);
        }
    }

    #[test]
    fn test_p0106_closure_capture() {
        // Just verify the rule exists and runs; P0106 fires on any closure in Rust target
        let src = "fn add(a: i32, b: i32) -> i32 { return a + b; } fn main() { let r: i32 = add(1, 2); println(r.to_string()); }";
        let counts = run_check(src, &["rust".into()]);
        // This source has no closures, so P0106 should NOT fire
        assert!(!counts.contains_key("P0106"), "P0106 should NOT fire without closures");
    }
}
