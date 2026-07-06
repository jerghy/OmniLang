//! Unit tests for javascript codegen.

#[allow(unused_imports)]
use super::*;
use crate::codegen::CodeWriter;
use crate::ir::*;
use std::collections::HashMap;

// ── js_type tests ──────────────────────────────────────

#[test]
fn test_js_type_no_ts() {
    assert_eq!(js_type("int", &[], false), "");
    assert_eq!(js_type("string", &[], false), "");
    assert_eq!(js_type("bool", &[], false), "");
    assert_eq!(js_type("void", &[], false), "");
}

#[test]
fn test_ts_type_primitives() {
    assert_eq!(js_type("int", &[], true), "number");
    assert_eq!(js_type("i32", &[], true), "number");
    assert_eq!(js_type("f64", &[], true), "number");
    assert_eq!(js_type("bool", &[], true), "boolean");
    assert_eq!(js_type("string", &[], true), "string");
    assert_eq!(js_type("void", &[], true), "void");
}

#[test]
fn test_ts_type_generics() {
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
        js_type("Vec", std::slice::from_ref(&i32_arg), true),
        "number[]"
    );
    assert_eq!(
        js_type("HashMap", &[string_arg.clone(), i32_arg.clone()], true),
        "Record<string, number>"
    );
}

#[test]
fn test_ts_type_option() {
    let i32_arg = TypeIR {
        kind: "i32".into(),
        type_args: vec![],
        is_nullable: false,
        needs_clone: false,
        needs_partial_eq: false,
    };
    assert_eq!(js_type("Option", &[i32_arg], true), "number | null");
}

#[test]
fn test_ts_type_user_defined() {
    assert_eq!(js_type("MyType", &[], true), "MyType");
    assert_eq!(js_type("unknown", &[], true), "any");
}

// ── format_json_value tests ────────────────────────────

#[test]
fn test_format_json_value() {
    assert_eq!(format_json_value(&serde_json::Value::Null), "null");
    assert_eq!(format_json_value(&serde_json::Value::Bool(true)), "true");
    assert_eq!(format_json_value(&serde_json::Value::Bool(false)), "false");
    assert_eq!(format_json_value(&serde_json::json!(42)), "42");
    assert_eq!(format_json_value(&serde_json::json!("hello")), "\"hello\"");
    assert_eq!(
        format_json_value(&serde_json::json!("he\"llo")),
        "\"he\\\"llo\""
    );
}

// ── binary_op_to_js tests ──────────────────────────────

#[test]
fn test_binary_ops() {
    assert_eq!(binary_op_to_js("add"), "+");
    assert_eq!(binary_op_to_js("eq"), "===");
    assert_eq!(binary_op_to_js("and"), "&&");
    assert_eq!(binary_op_to_js("or"), "||");
}

// ── format_expr_ir tests ─────────────────────────────

#[test]
fn test_format_expr_literal() {
    let expr = ExprIR::LiteralExpr {
        value: serde_json::json!(42),
    };
    assert_eq!(format_expr_ir(&expr, false), "42");
}

#[test]
fn test_format_expr_identifier() {
    let expr = ExprIR::IdentifierExpr {
        name: "count".into(),
    };
    assert_eq!(format_expr_ir(&expr, false), "count");
}

#[test]
fn test_format_expr_binary() {
    let expr = ExprIR::BinaryOpExpr {
        op: "add".into(),
        left: Box::new(ExprIR::IdentifierExpr { name: "a".into() }),
        right: Box::new(ExprIR::IdentifierExpr { name: "b".into() }),
    };
    assert_eq!(format_expr_ir(&expr, false), "a + b");
}

#[test]
fn test_format_expr_ternary() {
    let expr = ExprIR::TernaryIR {
        cond: Box::new(ExprIR::IdentifierExpr { name: "x".into() }),
        then_expr: Box::new(ExprIR::LiteralExpr {
            value: serde_json::json!(1),
        }),
        else_expr: Box::new(ExprIR::LiteralExpr {
            value: serde_json::json!(0),
        }),
    };
    assert_eq!(format_expr_ir(&expr, false), "x ? 1 : 0");
}

#[test]
fn test_format_expr_field_access() {
    let expr = ExprIR::FieldAccessExpr {
        object: Box::new(ExprIR::IdentifierExpr { name: "obj".into() }),
        field: "prop".into(),
    };
    assert_eq!(format_expr_ir(&expr, false), "obj.prop");
}

