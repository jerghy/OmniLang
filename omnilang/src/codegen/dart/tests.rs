//! Unit tests for dart codegen.

use super::expr::*;
use super::*;
use std::collections::HashMap;

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
        needs_clone: false,
        needs_partial_eq: false,
    };
    let string_arg = TypeIR {
        kind: "string".into(),
        type_args: vec![],
        is_nullable: false,
        needs_clone: false,
        needs_partial_eq: false,
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
        needs_clone: false,
        needs_partial_eq: false,
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
    assert_eq!(format_json_value(&serde_json::Value::Null), "null");
    assert_eq!(format_json_value(&serde_json::Value::Bool(true)), "true");
    assert_eq!(format_json_value(&serde_json::Value::Bool(false)), "false");
    assert_eq!(format_json_value(&serde_json::json!(42)), "42");
    assert_eq!(format_json_value(&serde_json::json!("hello")), "'hello'");
    assert_eq!(
        format_json_value(&serde_json::json!("he'llo")),
        "'he\\'llo'"
    );
}

// ── binary_op_to_dart tests ────────────────────────────

#[test]
fn test_binary_op_to_dart() {
    assert_eq!(binary_op_to_dart("add"), "+");
    assert_eq!(binary_op_to_dart("eq"), "==");
    assert_eq!(binary_op_to_dart("and"), "&&");
    assert_eq!(binary_op_to_dart("or"), "||");
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
    };
    assert_eq!(format_expr_ir(&expr), "count");
}

#[test]
fn test_format_expr_ir_binary() {
    let expr = ExprIR::BinaryOpExpr {
        op: "add".into(),
        left: Box::new(ExprIR::IdentifierExpr { name: "a".into() }),
        right: Box::new(ExprIR::IdentifierExpr { name: "b".into() }),
    };
    assert_eq!(format_expr_ir(&expr), "a + b");
}

#[test]
fn test_format_expr_ir_ternary() {
    let expr = ExprIR::TernaryIR {
        cond: Box::new(ExprIR::IdentifierExpr { name: "x".into() }),
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
        object: Box::new(ExprIR::IdentifierExpr { name: "obj".into() }),
        field: "prop".into(),
    };
    assert_eq!(format_expr_ir(&expr), "obj.prop");
}

#[test]
fn test_format_expr_ir_unary_not() {
    let expr = ExprIR::UnaryOpExpr {
        op: "not".into(),
        operand: Box::new(ExprIR::IdentifierExpr {
            name: "flag".into(),
        }),
    };
    assert_eq!(format_expr_ir(&expr), "!flag");
}

#[test]
fn test_format_expr_ir_unary_neg() {
    let expr = ExprIR::UnaryOpExpr {
        op: "neg".into(),
        operand: Box::new(ExprIR::IdentifierExpr { name: "x".into() }),
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
        visibility: "public".into(),
        params: vec![],
        return_type: None,
        body: BlockIR {
            statements: vec![],
            execution_mode: "client-only".into(),
        },
        is_async: false,
    };
    generate_function(&mut w, &func);
    let out = w.into_string();
    assert!(out.contains("doSomething() {"));
}

#[test]
fn test_generate_function_with_return_type() {
    let mut w = CodeWriter::new();
    let func = FunctionIR {
        name: "getValue".into(),
        visibility: "public".into(),
        params: vec![],
        return_type: Some(TypeIR {
            kind: "i32".into(),
            type_args: vec![],
            is_nullable: false,
            needs_clone: false,
            needs_partial_eq: false,
        }),
        body: BlockIR {
            statements: vec![],
            execution_mode: "client-only".into(),
        },
        is_async: false,
    };
    generate_function(&mut w, &func);
    let out = w.into_string();
    assert!(out.contains("int getValue() {"));
}

