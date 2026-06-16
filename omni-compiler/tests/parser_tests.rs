use omni_compiler::lexer::tokenize;
use omni_compiler::parser::parse;

#[test]
fn test_parse_empty_program() {
    let tokens = tokenize("");
    let result = parse(&tokens);
    assert!(result.is_ok());
    let program = result.unwrap();
    assert!(program.components.is_empty());
    assert!(program.functions.is_empty());
}

#[test]
fn test_parse_import() {
    let tokens = tokenize(r#"import React from "react";"#);
    let result = parse(&tokens);
    assert!(result.is_ok());
    let program = result.unwrap();
    assert_eq!(program.imports.len(), 1);
    assert_eq!(program.imports[0].names, vec!["React"]);
    assert_eq!(program.imports[0].path, "react");
}

#[test]
fn test_parse_struct_def() {
    let source = r#"
struct Point {
    x: i32,
    y: i32,
}
"#;
    let tokens = tokenize(source);
    let result = parse(&tokens);
    assert!(result.is_ok(), "Parse error: {:?}", result.err());
    let program = result.unwrap();
    assert_eq!(program.structs.len(), 1);
    assert_eq!(program.structs[0].name, "Point");
    assert_eq!(program.structs[0].fields.len(), 2);
}

#[test]
fn test_parse_function_def() {
    let source = r#"
fn add(a: i32, b: i32) -> i32 {
    return a + b;
}
"#;
    let tokens = tokenize(source);
    let result = parse(&tokens);
    assert!(result.is_ok(), "Parse error: {:?}", result.err());
    let program = result.unwrap();
    assert_eq!(program.functions.len(), 1);
    assert_eq!(program.functions[0].name, "add");
    assert_eq!(program.functions[0].params.len(), 2);
}

#[test]
fn test_parse_empty_component() {
    let source = r#"
component App {
    view {
        <Container/>
    }
}
"#;
    let tokens = tokenize(source);
    let result = parse(&tokens);
    assert!(result.is_ok(), "Parse error: {:?}", result.err());
}

#[test]
fn test_parse_counter_component() {
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
    let result = parse(&tokens);
    match &result {
        Ok(program) => {
            assert_eq!(program.components.len(), 1);
            let comp = &program.components[0];
            assert_eq!(comp.name, "Counter");
            assert_eq!(comp.signals.len(), 1);
            assert_eq!(comp.signals[0].name, "count");
            assert_eq!(comp.fns.len(), 1);
            assert!(comp.view.is_some());
        }
        Err(e) => panic!("Parse error: {}", e.message),
    }
}
