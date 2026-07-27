//! Unit tests for dart codegen.

use super::expr::format_type_ir_dart;
#[allow(unused_imports)]
use super::*;
use crate::codegen::{format_json_value, CodeWriter, JSON_DART};
use crate::ir::*;
use rustc_hash::FxHashMap;
use std::collections::HashMap;

/// Helper: create a DartVisitor for test use.
fn visitor() -> super::visitor::DartVisitor {
    super::visitor::DartVisitor
}

// ── dart_type tests ────────────────────────────────────

#[test]
fn test_dart_type_primitives() {
    assert_eq!(dart_type("int", &[]), "int");
    assert_eq!(dart_type("i32", &[]), "int");
    assert_eq!(dart_type("i64", &[]), "int");
    assert_eq!(dart_type("f64", &[]), "double");
    assert_eq!(dart_type("bool", &[]), "bool");
    assert_eq!(dart_type("string", &[]), "String");
    assert_eq!(dart_type("void", &[]), "void");
}

#[test]
fn test_dart_type_generics() {
    let i32_arg = TypeIR {
        kind: "i32".into(),
        type_args: vec![],
        is_nullable: false,
    };
    let string_arg = TypeIR {
        kind: "string".into(),
        type_args: vec![],
        is_nullable: false,
    };

    assert_eq!(
        dart_type("Vec", std::slice::from_ref(&i32_arg)),
        "List<int>"
    );
    assert_eq!(
        dart_type("HashMap", &[string_arg.clone(), i32_arg.clone()]),
        "Map<String, int>"
    );
}

#[test]
fn test_dart_type_option() {
    let i32_arg = TypeIR {
        kind: "i32".into(),
        type_args: vec![],
        is_nullable: false,
    };
    assert_eq!(dart_type("Option", &[i32_arg]), "int?");
}

#[test]
fn test_dart_type_user_defined() {
    assert_eq!(dart_type("MyType", &[]), "MyType");
    assert_eq!(dart_type("unknown", &[]), "dynamic");
}

// ── format_json_value tests ────────────────────────────

#[test]
fn test_format_json_value() {
    assert_eq!(
        format_json_value(&serde_json::Value::Null, &JSON_DART),
        "null"
    );
    assert_eq!(
        format_json_value(&serde_json::Value::Bool(true), &JSON_DART),
        "true"
    );
    assert_eq!(
        format_json_value(&serde_json::Value::Bool(false), &JSON_DART),
        "false"
    );
    assert_eq!(format_json_value(&serde_json::json!(42), &JSON_DART), "42");
    assert_eq!(
        format_json_value(&serde_json::json!("hello"), &JSON_DART),
        "'hello'"
    );
    assert_eq!(
        format_json_value(&serde_json::json!("he'llo"), &JSON_DART),
        "'he\\'llo'"
    );
}

// ── binary_op_to_dart tests ────────────────────────────

#[test]
fn test_binary_op_to_dart() {
    assert_eq!(
        crate::codegen::binary_op_to_str(&BinaryOpIR::Add, false, false),
        "+"
    );
    assert_eq!(
        crate::codegen::binary_op_to_str(&BinaryOpIR::Eq, false, false),
        "=="
    );
    assert_eq!(
        crate::codegen::binary_op_to_str(&BinaryOpIR::And, false, false),
        "&&"
    );
    assert_eq!(
        crate::codegen::binary_op_to_str(&BinaryOpIR::Or, false, false),
        "||"
    );
}

// ── format_expr_ir tests ─────────────────────────────

#[test]
fn test_format_expr_ir_literal() {
    let expr = ExprIR::LiteralExpr {
        value: serde_json::json!(42),
    };
    assert_eq!(format_expr_ir(&expr), "42");
}

#[test]
fn test_format_expr_ir_identifier() {
    let expr = ExprIR::IdentifierExpr {
        name: "count".into(),
        type_ann: None,
    };
    assert_eq!(format_expr_ir(&expr), "count");
}

