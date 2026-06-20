//! Integration tests for pure-logic codegen (Python, JavaScript, Rust, Dart).
//!
//! Tests the unimplemented stubs initially (RED phase), then the real
//! implementation (GREEN phase).

use std::collections::HashMap;

use omni_compiler::codegen::{compile, Target};
use omni_compiler::ir::*;

// ═══════════════════════════════════════════════════════════
// Helpers
// ═══════════════════════════════════════════════════════════

fn make_ir(
    components: HashMap<String, ComponentIR>,
    stores: HashMap<String, StoreIR>,
) -> IRProgram {
    let mut modules = HashMap::new();
    modules.insert(
        "main".to_string(),
        ModuleIR {
            name: "main".to_string(),
            components,
            stores,
            ..Default::default()
        },
    );
    IRProgram {
        version: "0.1".into(),
        modules,
        source_map: HashMap::new(),
        stdlib_imports: Vec::new(),
    }
}

fn empty_ir() -> IRProgram {
    make_ir(HashMap::new(), HashMap::new())
}

fn simple_component_ir(comp_name: &str) -> IRProgram {
    let mut components = HashMap::new();
    let comp = ComponentIR {
        name: comp_name.into(),
        generic_params: vec![],
        props: HashMap::new(),
        prop_defaults: HashMap::new(),
        states: vec![],
        computeds: vec![],
        effects: vec![],
        lifecycles: vec![],
        fns: vec![],
        store_refs: vec![],
        refs: vec![],
        event_names: vec![],
        view: None,
        styles: vec![],
    };
    components.insert(comp_name.into(), comp);
    make_ir(components, HashMap::new())
}

/// Build an IR program with one component that has a single function.
fn ir_with_function(comp_name: &str, fn_name: &str) -> IRProgram {
    let mut components = HashMap::new();
    let func = FunctionIR {
        name: fn_name.into(),
        visibility: "private".to_string(),
        params: vec![],
        return_type: None,
        body: BlockIR {
            statements: vec![],
            execution_mode: "client-only".into(),
        },
        is_async: false,
    };
    let comp = ComponentIR {
        name: comp_name.into(),
        generic_params: vec![],
        props: HashMap::new(),
        prop_defaults: HashMap::new(),
        states: vec![],
        computeds: vec![],
        effects: vec![],
        lifecycles: vec![],
        fns: vec![func],
        store_refs: vec![],
        refs: vec![],
        event_names: vec![],
        view: None,
        styles: vec![],
    };
    components.insert(comp_name.into(), comp);
    make_ir(components, HashMap::new())
}

// ═══════════════════════════════════════════════════════════
// Empty program tests (all targets)
// ═══════════════════════════════════════════════════════════

#[test]
fn test_python_empty_program() {
    let ir = empty_ir();
    let sources = HashMap::from([("main.omni".into(), "".into())]);
    let result = compile(&ir, Target::Python, &sources);
    assert!(result.errors.is_empty(), "errors: {:?}", result.errors);
    assert_eq!(result.files.len(), 1);
    assert_eq!(result.files[0].path, "main.py");
    assert!(
        result.files[0].content.contains("Generated from"),
        "content: {}",
        result.files[0].content
    );
}

#[test]
fn test_python_simple_component() {
    let ir = simple_component_ir("App");
    let sources = HashMap::from([("main.omni".into(), "".into())]);
    let result = compile(&ir, Target::Python, &sources);
    assert!(result.errors.is_empty(), "errors: {:?}", result.errors);
    let content = &result.files[0].content;
    assert!(
        content.contains("class App"),
        "Should contain class App: {}",
        content
    );
}

#[test]
fn test_python_simple_function() {
    let ir = ir_with_function("App", "greet");
    let sources = HashMap::from([("main.omni".into(), "".into())]);
    let result = compile(&ir, Target::Python, &sources);
    assert!(result.errors.is_empty(), "errors: {:?}", result.errors);
    let content = &result.files[0].content;
    assert!(
        content.contains("def greet"),
        "Should contain def greet: {}",
        content
    );
}

#[test]
fn test_javascript_empty_program() {
    let ir = empty_ir();
    let sources = HashMap::from([("main.omni".into(), "".into())]);
    let result = compile(&ir, Target::JavaScript, &sources);
    assert!(result.errors.is_empty(), "errors: {:?}", result.errors);
    assert_eq!(result.files[0].path, "main.js");
    assert!(
        result.files[0].content.contains("Generated from"),
        "content: {}",
        result.files[0].content
    );
}

#[test]
fn test_rust_empty_program() {
    let ir = empty_ir();
    let sources = HashMap::from([("main.omni".into(), "".into())]);
    let result = compile(&ir, Target::Rust, &sources);
    assert!(result.errors.is_empty(), "errors: {:?}", result.errors);
    assert_eq!(result.files[0].path, "main.rs");
    assert!(
        result.files[0].content.contains("Generated from"),
        "content: {}",
        result.files[0].content
    );
}

#[test]
fn test_dart_empty_program() {
    let ir = empty_ir();
    let sources = HashMap::from([("main.omni".into(), "".into())]);
    let result = compile(&ir, Target::Dart, &sources);
    assert!(result.errors.is_empty(), "errors: {:?}", result.errors);
    assert_eq!(result.files[0].path, "main.dart");
    assert!(
        result.files[0].content.contains("Generated from"),
        "content: {}",
        result.files[0].content
    );
}

#[test]
fn test_compile_multiple_files() {
    let mut modules = HashMap::new();
    modules.insert(
        "main".to_string(),
        ModuleIR {
            name: "main".to_string(),
            ..Default::default()
        },
    );
    modules.insert(
        "utils".to_string(),
        ModuleIR {
            name: "utils".to_string(),
            ..Default::default()
        },
    );
    let ir = IRProgram {
        version: "0.1".into(),
        modules,
        source_map: HashMap::new(),
    stdlib_imports: Vec::new(),
    };
    let sources = HashMap::from([
        ("main.omni".into(), "".into()),
        ("utils.omni".into(), "".into()),
    ]);
    let result = compile(&ir, Target::Python, &sources);
    assert!(result.errors.is_empty(), "errors: {:?}", result.errors);
    assert_eq!(result.files.len(), 2);
    // Files should have correct extensions
    let paths: Vec<&str> = result.files.iter().map(|f| f.path.as_str()).collect();
    assert!(paths.contains(&"main.py"));
    assert!(paths.contains(&"utils.py"));
}
