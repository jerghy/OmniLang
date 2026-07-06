use std::collections::HashMap;

use crate::codegen::{rust::RustCodegen, Codegen};

use super::*;

#[test]
fn test_regression_rust_empty() {
    let codegen = RustCodegen;
    let ir = fixture_empty();
    let sources = HashMap::from([("main.omni".into(), "".into())]);
    let output = codegen.generate(&ir, &sources);
    assert!(output.errors.is_empty());
    insta::assert_snapshot!("regression_rust_empty", &format_output(&output));
}

#[test]
fn test_regression_rust_component() {
    let codegen = RustCodegen;
    let ir = fixture_component();
    let sources = HashMap::from([("main.omni".into(), "".into())]);
    let output = codegen.generate(&ir, &sources);
    assert!(output.errors.is_empty());
    insta::assert_snapshot!("regression_rust_component", &format_output(&output));
}

#[test]
fn test_regression_rust_store() {
    let codegen = RustCodegen;
    let ir = fixture_store();
    let sources = HashMap::from([("main.omni".into(), "".into())]);
    let output = codegen.generate(&ir, &sources);
    assert!(output.errors.is_empty());
    insta::assert_snapshot!("regression_rust_store", &format_output(&output));
}

#[test]
fn test_regression_rust_component_with_store() {
    let codegen = RustCodegen;
    let ir = fixture_component_with_store();
    let sources = HashMap::from([("main.omni".into(), "".into())]);
    let output = codegen.generate(&ir, &sources);
    assert!(output.errors.is_empty());
    insta::assert_snapshot!(
        "regression_rust_component_with_store",
        &format_output(&output)
    );
}

#[test]
fn test_regression_rust_expressions() {
    let codegen = RustCodegen;
    let ir = fixture_expressions();
    let sources = HashMap::from([("main.omni".into(), "".into())]);
    let output = codegen.generate(&ir, &sources);
    assert!(output.errors.is_empty());
    insta::assert_snapshot!("regression_rust_expressions", &format_output(&output));
}
