#[cfg(test)]
mod tests {
    use super::super::*;
    use crate::codegen::capitalize_first;
    use std::collections::HashMap;

    // ── Helper tests ─────────────────────────────────────────

    #[test]
    fn test_capitalize_first() {
        assert_eq!(capitalize_first("hello"), "Hello");
        assert_eq!(capitalize_first(""), "");
        assert_eq!(capitalize_first("a"), "A");
        assert_eq!(capitalize_first("ABC"), "ABC");
    }

    #[test]
    fn test_format_json_value_vue() {
        assert_eq!(format_json_value(&serde_json::Value::Null, &JSON_VUE), "null");
        assert_eq!(format_json_value(&serde_json::json!(true), &JSON_VUE), "true");
        assert_eq!(format_json_value(&serde_json::json!(false), &JSON_VUE), "false");
        assert_eq!(format_json_value(&serde_json::json!(42), &JSON_VUE), "42");
        assert_eq!(format_json_value(&serde_json::json!(2.71), &JSON_VUE), "2.71");
        assert_eq!(
            format_json_value(&serde_json::json!("hello"), &JSON_VUE),
            "\"hello\""
        );
        assert_eq!(
            format_json_value(&serde_json::json!("he\"llo"), &JSON_VUE),
            "\"he\\\"llo\""
        );
    }

    #[test]
    fn test_camel_to_kebab() {
        assert_eq!(camel_to_kebab("backgroundColor"), "background-color");
        assert_eq!(camel_to_kebab("fontSize"), "font-size");
        assert_eq!(camel_to_kebab("display"), "display");
        assert_eq!(camel_to_kebab("borderRadius"), "border-radius");
    }

    #[test]
    fn test_to_vue_attr() {
        assert_eq!(to_vue_attr("class"), "class");
        assert_eq!(to_vue_attr("className"), "class");
        assert_eq!(to_vue_attr("for"), "for");
        assert_eq!(to_vue_attr("htmlFor"), "for");
        assert_eq!(to_vue_attr("tabindex"), "tabindex");
        assert_eq!(to_vue_attr("autocomplete"), "autocomplete");
    }

    #[test]
    fn test_to_vue_event() {
        assert_eq!(to_vue_event("click"), "click");
        assert_eq!(to_vue_event("change"), "change");
        assert_eq!(to_vue_event("submit"), "submit");
        assert_eq!(to_vue_event("focus"), "focus");
        assert_eq!(to_vue_event("customEvent"), "customEvent");
    }

    #[test]
    fn test_binary_op_to_vue() {
        assert_eq!(crate::codegen::binary_op_to_str("add", true, false), "+");
        assert_eq!(crate::codegen::binary_op_to_str("eq", true, false), "===");
        assert_eq!(crate::codegen::binary_op_to_str("and", true, false), "&&");
        assert_eq!(crate::codegen::binary_op_to_str("or", true, false), "||");
        assert_eq!(crate::codegen::binary_op_to_str("neq", true, false), "!==");
    }

    // ── format_expr_vue tests ───────────────────────────────

    #[test]
    fn test_format_expr_literal() {
        let expr = ExprIR::LiteralExpr {
            value: serde_json::json!(42),
        };
        assert_eq!(format_expr_vue(&expr), "42");
    }

    #[test]
    fn test_format_expr_identifier() {
        let expr = ExprIR::IdentifierExpr {
            name: "count".into(),
        };
        assert_eq!(format_expr_vue(&expr), "count");
    }

    #[test]
    fn test_format_expr_binary() {
        let expr = ExprIR::BinaryOpExpr {
            op: "add".into(),
            left: Box::new(ExprIR::IdentifierExpr { name: "a".into() }),
            right: Box::new(ExprIR::IdentifierExpr { name: "b".into() }),
        };
        assert_eq!(format_expr_vue(&expr), "a + b");
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
        assert_eq!(format_expr_vue(&expr), "x ? 1 : 0");
    }

    #[test]
    fn test_format_expr_field_access() {
        let expr = ExprIR::FieldAccessExpr {
            object: Box::new(ExprIR::IdentifierExpr { name: "obj".into() }),
            field: "prop".into(),
        };
        assert_eq!(format_expr_vue(&expr), "obj.prop");
    }

