#[cfg(test)]
mod tests {
    use super::super::*;
    use std::collections::HashMap;

    // ── Helper tests ─────────────────────────────────────────

    #[test]
    fn test_to_snake_case() {
        assert_eq!(to_snake_case("AppStore"), "app_store");
        assert_eq!(to_snake_case("Counter"), "counter");
        assert_eq!(to_snake_case("ABC"), "a_b_c");
        assert_eq!(to_snake_case(""), "");
    }

    #[test]
    fn test_to_dioxus_attr() {
        assert_eq!(to_dioxus_attr("class"), "class");
        assert_eq!(to_dioxus_attr("className"), "class");
        assert_eq!(to_dioxus_attr("tabindex"), "tabindex");
        assert_eq!(to_dioxus_attr("autocomplete"), "autocomplete");
    }

    #[test]
    fn test_to_dioxus_event() {
        assert_eq!(to_dioxus_event("click"), "onclick");
        assert_eq!(to_dioxus_event("change"), "onchange");
        assert_eq!(to_dioxus_event("submit"), "onsubmit");
        assert_eq!(to_dioxus_event("focus"), "onfocus");
        assert_eq!(to_dioxus_event("dblclick"), "ondblclick");
        assert_eq!(to_dioxus_event("keydown"), "onkeydown");
    }

    #[test]
    fn test_binary_op_to_dioxus() {
        assert_eq!(crate::codegen::binary_op_to_str("add", false, false), "+");
        assert_eq!(crate::codegen::binary_op_to_str("eq", false, false), "==");
        assert_eq!(crate::codegen::binary_op_to_str("and", false, false), "&&");
        assert_eq!(crate::codegen::binary_op_to_str("or", false, false), "||");
        assert_eq!(crate::codegen::binary_op_to_str("neq", false, false), "!=");
    }

    #[test]
    fn test_format_json_value_dioxus() {
        assert_eq!(
            format_json_value(&serde_json::Value::Null, &JSON_DIOXUS),
            "None"
        );
        assert_eq!(
            format_json_value(&serde_json::Value::Bool(true), &JSON_DIOXUS),
            "true"
        );
        assert_eq!(
            format_json_value(&serde_json::Value::Bool(false), &JSON_DIOXUS),
            "false"
        );
        assert_eq!(
            format_json_value(&serde_json::json!(42), &JSON_DIOXUS),
            "42"
        );
        assert_eq!(
            format_json_value(&serde_json::json!(2.71), &JSON_DIOXUS),
            "2.71"
        );
        assert_eq!(
            format_json_value(&serde_json::json!("hello"), &JSON_DIOXUS),
            "\"hello\".to_string()"
        );
        assert_eq!(
            format_json_value(&serde_json::json!("he\"llo"), &JSON_DIOXUS),
            "\"he\\\"llo\".to_string()"
        );
        assert_eq!(
            format_json_value(&serde_json::json!(vec![1, 2, 3]), &JSON_DIOXUS),
            "vec![1, 2, 3]"
        );
    }

    // ── format_expr_ir_dioxus tests ─────────────────────────

    #[test]
    fn test_format_expr_literal() {
        let expr = ExprIR::LiteralExpr {
            value: serde_json::json!(42),
        };
        assert_eq!(format_expr_ir_dioxus(&expr), "42");
    }

    #[test]
    fn test_format_expr_identifier() {
        let expr = ExprIR::IdentifierExpr {
            name: "count".into(),
        };
        assert_eq!(format_expr_ir_dioxus(&expr), "count");
    }

    #[test]
    fn test_format_expr_binary() {
        let expr = ExprIR::BinaryOpExpr {
            op: "add".into(),
            left: Box::new(ExprIR::IdentifierExpr { name: "a".into() }),
            right: Box::new(ExprIR::IdentifierExpr { name: "b".into() }),
        };
        assert_eq!(format_expr_ir_dioxus(&expr), "a + b");
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
        assert_eq!(format_expr_ir_dioxus(&expr), "if x { 1 } else { 0 }");
    }

    #[test]
    fn test_format_expr_field_access() {
        let expr = ExprIR::FieldAccessExpr {
            object: Box::new(ExprIR::IdentifierExpr { name: "obj".into() }),
            field: "prop".into(),
        };
        assert_eq!(format_expr_ir_dioxus(&expr), "obj.prop");
    }

    #[test]
    fn test_format_expr_unary_not() {
        let expr = ExprIR::UnaryOpExpr {
            op: "not".into(),
            operand: Box::new(ExprIR::IdentifierExpr {
                name: "flag".into(),
            }),
        };
        assert_eq!(format_expr_ir_dioxus(&expr), "!flag");
    }

