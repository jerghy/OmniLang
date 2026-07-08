#[cfg(test)]
mod tests {
    use super::super::expr::format_type_ir_rust;
    use super::super::*;
    use std::collections::HashMap;

    /// Helper: create a RustVisitor for test use.
    fn visitor() -> super::super::visitor::RustVisitor {
        super::super::visitor::RustVisitor
    }

    // ── rs_type tests ──────────────────────────────────────

    #[test]
    fn test_rs_type_primitives() {
        assert_eq!(rs_type("int", &[]), "i32");
        assert_eq!(rs_type("i32", &[]), "i32");
        assert_eq!(rs_type("i64", &[]), "i64");
        assert_eq!(rs_type("f64", &[]), "f64");
        assert_eq!(rs_type("bool", &[]), "bool");
        assert_eq!(rs_type("string", &[]), "String");
        assert_eq!(rs_type("void", &[]), "()");
    }

    #[test]
    fn test_rs_type_generics() {
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

        assert_eq!(rs_type("Vec", std::slice::from_ref(&i32_arg)), "Vec<i32>");
        assert_eq!(
            rs_type("HashMap", &[string_arg.clone(), i32_arg.clone()]),
            "HashMap<String, i32>"
        );
    }

    #[test]
    fn test_rs_type_option() {
        let i32_arg = TypeIR {
            kind: "i32".into(),
            type_args: vec![],
            is_nullable: false,
        };
        assert_eq!(rs_type("Option", &[i32_arg]), "Option<i32>");
    }

    #[test]
    fn test_rs_type_user_defined() {
        assert_eq!(rs_type("MyType", &[]), "MyType");
        assert_eq!(rs_type("unknown", &[]), "()");
    }

    // ── format_json_value tests ────────────────────────────

    #[test]
    fn test_format_json_value() {
        assert_eq!(
            format_json_value(&serde_json::Value::Null, &JSON_RUST),
            "None"
        );
        assert_eq!(
            format_json_value(&serde_json::Value::Bool(true), &JSON_RUST),
            "true"
        );
        assert_eq!(
            format_json_value(&serde_json::Value::Bool(false), &JSON_RUST),
            "false"
        );
        assert_eq!(format_json_value(&serde_json::json!(42), &JSON_RUST), "42");
        assert_eq!(
            format_json_value(&serde_json::json!("hello"), &JSON_RUST),
            "\"hello\".to_string()"
        );
        assert_eq!(
            format_json_value(&serde_json::json!("he\"llo"), &JSON_RUST),
            "\"he\\\"llo\".to_string()"
        );
    }

    // ── binary_op_to_rust tests ────────────────────────────