    #[test]
    fn test_format_expr_unary_not() {
        let expr = ExprIR::UnaryOpExpr {
            op: "not".into(),
            operand: Box::new(ExprIR::IdentifierExpr {
                name: "flag".into(),
            }),
        };
        assert_eq!(format_expr_vue(&expr), "!flag");
    }

    #[test]
    fn test_format_expr_store_field() {
        let expr = ExprIR::StoreFieldAccessExpr {
            store_alias: "store".into(),
            field_name: "getCount".into(),
            call_args: vec![],
        };
        assert_eq!(format_expr_vue(&expr), "store.getCount");
    }

    // ── generate_statement_vue tests ────────────────────────

    #[test]
    fn test_vue_stmt_let() {
        let mut w = CodeWriter::new();
        let stmt = StatementIR::LetStmtIR {
            name: "x".into(),
            value: Some(ExprIR::LiteralExpr {
                value: serde_json::json!(42),
            }),
            r#type: None,
        };
        generate_statement_vue(&mut w, &stmt);
        assert_eq!(w.into_string(), "const x = 42;\n");
    }

    #[test]
    fn test_vue_stmt_return() {
        let mut w = CodeWriter::new();
        let stmt = StatementIR::ReturnStmtIR {
            value: Some(ExprIR::IdentifierExpr {
                name: "result".into(),
            }),
        };
        generate_statement_vue(&mut w, &stmt);
        assert_eq!(w.into_string(), "return result;\n");
    }

    #[test]
    fn test_vue_stmt_mutation_increment() {
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
        generate_statement_vue(&mut w, &stmt);
        assert_eq!(w.into_string(), "count.value += 1;\n");
    }

    #[test]
    fn test_vue_stmt_mutation_assign() {
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
        generate_statement_vue(&mut w, &stmt);
        assert_eq!(w.into_string(), "name.value = \"alice\";\n");
    }

    #[test]
    fn test_vue_stmt_if() {
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
        generate_statement_vue(&mut w, &stmt);
        let out = w.into_string();
        assert!(out.contains("if (cond) {"));
        assert!(out.contains("const x = 1;"));
        assert!(out.contains("}"));
    }

    // ── gen_component_sfc tests ──────────────────────────────

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
    fn test_vue_empty_component() {
        let codegen = VueCodegen;
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
        let sfc = codegen.gen_component_sfc("App", comp_ref, &ir);
        assert!(sfc.contains("<template>"));
        assert!(sfc.contains("</template>"));
        assert!(sfc.contains("<script setup lang=\"ts\">"));
        assert!(sfc.contains("</script>"));
        assert!(sfc.contains("<style scoped>"));
        assert!(sfc.contains("</style>"));
        assert!(sfc.contains("import { ref } from 'vue';"));
        assert!(sfc.contains("<div class=\"container\">"));
        assert!(sfc.contains(".container {"));
    }

    #[test]
    fn test_vue_component_with_state() {
        let codegen = VueCodegen;
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
        let sfc = codegen.gen_component_sfc("Counter", comp_ref, &ir);
        assert!(sfc.contains("const count = ref<number>(0);"));
        assert!(sfc.contains("<template>"));
        assert!(sfc.contains("</template>"));
        assert!(sfc.contains("<script setup lang=\"ts\">"));
        assert!(sfc.contains("</script>"));
    }

    #[test]
    fn test_vue_component_with_props() {
        let codegen = VueCodegen;
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
        let sfc = codegen.gen_component_sfc("Greeting", comp_ref, &ir);
        assert!(sfc.contains("defineProps"));
        assert!(sfc.contains("name: string;"));
        assert!(sfc.contains("age: number;"));
    }

