use std::collections::HashMap;

use crate::codegen::{vue::VueCodegen, Codegen};

use super::*;

#[test]
fn test_regression_vue_empty() {
    let codegen = VueCodegen;
    let ir = fixture_empty();
    let sources = HashMap::from([("main.omni".into(), "".into())]);
    let output = codegen.generate(&ir, &sources);
    assert!(output.errors.is_empty());
    insta::assert_snapshot!("regression_vue_empty", &format_output(&output));
}

#[test]
fn test_regression_vue_component() {
    let codegen = VueCodegen;
    let ir = fixture_component();
    let sources = HashMap::from([("main.omni".into(), "".into())]);
    let output = codegen.generate(&ir, &sources);
    assert!(output.errors.is_empty());
    insta::assert_snapshot!("regression_vue_component", &format_output(&output));
}

#[test]
fn test_regression_vue_store() {
    let codegen = VueCodegen;
    let ir = fixture_store();
    let sources = HashMap::from([("main.omni".into(), "".into())]);
    let output = codegen.generate(&ir, &sources);
    assert!(output.errors.is_empty());
    insta::assert_snapshot!("regression_vue_store", &format_output(&output));
}

#[test]
fn test_regression_vue_component_with_store() {
    let codegen = VueCodegen;
    let ir = fixture_component_with_store();
    let sources = HashMap::from([("main.omni".into(), "".into())]);
    let output = codegen.generate(&ir, &sources);
    assert!(output.errors.is_empty());
    insta::assert_snapshot!(
        "regression_vue_component_with_store",
        &format_output(&output)
    );
}

#[test]
fn test_regression_vue_expressions() {
    let codegen = VueCodegen;
    let ir = fixture_expressions();
    let sources = HashMap::from([("main.omni".into(), "".into())]);
    let output = codegen.generate(&ir, &sources);
    assert!(output.errors.is_empty());
    insta::assert_snapshot!("regression_vue_expressions", &format_output(&output));
}