#[test]
fn test_generate_function_with_void_return() {
    let mut w = CodeWriter::new();
    let func = FunctionIR {
        name: "doThing".into(),
        visibility: "public".into(),
        params: vec![],
        return_type: Some(TypeIR {
            kind: "void".into(),
            type_args: vec![],
            is_nullable: false,
            needs_clone: false,
            needs_partial_eq: false,
        }),
        body: BlockIR {
            statements: vec![],
            execution_mode: "client-only".into(),
        },
        is_async: false,
    };
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
        visibility: "public".into(),
        params: vec![ParamIR {
            name: "name".into(),
            r#type: Some(TypeIR {
                kind: "string".into(),
                type_args: vec![],
                is_nullable: false,
                needs_clone: false,
                needs_partial_eq: false,
            }),
        }],
        return_type: Some(TypeIR {
            kind: "String".into(),
            type_args: vec![],
            is_nullable: false,
            needs_clone: false,
            needs_partial_eq: false,
        }),
        body: BlockIR {
            statements: vec![],
            execution_mode: "client-only".into(),
        },
        is_async: false,
    };
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
        motions: vec![],
        if_mode_default: None,
        hero_defs: vec![],
        timelines: vec![],
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
        props: HashMap::new(),
        prop_defaults: HashMap::new(),
        states: vec![],
        computeds: vec![],
        effects: vec![],
        lifecycles: vec![],
        fns: vec![FunctionIR {
            name: "greet".into(),
            visibility: "public".into(),
            params: vec![],
            return_type: None,
            body: BlockIR {
                statements: vec![],
                execution_mode: "client-only".into(),
            },
            is_async: false,
        }],
        store_refs: vec![],
        refs: vec![],
        event_names: vec![],
        view: None,
        styles: vec![],
        motions: vec![],
        if_mode_default: None,
        hero_defs: vec![],
        timelines: vec![],
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
        props: HashMap::new(),
        prop_defaults: HashMap::new(),
        states: vec![StateIntent {
            name: "count".into(),
            r#type: TypeIR {
                kind: "i32".into(),
                type_args: vec![],
                is_nullable: false,
                needs_clone: false,
                needs_partial_eq: false,
            },
            initial_value: ExprIR::LiteralExpr {
                value: serde_json::json!(0),
            },
            mutability: "mutable".into(),
            update_semantics: "setter-call".into(),
            accessor_style: "direct".into(),
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
        signals: HashMap::from([
            (
                "count".into(),
                TypeIR {
                    kind: "i32".into(),
                    type_args: vec![],
                    is_nullable: false,
                    needs_clone: false,
                    needs_partial_eq: false,
                },
            ),
            (
                "name".into(),
                TypeIR {
                    kind: "string".into(),
                    type_args: vec![],
                    is_nullable: false,
                    needs_clone: false,
                    needs_partial_eq: false,
                },
            ),
        ]),
        signal_defaults: HashMap::from([
            ("count".into(), serde_json::json!(0)),
            ("name".into(), serde_json::json!("world")),
        ]),
        getters: HashMap::new(),
        actions: vec![],
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
        signals: HashMap::new(),
        signal_defaults: HashMap::new(),
        getters: HashMap::new(),
        actions: vec![],
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
        }),
    };
    generate_statement(&mut w, &stmt);
    assert_eq!(w.into_string(), "return result;\n");
}

#[test]
fn test_generate_statement_mutation_increment() {
    let mut w = CodeWriter::new();
    let stmt = StatementIR::MutationStmt {
        intent: Some(MutationIntent {
            target: SignalRefIR {
                owner: None,
                name: "count".into(),
            },
            operation: "increment".into(),
            amount: Some(ExprIR::LiteralExpr {
                value: serde_json::json!(1),
            }),
            item: None,
        }),
    };
    generate_statement(&mut w, &stmt);
    assert_eq!(w.into_string(), "this.count += 1;\n");
}

#[test]
fn test_generate_statement_mutation_assign() {
    let mut w = CodeWriter::new();
    let stmt = StatementIR::MutationStmt {
        intent: Some(MutationIntent {
            target: SignalRefIR {
                owner: None,
                name: "name".into(),
            },
            operation: "direct_assign".into(),
            amount: Some(ExprIR::LiteralExpr {
                value: serde_json::json!("hello"),
            }),
            item: None,
        }),
    };
    generate_statement(&mut w, &stmt);
    assert_eq!(w.into_string(), "this.name = 'hello';\n");
}

