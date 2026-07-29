//! Integration tests for element ref system in OmniView codegen.
//! Tests ref declaration → wrapper object generation → element binding pipeline.

/// Test: basic ref declaration and binding generates wrapper object
#[test]
fn test_ref_basic_wrapper() {
    let source = r#"
component TestRef {
  ref box: Element
  view {
    <div ref:box>content</div>
  }
}
"#;
    let result = crate::compile_omniview(source, false, false);
    assert!(result.success, "compilation failed: {:?}", result.errors);
    let js = result
        .files
        .as_ref()
        .expect("Expected JS output")
        .js
        .clone();
    let html = result
        .files
        .as_ref()
        .expect("Expected HTML output")
        .html
        .clone();

    // Wrapper object exists
    assert!(
        js.contains("const box = (function()"),
        "JS should contain ref wrapper. Got:\n{}",
        js
    );
    // Uses __omni_el for element lookup
    assert!(
        js.contains("__omni_el("),
        "Wrapper should use __omni_el. Got:\n{}",
        js
    );
    // No stale data-ref attribute in HTML
    assert!(
        !html.contains("data-ref=\"box\""),
        "HTML should not have data-ref. Got:\n{}",
        html
    );
    // HTML still has the div with content
    assert!(
        html.contains("content"),
        "HTML should contain the element content. Got:\n{}",
        html
    );
}

/// Test: ref in component with @for (ref outside template)
#[test]
fn test_ref_not_in_for_template() {
    // For now, ref inside @for is not supported. Verify it at least
    // doesn't crash the compiler.
    let source = r#"
component TestRefFor {
  signal items: String[] = ["a", "b"]
  ref first: Element
  view {
    <div>
      <div ref:first>static</div>
      @for item in items {
        <span>{item}</span>
      }
    </div>
  }
}
"#;
    let result = crate::compile_omniview(source, false, false);
    assert!(
        result.success,
        "compilation should succeed without panic: {:?}",
        result.errors
    );
    let js = result
        .files
        .as_ref()
        .expect("Expected JS output")
        .js
        .clone();

    // Ref outside @for should still work
    assert!(
        js.contains("const first = (function()"),
        "Ref outside @for should still work. Got:\n{}",
        js
    );
}

/// Test: multiple refs in one component
#[test]
fn test_multiple_refs() {
    let source = r#"
component MultiRef {
  ref header: Element
  ref footer: Element
  view {
    <div>
      <div ref:header>Header</div>
      <div ref:footer>Footer</div>
    </div>
  }
}
"#;
    let result = crate::compile_omniview(source, false, false);
    assert!(result.success, "compilation failed: {:?}", result.errors);
    let js = result
        .files
        .as_ref()
        .expect("Expected JS output")
        .js
        .clone();

    assert!(
        js.contains("const header = (function()"),
        "Expected header ref wrapper"
    );
    assert!(
        js.contains("const footer = (function()"),
        "Expected footer ref wrapper"
    );
}