#[test]
fn test_format_expr_unary_not() {
    let expr = ExprIR::UnaryOpExpr {
        op: "not".into(),
        operand: Box::new(ExprIR::IdentifierExpr {
            name: "flag".into(),
        }),
    };
    assert_eq!(format_expr_ir(&expr, false), "!flag");
}

#[test]
fn test_format_expr_unary_neg() {
    let expr = ExprIR::UnaryOpExpr {
        op: "neg".into(),
        operand: Box::new(ExprIR::IdentifierExpr { name: "x".into() }),
    };
    assert_eq!(format_expr_ir(&expr, false), "-x");
}

// ── generate_function tests ───────────────────────────

#[test]
fn test_generate_function_empty_js() {
    let mut w = CodeWriter::new();
    let func = FunctionIR {
        name: "doThing".into(),
        visibility: "private".to_string(),
        params: vec![],
        return_type: None,
        body: BlockIR {
            statements: vec![],
            execution_mode: "client-only".into(),
        },
        is_async: false,
    };
    generate_function(&mut w, &func, false);
    let out = w.into_string();
    assert!(out.contains("doThing() {"));
}

#[test]
fn test_generate_function_empty_ts() {
    let mut w = CodeWriter::new();
    let func = FunctionIR {
        name: "doThing".into(),
        visibility: "private".to_string(),
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
    generate_function(&mut w, &func, true);
    let out = w.into_string();
    assert!(out.contains("doThing(): void {"));
}

#[test]
fn test_generate_function_with_params_ts() {
    let mut w = CodeWriter::new();
    let func = FunctionIR {
        name: "greet".into(),
        visibility: "private".to_string(),
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
            kind: "string".into(),
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
    generate_function(&mut w, &func, true);
    let out = w.into_string();
    assert!(out.contains("greet(name: string): string {"));
}

#[test]
fn test_generate_function_with_params_js() {
    let mut w = CodeWriter::new();
    let func = FunctionIR {
        name: "greet".into(),
        visibility: "private".to_string(),
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
        return_type: None,
        body: BlockIR {
            statements: vec![],
            execution_mode: "client-only".into(),
        },
        is_async: false,
    };
    generate_function(&mut w, &func, false);
    let out = w.into_string();
    assert!(out.contains("greet(name) {"));
    assert!(!out.contains(": string"));
}

#[test]
fn test_generate_async_function_js() {
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
    generate_function(&mut w, &func, false);
    let out = w.into_string();
    assert!(
        out.contains("async fetchData()"),
        "JS async function should have 'async' prefix. Output:\n{}",
        out
    );
}

#[test]
fn test_generate_async_function_ts() {
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
    generate_function(&mut w, &func, true);
    let out = w.into_string();
    assert!(
        out.contains("async fetchData()"),
        "TS async function should have 'async' prefix. Output:\n{}",
        out
    );
}

#[test]
fn test_format_await_expr_js() {
    let expr = ExprIR::AwaitExpr {
        operand: Box::new(ExprIR::CallExpr {
            name: "fetch".into(),
            args: vec![ExprIR::LiteralExpr {
                value: serde_json::json!("/api/data"),
            }],
        }),
    };
    let out = format_expr_ir(&expr, false);
    assert!(
        out.starts_with("await "),
        "JS await should be prefix 'await'. Output: {}",
        out
    );
    assert!(
        out.contains("fetch"),
        "await expr should contain the awaited call. Output: {}",
        out
    );
}

#[test]
fn test_js_empty_component() {
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
        ..Default::default()
    };
    generate_component(&mut w, "App", &comp, false);
    let out = w.into_string();
    assert!(out.contains("class App {"));
    assert!(out.contains("}"));
}

#[test]
fn test_ts_empty_component() {
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
        ..Default::default()
    };
    generate_component(&mut w, "App", &comp, true);
    let out = w.into_string();
    assert!(out.contains("class App {"));
    assert!(out.contains("}"));
}

#[test]
fn test_js_component_with_function() {
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
            visibility: "private".to_string(),
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
        ..Default::default()
    };
    generate_component(&mut w, "App", &comp, false);
    let out = w.into_string();
    assert!(out.contains("class App {"));
    assert!(out.contains("greet() {"));
}

#[test]
fn test_js_component_with_state() {
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
        ..Default::default()
    };
    generate_component(&mut w, "Counter", &comp, false);
    let out = w.into_string();
    assert!(out.contains("class Counter {"));
    assert!(out.contains("constructor() {"));
    assert!(out.contains("this.count = 0;"));
}