#[test]
fn test_generate_statement_mutation_decrement() {
    let mut w = CodeWriter::new();
    let stmt = StatementIR::MutationStmt {
        intent: Some(MutationIntent {
            target: SignalRefIR {
                owner: None,
                name: "count".into(),
            },
            operation: "decrement".into(),
            amount: Some(ExprIR::LiteralExpr {
                value: serde_json::json!(1),
            }),
            item: None,
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
        },
        then_body: BlockIR {
            statements: vec![StatementIR::LetStmtIR {
                name: "x".into(),
                value: Some(ExprIR::LiteralExpr {
                    value: serde_json::json!(1),
                }),
                r#type: None,
            }],
            execution_mode: "client-only".into(),
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
            execution_mode: "client-only".into(),
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
        modules: HashMap::from([(
            "main".to_string(),
            ModuleIR {
                name: "main".to_string(),
                components: HashMap::from([(
                    "App".into(),
                    ComponentIR {
                        name: "App".into(),
                        generic_params: vec![],
                        props: HashMap::new(),
                        prop_defaults: HashMap::new(),
                        states: vec![],
                        computeds: vec![],
                        effects: vec![],
                        lifecycles: vec![],
                        fns: vec![FunctionIR {
                            name: "run".into(),
                            visibility: "public".into(),
                            params: vec![],
                            return_type: None,
                            body: BlockIR {
                                statements: vec![],
                                execution_mode: "client-only".into(),
                            },
                            is_async: false,
                        }],
                        store_refs: vec![],
                        refs: vec![],
                        event_names: vec![],
                        view: None,
                        styles: vec![],
                        motions: vec![],
                        if_mode_default: None,
                        hero_defs: vec![],
                        timelines: vec![],
                    },
                )]),
                stores: HashMap::new(),
                ..Default::default()
            },
        )]),
        source_map: HashMap::new(),
        stdlib_imports: Vec::new(),
    };
    let sources = HashMap::from([("main.omni".into(), "".into())]);
    let output = codegen.generate(&ir, &sources);
    assert!(output.errors.is_empty());
    assert_eq!(output.files.len(), 1);
    assert_eq!(output.files[0].path, "main.dart");
    let content = &output.files[0].content;
    assert!(content.contains("class App {"));
    assert!(content.contains("run() {"));
}

#[test]
fn test_full_pipeline_with_store() {
    let codegen = DartCodegen;
    let ir = IRProgram {
        version: "0.1".into(),
        modules: HashMap::from([(
            "main".to_string(),
            ModuleIR {
                name: "main".to_string(),
                components: HashMap::new(),
                stores: HashMap::from([(
                    "AppStore".into(),
                    StoreIR {
                        name: "AppStore".into(),
                        signals: HashMap::from([(
                            "count".into(),
                            TypeIR {
                                kind: "i32".into(),
                                type_args: vec![],
                                is_nullable: false,
                                needs_clone: false,
                                needs_partial_eq: false,
                            },
                        )]),
                        signal_defaults: HashMap::from([("count".into(), serde_json::json!(0))]),
                        getters: HashMap::new(),
                        actions: vec![],
                    },
                )]),
                ..Default::default()
            },
        )]),
        source_map: HashMap::new(),
        stdlib_imports: Vec::new(),
    };
    let sources = HashMap::from([("main.omni".into(), "".into())]);
    let output = codegen.generate(&ir, &sources);
    assert!(output.errors.is_empty());
    let content = &output.files[0].content;
    assert!(content.contains("class AppStore {"));
    assert!(content.contains("int count = 0;"));
}

// ── Logic construct tests (structs, enums, type aliases, standalone functions) ──

#[test]
fn test_dart_generate_struct() {
    let mut w = CodeWriter::new();
    let struct_ir = StructIR {
        name: "Point".into(),
        visibility: "public".to_string(),
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
    };
    generate_struct(&mut w, &struct_ir);
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
        visibility: "public".to_string(),
        fields: vec![],
    };
    generate_struct(&mut w, &struct_ir);
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
        visibility: "public".to_string(),
        variants: vec![
            VariantIR {
                name: "Red".into(),
                payload: None,
            },
            VariantIR {
                name: "Green".into(),
                payload: None,
            },
            VariantIR {
                name: "Blue".into(),
                payload: None,
            },
        ],
    };
    generate_enum(&mut w, &enum_ir);
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
        visibility: "public".to_string(),
        variants: vec![],
    };
    generate_enum(&mut w, &enum_ir);
    let out = w.into_string();
    assert!(out.contains("enum Empty {"));
}