#[test]
fn test_format_expr_ir_binary() {
    let expr = ExprIR::BinaryOpExpr {
        op: BinaryOpIR::Add,
        left: Box::new(ExprIR::IdentifierExpr {
            name: "a".into(),
            type_ann: None,
        }),
        right: Box::new(ExprIR::IdentifierExpr {
            name: "b".into(),
            type_ann: None,
        }),
    };
    assert_eq!(format_expr_ir(&expr), "a + b");
}

#[test]
fn test_format_expr_ir_ternary() {
    let expr = ExprIR::TernaryIR {
        cond: Box::new(ExprIR::IdentifierExpr {
            name: "x".into(),
            type_ann: None,
        }),
        then_expr: Box::new(ExprIR::LiteralExpr {
            value: serde_json::json!(1),
        }),
        else_expr: Box::new(ExprIR::LiteralExpr {
            value: serde_json::json!(0),
        }),
    };
    assert_eq!(format_expr_ir(&expr), "x ? 1 : 0");
}

#[test]
fn test_format_expr_ir_field_access() {
    let expr = ExprIR::FieldAccessExpr {
        object: Box::new(ExprIR::IdentifierExpr {
            name: "obj".into(),
            type_ann: None,
        }),
        field: "prop".into(),
    };
    assert_eq!(format_expr_ir(&expr), "obj.prop");
}

#[test]
fn test_format_expr_ir_unary_not() {
    let expr = ExprIR::UnaryOpExpr {
        op: UnaryOpIR::Not,
        operand: Box::new(ExprIR::IdentifierExpr {
            name: "flag".into(),
            type_ann: None,
        }),
    };
    assert_eq!(format_expr_ir(&expr), "!flag");
}

#[test]
fn test_format_expr_ir_unary_neg() {
    let expr = ExprIR::UnaryOpExpr {
        op: UnaryOpIR::Neg,
        operand: Box::new(ExprIR::IdentifierExpr {
            name: "x".into(),
            type_ann: None,
        }),
    };
    assert_eq!(format_expr_ir(&expr), "-x");
}

#[test]
fn test_format_expr_ir_store_field_access() {
    let expr = ExprIR::StoreFieldAccessExpr {
        store_alias: "store".into(),
        field_name: "getCount".into(),
        call_args: vec![],
    };
    assert_eq!(format_expr_ir(&expr), "store.getCount");
}

// ── generate_function tests ───────────────────────────

#[test]
fn test_generate_function_empty() {
    let mut w = CodeWriter::new();
    let func = FunctionIR {
        name: "doSomething".into(),
        visibility: Visibility::Public,
        params: vec![],
        return_type: None,
        body: BlockIR { statements: vec![] },
        is_async: false,
    
    source_span: None,};
    generate_function(&mut w, &func);
    let out = w.into_string();
    assert!(out.contains("doSomething() {"));
}

#[test]
fn test_generate_function_with_return_type() {
    let mut w = CodeWriter::new();
    let func = FunctionIR {
        name: "getValue".into(),
        visibility: Visibility::Public,
        params: vec![],
        return_type: Some(TypeIR {
            kind: "i32".into(),
            type_args: vec![],
            is_nullable: false,
        }),
        body: BlockIR { statements: vec![] },
        is_async: false,
    
    source_span: None,};
    generate_function(&mut w, &func);
    let out = w.into_string();
    assert!(out.contains("int getValue() {"));
}

#[test]
fn test_generate_function_with_void_return() {
    let mut w = CodeWriter::new();
    let func = FunctionIR {
        name: "doThing".into(),
        visibility: Visibility::Public,
        params: vec![],
        return_type: Some(TypeIR {
            kind: "void".into(),
            type_args: vec![],
            is_nullable: false,
        }),
        body: BlockIR { statements: vec![] },
        is_async: false,
    
    source_span: None,};
    generate_function(&mut w, &func);
    let out = w.into_string();
    // void return type is implicit — no annotation emitted
    assert!(out.contains("doThing() {"));
}