#[test]
fn test_ts_component_with_state() {
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
        ..Default::default()
    };
    generate_component(&mut w, "Counter", &comp, true);
    let out = w.into_string();
    assert!(out.contains("class Counter {"));
    assert!(out.contains("count: number;"));
    assert!(out.contains("constructor() {"));
    assert!(out.contains("this.count = 0;"));
}

// ── generate_store tests ──────────────────────────────

#[test]
fn test_js_generate_store() {
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
        ..Default::default()
    };
    generate_store(&mut w, "AppStore", &store, false);
    let out = w.into_string();
    assert!(out.contains("class AppStore {"));
    assert!(out.contains("constructor() {"));
    assert!(out.contains("this.count = 0;"));
    assert!(out.contains("this.name = \"world\";"));
}

#[test]
fn test_ts_generate_store() {
    let mut w = CodeWriter::new();
    let store = StoreIR {
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
        ..Default::default()
    };
    generate_store(&mut w, "AppStore", &store, true);
    let out = w.into_string();
    assert!(out.contains("class AppStore {"));
    assert!(out.contains("count: number;"));
    assert!(out.contains("constructor() {"));
    assert!(out.contains("this.count = 0;"));
}

// ── generate_statement tests ──────────────────────────

#[test]
fn test_js_generate_statement_let() {
    let mut w = CodeWriter::new();
    let stmt = StatementIR::LetStmtIR {
        name: "x".into(),
        value: Some(ExprIR::LiteralExpr {
            value: serde_json::json!(42),
        }),
        r#type: None,
    };
    generate_statement(&mut w, &stmt, false);
    assert_eq!(w.into_string(), "let x = 42;\n");
}

#[test]
fn test_js_generate_statement_return() {
    let mut w = CodeWriter::new();
    let stmt = StatementIR::ReturnStmtIR {
        value: Some(ExprIR::IdentifierExpr {
            name: "result".into(),
        }),
    };
    generate_statement(&mut w, &stmt, false);
    assert_eq!(w.into_string(), "return result;\n");
}

#[test]
fn test_js_generate_statement_mutation() {
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
    generate_statement(&mut w, &stmt, false);
    assert_eq!(w.into_string(), "this.count += 1;\n");
}

#[test]
fn test_js_generate_statement_mutation_assign() {
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
    generate_statement(&mut w, &stmt, false);
    assert_eq!(w.into_string(), "this.name = \"hello\";\n");
}

// ── Full pipeline through trait ────────────────────────

#[test]
fn test_js_full_pipeline_through_trait() {
    let codegen = JavaScriptCodegen::new(false);
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
                            visibility: "private".to_string(),
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
                        ..Default::default()
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
    assert_eq!(output.files[0].path, "main.js");
    insta::with_settings!({snapshot_path => "snapshots"}, {
        insta::assert_snapshot!("test_js_full_pipeline_through_trait", &output.files[0].content);
    });
}

#[test]
fn test_ts_full_pipeline_through_trait() {
    let codegen = JavaScriptCodegen::new(true);
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
                            visibility: "private".to_string(),
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
                        ..Default::default()
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
    assert_eq!(output.files[0].path, "main.ts");
    insta::with_settings!({snapshot_path => "snapshots"}, {
        insta::assert_snapshot!("test_ts_full_pipeline_through_trait", &output.files[0].content);
    });
}

// ── Logic construct tests (standalone functions, structs, enums, type aliases) ──

#[test]
fn test_js_generate_struct() {
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
    generate_struct(&mut w, &struct_ir, false);
    let out = w.into_string();
    assert!(out.contains("class Point {"));
    assert!(out.contains("constructor(x, y) {"));
    assert!(out.contains("this.x = x;"));
    assert!(out.contains("this.y = y;"));
}

#[test]
fn test_ts_generate_struct() {
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
    generate_struct(&mut w, &struct_ir, true);
    let out = w.into_string();
    assert!(out.contains("class Point {"));
    assert!(out.contains("constructor(x: number, y: number) {"));
}

#[test]
fn test_js_generate_empty_struct() {
    let mut w = CodeWriter::new();
    let struct_ir = StructIR {
        name: "Empty".into(),
        visibility: "public".to_string(),
        fields: vec![],
    };
    generate_struct(&mut w, &struct_ir, false);
    let out = w.into_string();
    assert!(out.contains("class Empty {"));
    assert!(out.contains("}"));
}

#[test]
fn test_js_generate_enum() {
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
    generate_enum(&mut w, &enum_ir, false);
    let out = w.into_string();
    assert!(out.contains("const Color = Object.freeze({"));
    assert!(out.contains("Red: 0,"));
    assert!(out.contains("Green: 1,"));
    assert!(out.contains("Blue: 2,"));
    assert!(out.contains("});"));
}

