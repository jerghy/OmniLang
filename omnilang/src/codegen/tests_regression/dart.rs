use std::collections::HashMap;

use crate::codegen::{dart::DartCodegen, Codegen};

use super::*;

#[test]
fn test_regression_dart_empty() {
    let codegen = DartCodegen;
    let ir = fixture_empty();
    let sources = HashMap::from([("main.omni".into(), "".into())]);
    let output = codegen.generate(&ir, &sources);
    assert!(output.errors.is_empty());
    insta::assert_snapshot!("regression_dart_empty", &format_output(&output));
}

#[test]
fn test_regression_dart_component() {
    let codegen = DartCodegen;
    let ir = fixture_component();
    let sources = HashMap::from([("main.omni".into(), "".into())]);
    let output = codegen.generate(&ir, &sources);
    assert!(output.errors.is_empty());
    insta::assert_snapshot!("regression_dart_component", &format_output(&output));
}

#[test]
fn test_regression_dart_store() {
    let codegen = DartCodegen;
    let ir = fixture_store();
    let sources = HashMap::from([("main.omni".into(), "".into())]);
    let output = codegen.generate(&ir, &sources);
    assert!(output.errors.is_empty());
    insta::assert_snapshot!("regression_dart_store", &format_output(&output));
}

#[test]
fn test_regression_dart_component_with_store() {
    let codegen = DartCodegen;
    let ir = fixture_component_with_store();
    let sources = HashMap::from([("main.omni".into(), "".into())]);
    let output = codegen.generate(&ir, &sources);
    assert!(output.errors.is_empty());
    insta::assert_snapshot!(
        "regression_dart_component_with_store",
        &format_output(&output)
    );
}

#[test]
fn test_regression_dart_expressions() {
    let codegen = DartCodegen;
    let ir = fixture_expressions();
    let sources = HashMap::from([("main.omni".into(), "".into())]);
    let output = codegen.generate(&ir, &sources);
    assert!(output.errors.is_empty());
    insta::assert_snapshot!("regression_dart_expressions", &format_output(&output));
}