    #[test]
    fn test_vue_component_with_effect() {
        let codegen = VueCodegen;
        let effect = EffectIntent {
            deps: vec!["count".into()],
            body: BlockIR {
                statements: vec![StatementIR::ExprStmtIR {
                    expr: ExprIR::IdentifierExpr {
                        name: "console.log(count)".into(),
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
        let sfc = codegen.gen_component_sfc("Watcher", comp_ref, &ir);
        assert!(sfc.contains("watchEffect(() => {"));
        assert!(sfc.contains("import { ref, watchEffect } from 'vue';"));
    }

    #[test]
    fn test_vue_component_with_lifecycle_mount() {
        let codegen = VueCodegen;
        let lc = LifecycleIntent {
            kind: "mount".into(),
            body: BlockIR {
                statements: vec![],
                execution_mode: "client-only".into(),
            },
        };
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
                lifecycles: vec![lc],
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
        let sfc = codegen.gen_component_sfc("App", comp_ref, &ir);
        assert!(sfc.contains("onMounted(() => {"));
        assert!(sfc.contains("import { ref, onMounted } from 'vue';"));
    }

    // ── gen_store_hook tests ─────────────────────────────────

    #[test]
    fn test_vue_store_hook() {
        let codegen = VueCodegen;
        let store = StoreIR {
            name: "AppStore".into(),
            signals: HashMap::new(),
            signal_defaults: HashMap::new(),
            getters: HashMap::new(),
            actions: vec![],
            ..Default::default()
        };
        let hook = codegen.gen_store_hook("AppStore", &store);
        assert!(hook.contains("export function useAppStore() {"));
        assert!(hook.contains("import { ref, computed } from 'vue';"));
    }

    #[test]
    fn test_vue_store_hook_with_signals() {
        let codegen = VueCodegen;
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
        let hook = codegen.gen_store_hook("AppStore", &store);
        assert!(hook.contains("const count = ref<number>(0);"));
        assert!(hook.contains("const name = ref<string>(\"world\");"));
        // HashMap ordering is non-deterministic, check both possibilities
        assert!(
            hook.contains("return { count, name };") || hook.contains("return { name, count };")
        );
    }

    #[test]
    fn test_vue_store_hook_with_getters() {
        let codegen = VueCodegen;
        let mut getters = HashMap::new();
        getters.insert(
            "doubleCount".into(),
            ExprIR::BinaryOpExpr {
                op: "mul".into(),
                left: Box::new(ExprIR::IdentifierExpr {
                    name: "count".into(),
                }),
                right: Box::new(ExprIR::LiteralExpr {
                    value: serde_json::json!(2),
                }),
            },
        );
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
        let mut defaults = HashMap::new();
        defaults.insert("count".into(), serde_json::json!(0));
        let store = StoreIR {
            name: "CalcStore".into(),
            signals,
            signal_defaults: defaults,
            getters,
            actions: vec![],
            ..Default::default()
        };
        let hook = codegen.gen_store_hook("CalcStore", &store);
        assert!(hook.contains("const count = ref<number>(0);"));
        assert!(hook.contains("const doubleCount = computed(() => {"));
        assert!(hook.contains("return count * 2;"));
        assert!(hook.contains("return { count, doubleCount };"));
    }

    // ── Full pipeline through Codegen trait ──────────────────

    #[test]
    fn test_full_pipeline_empty() {
        let codegen = VueCodegen;
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
        let codegen = VueCodegen;
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
        assert_eq!(output.files.len(), 1); // just App.vue

        let sfc = &output.files[0];
        assert_eq!(sfc.path, "App.vue");
        insta::with_settings!({snapshot_path => "snapshots"}, {
            insta::assert_snapshot!("test_full_pipeline_component", &sfc.content);
        });
    }

    #[test]
    fn test_full_pipeline_with_store() {
        let codegen = VueCodegen;
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
        assert_eq!(output.files.len(), 1); // just useAppStore.ts
        let hook = &output.files[0];
        assert_eq!(hook.path, "useAppStore.ts");
        insta::with_settings!({snapshot_path => "snapshots"}, {
            insta::assert_snapshot!("test_full_pipeline_with_store", &hook.content);
        });
    }

    #[test]
    fn test_full_pipeline_component_with_store_ref() {
        let codegen = VueCodegen;
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
        // Expect App.vue + useAppStore.ts
        assert_eq!(output.files.len(), 2);
        let sfc = output.files.iter().find(|f| f.path == "App.vue").unwrap();
        insta::with_settings!({snapshot_path => "snapshots"}, {
            insta::assert_snapshot!("test_full_pipeline_component_with_store_ref", &sfc.content);
        });
    }
}