#[test]
fn test_generate_function_with_params() {
    let mut w = CodeWriter::new();
    let func = FunctionIR {
        name: "greet".into(),
        visibility: Visibility::Public,
        params: vec![ParamIR {
            name: "name".into(),
            r#type: Some(TypeIR {
                kind: "string".into(),
                type_args: vec![],
                is_nullable: false,
            }),
        }],
        return_type: Some(TypeIR {
            kind: "String".into(),
            type_args: vec![],
            is_nullable: false,
        }),
        body: BlockIR { statements: vec![] },
        is_async: false,
    
    source_span: None,};
    generate_function(&mut w, &func);
    let out = w.into_string();
    // Note: dart_type("string") = "String", so the param type annotation looks like "name  String "
    assert!(out.contains("String greet("));
}

// ── generate_component tests ──────────────────────────

#[test]
fn test_generate_component_empty() {
    let mut w = CodeWriter::new();
    let comp = ComponentIR {
        name: "App".into(),
        generic_params: vec![],
        props: FxHashMap::default(),
        prop_defaults: FxHashMap::default(),
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
        motions: vec![],
        if_mode_default: None,
        hero_defs: vec![],
        timelines: vec![],
        ..Default::default()
    };
    generate_component(&mut w, "App", &comp);
    let out = w.into_string();
    assert!(out.contains("class App {"));
    assert!(out.contains("App();"));
    assert!(out.contains("}"));
}

#[test]
fn test_generate_component_with_function() {
    let mut w = CodeWriter::new();
    let comp = ComponentIR {
        name: "App".into(),
        generic_params: vec![],
        props: FxHashMap::default(),
        prop_defaults: FxHashMap::default(),
        states: vec![],
        computeds: vec![],
        effects: vec![],
        lifecycles: vec![],
        fns: vec![FunctionIR {
            name: "greet".into(),
            visibility: Visibility::Public,
            params: vec![],
            return_type: None,
            body: BlockIR { statements: vec![] },
            is_async: false,
        
        source_span: None,}],
        store_refs: vec![],
        refs: vec![],
        event_names: vec![],
        view: None,
        styles: vec![],
        motions: vec![],
        if_mode_default: None,
        hero_defs: vec![],
        timelines: vec![],
        ..Default::default()
    };
    generate_component(&mut w, "App", &comp);
    let out = w.into_string();
    assert!(out.contains("class App {"));
    assert!(out.contains("greet() {"));
}

#[test]
fn test_generate_component_with_state() {
    let mut w = CodeWriter::new();
    let comp = ComponentIR {
        name: "Counter".into(),
        generic_params: vec![],
        props: FxHashMap::default(),
        prop_defaults: FxHashMap::default(),
        states: vec![StateIntent {
            name: "count".into(),
            r#type: TypeIR {
                kind: "i32".into(),
                type_args: vec![],
                is_nullable: false,
            },
            initial_value: ExprIR::LiteralExpr {
                value: serde_json::json!(0),
            },
            is_copy: false,
        }],
        computeds: vec![],
        effects: vec![],
        lifecycles: vec![],
        fns: vec![],
        store_refs: vec![],
        refs: vec![],
        event_names: vec![],
        view: None,
        styles: vec![],
        motions: vec![],
        if_mode_default: None,
        hero_defs: vec![],
        timelines: vec![],
        ..Default::default()
    };
    generate_component(&mut w, "Counter", &comp);
    let out = w.into_string();
    assert!(out.contains("class Counter {"));
    assert!(out.contains("int count = 0;"));
}

// ── generate_store tests ──────────────────────────────

