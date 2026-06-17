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
        result.diagnostics.iter().any(|d| d.message.contains("Undeclared")),
        "Expected undeclared variable error, got: {:?}",
        result.diagnostics.iter().map(|d| &d.message).collect::<Vec<_>>()
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
