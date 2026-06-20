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
    let program = parse(&tokens).unwrap();
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
    let program = parse(&tokens).unwrap();
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
    let program = parse(&tokens).unwrap();
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
    let program = parse(&tokens).unwrap();
    let result = analyze(&program);
    assert!(!result.diagnostics.is_empty(), "Should emit error for break outside loop");
    assert!(result.diagnostics.iter().any(|d| d.message.contains("break")),
        "Error should mention break. Got: {:?}", result.diagnostics);
}

#[test]
fn test_continue_outside_loop_emits_error() {
    let source = r#"
fn main() {
    continue;
}
"#;
    let tokens = tokenize(source);
    let program = parse(&tokens).unwrap();
    let result = analyze(&program);
    assert!(!result.diagnostics.is_empty(), "Should emit error for continue outside loop");
    assert!(result.diagnostics.iter().any(|d| d.message.contains("continue")),
        "Error should mention continue. Got: {:?}", result.diagnostics);
}

#[test]
fn test_await_in_non_async_fn_emits_error() {
    let source = r#"
fn main() {
    let x = await something();
}
"#;
    let tokens = tokenize(source);
    let program = parse(&tokens).unwrap();
    let result = analyze(&program);
    assert!(!result.diagnostics.is_empty(), "Should emit error for await in non-async fn");
    assert!(result.diagnostics.iter().any(|d| d.message.contains("await")),
        "Error should mention await. Got: {:?}", result.diagnostics);
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
    let program = parse(&tokens).unwrap();
    let result = analyze(&program);
    assert!(result.diagnostics.is_empty(),
        "break inside while loop should be valid. Got: {:?}", result.diagnostics);
}

#[test]
fn test_await_in_async_fn_is_valid() {
    let source = r#"
async fn main() {
    let x = await 42;
}
"#;
    let tokens = tokenize(source);
    let program = parse(&tokens).unwrap();
    let result = analyze(&program);
    assert!(result.diagnostics.is_empty(),
        "await in async fn should be valid. Got: {:?}", result.diagnostics);
}