#[test]
fn test_dart_generate_type_alias() {
    let mut w = CodeWriter::new();
    let alias = TypeAliasIR {
        name: "UserId".into(),
        visibility: "public".to_string(),
        target: TypeIR {
            kind: "i32".into(),
            type_args: vec![],
            is_nullable: false,
            needs_clone: false,
            needs_partial_eq: false,
        },
    };
    generate_type_alias(&mut w, &alias);
    assert_eq!(w.into_string(), "typedef UserId = int;\n");
}

#[test]
fn test_dart_generate_standalone_function() {
    let mut w = CodeWriter::new();
    let func = FunctionIR {
        name: "main".into(),
        visibility: "public".to_string(),
        params: vec![],
        return_type: None,
        body: BlockIR {
            statements: vec![],
            execution_mode: "client-only".into(),
        },
        is_async: false,
    };
    generate_standalone_function(&mut w, &func);
    let out = w.into_string();
    assert!(out.contains("main() {"));
}

#[test]
fn test_dart_generate_standalone_function_with_return_type() {
    let mut w = CodeWriter::new();
    let func = FunctionIR {
        name: "add".into(),
        visibility: "public".to_string(),
        params: vec![ParamIR {
            name: "a".into(),
            r#type: Some(TypeIR {
                kind: "i32".into(),
                type_args: vec![],
                is_nullable: false,
                needs_clone: false,
                needs_partial_eq: false,
            }),
        }],
        return_type: Some(TypeIR {
            kind: "i32".into(),
            type_args: vec![],
            is_nullable: false,
            needs_clone: false,
            needs_partial_eq: false,
        }),
        body: BlockIR {
            statements: vec![],
            execution_mode: "client-only".into(),
        },
        is_async: false,
    };
    generate_standalone_function(&mut w, &func);
    let out = w.into_string();
    assert!(out.contains("int add(int a) {"));
}

#[test]
fn test_dart_format_type_ir_dart() {
    let ty = TypeIR {
        kind: "i32".into(),
        type_args: vec![],
        is_nullable: false,
        needs_clone: false,
        needs_partial_eq: false,
    };
    assert_eq!(format_type_ir_dart(&ty), "int");

    let str_ty = TypeIR {
        kind: "string".into(),
        type_args: vec![],
        is_nullable: false,
        needs_clone: false,
        needs_partial_eq: false,
    };
    assert_eq!(format_type_ir_dart(&str_ty), "String");

    let vec_ty = TypeIR {
        kind: "Vec".into(),
        type_args: vec![ty],
        is_nullable: false,
        needs_clone: false,
        needs_partial_eq: false,
    };
    assert_eq!(format_type_ir_dart(&vec_ty), "List<int>");
}

#[test]
fn test_generate_async_function_dart() {
    let mut w = CodeWriter::new();
    let func = FunctionIR {
        name: "fetchData".into(),
        visibility: "private".to_string(),
        params: vec![],
        return_type: Some(TypeIR {
            kind: "String".into(),
            type_args: vec![],
            is_nullable: false,
            needs_clone: false,
            needs_partial_eq: false,
        }),
        body: BlockIR {
            statements: vec![],
            execution_mode: "client-only".into(),
        },
        is_async: true,
    };
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
        visibility: "private".to_string(),
        params: vec![],
        return_type: None,
        body: BlockIR {
            statements: vec![],
            execution_mode: "client-only".into(),
        },
        is_async: true,
    };
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
