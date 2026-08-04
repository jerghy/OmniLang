//! End-to-end compilation tests for the OmniLang compiler.
//!
//! Tests the full pipeline: source text -> lexer -> parser -> semantic analysis
//! -> IR lowering -> codegen, verifying output at each stage.

#[cfg(feature = "codegen-python")]
use rustc_hash::FxHashMap;

/// Helper: compile single source and unwrap the IR.
fn compile_ok(source: &str) -> omnilang::ir::IRProgram {
    let result = omnilang::compile(source);
    assert!(
        result.success,
        "compilation should succeed, got errors: {:?}",
        result.errors
    );
    result.ir.expect("IR should be present on success")
}

#[test]
fn e2e_simple_function() {
    let ir = compile_ok("fn main() -> i32 { return 42; }");
    assert_eq!(ir.version, "1.0");
    let module = ir.modules.get("input.omni").expect("should have module");
    assert!(
        module.functions.contains_key("main"),
        "should contain 'main' function"
    );
}

#[test]
fn e2e_function_with_params() {
    let ir = compile_ok("fn add(a: i32, b: i32) -> i32 { return a + b; }");
    let module = ir.modules.get("input.omni").expect("should have module");
    let func = module
        .functions
        .get("add")
        .expect("should have add function");
    assert_eq!(func.params.len(), 2, "add should have 2 params");
    assert_eq!(func.params[0].name, "a");
    assert_eq!(func.params[1].name, "b");
}

#[test]
fn e2e_invalid_syntax_produces_errors() {
    let result = omnilang::compile("fn main() { let x = ; }");
    assert!(
        !result.success,
        "compilation with invalid syntax should fail"
    );
    assert!(
        !result.errors.is_empty(),
        "should have at least one error diagnostic"
    );
}

#[test]
fn e2e_struct_and_function() {
    let ir = compile_ok(
        "struct Point { x: i32, y: i32 }
         fn origin() -> Point { return Point { x: 0, y: 0 }; }",
    );
    let module = ir.modules.get("input.omni").expect("should have module");
    assert!(
        module.structs.contains_key("Point"),
        "should contain Point struct"
    );
    assert!(
        module.functions.contains_key("origin"),
        "should contain origin function"
    );
}

#[cfg(feature = "codegen-python")]
#[test]
fn e2e_compile_to_python_json() {
    let mut sources = FxHashMap::default();
    sources.insert(
        "main.omni".to_string(),
        "fn main() { print(\"hello\"); }".to_string(),
    );
    let mut path_mapping = FxHashMap::default();
    path_mapping.insert("main.omni".to_string(), "main.omni".to_string());
    path_mapping.insert("./main.omni".to_string(), "main.omni".to_string());

    let json = omnilang::compile_multi_to_target(
        sources,
        omnilang::codegen::Target::Python,
        path_mapping,
        false,
        false,
        None,
    );
    let data: serde_json::Value = serde_json::from_str(&json).expect("should be valid JSON");
    assert!(
        data["success"].as_bool().unwrap_or(false),
        "Python compilation should succeed, got: {:?}",
        data
    );
    let files = data["files"].as_object().expect("should have files");
    assert!(!files.is_empty(), "should have at least one output file");
    let has_py_file = files.keys().any(|k| k.ends_with(".py"));
    assert!(
        has_py_file,
        "should have a .py output file, got keys: {:?}",
        files.keys()
    );
}

/// Regression test: TemplateString lowering must produce a non-null value.
/// Before the fix, the catch-all wildcard in lower_expr silently converted
/// TemplateString to serde_json::Value::Null.
#[test]
fn template_string_lowering_not_null() {
    let ir = compile_ok(r#"fn greet(name: string) -> string { return `Hello ${name}!`; }"#);
    let module = ir.modules.get("input.omni").expect("should have module");
    let func = module
        .functions
        .get("greet")
        .expect("should have greet function");
    // Verify the function body contains the template string expression, not null
    let body_str = format!("{:?}", func.body);
    assert!(
        !body_str.contains("Null"),
        "template string should not be lowered to Null"
    );
}

/// Regression test: verify that valid expressions are not caught by catch-all.
#[test]
fn catch_all_wildcard_regression() {
    // Test expressions that should produce non-null IR values
    let tests = vec![
        "fn main() -> i32 { return 1 + 2; }",
        "fn main() -> bool { return true && false; }",
        "fn main() -> string { return \"hello\"; }",
    ];
    for src in tests {
        let result = omnilang::compile(src);
        assert!(result.success, "compilation should succeed for: {}", src);
    }
}

/// Array language methods (design doc 域 8): `.filter/.map/.count` are
/// rewritten to ExprIR::ArrayMethod by the semantic pass. `count()` without
/// a predicate is the design-doc diagnostic 「count 需要一个谓词闭包」.
#[test]
fn e2e_array_method_rewrite_and_count_diagnostic() {
    // Well-formed filter → IR node; module-level fn path (script-style).
    let ir = compile_ok(
        "fn pick(items: Array<i32>) -> Array<i32> { return items.filter(|i| i % 2 == 0); }",
    );
    let module = ir.modules.get("input.omni").expect("should have module");
    let body_str = format!("{:?}", module.functions["pick"].body);
    assert!(
        body_str.contains("ArrayMethod") && body_str.contains("Filter"),
        "filter call must lower to ExprIR::ArrayMethod, got: {}",
        body_str
    );

    // count() with no predicate → design-doc diagnostic (Chinese, verbatim
    // per the v2 policy recorded in the SDD ledger).
    let result = omnilang::compile("fn main() -> i32 { return items.count(); }");
    assert!(!result.success, "count() without predicate must fail");
    assert!(
        result
            .errors
            .iter()
            .any(|d| d.message == "count 需要一个谓词闭包"),
        "expected the design-doc diagnostic, got: {:?}",
        result.errors
    );

    // count() with a predicate → ArrayMethod::Count.
    let ir = compile_ok("fn main(items: Array<i32>) -> i32 { return items.count(|i| i > 0); }");
    let module = ir.modules.get("input.omni").expect("should have module");
    let body_str = format!("{:?}", module.functions["main"].body);
    assert!(
        body_str.contains("ArrayMethod") && body_str.contains("Count"),
        "count call must lower to ExprIR::ArrayMethod, got: {}",
        body_str
    );
}