#[test]
fn test_generate_store() {
    let mut w = CodeWriter::new();
    let store = StoreIR {
        name: "AppStore".into(),
        signals: FxHashMap::from_iter([
            (
                "count".into(),
                TypeIR {
                    kind: "i32".into(),
                    type_args: vec![],
                    is_nullable: false,
                },
            ),
            (
                "name".into(),
                TypeIR {
                    kind: "string".into(),
                    type_args: vec![],
                    is_nullable: false,
                },
            ),
        ]),
        signal_defaults: FxHashMap::from_iter([
            ("count".into(), serde_json::json!(0)),
            ("name".into(), serde_json::json!("world")),
        ]),
        getters: FxHashMap::default(),
        actions: vec![],
        ..Default::default()
    };
    generate_store(&mut w, "AppStore", &store);
    let out = w.into_string();
    assert!(out.contains("class AppStore {"));
    assert!(out.contains("int count = 0;"));
    assert!(out.contains("String name = 'world';"));
}

#[test]
fn test_generate_store_empty() {
    let mut w = CodeWriter::new();
    let store = StoreIR {
        name: "EmptyStore".into(),
        signals: FxHashMap::default(),
        signal_defaults: FxHashMap::default(),
        getters: FxHashMap::default(),
        actions: vec![],
        ..Default::default()
    };
    generate_store(&mut w, "EmptyStore", &store);
    let out = w.into_string();
    assert!(out.contains("class EmptyStore {"));
    assert!(out.contains("EmptyStore();"));
}

// ── generate_statement tests ──────────────────────────

#[test]
fn test_generate_statement_let() {
    let mut w = CodeWriter::new();
    let stmt = StatementIR::LetStmtIR {
        name: "x".into(),
        value: Some(ExprIR::LiteralExpr {
            value: serde_json::json!(42),
        }),
        r#type: None,
    };
    generate_statement(&mut w, &stmt);
    assert_eq!(w.into_string(), "var x = 42;\n");
}

#[test]
fn test_generate_statement_return() {
    let mut w = CodeWriter::new();
    let stmt = StatementIR::ReturnStmtIR {
        value: Some(ExprIR::IdentifierExpr {
            name: "result".into(),
            type_ann: None,
        }),
    };
    generate_statement(&mut w, &stmt);
    assert_eq!(w.into_string(), "return result;\n");
}

#[test]
fn test_generate_statement_mutation_increment() {
    let mut w = CodeWriter::new();
    let stmt = StatementIR::MutationStmt {
        intent: Some(MutationIntent::Increment {
            target: SignalRefIR {
                owner: None,
                name: "count".into(),
            },

            amount: Some(Box::new(ExprIR::LiteralExpr {
                value: serde_json::json!(1),
            })),
        }),
    };
    generate_statement(&mut w, &stmt);
    assert_eq!(w.into_string(), "this.count += 1;\n");
}

#[test]
fn test_generate_statement_mutation_assign() {
    let mut w = CodeWriter::new();
    let stmt = StatementIR::MutationStmt {
        intent: Some(MutationIntent::DirectAssign {
            target: SignalRefIR {
                owner: None,
                name: "name".into(),
            },

            value: Some(Box::new(ExprIR::LiteralExpr {
                value: serde_json::json!("hello"),
            })),
        }),
    };
    generate_statement(&mut w, &stmt);
    assert_eq!(w.into_string(), "this.name = 'hello';\n");
}

#[test]
fn test_generate_statement_mutation_decrement() {
    let mut w = CodeWriter::new();
    let stmt = StatementIR::MutationStmt {
        intent: Some(MutationIntent::Decrement {
            target: SignalRefIR {
                owner: None,
                name: "count".into(),
            },

            amount: Some(Box::new(ExprIR::LiteralExpr {
                value: serde_json::json!(1),
            })),
        }),
    };
    generate_statement(&mut w, &stmt);
    assert_eq!(w.into_string(), "this.count -= 1;\n");
}

