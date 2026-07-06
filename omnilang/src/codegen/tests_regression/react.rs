use std::collections::HashMap;

use crate::codegen::{react::ReactCodegen, Codegen};

use super::*;

#[test]
fn test_regression_react_empty() {
    let codegen = ReactCodegen;
    let ir = fixture_empty();
    let sources = HashMap::from([("main.omni".into(), "".into())]);
    let output = codegen.generate(&ir, &sources);
    assert!(output.errors.is_empty());
    insta::assert_snapshot!("regression_react_empty", &format_output(&output));
}

#[test]
fn test_regression_react_component() {
    let codegen = ReactCodegen;
    let ir = fixture_component();
    let sources = HashMap::from([("main.omni".into(), "".into())]);
    let output = codegen.generate(&ir, &sources);
    assert!(output.errors.is_empty());
    insta::assert_snapshot!("regression_react_component", &format_output(&output));
}

#[test]
fn test_regression_react_store() {
    let codegen = ReactCodegen;
    let ir = fixture_store();
    let sources = HashMap::from([("main.omni".into(), "".into())]);
    let output = codegen.generate(&ir, &sources);
    assert!(output.errors.is_empty());
    insta::assert_snapshot!("regression_react_store", &format_output(&output));
}

#[test]
fn test_regression_react_component_with_store() {
    let codegen = ReactCodegen;
    let ir = fixture_component_with_store();
    let sources = HashMap::from([("main.omni".into(), "".into())]);
    let output = codegen.generate(&ir, &sources);
    assert!(output.errors.is_empty());
    insta::assert_snapshot!(
        "regression_react_component_with_store",
        &format_output(&output)
    );
}

#[test]
fn test_regression_react_expressions() {
    let codegen = ReactCodegen;
    let ir = fixture_expressions();
    let sources = HashMap::from([("main.omni".into(), "".into())]);
    let output = codegen.generate(&ir, &sources);
    assert!(output.errors.is_empty());
    insta::assert_snapshot!("regression_react_expressions", &format_output(&output));
}