#[test]
fn test_js_generate_empty_enum() {
    let mut w = CodeWriter::new();
    let enum_ir = EnumIR {
        name: "Empty".into(),
        visibility: "public".to_string(),
        variants: vec![],
    };
    generate_enum(&mut w, &enum_ir, false);
    let out = w.into_string();
    assert!(out.contains("const Empty = Object.freeze({"));
    assert!(out.contains("});"));
}

#[test]
fn test_ts_generate_type_alias() {
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
    generate_type_alias(&mut w, &alias, true);
    assert_eq!(w.into_string(), "export type UserId = number;\n");
}

#[test]
fn test_js_skip_type_alias() {
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
    generate_type_alias(&mut w, &alias, false);
    // CodeWriter always appends trailing newline, even when empty
    assert_eq!(w.into_string(), "\n");
}

#[test]
fn test_js_generate_standalone_function() {
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
    generate_standalone_function(&mut w, &func, false);
    let out = w.into_string();
    assert!(out.contains("function main() {"));
    assert!(out.contains(";"));
}

#[test]
fn test_ts_generate_standalone_function() {
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
    generate_standalone_function(&mut w, &func, true);
    let out = w.into_string();
    assert!(out.contains("function add(a: number): number {"));
}

#[test]
fn test_ts_generate_standalone_function_private() {
    let mut w = CodeWriter::new();
    let func = FunctionIR {
        name: "helper".into(),
        visibility: "private".to_string(),
        params: vec![],
        return_type: None,
        body: BlockIR {
            statements: vec![],
            execution_mode: "client-only".into(),
        },
        is_async: false,
    };
    generate_standalone_function(&mut w, &func, false);
    let out = w.into_string();
    assert!(out.contains("function helper() {"));
    assert!(!out.contains("export"));
}

#[test]
fn test_format_type_ir_js() {
    let ty = TypeIR {
        kind: "i32".into(),
        type_args: vec![],
        is_nullable: false,
        needs_clone: false,
        needs_partial_eq: false,
    };
    assert_eq!(format_type_ir_js(&ty, false), "");
    assert_eq!(format_type_ir_js(&ty, true), "number");

    let str_ty = TypeIR {
        kind: "string".into(),
        type_args: vec![],
        is_nullable: false,
        needs_clone: false,
        needs_partial_eq: false,
    };
    assert_eq!(format_type_ir_js(&str_ty, true), "string");

    let vec_ty = TypeIR {
        kind: "Vec".into(),
        type_args: vec![ty.clone()],
        is_nullable: false,
        needs_clone: false,
        needs_partial_eq: false,
    };
    assert_eq!(format_type_ir_js(&vec_ty, true), "number[]");
}

#[test]
fn test_js_generate_struct_private() {
    let mut w = CodeWriter::new();
    let struct_ir = StructIR {
        name: "Internal".into(),
        visibility: "private".to_string(),
        fields: vec![],
    };
    generate_struct(&mut w, &struct_ir, false);
    let out = w.into_string();
    assert!(!out.contains("export"));
}

#[test]
fn test_format_variant_literal_js() {
    let expr = ExprIR::VariantLiteral {
        union_type: "HttpResult".into(),
        variant: "Ok".into(),
        fields: vec![(
            "value".into(),
            ExprIR::LiteralExpr {
                value: serde_json::json!(42),
            },
        )],
    };
    let out = format_expr_ir(&expr, false);
    assert!(
        out.contains("\"_variant\": \"Ok\""),
        "Variant literal should contain _variant marker. Output: {}",
        out
    );
    assert!(
        out.contains("\"value\": 42"),
        "Variant literal should contain fields. Output: {}",
        out
    );
}

// ── Match expression tests ─────────────────────────

#[test]
fn test_match_expr_single_variant() {
    let expr = ExprIR::MatchExpr {
        matched: Box::new(ExprIR::IdentifierExpr {
            name: "result".into(),
        }),
        arms: vec![MatchArmIR {
            pattern: MatchPatternIR {
                kind: MatchPatternKindIR::Variant {
                    name: "Ok".into(),
                    bindings: vec!["value".into()],
                },
            },
            body: BlockIR {
                statements: vec![StatementIR::ReturnStmtIR {
                    value: Some(ExprIR::IdentifierExpr {
                        name: "value".into(),
                    }),
                }],
                execution_mode: "client-only".into(),
            },
        }],
    };
    let out = format_expr_ir(&expr, false);
    assert!(
        out.contains("_v._variant"),
        "Match should check _variant. Output: {}",
        out
    );
    assert!(
        out.contains("\"Ok\""),
        "Match should check for Ok variant. Output: {}",
        out
    );
    assert!(
        out.contains("return value"),
        "Match arm should return value. Output: {}",
        out
    );
}

