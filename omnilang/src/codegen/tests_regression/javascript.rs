use rustc_hash::FxHashMap;

use crate::codegen::{javascript::JavaScriptCodegen, Codegen};

use super::*;

#[test]
fn test_regression_javascript_empty() {
    let codegen = JavaScriptCodegen::new(false);
    let ir = fixture_empty();
    let sources = FxHashMap::from_iter([("main.omni".into(), "".into())]);
    let output = codegen.generate(&ir, &sources);
    assert!(output.errors.is_empty());
    insta::assert_snapshot!("regression_javascript_empty", &format_output(&output));
}

#[test]
fn test_regression_typescript_empty() {
    let codegen = JavaScriptCodegen::new(true);
    let ir = fixture_empty();
    let sources = FxHashMap::from_iter([("main.omni".into(), "".into())]);
    let output = codegen.generate(&ir, &sources);
    assert!(output.errors.is_empty());
    insta::assert_snapshot!("regression_typescript_empty", &format_output(&output));
}

#[test]
fn test_regression_javascript_component() {
    let codegen = JavaScriptCodegen::new(false);
    let ir = fixture_component();
    let sources = FxHashMap::from_iter([("main.omni".into(), "".into())]);
    let output = codegen.generate(&ir, &sources);
    assert!(output.errors.is_empty());
    insta::assert_snapshot!("regression_javascript_component", &format_output(&output));
}

#[test]
fn test_regression_typescript_component() {
    let codegen = JavaScriptCodegen::new(true);
    let ir = fixture_component();
    let sources = FxHashMap::from_iter([("main.omni".into(), "".into())]);
    let output = codegen.generate(&ir, &sources);
    assert!(output.errors.is_empty());
    insta::assert_snapshot!("regression_typescript_component", &format_output(&output));
}

#[test]
fn test_regression_javascript_store() {
    let codegen = JavaScriptCodegen::new(false);
    let ir = fixture_store();
    let sources = FxHashMap::from_iter([("main.omni".into(), "".into())]);
    let output = codegen.generate(&ir, &sources);
    assert!(output.errors.is_empty());
    insta::assert_snapshot!("regression_javascript_store", &format_output(&output));
}

#[test]
fn test_regression_typescript_store() {
    let codegen = JavaScriptCodegen::new(true);
    let ir = fixture_store();
    let sources = FxHashMap::from_iter([("main.omni".into(), "".into())]);
    let output = codegen.generate(&ir, &sources);
    assert!(output.errors.is_empty());
    insta::assert_snapshot!("regression_typescript_store", &format_output(&output));
}

#[test]
fn test_regression_javascript_component_with_store() {
    let codegen = JavaScriptCodegen::new(false);
    let ir = fixture_component_with_store();
    let sources = FxHashMap::from_iter([("main.omni".into(), "".into())]);
    let output = codegen.generate(&ir, &sources);
    assert!(output.errors.is_empty());
    insta::assert_snapshot!(
        "regression_javascript_component_with_store",
        &format_output(&output)
    );
}

#[test]
fn test_regression_typescript_component_with_store() {
    let codegen = JavaScriptCodegen::new(true);
    let ir = fixture_component_with_store();
    let sources = FxHashMap::from_iter([("main.omni".into(), "".into())]);
    let output = codegen.generate(&ir, &sources);
    assert!(output.errors.is_empty());
    insta::assert_snapshot!(
        "regression_typescript_component_with_store",
        &format_output(&output)
    );
}

#[test]
fn test_regression_javascript_expressions() {
    let codegen = JavaScriptCodegen::new(false);
    let ir = fixture_expressions();
    let sources = FxHashMap::from_iter([("main.omni".into(), "".into())]);
    let output = codegen.generate(&ir, &sources);
    assert!(output.errors.is_empty());
    insta::assert_snapshot!("regression_javascript_expressions", &format_output(&output));
}

#[test]
fn test_regression_typescript_expressions() {
    let codegen = JavaScriptCodegen::new(true);
    let ir = fixture_expressions();
    let sources = FxHashMap::from_iter([("main.omni".into(), "".into())]);
    let output = codegen.generate(&ir, &sources);
    assert!(output.errors.is_empty());
    insta::assert_snapshot!("regression_typescript_expressions", &format_output(&output));
}
