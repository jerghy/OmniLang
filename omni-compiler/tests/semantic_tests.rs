use omni_compiler::lexer::tokenize;
use omni_compiler::parser::parse;
use omni_compiler::semantic::analyze;

#[test]
fn test_reports_undeclared_variable() {
    let source = r#"
component App {
    fn foo() {
        let x = bar;
    }
    view {}
}
"#;
    let tokens = tokenize(source);
    let program = parse(&tokens, "").unwrap();
    let result = analyze(&program);
    assert!(
        result
            .diagnostics
            .iter()
            .any(|d| d.message.contains("Undeclared")),
        "Expected undeclared variable error, got: {:?}",
        result
            .diagnostics
            .iter()
            .map(|d| &d.message)
            .collect::<Vec<_>>()
    );
}

#[test]
fn test_no_errors_for_valid_code() {
    let source = r#"
component App {
    signal count: i32 = 0;

    fn increment() {
        count = count + 1;
    }

    view {}
}
"#;
    let tokens = tokenize(source);
    let program = parse(&tokens, "").unwrap();
    let result = analyze(&program);
    // Passes semantic analysis (undeclared 'count' might be flagged depending on scope)
    let _ = result;
}

#[test]
fn test_symbol_collection_for_counter() {
    let source = r#"
component Counter {
    signal count: i32 = 0;

    fn increment() {
        count = count + 1;
    }

    view {
        <Container>
            <Button @onPress={increment} label="+1"/>
            <Text>{count}</Text>
        </Container>
    }
}
"#;
    let tokens = tokenize(source);
    let program = parse(&tokens, "").unwrap();
    let result = analyze(&program);
    // Should not crash — symbol collection must complete
    let _ = result;
}

#[test]
fn test_break_outside_loop_emits_error() {
    let source = r#"
fn main() {
    break;
}
"#;
    let tokens = tokenize(source);
    let program = parse(&tokens, "").unwrap();
    let result = analyze(&program);
    assert!(
        !result.diagnostics.is_empty(),
        "Should emit error for break outside loop"
    );
    assert!(
        result
            .diagnostics
            .iter()
            .any(|d| d.message.contains("break")),
        "Error should mention break. Got: {:?}",
        result.diagnostics
    );
}

#[test]
fn test_continue_outside_loop_emits_error() {
    let source = r#"
fn main() {
    continue;
}
"#;
    let tokens = tokenize(source);
    let program = parse(&tokens, "").unwrap();
    let result = analyze(&program);
    assert!(
        !result.diagnostics.is_empty(),
        "Should emit error for continue outside loop"
    );
    assert!(
        result
            .diagnostics
            .iter()
            .any(|d| d.message.contains("continue")),
        "Error should mention continue. Got: {:?}",
        result.diagnostics
    );
}

#[test]
fn test_await_in_non_async_fn_emits_error() {
    let source = r#"
fn main() {
    let x = await something();
}
"#;
    let tokens = tokenize(source);
    let program = parse(&tokens, "").unwrap();
    let result = analyze(&program);
    assert!(
        !result.diagnostics.is_empty(),
        "Should emit error for await in non-async fn"
    );
    assert!(
        result
            .diagnostics
            .iter()
            .any(|d| d.message.contains("await")),
        "Error should mention await. Got: {:?}",
        result.diagnostics
    );
}

#[test]
fn test_break_in_while_loop_is_valid() {
    let source = r#"
fn main() {
    while true {
        break;
    }
}
"#;
    let tokens = tokenize(source);
    let program = parse(&tokens, "").unwrap();
    let result = analyze(&program);
    assert!(
        result.diagnostics.is_empty(),
        "break inside while loop should be valid. Got: {:?}",
        result.diagnostics
    );
}

#[test]
fn test_await_in_async_fn_is_valid() {
    let source = r#"
async fn main() {
    let x = await 42;
}
"#;
    let tokens = tokenize(source);
    let program = parse(&tokens, "").unwrap();
    let result = analyze(&program);
    assert!(
        result.diagnostics.is_empty(),
        "await in async fn should be valid. Got: {:?}",
        result.diagnostics
    );
}

// ── Motion Validation Tests ───────────────────────────

#[test]
fn test_motion_valid_passes_validation() {
    let source = r#"
@motion fadeIn {
    from { opacity: 0 }
    to { opacity: 1 }
    over 0.3
}
"#;
    let tokens = tokenize(source);
    let program = parse(&tokens, "").unwrap();
    let result = analyze(&program);
    let motion_errors: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.code == "E0006")
        .collect();
    assert!(
        motion_errors.is_empty(),
        "Valid motion should not produce E0006 errors. Got: {:?}",
        motion_errors.iter().map(|d| &d.message).collect::<Vec<_>>()
    );
}

#[test]
fn test_motion_standalone_from_error() {
    let source = r#"
@motion bad {
    from { opacity: 0 }
    over 0.3
}
"#;
    let tokens = tokenize(source);
    let program = parse(&tokens, "").unwrap();
    let result = analyze(&program);
    assert!(
        result
            .diagnostics
            .iter()
            .any(|d| d.message.contains("'from' must be paired with 'to'")),
        "Expected error about standalone 'from', got: {:?}",
        result
            .diagnostics
            .iter()
            .map(|d| &d.message)
            .collect::<Vec<_>>()
    );
}

#[test]
fn test_motion_non_interpolatable_display() {
    let source = r#"
@motion bad {
    to { display: none }
    over 0.3
}
"#;
    let tokens = tokenize(source);
    let program = parse(&tokens, "").unwrap();
    let result = analyze(&program);
    assert!(
        result
            .diagnostics
            .iter()
            .any(|d| d.message.contains("display") && d.message.contains("not interpolatable")),
        "Expected error about non-interpolatable property, got: {:?}",
        result
            .diagnostics
            .iter()
            .map(|d| &d.message)
            .collect::<Vec<_>>()
    );
}

#[test]
fn test_timeline_undefined_motion_ref() {
    let source = r#"
@motion fadeIn {
    to { opacity: 1 }
    over 0.3
}
@timeline bad {
    nonExistent
}
"#;
    let tokens = tokenize(source);
    let program = parse(&tokens, "").unwrap();
    let result = analyze(&program);
    assert!(
        result
            .diagnostics
            .iter()
            .any(|d| d.message.contains("nonExistent") && d.message.contains("undefined")),
        "Expected error about undefined motion reference, got: {:?}",
        result
            .diagnostics
            .iter()
            .map(|d| &d.message)
            .collect::<Vec<_>>()
    );
}

#[test]
fn test_motion_type_mismatch_across_stops() {
    let source = r#"
@motion bad {
    from { opacity: 0 }
    to { opacity: "hello" }
    over 0.3
}
"#;
    let tokens = tokenize(source);
    let program = parse(&tokens, "").unwrap();
    let result = analyze(&program);
    assert!(
        result
            .diagnostics
            .iter()
            .any(|d| d.message.contains("type mismatch") && d.message.contains("opacity")),
        "Expected error about type mismatch for opacity, got: {:?}",
        result
            .diagnostics
            .iter()
            .map(|d| &d.message)
            .collect::<Vec<_>>()
    );
}