    #[test]
    fn test_format_expr_store_field() {
        let expr = ExprIR::StoreFieldAccessExpr {
            store_alias: "AppStore".into(),
            field_name: "getCount".into(),
            call_args: vec![],
        };
        assert_eq!(format_expr_ir_dioxus(&expr), "AppStore.getCount");
    }

    // ── generate_statement_dioxus tests ─────────────────────

    #[test]
    fn test_stmt_let() {
        let mut w = CodeWriter::new();
        let stmt = StatementIR::LetStmtIR {
            name: "x".into(),
            value: Some(ExprIR::LiteralExpr {
                value: serde_json::json!(42),
            }),
            r#type: None,
        };
        generate_statement_dioxus(&mut w, &stmt);
        assert_eq!(w.into_string(), "let x = 42;\n");
    }

    #[test]
    fn test_stmt_let_with_type() {
        let mut w = CodeWriter::new();
        let stmt = StatementIR::LetStmtIR {
            name: "name".into(),
            value: Some(ExprIR::LiteralExpr {
                value: serde_json::json!("hello"),
            }),
            r#type: Some(TypeIR {
                kind: "string".into(),
                type_args: vec![],
                is_nullable: false,
                needs_clone: false,
                needs_partial_eq: false,
            }),
        };
        generate_statement_dioxus(&mut w, &stmt);
        assert_eq!(
            w.into_string(),
            "let name: String = \"hello\".to_string();\n"
        );
    }

    #[test]
    fn test_stmt_return() {
        let mut w = CodeWriter::new();
        let stmt = StatementIR::ReturnStmtIR {
            value: Some(ExprIR::IdentifierExpr {
                name: "result".into(),
            }),
        };
        generate_statement_dioxus(&mut w, &stmt);
        assert_eq!(w.into_string(), "return result;\n");
    }

    #[test]
    fn test_stmt_mutation_direct_assign() {
        let mut w = CodeWriter::new();
        let stmt = StatementIR::MutationStmt {
            intent: Some(MutationIntent {
                target: SignalRefIR {
                    owner: None,
                    name: "name".into(),
                },
                operation: "direct_assign".into(),
                amount: Some(ExprIR::LiteralExpr {
                    value: serde_json::json!("alice"),
                }),
                item: None,
            }),
        };
        generate_statement_dioxus(&mut w, &stmt);
        assert_eq!(w.into_string(), "name.set(\"alice\".to_string());\n");
    }

    #[test]
    fn test_stmt_mutation_increment() {
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
        generate_statement_dioxus(&mut w, &stmt);
        assert_eq!(w.into_string(), "count += 1;\n");
    }

    #[test]
    fn test_stmt_mutation_decrement() {
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
        generate_statement_dioxus(&mut w, &stmt);
        assert_eq!(w.into_string(), "count -= 1;\n");
    }

    #[test]
    fn test_stmt_if() {
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
            else_body: None,
        };
        generate_statement_dioxus(&mut w, &stmt);
        let out = w.into_string();
        assert!(out.contains("if cond {"));
        assert!(out.contains("let x = 1;"));
        assert!(out.contains("}"));
    }

    // ── gen_component_rs tests ──────────────────────────────

    /// Build IRProgram with a single module containing the given components.
    fn make_ir(components: HashMap<String, ComponentIR>) -> IRProgram {
        IRProgram {
            version: "1.0".into(),
            modules: HashMap::from([(
                "main".to_string(),
                ModuleIR {
                    name: "main".to_string(),
                    components,
                    ..Default::default()
                },
            )]),
            stdlib_imports: Vec::new(),
            source_map: HashMap::new(),
        }
    }

    #[test]
    fn test_empty_component() {
        let codegen = DioxusCodegen;
        let mut components = HashMap::new();
        components.insert(
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
            },
        );
        let ir = make_ir(components);
        let comp_ref = ir
            .modules
            .get("main")
            .unwrap()
            .components
            .get("App")
            .unwrap();
        let rs = codegen.gen_component_rs("App", comp_ref, &ir);
        assert!(rs.contains("use dioxus::prelude::*;"));
        assert!(rs.contains("#[component]"));
        assert!(rs.contains("pub fn App() -> Element {"));
        assert!(rs.contains("rsx! {"));
        assert!(rs.contains("\"App\""));
    }

    #[test]
    fn test_component_with_state() {
        let codegen = DioxusCodegen;
        let state = StateIntent {
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
        };
        let mut components = HashMap::new();
        components.insert(
            "Counter".into(),
            ComponentIR {
                name: "Counter".into(),
                generic_params: vec![],
                props: HashMap::new(),
                prop_defaults: HashMap::new(),
                states: vec![state],
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
            },
        );
        let ir = make_ir(components);
        let comp_ref = ir
            .modules
            .get("main")
            .unwrap()
            .components
            .get("Counter")
            .unwrap();
        let rs = codegen.gen_component_rs("Counter", comp_ref, &ir);
        assert!(rs.contains("pub fn Counter() -> Element {"));
        assert!(rs.contains("let mut count = use_signal(|| 0);"));
    }

    #[test]
    fn test_component_with_props() {
        let codegen = DioxusCodegen;
        let mut props = HashMap::new();
        props.insert(
            "name".into(),
            TypeIR {
                kind: "string".into(),
                type_args: vec![],
                is_nullable: false,
                needs_clone: false,
                needs_partial_eq: false,
            },
        );
        props.insert(
            "age".into(),
            TypeIR {
                kind: "i32".into(),
                type_args: vec![],
                is_nullable: false,
                needs_clone: false,
                needs_partial_eq: false,
            },
        );
        let mut components = HashMap::new();
        components.insert(
            "Greeting".into(),
            ComponentIR {
                name: "Greeting".into(),
                generic_params: vec![],
                props,
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
            },
        );
        let ir = make_ir(components);
        let comp_ref = ir
            .modules
            .get("main")
            .unwrap()
            .components
            .get("Greeting")
            .unwrap();
        let rs = codegen.gen_component_rs("Greeting", comp_ref, &ir);
        assert!(rs.contains("#[derive(Props, PartialEq, Clone)]"));
        assert!(rs.contains("struct GreetingProps {"));
        assert!(rs.contains("name: String,"));
        assert!(rs.contains("age: i32,"));
        assert!(rs.contains("pub fn Greeting(props: GreetingProps) -> Element {"));
    }

    #[test]
    fn test_component_with_effect() {
        let codegen = DioxusCodegen;
        let effect = EffectIntent {
            deps: vec!["count".into()],
            body: BlockIR {
                statements: vec![StatementIR::ExprStmtIR {
                    expr: ExprIR::IdentifierExpr {
                        name: "println!(\"count changed\")".into(),
                    },
                }],
                execution_mode: "client-only".into(),
            },
        };
        let mut components = HashMap::new();
        components.insert(
            "Watcher".into(),
            ComponentIR {
                name: "Watcher".into(),
                generic_params: vec![],
                props: HashMap::new(),
                prop_defaults: HashMap::new(),
                states: vec![],
                computeds: vec![],
                effects: vec![effect],
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
            },
        );
        let ir = make_ir(components);
        let comp_ref = ir
            .modules
            .get("main")
            .unwrap()
            .components
            .get("Watcher")
            .unwrap();
        let rs = codegen.gen_component_rs("Watcher", comp_ref, &ir);
        assert!(rs.contains("use_effect(move || async move {"));
    }

    #[test]
    fn test_component_with_computed() {
        let codegen = DioxusCodegen;
        let computed = ComputedIntent {
            name: "double".into(),
            r#type: TypeIR {
                kind: "i32".into(),
                type_args: vec![],
                is_nullable: false,
                needs_clone: false,
                needs_partial_eq: false,
            },
            body: ExprIR::BinaryOpExpr {
                op: "mul".into(),
                left: Box::new(ExprIR::IdentifierExpr {
                    name: "count".into(),
                }),
                right: Box::new(ExprIR::LiteralExpr {
                    value: serde_json::json!(2),
                }),
            },
            deps: vec!["count".into()],
        };
        let mut components = HashMap::new();
        components.insert(
            "Calc".into(),
            ComponentIR {
                name: "Calc".into(),
                generic_params: vec![],
                props: HashMap::new(),
                prop_defaults: HashMap::new(),
                states: vec![],
                computeds: vec![computed],
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
            },
        );
        let ir = make_ir(components);
        let comp_ref = ir
            .modules
            .get("main")
            .unwrap()
            .components
            .get("Calc")
            .unwrap();
        let rs = codegen.gen_component_rs("Calc", comp_ref, &ir);
        assert!(rs.contains("let double = use_memo(move || {"));
        assert!(rs.contains("count * 2"));
    }

    // ── gen_store_module tests ──────────────────────────────

    #[test]
    fn test_store_module_empty() {
        let codegen = DioxusCodegen;
        let store = StoreIR {
            name: "EmptyStore".into(),
            signals: HashMap::new(),
            signal_defaults: HashMap::new(),
            getters: HashMap::new(),
            actions: vec![],
            ..Default::default()
        };
        let mod_str = codegen.gen_store_module("EmptyStore", &store);
        assert!(mod_str.contains("pub struct EmptyStore;"));
        assert!(mod_str.contains("pub fn use_empty_store() -> EmptyStore {"));
    }

    #[test]
    fn test_store_module_with_signals() {
        let codegen = DioxusCodegen;
        let mut signals = HashMap::new();
        signals.insert(
            "count".into(),
            TypeIR {
                kind: "i32".into(),
                type_args: vec![],
                is_nullable: false,
                needs_clone: false,
                needs_partial_eq: false,
            },
        );
        signals.insert(
            "name".into(),
            TypeIR {
                kind: "string".into(),
                type_args: vec![],
                is_nullable: false,
                needs_clone: false,
                needs_partial_eq: false,
            },
        );
        let mut defaults = HashMap::new();
        defaults.insert("count".into(), serde_json::json!(0));
        defaults.insert("name".into(), serde_json::json!("world"));
        let store = StoreIR {
            name: "AppStore".into(),
            signals,
            signal_defaults: defaults,
            getters: HashMap::new(),
            actions: vec![],
            ..Default::default()
        };
        let mod_str = codegen.gen_store_module("AppStore", &store);
        assert!(mod_str.contains("pub struct AppStore {"));
        assert!(mod_str.contains("pub count: Signal<i32>,"));
        assert!(mod_str.contains("pub name: Signal<String>,"));
        assert!(mod_str.contains("pub fn use_app_store() -> AppStore {"));
        assert!(mod_str.contains("count: use_signal(|| 0),"));
        assert!(mod_str.contains("name: use_signal(|| \"world\".to_string()),"));
    }

    // ── Full pipeline through Codegen trait ──────────────────

    #[test]
    fn test_full_pipeline_empty() {
        let codegen = DioxusCodegen;
        let ir = IRProgram {
            version: "0.1".into(),
            modules: HashMap::from([(
                "main".to_string(),
                ModuleIR {
                    name: "main".to_string(),
                    components: HashMap::new(),
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
        assert!(output.files.is_empty());
    }

    #[test]
    fn test_full_pipeline_component() {
        let codegen = DioxusCodegen;
        let comp = ComponentIR {
            name: "App".into(),
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
        let ir = IRProgram {
            version: "0.1".into(),
            modules: HashMap::from([(
                "main".to_string(),
                ModuleIR {
                    name: "main".to_string(),
                    components: HashMap::from([("App".into(), comp)]),
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
        let rs = &output.files[0];
        assert!(rs.path.ends_with(".rs"));
        insta::with_settings!({snapshot_path => "snapshots"}, {
            insta::assert_snapshot!("test_full_pipeline_component", &rs.content);
        });
    }

    #[test]
    fn test_full_pipeline_with_store() {
        let codegen = DioxusCodegen;
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
        let ir = IRProgram {
            version: "0.1".into(),
            modules: HashMap::from([(
                "main".to_string(),
                ModuleIR {
                    name: "main".to_string(),
                    components: HashMap::new(),
                    stores: HashMap::from([("AppStore".into(), store)]),
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
        let store_file = &output.files[0];
        assert_eq!(store_file.path, "app_store.rs");
        insta::with_settings!({snapshot_path => "snapshots"}, {
            insta::assert_snapshot!("test_full_pipeline_with_store", &store_file.content);
        });
    }

    #[test]
    fn test_full_pipeline_component_with_store_ref() {
        let codegen = DioxusCodegen;
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
            store_refs: vec!["AppStore".into()],
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
        let ir = IRProgram {
            version: "0.1".into(),
            modules: HashMap::from([(
                "main".to_string(),
                ModuleIR {
                    name: "main".to_string(),
                    components: HashMap::from([("App".into(), comp)]),
                    stores: HashMap::from([("AppStore".into(), store)]),
                    ..Default::default()
                },
            )]),
            source_map: HashMap::new(),
            stdlib_imports: Vec::new(),
        };
        let sources = HashMap::from([("main.omni".into(), "".into())]);
        let output = codegen.generate(&ir, &sources);
        assert!(output.errors.is_empty());
        assert_eq!(output.files.len(), 2); // App.rs + app_store.rs
        let comp_file = output.files.iter().find(|f| f.path == "App.rs").unwrap();
        insta::with_settings!({snapshot_path => "snapshots"}, {
            insta::assert_snapshot!("test_full_pipeline_component_with_store_ref", &comp_file.content);
        });
    }
}