#[test]
fn test_match_expr_with_wildcard() {
    let expr = ExprIR::MatchExpr {
        matched: Box::new(ExprIR::IdentifierExpr { name: "r".into() }),
        arms: vec![
            MatchArmIR {
                pattern: MatchPatternIR {
                    kind: MatchPatternKindIR::Variant {
                        name: "Ok".into(),
                        bindings: vec!["v".into()],
                    },
                },
                body: BlockIR {
                    statements: vec![StatementIR::ReturnStmtIR {
                        value: Some(ExprIR::IdentifierExpr { name: "v".into() }),
                    }],
                    execution_mode: "client-only".into(),
                },
            },
            MatchArmIR {
                pattern: MatchPatternIR {
                    kind: MatchPatternKindIR::Wildcard,
                },
                body: BlockIR {
                    statements: vec![StatementIR::ReturnStmtIR {
                        value: Some(ExprIR::LiteralExpr {
                            value: serde_json::json!(-1),
                        }),
                    }],
                    execution_mode: "client-only".into(),
                },
            },
        ],
    };
    let out = format_expr_ir(&expr, false);
    assert!(
        out.contains("return -1"),
        "Wildcard arm should return -1. Output: {}",
        out
    );
    // Wildcard arm should not be guarded by an if-check
    let ok_count = out.matches("\"Ok\"").count();
    assert_eq!(
        ok_count, 1,
        "Ok should appear exactly once. Output: {}",
        out
    );
}

#[test]
fn test_match_expr_multiple_variants() {
    let expr = ExprIR::MatchExpr {
        matched: Box::new(ExprIR::IdentifierExpr { name: "r".into() }),
        arms: vec![
            MatchArmIR {
                pattern: MatchPatternIR {
                    kind: MatchPatternKindIR::Variant {
                        name: "Ok".into(),
                        bindings: vec![],
                    },
                },
                body: BlockIR {
                    statements: vec![StatementIR::ReturnStmtIR {
                        value: Some(ExprIR::LiteralExpr {
                            value: serde_json::json!("success"),
                        }),
                    }],
                    execution_mode: "client-only".into(),
                },
            },
            MatchArmIR {
                pattern: MatchPatternIR {
                    kind: MatchPatternKindIR::Variant {
                        name: "Err".into(),
                        bindings: vec![],
                    },
                },
                body: BlockIR {
                    statements: vec![StatementIR::ReturnStmtIR {
                        value: Some(ExprIR::LiteralExpr {
                            value: serde_json::json!("failure"),
                        }),
                    }],
                    execution_mode: "client-only".into(),
                },
            },
        ],
    };
    let out = format_expr_ir(&expr, false);
    assert!(out.contains("\"Ok\""), "Should check Ok. Output: {}", out);
    assert!(out.contains("\"Err\""), "Should check Err. Output: {}", out);
    assert!(
        out.contains("\"success\""),
        "Should return success. Output: {}",
        out
    );
    assert!(
        out.contains("\"failure\""),
        "Should return failure. Output: {}",
        out
    );
}

#[test]
fn test_format_block_as_js_expr_with_return() {
    let block = BlockIR {
        statements: vec![
            StatementIR::LetStmtIR {
                name: "x".into(),
                value: Some(ExprIR::LiteralExpr {
                    value: serde_json::json!(1),
                }),
                r#type: None,
            },
            StatementIR::ReturnStmtIR {
                value: Some(ExprIR::LiteralExpr {
                    value: serde_json::json!(42),
                }),
            },
        ],
        execution_mode: "client-only".into(),
    };
    let out = format_block_as_js_expr(&block, false);
    assert_eq!(out, "42", "Should extract return value. Got: {}", out);
}

#[test]
fn test_format_block_as_js_expr_with_expr_stmt() {
    let block = BlockIR {
        statements: vec![StatementIR::ExprStmtIR {
            expr: ExprIR::IdentifierExpr {
                name: "result".into(),
            },
        }],
        execution_mode: "client-only".into(),
    };
    let out = format_block_as_js_expr(&block, false);
    assert_eq!(out, "result", "Should extract expr stmt. Got: {}", out);
}