#[test]
fn test_generate_statement_if() {
    let mut w = CodeWriter::new();
    let stmt = StatementIR::IfStmtIR {
        condition: ExprIR::IdentifierExpr {
            name: "cond".into(),
            type_ann: None,
        },
        then_body: BlockIR {
            statements: vec![StatementIR::LetStmtIR {
                name: "x".into(),
                value: Some(ExprIR::LiteralExpr {
                    value: serde_json::json!(1),
                }),
                r#type: None,
            }],
        },
        else_ifs: vec![],
        else_body: Some(BlockIR {
            statements: vec![StatementIR::LetStmtIR {
                name: "x".into(),
                value: Some(ExprIR::LiteralExpr {
                    value: serde_json::json!(2),
                }),
                r#type: None,
            }],
        }),
    };
    generate_statement(&mut w, &stmt);
    let out = w.into_string();
    assert!(out.contains("if (cond) {"));
    assert!(out.contains("var x = 1;"));
    assert!(out.contains("} else {"));
    assert!(out.contains("var x = 2;"));
    assert!(out.contains("}"));
}

// ── Full pipeline through trait ────────────────────────

#[test]
fn test_full_pipeline_through_trait() {
    let codegen = DartCodegen;
    let ir = IRProgram {
        version: "0.1".into(),
        modules: FxHashMap::from_iter([(
            "main".to_string(),
            ModuleIR {
                name: "main".to_string(),
                components: FxHashMap::from_iter([(
                    "App".into(),
                    ComponentIR {
                        name: "App".into(),
                        generic_params: vec![],
                        props: FxHashMap::default(),
                        prop_defaults: FxHashMap::default(),
                        states: vec![],
                        computeds: vec![],
                        effects: vec![],
                        lifecycles: vec![],
                        fns: vec![FunctionIR {
                            name: "run".into(),
                            visibility: Visibility::Public,
                            params: vec![],
                            return_type: None,
                            body: BlockIR { statements: vec![] },
                            is_async: false,
                        
                        source_span: None,}],
                        store_refs: vec![],
                        refs: vec![],
                        event_names: vec![],
                        view: None,
                        styles: vec![],
                        motions: vec![],
                        if_mode_default: None,
                        hero_defs: vec![],
                        timelines: vec![],
                        ..Default::default()
                    },
                )]),
                stores: FxHashMap::default(),
                ..Default::default()
            },
        )]),
        source_map: FxHashMap::default(),
        stdlib_imports: Default::default(),
    };
    let sources = FxHashMap::from_iter([("main.omni".into(), "".into())]);
    let output = codegen.generate(&ir, &sources);
    assert!(output.errors.is_empty());
    assert_eq!(output.files.len(), 1);
    assert_eq!(output.files[0].path, "main.dart");
    insta::with_settings!({snapshot_path => "snapshots"}, {
        insta::assert_snapshot!("test_full_pipeline_through_trait", &output.files[0].content);
    });
}

#[test]
fn test_full_pipeline_with_store() {
    let codegen = DartCodegen;
    let ir = IRProgram {
        version: "0.1".into(),
        modules: FxHashMap::from_iter([(
            "main".to_string(),
            ModuleIR {
                name: "main".to_string(),
                components: FxHashMap::default(),
                stores: FxHashMap::from_iter([(
                    "AppStore".into(),
                    StoreIR {
                        name: "AppStore".into(),
                        signals: FxHashMap::from_iter([(
                            "count".into(),
                            TypeIR {
                                kind: "i32".into(),
                                type_args: vec![],
                                is_nullable: false,
                            },
                        )]),
                        signal_defaults: FxHashMap::from_iter([(
                            "count".into(),
                            serde_json::json!(0),
                        )]),
                        getters: FxHashMap::default(),
                        actions: vec![],
                        ..Default::default()
                    },
                )]),
                ..Default::default()
            },
        )]),
        source_map: FxHashMap::default(),
        stdlib_imports: Default::default(),
    };
    let sources = FxHashMap::from_iter([("main.omni".into(), "".into())]);
    let output = codegen.generate(&ir, &sources);
    assert!(output.errors.is_empty());
    assert_eq!(output.files.len(), 1);
    insta::with_settings!({snapshot_path => "snapshots"}, {
        insta::assert_snapshot!("test_full_pipeline_with_store", &output.files[0].content);
    });
}

