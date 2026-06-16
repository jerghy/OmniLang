use omni_compiler::lexer::tokenize;
use omni_compiler::parser::parse;
use omni_compiler::parser::ast::{ImportStmt, Visibility};

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
    let tokens = tokenize(r#"import { React } from "./react.omni";"#);
    let result = parse(&tokens);
    assert!(result.is_ok());
    let program = result.unwrap();
    assert_eq!(program.imports.len(), 1);
    match &program.imports[0] {
        ImportStmt::Named(n) => {
            assert_eq!(n.names, vec!["React"]);
            assert_eq!(n.path, "./react.omni");
        }
        _ => panic!("expected Named import"),
    }
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

// ── New import/export/pub syntax tests (Task 3.3) ──────

#[test]
fn test_parse_namespace_import() {
    let tokens = tokenize(r#"import "./lib/math.omni" as math;"#);
    let result = parse(&tokens);
    assert!(result.is_ok(), "Parse error: {:?}", result.err());
    let program = result.unwrap();
    match &program.imports[0] {
        ImportStmt::Namespace(ns) => {
            assert_eq!(ns.path, "./lib/math.omni");
            assert_eq!(ns.alias, "math");
        }
        _ => panic!("expected Namespace import"),
    }
}

#[test]
fn test_parse_type_import() {
    let tokens = tokenize(r#"import type { User, Order } from "./types.omni";"#);
    let result = parse(&tokens);
    assert!(result.is_ok(), "Parse error: {:?}", result.err());
    let program = result.unwrap();
    match &program.imports[0] {
        ImportStmt::TypeOnly(t) => {
            assert_eq!(t.names, vec!["User", "Order"]);
            assert_eq!(t.path, "./types.omni");
        }
        _ => panic!("expected TypeOnly import"),
    }
}

#[test]
fn test_parse_lib_import() {
    let tokens = tokenize("import omniview;");
    let result = parse(&tokens);
    assert!(result.is_ok(), "Parse error: {:?}", result.err());
    let program = result.unwrap();
    match &program.imports[0] {
        ImportStmt::Lib(name) => assert_eq!(name, "omniview"),
        _ => panic!("expected Lib import"),
    }
}

#[test]
fn test_parse_export_from() {
    let tokens = tokenize(r#"export { Button, Card } from "./ui.omni";"#);
    let result = parse(&tokens);
    assert!(result.is_ok(), "Parse error: {:?}", result.err());
    let program = result.unwrap();
    assert_eq!(program.exports.len(), 1);
    assert_eq!(program.exports[0].names, vec!["Button", "Card"]);
    assert_eq!(program.exports[0].path, "./ui.omni");
}

#[test]
fn test_parse_pub_function() {
    let tokens = tokenize("pub fn add(a: i32, b: i32) -> i32 { a + b }");
    let result = parse(&tokens);
    assert!(result.is_ok(), "Parse error: {:?}", result.err());
    let program = result.unwrap();
    assert_eq!(program.functions.len(), 1);
    assert_eq!(program.functions[0].visibility, Visibility::Public);
    assert_eq!(program.functions[0].name, "add");
}

#[test]
fn test_parse_private_function() {
    let tokens = tokenize("fn helper(x: i32) -> i32 { x * 2 }");
    let result = parse(&tokens);
    assert!(result.is_ok(), "Parse error: {:?}", result.err());
    let program = result.unwrap();
    assert_eq!(program.functions[0].visibility, Visibility::Private);
}

#[test]
fn test_parse_pub_struct() {
    let tokens = tokenize("pub struct Point { x: i32, y: i32, }");
    let result = parse(&tokens);
    assert!(result.is_ok(), "Parse error: {:?}", result.err());
    let program = result.unwrap();
    assert_eq!(program.structs.len(), 1);
    assert_eq!(program.structs[0].visibility, Visibility::Public);
}

#[test]
fn test_parse_pub_type_alias() {
    let tokens = tokenize("pub type ID = i32;");
    let result = parse(&tokens);
    assert!(result.is_ok(), "Parse error: {:?}", result.err());
    let program = result.unwrap();
    assert_eq!(program.type_aliases.len(), 1);
    assert_eq!(program.type_aliases[0].visibility, Visibility::Public);
    assert_eq!(program.type_aliases[0].name, "ID");
}

#[test]
fn test_parse_private_type_alias() {
    let tokens = tokenize("type Secret = string;");
    let result = parse(&tokens);
    assert!(result.is_ok(), "Parse error: {:?}", result.err());
    let program = result.unwrap();
    assert_eq!(program.type_aliases.len(), 1);
    assert_eq!(program.type_aliases[0].visibility, Visibility::Private);
    assert_eq!(program.type_aliases[0].name, "Secret");
}
