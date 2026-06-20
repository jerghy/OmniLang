//! Codegen snapshot tests.
//!
//! These tests construct IR programs directly and verify that each codegen
//! backend produces the expected output. They serve as "snapshot" tests
//! for the basic structure of generated code.

use std::collections::HashMap;

use omni_compiler::codegen::javascript::JavaScriptCodegen;
use omni_compiler::codegen::python::PythonCodegen;
use omni_compiler::codegen::*;
use omni_compiler::ir::*;

// ═══════════════════════════════════════════════════════════════
// Test IR factories
// ═══════════════════════════════════════════════════════════════

fn make_test_module() -> ModuleIR {
    ModuleIR {
        name: "test.omni".into(),
        functions: {
            let mut m = HashMap::new();
            m.insert(
                "main".into(),
                FunctionIR {
                    name: "main".into(),
                    visibility: "private".into(),
                    params: vec![],
                    return_type: None,
                    body: BlockIR {
                        statements: vec![],
                        execution_mode: "client-only".into(),
                    },
                    is_async: false,
                },
            );
            m
        },
        structs: {
            let mut m = HashMap::new();
            m.insert(
                "Point".into(),
                StructIR {
                    name: "Point".into(),
                    visibility: "public".into(),
                    fields: vec![
                        FieldIR {
                            name: "x".into(),
                            r#type: TypeIR {
                                kind: "i32".into(),
                                type_args: vec![],
                                is_nullable: false,
                                needs_clone: false,
                                needs_partial_eq: false,
                            },
                            visibility: "public".into(),
                        },
                        FieldIR {
                            name: "y".into(),
                            r#type: TypeIR {
                                kind: "i32".into(),
                                type_args: vec![],
                                is_nullable: false,
                                needs_clone: false,
                                needs_partial_eq: false,
                            },
                            visibility: "public".into(),
                        },
                    ],
                },
            );
            m
        },
        ..Default::default()
    }
}

fn make_test_ir() -> IRProgram {
    let mut modules = HashMap::new();
    modules.insert("test.omni".into(), make_test_module());
    IRProgram {
        version: "1.0".into(),
        modules,
        source_map: HashMap::new(),
        stdlib_imports: Vec::new(),
    }
}

// ═══════════════════════════════════════════════════════════════
// Python codegen
// ═══════════════════════════════════════════════════════════════

#[test]
fn snapshot_python_basic() {
    let ir = make_test_ir();
    let codegen = PythonCodegen;
    let output = codegen.generate(&ir, &HashMap::new());
    assert!(output.errors.is_empty());
    let content = &output.files[0].content;
    assert!(content.contains("def main():"));
    assert!(content.contains("class Point:"));
    assert!(content.contains("x: int"));
}

// ═══════════════════════════════════════════════════════════════
// JavaScript codegen (non-TS mode)
// ═══════════════════════════════════════════════════════════════

#[test]
fn snapshot_javascript_basic() {
    let ir = make_test_ir();
    let codegen = JavaScriptCodegen::new(false);
    let output = codegen.generate(&ir, &HashMap::new());
    assert!(output.errors.is_empty());
    let content = &output.files[0].content;
    assert!(content.contains("function main()"));
    assert!(content.contains("class Point"));
}

// ═══════════════════════════════════════════════════════════════
// TypeScript codegen (TS mode)
// ═══════════════════════════════════════════════════════════════

#[test]
fn snapshot_typescript_basic() {
    let ir = make_test_ir();
    let codegen = JavaScriptCodegen::new(true);
    let output = codegen.generate(&ir, &HashMap::new());
    assert!(output.errors.is_empty());
    let content = &output.files[0].content;
    // In TS mode, struct constructor params have type annotations
    assert!(content.contains(": number"));
}
