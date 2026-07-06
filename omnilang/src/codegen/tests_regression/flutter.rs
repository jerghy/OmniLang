use std::collections::HashMap;

use crate::codegen::{flutter::FlutterCodegen, Codegen};

use super::*;

#[test]
fn test_regression_flutter_empty() {
    let codegen = FlutterCodegen;
    let ir = fixture_empty();
    let sources = HashMap::from([("main.omni".into(), "".into())]);
    let output = codegen.generate(&ir, &sources);
    assert!(output.errors.is_empty());
    insta::assert_snapshot!("regression_flutter_empty", &format_output(&output));
}

#[test]
fn test_regression_flutter_component() {
    let codegen = FlutterCodegen;
    let ir = fixture_component();
    let sources = HashMap::from([("main.omni".into(), "".into())]);
    let output = codegen.generate(&ir, &sources);
    assert!(output.errors.is_empty());
    insta::assert_snapshot!("regression_flutter_component", &format_output(&output));
}

#[test]
fn test_regression_flutter_store() {
    let codegen = FlutterCodegen;
    let ir = fixture_store();
    let sources = HashMap::from([("main.omni".into(), "".into())]);
    let output = codegen.generate(&ir, &sources);
    assert!(output.errors.is_empty());
    insta::assert_snapshot!("regression_flutter_store", &format_output(&output));
}

#[test]
fn test_regression_flutter_component_with_store() {
    let codegen = FlutterCodegen;
    let ir = fixture_component_with_store();
    let sources = HashMap::from([("main.omni".into(), "".into())]);
    let output = codegen.generate(&ir, &sources);
    assert!(output.errors.is_empty());
    insta::assert_snapshot!(
        "regression_flutter_component_with_store",
        &format_output(&output)
    );
}

#[test]
fn test_regression_flutter_expressions() {
    let codegen = FlutterCodegen;
    let ir = fixture_expressions();
    let sources = HashMap::from([("main.omni".into(), "".into())]);
    let output = codegen.generate(&ir, &sources);
    assert!(output.errors.is_empty());
    insta::assert_snapshot!("regression_flutter_expressions", &format_output(&output));
}