// ── Logic construct tests (structs, enums, type aliases, standalone functions) ──

#[test]
fn test_dart_generate_struct() {
    let mut w = CodeWriter::new();
    let struct_ir = StructIR {
        name: "Point".into(),
        visibility: Visibility::Public,
        fields: vec![
            FieldIR {
                name: "x".into(),
                r#type: TypeIR {
                    kind: "i32".into(),
                    type_args: vec![],
                    is_nullable: false,
                },
                visibility: Visibility::Public,
            
            source_span: None,},
            FieldIR {
                name: "y".into(),
                r#type: TypeIR {
                    kind: "i32".into(),
                    type_args: vec![],
                    is_nullable: false,
                },
                visibility: Visibility::Public,
            
            source_span: None,},
        ],
    
    source_span: None,};
    visitor().generate_struct(&mut w, &struct_ir);
    let out = w.into_string();
    assert!(out.contains("class Point {"));
    assert!(out.contains("final int x;"));
    assert!(out.contains("final int y;"));
    assert!(out.contains("Point({ required this.x, required this.y });"));
}

#[test]
fn test_dart_generate_empty_struct() {
    let mut w = CodeWriter::new();
    let struct_ir = StructIR {
        name: "Empty".into(),
        visibility: Visibility::Public,
        fields: vec![],
    
    source_span: None,};
    visitor().generate_struct(&mut w, &struct_ir);
    let out = w.into_string();
    assert!(out.contains("class Empty {"));
    assert!(out.contains("Empty();"));
    assert!(out.contains("}"));
}

#[test]
fn test_dart_generate_enum() {
    let mut w = CodeWriter::new();
    let enum_ir = EnumIR {
        name: "Color".into(),
        visibility: Visibility::Public,
        variants: vec![
            VariantIR {
                name: "Red".into(),
                payload: None,
            
            source_span: None,},
            VariantIR {
                name: "Green".into(),
                payload: None,
            
            source_span: None,},
            VariantIR {
                name: "Blue".into(),
                payload: None,
            
            source_span: None,},
        ],
        source_span: None,
    };
    visitor().generate_enum(&mut w, &enum_ir);
    let out = w.into_string();
    assert!(out.contains("enum Color {"));
    assert!(out.contains("Red, Green, Blue"));
    assert!(out.contains("}"));
}

#[test]
fn test_dart_generate_empty_enum() {
    let mut w = CodeWriter::new();
    let enum_ir = EnumIR {
        name: "Empty".into(),
        visibility: Visibility::Public,
        variants: vec![],
    
    source_span: None,};
    visitor().generate_enum(&mut w, &enum_ir);
    let out = w.into_string();
    assert!(out.contains("enum Empty {"));
}

#[test]
fn test_dart_generate_type_alias() {
    let mut w = CodeWriter::new();
    let alias = TypeAliasIR {
        name: "UserId".into(),
        visibility: Visibility::Public,
        target: TypeIR {
            kind: "i32".into(),
            type_args: vec![],
            is_nullable: false,
        },
    
    source_span: None,};
    visitor().generate_type_alias(&mut w, &alias);
    assert_eq!(w.into_string(), "typedef UserId = int;\n");
}

#[test]
fn test_dart_generate_standalone_function() {
    let mut w = CodeWriter::new();
    let func = FunctionIR {
        name: "main".into(),
        visibility: Visibility::Public,
        params: vec![],
        return_type: None,
        body: BlockIR { statements: vec![] },
        is_async: false,
    
    source_span: None,};
    visitor().generate_function(&mut w, &func);
    let out = w.into_string();
    assert!(out.contains("main() {"));
}

