use omni_compiler::lexer::tokenize;
use omni_compiler::parser::ast::{ImportStmt, Module, Visibility};
use omni_compiler::parser::parse;

/// Helper: extract the single module from a parsed Program.
fn single_module(program: omni_compiler::parser::ast::Program) -> Module {
    program.modules.into_iter().next().unwrap().1
}

#[test]
fn test_parse_empty_program() {
    let tokens = tokenize("");
    let result = parse(&tokens, "");
    assert!(result.is_ok());
    let program = result.unwrap();
    let module = single_module(program);
    assert!(module.components.is_empty());
    assert!(module.functions.is_empty());
}

#[test]
fn test_parse_import() {
    let tokens = tokenize(r#"import { React } from "./react.omni";"#);
    let result = parse(&tokens, "");
    assert!(result.is_ok());
    let program = result.unwrap();
    let module = single_module(program);
    assert_eq!(module.imports.len(), 1);
    match &module.imports[0] {
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
    let result = parse(&tokens, "");
    assert!(result.is_ok(), "Parse error: {:?}", result.err());
    let program = result.unwrap();
    let module = single_module(program);
    assert_eq!(module.structs.len(), 1);
    assert_eq!(module.structs[0].name, "Point");
    assert_eq!(module.structs[0].fields.len(), 2);
}

#[test]
fn test_parse_function_def() {
    let source = r#"
fn add(a: i32, b: i32) -> i32 {
    return a + b;
}
"#;
    let tokens = tokenize(source);
    let result = parse(&tokens, "");
    assert!(result.is_ok(), "Parse error: {:?}", result.err());
    let program = result.unwrap();
    let module = single_module(program);
    assert_eq!(module.functions.len(), 1);
    assert_eq!(module.functions[0].name, "add");
    assert_eq!(module.functions[0].params.len(), 2);
}

#[test]
fn test_parse_async_fn_before_keyword() {
    let source = r#"
async fn fetchData() -> String {
    return "data";
}
"#;
    let tokens = tokenize(source);
    let result = parse(&tokens, "");
    assert!(result.is_ok(), "Parse error: {:?}", result.err());
    let program = result.unwrap();
    let module = single_module(program);
    assert_eq!(module.functions.len(), 1);
    assert!(module.functions[0].is_async, "function should be async");
    assert_eq!(module.functions[0].name, "fetchData");
}

#[test]
fn test_parse_async_fn_after_return_type() {
    let source = r#"
fn fetchData() -> String async {
    return "data";
}
"#;
    let tokens = tokenize(source);
    let result = parse(&tokens, "");
    assert!(result.is_ok(), "Parse error: {:?}", result.err());
    let program = result.unwrap();
    let module = single_module(program);
    assert_eq!(module.functions.len(), 1);
    assert!(module.functions[0].is_async, "function should be async");
    assert_eq!(module.functions[0].name, "fetchData");
}

#[test]
fn test_parse_async_fn_no_return_type() {
    let source = r#"
async fn log(msg: String) {
    print(msg);
}
"#;
    let tokens = tokenize(source);
    let result = parse(&tokens, "");
    assert!(result.is_ok(), "Parse error: {:?}", result.err());
    let program = result.unwrap();
    let module = single_module(program);
    assert_eq!(module.functions.len(), 1);
    assert!(module.functions[0].is_async, "function should be async");
    assert_eq!(module.functions[0].name, "log");
}

#[test]
fn test_parse_await_expression() {
    let source = r#"
async fn demo() -> String {
    return await fetch("/api");
}
"#;
    let tokens = tokenize(source);
    let result = parse(&tokens, "");
    assert!(result.is_ok(), "Parse error: {:?}", result.err());
    let program = result.unwrap();
    let module = single_module(program);
    assert_eq!(module.functions.len(), 1);
    assert!(module.functions[0].is_async);
}

#[test]
fn test_parse_await_method_call() {
    // await obj.method(args) should parse as await(obj.method(args))
    let source = r#"
async fn demo() -> Data {
    return await http.get("/api/users");
}
"#;
    let tokens = tokenize(source);
    let result = parse(&tokens, "");
    assert!(result.is_ok(), "Parse error: {:?}", result.err());
}

#[test]
fn test_parse_await_in_non_async_fn() {
    // await in non-async fn is syntactically valid (semantic lint is separate)
    let source = r#"
fn demo() -> String {
    return await fetch("/api");
}
"#;
    let tokens = tokenize(source);
    let result = parse(&tokens, "");
    assert!(result.is_ok(), "Parse error: {:?}", result.err());
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
    let result = parse(&tokens, "");
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
    let result = parse(&tokens, "");
    match &result {
        Ok(program) => {
            let module = &program.modules["input.omni"];
            assert_eq!(module.components.len(), 1);
            let comp = &module.components[0];
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
    let result = parse(&tokens, "");
    assert!(result.is_ok(), "Parse error: {:?}", result.err());
    let program = result.unwrap();
    let module = single_module(program);
    match &module.imports[0] {
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
    let result = parse(&tokens, "");
    assert!(result.is_ok(), "Parse error: {:?}", result.err());
    let program = result.unwrap();
    let module = single_module(program);
    match &module.imports[0] {
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
    let result = parse(&tokens, "");
    assert!(result.is_ok(), "Parse error: {:?}", result.err());
    let program = result.unwrap();
    let module = single_module(program);
    match &module.imports[0] {
        ImportStmt::Lib(name) => assert_eq!(name, "omniview"),
        _ => panic!("expected Lib import"),
    }
}

#[test]
fn test_parse_export_from() {
    let tokens = tokenize(r#"export { Button, Card } from "./ui.omni";"#);
    let result = parse(&tokens, "");
    assert!(result.is_ok(), "Parse error: {:?}", result.err());
    let program = result.unwrap();
    let module = single_module(program);
    assert_eq!(module.exports.len(), 1);
    assert_eq!(module.exports[0].names, vec!["Button", "Card"]);
    assert_eq!(module.exports[0].path, "./ui.omni");
}

#[test]
fn test_parse_pub_function() {
    let tokens = tokenize("pub fn add(a: i32, b: i32) -> i32 { a + b }");
    let result = parse(&tokens, "");
    assert!(result.is_ok(), "Parse error: {:?}", result.err());
    let program = result.unwrap();
    let module = single_module(program);
    assert_eq!(module.functions.len(), 1);
    assert_eq!(module.functions[0].visibility, Visibility::Public);
    assert_eq!(module.functions[0].name, "add");
}

#[test]
fn test_parse_private_function() {
    let tokens = tokenize("fn helper(x: i32) -> i32 { x * 2 }");
    let result = parse(&tokens, "");
    assert!(result.is_ok(), "Parse error: {:?}", result.err());
    let program = result.unwrap();
    let module = single_module(program);
    assert_eq!(module.functions[0].visibility, Visibility::Private);
}

#[test]
fn test_parse_pub_struct() {
    let tokens = tokenize("pub struct Point { x: i32, y: i32, }");
    let result = parse(&tokens, "");
    assert!(result.is_ok(), "Parse error: {:?}", result.err());
    let program = result.unwrap();
    let module = single_module(program);
    assert_eq!(module.structs.len(), 1);
    assert_eq!(module.structs[0].visibility, Visibility::Public);
}

#[test]
fn test_parse_pub_type_alias() {
    let tokens = tokenize("pub type ID = i32;");
    let result = parse(&tokens, "");
    assert!(result.is_ok(), "Parse error: {:?}", result.err());
    let program = result.unwrap();
    let module = single_module(program);
    assert_eq!(module.type_aliases.len(), 1);
    assert_eq!(module.type_aliases[0].visibility, Visibility::Public);
    assert_eq!(module.type_aliases[0].name, "ID");
}

#[test]
fn test_parse_private_type_alias() {
    let tokens = tokenize("type Secret = string;");
    let result = parse(&tokens, "");
    assert!(result.is_ok(), "Parse error: {:?}", result.err());
    let program = result.unwrap();
    let module = single_module(program);
    assert_eq!(module.type_aliases.len(), 1);
    assert_eq!(module.type_aliases[0].visibility, Visibility::Private);
    assert_eq!(module.type_aliases[0].name, "Secret");
}

// ══════════════════════════════════════════════════════
// Struct literal / tagged union variant construction
// ══════════════════════════════════════════════════════

#[test]
fn test_parse_struct_literal() {
    let tokens = tokenize("fn f() -> Point { Point { x: 1, y: 2 } }");
    let result = parse(&tokens, "");
    assert!(result.is_ok(), "Parse error: {:?}", result.err());
}

#[test]
fn test_parse_tagged_union_variant() {
    let tokens = tokenize(
        "fn f() -> HttpResult { Ok { value: 42 } }\n\
         type HttpResult = Ok { value: i32 } | Err { error: String }",
    );
    let result = parse(&tokens, "");
    assert!(result.is_ok(), "Parse error: {:?}", result.err());
    let program = result.unwrap();
    let module = single_module(program);
    // The tagged union should be parsed
    assert_eq!(module.tagged_unions.len(), 1);
    assert_eq!(module.tagged_unions[0].name, "HttpResult");
    assert_eq!(module.tagged_unions[0].variants.len(), 2);
}

#[test]
fn test_parse_struct_literal_nested() {
    let tokens = tokenize(
        "fn f() -> Rect { Rect { top_left: Point { x: 0, y: 0 }, bottom_right: Point { x: 10, y: 10 } } }",
    );
    let result = parse(&tokens, "");
    assert!(result.is_ok(), "Parse error: {:?}", result.err());
}

// ══════════════════════════════════════════════════════
// Match expression
// ══════════════════════════════════════════════════════

#[test]
fn test_parse_match_expression() {
    let tokens = tokenize(
        "fn f(r: HttpResult) -> i32 { match r { Ok { value } => { return value; }, Err => { return 0; } } }",
    );
    let result = parse(&tokens, "");
    assert!(result.is_ok(), "Parse error: {:?}", result.err());
}

#[test]
fn test_parse_match_with_wildcard() {
    let tokens = tokenize(
        "fn f(r: HttpResult) -> i32 { match r { Ok { value } => { return value; }, _ => { return -1; } } }",
    );
    let result = parse(&tokens, "");
    assert!(result.is_ok(), "Parse error: {:?}", result.err());
}