    #[test]
    fn test_binary_op_to_rust() {
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
            })),
            right: Box::new(ExprIR::IdentifierExpr {
                name: "b".into(),
                type_ann: None,
            })),
        };
        assert_eq!(format_expr_ir(&expr), "a + b");
    }

    #[test]
    fn test_format_expr_ir_ternary() {
        let expr = ExprIR::TernaryIR {
            cond: Box::new(ExprIR::IdentifierExpr {
                name: "x".into(),
                type_ann: None,
            })),
            then_expr: Box::new(ExprIR::LiteralExpr {
                value: serde_json::json!(1),
            })),
            else_expr: Box::new(ExprIR::LiteralExpr {
                value: serde_json::json!(0),
            })),
        };
        assert_eq!(format_expr_ir(&expr), "if x { 1 } else { 0 }");
    }

    #[test]
    fn test_format_expr_ir_field_access() {
        let expr = ExprIR::FieldAccessExpr {
            object: Box::new(ExprIR::IdentifierExpr {
                name: "obj".into(),
                type_ann: None,
            })),
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
            })),
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
            })),
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

    #[test]
    fn test_format_expr_ir_await() {
        let expr = ExprIR::AwaitExpr {
            operand: Box::new(ExprIR::CallExpr {
                name: "fetch_data".into(),
                args: vec![],
            })),
        };
        let out = format_expr_ir(&expr);
        assert!(
            out.ends_with(".await"),
            "Rust await should be postfix '.await'. Output: {}",
            out
        );
        assert!(
            out.contains("fetch_data()"),
            "Rust await should contain the awaited call. Output: {}",
            out
        );
    }

    // ── generate_function tests ───────────────────────────

    #[test]
    fn test_generate_function_empty() {
        let mut w = CodeWriter::new();
        let func = FunctionIR {
            name: "do_thing".into(),
            visibility: Visibility::Private,
            params: vec![],
            return_type: None,
            body: BlockIR { statements: vec![] },
            is_async: false,
        };
        generate_function(&mut w, &func);
        let out = w.into_string();
        assert!(out.contains("fn do_thing(&self) {"));
    }

    #[test]
    fn test_generate_function_with_return_type() {
        let mut w = CodeWriter::new();
        let func = FunctionIR {
            name: "get_value".into(),
            visibility: Visibility::Private,
            params: vec![],
            return_type: Some(TypeIR {
                kind: "i32".into(),
                type_args: vec![],
                is_nullable: false,
            }),
            body: BlockIR { statements: vec![] },
            is_async: false,
        };
        generate_function(&mut w, &func);
        let out = w.into_string();
        assert!(out.contains("fn get_value(&self) -> i32 {"));
    }

    #[test]
    fn test_generate_function_with_params() {
        let mut w = CodeWriter::new();
        let func = FunctionIR {
            name: "greet".into(),
            visibility: Visibility::Private,
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
        };
        generate_function(&mut w, &func);
        let out = w.into_string();
        assert!(out.contains("fn greet(&self, name: String) -> String {"));
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
            ..Default::default()
        };
        generate_component(&mut w, "App", &comp);
        let out = w.into_string();
        assert!(out.contains("struct App;"));
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
                visibility: Visibility::Private,
                params: vec![],
                return_type: None,
                body: BlockIR { statements: vec![] },
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
        generate_component(&mut w, "App", &comp);
        let out = w.into_string();
        assert!(out.contains("struct App;"));
        assert!(out.contains("impl App {"));
        assert!(out.contains("fn greet(&self) {"));
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
        assert!(out.contains("struct Counter {"));
        assert!(out.contains("count: i32,"));
        assert!(out.contains("impl Counter {"));
        assert!(out.contains("fn new() -> Self {"));
        assert!(out.contains("count: 0,"));
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
            signal_defaults: HashMap::from([
                ("count".into(), serde_json::json!(0)),
                ("name".into(), serde_json::json!("world")),
            ]),
            getters: HashMap::new(),
            actions: vec![],
            ..Default::default()
        };
        generate_store(&mut w, "AppStore", &store);
        let out = w.into_string();
        assert!(out.contains("struct AppStore {"));
        assert!(out.contains("count: i32,"));
        assert!(out.contains("name: String,"));
        assert!(out.contains("fn new() -> Self {"));
        assert!(out.contains("count: 0,"));
        assert!(out.contains("name: \"world\".to_string(),"));
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
            ..Default::default()
        };
        generate_store(&mut w, "EmptyStore", &store);
        let out = w.into_string();
        assert!(out.contains("struct EmptyStore;"));
        assert!(out.contains("fn new() -> Self"));
    }

    // ── generate_statement tests ──────────────────────────

    #[test]
    fn test_generate_statement_let() {
        let mut w = CodeWriter::new();
        let stmt = StatementIR::LetStmtIR {
            name: "x".into(),
            value: Some(Box::new(ExprIR::LiteralExpr {
                value: serde_json::json!(42),
            })),
            r#type: None,
        };
        generate_statement(&mut w, &stmt);
        assert_eq!(w.into_string(), "let x = 42;\n");
    }

    #[test]
    fn test_generate_statement_let_with_type() {
        let mut w = CodeWriter::new();
        let stmt = StatementIR::LetStmtIR {
            name: "name".into(),
            value: Some(Box::new(ExprIR::LiteralExpr {
                value: serde_json::json!("hello"),
            })),
            r#type: Some(TypeIR {
                kind: "string".into(),
                type_args: vec![],
                is_nullable: false,
            })),
        };
        generate_statement(&mut w, &stmt);
        assert_eq!(
            w.into_string(),
            "let name: String = \"hello\".to_string();\n"
        );
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
    fn test_generate_statement_return_void() {
        let mut w = CodeWriter::new();
        let stmt = StatementIR::ReturnStmtIR { value: None };
        generate_statement(&mut w, &stmt);
        assert_eq!(w.into_string(), "return ();\n");
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
                item: None,
            })),
        };
        generate_statement(&mut w, &stmt);
        assert_eq!(w.into_string(), "self.count += 1;\n");
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
                item: None,
            })),
        };
        generate_statement(&mut w, &stmt);
        assert_eq!(w.into_string(), "self.name = \"hello\".to_string();\n");
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
                item: None,
            })),
        };
        generate_statement(&mut w, &stmt);
        assert_eq!(w.into_string(), "self.count -= 1;\n");
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
                    value: Some(Box::new(ExprIR::LiteralExpr {
                        value: serde_json::json!(1),
                    })),
                    r#type: None,
                }],
            },
            else_ifs: vec![],
            else_body: Some(BlockIR {
                statements: vec![StatementIR::LetStmtIR {
                    name: "x".into(),
                    value: Some(Box::new(ExprIR::LiteralExpr {
                        value: serde_json::json!(2),
                    })),
                    r#type: None,
                }],
            })),
        };
        generate_statement(&mut w, &stmt);
        let out = w.into_string();
        assert!(out.contains("if cond {"));
        assert!(out.contains("let x = 1;"));
        assert!(out.contains("} else {"));
        assert!(out.contains("let x = 2;"));
        assert!(out.contains("}"));
    }

    // ── Full pipeline through trait ────────────────────────

    #[test]
    fn test_full_pipeline_through_trait() {
        let codegen = RustCodegen;
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
                                visibility: Visibility::Private,
                                params: vec![],
                                return_type: None,
                                body: BlockIR { statements: vec![] },
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
            stdlib_imports: Default::default(),
        };
        let sources = HashMap::from([("main.omni".into(), "".into())]);
        let output = codegen.generate(&ir, &sources);
        assert!(output.errors.is_empty());
        assert_eq!(output.files.len(), 1);
        assert_eq!(output.files[0].path, "main.rs");
        insta::with_settings!({snapshot_path => "snapshots"}, {
            insta::assert_snapshot!("test_full_pipeline_through_trait", &output.files[0].content);
        });
    }

    #[test]
    fn test_full_pipeline_with_store() {
        let codegen = RustCodegen;
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
                                },
                            )]),
                            signal_defaults: HashMap::from([(
                                "count".into(),
                                serde_json::json!(0),
                            )]),
                            getters: HashMap::new(),
                            actions: vec![],
                            ..Default::default()
                        },
                    )]),
                    ..Default::default()
                },
            )]),
            source_map: HashMap::new(),
            stdlib_imports: Default::default(),
        };
        let sources = HashMap::from([("main.omni".into(), "".into())]);
        let output = codegen.generate(&ir, &sources);
        assert!(output.errors.is_empty());
        assert_eq!(output.files.len(), 1);
        insta::with_settings!({snapshot_path => "snapshots"}, {
            insta::assert_snapshot!("test_full_pipeline_with_store", &output.files[0].content);
        });
    }

    // ── Logic construct tests (structs, enums, type aliases, standalone functions) ──

    #[test]
    fn test_rs_generate_struct() {
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
                },
                FieldIR {
                    name: "y".into(),
                    r#type: TypeIR {
                        kind: "i32".into(),
                        type_args: vec![],
                        is_nullable: false,
                    },
                    visibility: Visibility::Public,
                },
            ],
        };
        visitor().generate_struct(&mut w, &struct_ir);
        let out = w.into_string();
        assert!(out.contains("pub struct Point {"));
        assert!(out.contains("pub x: i32,"));
        assert!(out.contains("pub y: i32,"));
    }

    #[test]
    fn test_rs_generate_empty_struct() {
        let mut w = CodeWriter::new();
        let struct_ir = StructIR {
            name: "Empty".into(),
            visibility: Visibility::Public,
            fields: vec![],
        };
        visitor().generate_struct(&mut w, &struct_ir);
        let out = w.into_string();
        assert!(out.contains("pub struct Empty {"));
        assert!(out.contains("}"));
    }

    #[test]
    fn test_rs_generate_enum() {
        let mut w = CodeWriter::new();
        let enum_ir = EnumIR {
            name: "Color".into(),
            visibility: Visibility::Public,
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
        visitor().generate_enum(&mut w, &enum_ir);
        let out = w.into_string();
        assert!(out.contains("pub enum Color {"));
        assert!(out.contains("Red,"));
        assert!(out.contains("Green,"));
        assert!(out.contains("Blue,"));
    }

    #[test]
    fn test_rs_generate_enum_with_payload() {
        let mut w = CodeWriter::new();
        let enum_ir = EnumIR {
            name: "Value".into(),
            visibility: Visibility::Public,
            variants: vec![
                VariantIR {
                    name: "Int".into(),
                    payload: Some(TypeIR {
                        kind: "i32".into(),
                        type_args: vec![],
                        is_nullable: false,
                    }),
                },
                VariantIR {
                    name: "Text".into(),
                    payload: Some(TypeIR {
                        kind: "string".into(),
                        type_args: vec![],
                        is_nullable: false,
                    }),
                },
            ],
        };
        visitor().generate_enum(&mut w, &enum_ir);
        let out = w.into_string();
        assert!(out.contains("pub enum Value {"));
        assert!(out.contains("Int(i32),"));
        assert!(out.contains("Text(String),"));
    }

    #[test]
    fn test_rs_generate_empty_enum() {
        let mut w = CodeWriter::new();
        let enum_ir = EnumIR {
            name: "Empty".into(),
            visibility: Visibility::Public,
            variants: vec![],
        };
        visitor().generate_enum(&mut w, &enum_ir);
        let out = w.into_string();
        assert!(out.contains("pub enum Empty {"));
        assert!(out.contains("}"));
    }

    #[test]
    fn test_rs_generate_type_alias() {
        let mut w = CodeWriter::new();
        let alias = TypeAliasIR {
            name: "UserId".into(),
            visibility: Visibility::Public,
            target: TypeIR {
                kind: "i32".into(),
                type_args: vec![],
                is_nullable: false,
            },
        };
        visitor().generate_type_alias(&mut w, &alias);
        assert_eq!(w.into_string(), "pub type UserId = i32;\n");
    }

    #[test]
    fn test_rs_generate_standalone_function() {
        let mut w = CodeWriter::new();
        let func = FunctionIR {
            name: "main".into(),
            visibility: Visibility::Public,
            params: vec![],
            return_type: None,
            body: BlockIR { statements: vec![] },
            is_async: false,
        };
        visitor().generate_function(&mut w, &func);
        let out = w.into_string();
        assert!(out.contains("pub fn main() {"));
        assert!(out.contains(";"));
    }

    #[test]
    fn test_rs_generate_standalone_function_with_params() {
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
        };
        visitor().generate_function(&mut w, &func);
        let out = w.into_string();
        assert!(out.contains("pub fn add(a: i32) -> i32 {"));
        assert!(!out.contains("&self"));
    }

    #[test]
    fn test_rs_generate_standalone_function_private() {
        let mut w = CodeWriter::new();
        let func = FunctionIR {
            name: "helper".into(),
            visibility: Visibility::Private,
            params: vec![],
            return_type: None,
            body: BlockIR { statements: vec![] },
            is_async: false,
        };
        visitor().generate_function(&mut w, &func);
        let out = w.into_string();
        assert!(out.contains("fn helper() {"));
        assert!(!out.contains("pub fn"));
    }

    #[test]
    fn test_format_type_ir_rust() {
        let ty = TypeIR {
            kind: "i32".into(),
            type_args: vec![],
            is_nullable: false,
        };
        assert_eq!(format_type_ir_rust(&ty), "i32");

        let str_ty = TypeIR {
            kind: "string".into(),
            type_args: vec![],
            is_nullable: false,
        };
        assert_eq!(format_type_ir_rust(&str_ty), "String");

        let vec_ty = TypeIR {
            kind: "Vec".into(),
            type_args: vec![ty],
            is_nullable: false,
        };
        assert_eq!(format_type_ir_rust(&vec_ty), "Vec<i32>");
    }

    #[test]
    fn test_rs_generate_struct_private() {
        let mut w = CodeWriter::new();
        let struct_ir = StructIR {
            name: "Internal".into(),
            visibility: Visibility::Private,
            fields: vec![],
        };
        visitor().generate_struct(&mut w, &struct_ir);
        let out = w.into_string();
        assert!(out.contains("struct Internal {"));
        assert!(!out.contains("pub struct"));
    }

    #[test]
    fn test_rs_generate_enum_private() {
        let mut w = CodeWriter::new();
        let enum_ir = EnumIR {
            name: "Internal".into(),
            visibility: Visibility::Private,
            variants: vec![],
        };
        visitor().generate_enum(&mut w, &enum_ir);
        let out = w.into_string();
        assert!(out.contains("enum Internal {"));
        assert!(!out.contains("pub enum"));
    }

    #[test]
    fn test_rs_format_variant_literal() {
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
        let out = format_expr_ir(&expr);
        assert!(
            out.contains("HttpResult::Ok"),
            "Rust variant should use :: syntax. Output: {}",
            out
        );
        assert!(
            out.contains("value: 42"),
            "Rust variant should contain fields. Output: {}",
            out
        );
    }
}
