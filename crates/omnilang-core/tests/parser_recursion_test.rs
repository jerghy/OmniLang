//! Regression tests for the parser recursion-depth guard.
//!
//! The Pratt expression parser recurses through `parse_expr_bp()` from 7
//! direct paths (unary `-`/`!`, `await`, binary ops, assignment,
//! compound-assign, ternary) that previously bypassed `MAX_RECURSION_DEPTH`,
//! letting a ~300-byte input of `!` crash the compiler with a stack overflow
//! (exit 127; with `panic = "abort"` in WASM this kills the compiler instance).
//!
//! The deep expressions are wrapped in a function body so they actually go
//! through expression parsing — bare operators at top level are rejected by
//! the top-level dispatcher without ever recursing.

use omnilang_core::error::ParseError;
use omnilang_core::lexer::tokenize;
use omnilang_core::parser::{self, ast::Program};

/// Parse `expr` inside a function body, so expression parsing actually runs.
/// Returns the collected parse errors.
fn parse_expression_in_function(expr: &str) -> Vec<ParseError> {
    let source = format!("pub fn main() {{ {} }}", expr);
    let tokens = tokenize(&source);
    let (program, errors): (Program, Vec<ParseError>) = parser::parse(tokens, &source);
    assert!(
        !program.modules.is_empty(),
        "wrapped source should always produce a module"
    );
    errors
}

/// Run a parse that previously crashed with a stack overflow, asserting that
/// it returns normally instead of panicking.
fn assert_parse_does_not_panic(expr: &str, name: &str) -> Vec<ParseError> {
    let expr = expr.to_string();
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        parse_expression_in_function(&expr)
    }));
    assert!(
        result.is_ok(),
        "parser must not panic on deep {name}: {:?}",
        result.err()
    );
    result.unwrap()
}

#[test]
fn deep_unary_chain_does_not_overflow() {
    // 100,000 consecutive `!` operators — previously crashed with stack
    // overflow at ~300 characters. Must be rejected by the depth guard.
    let errors = assert_parse_does_not_panic(&"!".repeat(100_000), "unary chain");
    assert!(
        errors.iter().any(|e| e.message.contains("recursion depth")),
        "expected depth guard to fire, got errors: {:?}",
        errors
    );
}

#[test]
fn deep_nested_ternary_does_not_overflow() {
    // a ? b : (a ? b : (a ? b : ...)) — else-branch recurses through
    // parse_expr_bp(bp.1) at infix.rs, previously unguarded.
    let mut source = String::from("a");
    for _ in 0..5_000 {
        source = format!("a ? b : ({})", source);
    }
    let errors = assert_parse_does_not_panic(&source, "ternary chain");
    assert!(
        errors.iter().any(|e| e.message.contains("recursion depth")),
        "expected depth guard to fire, got errors: {:?}",
        errors
    );
}

#[test]
fn deep_assignment_chain_does_not_overflow() {
    // a = b = c = ... — right-recursive through parse_expr_bp(1) at
    // infix.rs parse_assign/parse_compound_assign, previously unguarded.
    let source = "a = ".repeat(5_000) + "0";
    let errors = assert_parse_does_not_panic(&source, "assignment chain");
    assert!(
        errors.iter().any(|e| e.message.contains("recursion depth")),
        "expected depth guard to fire, got errors: {:?}",
        errors
    );
}
