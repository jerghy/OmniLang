use std::collections::HashMap;

use crate::codegen::{dioxus::DioxusCodegen, Codegen};

use super::*;

#[test]
fn test_regression_dioxus_empty() {
    let codegen = DioxusCodegen;
    let ir = fixture_empty();
    let sources = HashMap::from([("main.omni".into(), "".into())]);
    let output = codegen.generate(&ir, &sources);
    assert!(output.errors.is_empty());
    insta::assert_snapshot!("regression_dioxus_empty", &format_output(&output));
}

#[test]
fn test_regression_dioxus_component() {
    let codegen = DioxusCodegen;
    let ir = fixture_component();
    let sources = HashMap::from([("main.omni".into(), "".into())]);
    let output = codegen.generate(&ir, &sources);
    assert!(output.errors.is_empty());
    insta::assert_snapshot!("regression_dioxus_component", &format_output(&output));
}

#[test]
fn test_regression_dioxus_store() {
    let codegen = DioxusCodegen;
    let ir = fixture_store();
    let sources = HashMap::from([("main.omni".into(), "".into())]);
    let output = codegen.generate(&ir, &sources);
    assert!(output.errors.is_empty());
    insta::assert_snapshot!("regression_dioxus_store", &format_output(&output));
}

#[test]
fn test_regression_dioxus_component_with_store() {
    let codegen = DioxusCodegen;
    let ir = fixture_component_with_store();
    let sources = HashMap::from([("main.omni".into(), "".into())]);
    let output = codegen.generate(&ir, &sources);
    assert!(output.errors.is_empty());
    insta::assert_snapshot!(
        "regression_dioxus_component_with_store",
        &format_output(&output)
    );
}

#[test]
fn test_regression_dioxus_expressions() {
    let codegen = DioxusCodegen;
    let ir = fixture_expressions();
    let sources = HashMap::from([("main.omni".into(), "".into())]);
    let output = codegen.generate(&ir, &sources);
    assert!(output.errors.is_empty());
    insta::assert_snapshot!("regression_dioxus_expressions", &format_output(&output));
}