#[test]
fn test_dart_generate_standalone_function_with_return_type() {
    let mut w = CodeWriter::new();
    let func = FunctionIR {
        name: "add".into(),
        visibility: Visibility::Public,
        params: vec![ParamIR {
            name: "a".into(),
            r#type: Some(TypeIR {
                kind: "i32".into(),
                type_args: vec![],
                is_nullable: false,
            }),
        }],
        return_type: Some(TypeIR {
            kind: "i32".into(),
            type_args: vec![],
            is_nullable: false,
        }),
        body: BlockIR { statements: vec![] },
        is_async: false,
    
    source_span: None,};
    visitor().generate_function(&mut w, &func);
    let out = w.into_string();
    assert!(out.contains("int add(int a) {"));
}

#[test]
fn test_dart_format_type_ir_dart() {
    let ty = TypeIR {
        kind: "i32".into(),
        type_args: vec![],
        is_nullable: false,
    };
    assert_eq!(format_type_ir_dart(&ty), "int");

    let str_ty = TypeIR {
        kind: "string".into(),
        type_args: vec![],
        is_nullable: false,
    };
    assert_eq!(format_type_ir_dart(&str_ty), "String");

    let vec_ty = TypeIR {
        kind: "Vec".into(),
        type_args: vec![ty],
        is_nullable: false,
    };
    assert_eq!(format_type_ir_dart(&vec_ty), "List<int>");
}

#[test]
fn test_generate_async_function_dart() {
    let mut w = CodeWriter::new();
    let func = FunctionIR {
        name: "fetchData".into(),
        visibility: Visibility::Private,
        params: vec![],
        return_type: Some(TypeIR {
            kind: "String".into(),
            type_args: vec![],
            is_nullable: false,
        }),
        body: BlockIR { statements: vec![] },
        is_async: true,
    
    source_span: None,};
    generate_function(&mut w, &func);
    let out = w.into_string();
    assert!(
        out.contains("Future<String> fetchData() async {"),
        "Dart async function should wrap return in Future. Output:\n{}",
        out
    );
}

#[test]
fn test_generate_async_function_dart_void() {
    let mut w = CodeWriter::new();
    let func = FunctionIR {
        name: "doAsync".into(),
        visibility: Visibility::Private,
        params: vec![],
        return_type: None,
        body: BlockIR { statements: vec![] },
        is_async: true,
    
    source_span: None,};
    generate_function(&mut w, &func);
    let out = w.into_string();
    assert!(
        out.contains("Future<void> doAsync() async {"),
        "Dart async void function should have Future<void>. Output:\n{}",
        out
    );
}

#[test]
fn test_format_await_expr_dart() {
    let expr = ExprIR::AwaitExpr {
        operand: Box::new(ExprIR::CallExpr {
            name: "fetchData".into(),
            args: vec![],
        }),
    };
    let out = format_expr_ir(&expr);
    assert!(
        out.starts_with("await "),
        "Dart await should be prefix 'await'. Output: {}",
        out
    );
    assert!(
        out.contains("fetchData()"),
        "Dart await should contain the awaited call. Output: {}",
        out
    );
}

#[test]
fn test_format_variant_literal_dart() {
    let expr = ExprIR::VariantLiteral {
        union_type: "HttpResult".into(),
        variant: "ok".into(),
        fields: vec![(
            "value".into(),
            ExprIR::LiteralExpr {
                value: serde_json::json!(42),
            },
        )],
    };
    let out = format_expr_ir(&expr);
    assert!(
        out.contains("HttpResult.ok"),
        "Dart variant should use dot syntax. Output: {}",
        out
    );
    assert!(
        out.contains("value: 42"),
        "Dart variant should contain fields. Output: {}",
        out
    );
}
