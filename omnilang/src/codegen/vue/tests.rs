#[cfg(test)]
mod tests {
    use super::super::*;
    use crate::codegen::camel_to_kebab;
    use crate::codegen::capitalize_first;
    use rustc_hash::FxHashMap;
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::rc::Rc;

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
        assert_eq!(
            format_json_value(&serde_json::Value::Null, &JSON_VUE),
            "null"
        );
        assert_eq!(
            format_json_value(&serde_json::json!(true), &JSON_VUE),
            "true"
        );
        assert_eq!(
            format_json_value(&serde_json::json!(false), &JSON_VUE),
            "false"
        );
        assert_eq!(format_json_value(&serde_json::json!(42), &JSON_VUE), "42");
        assert_eq!(
            format_json_value(&serde_json::json!(2.71), &JSON_VUE),
            "2.71"
        );
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
        // CSS custom properties (--prefixed) should be returned unchanged
        assert_eq!(camel_to_kebab("--myProp"), "--myProp");
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
        // IR carries "on" prefix; function strips it and maps to bare DOM names
        assert_eq!(to_vue_event("onClick"), "click");
        assert_eq!(to_vue_event("onChange"), "change");
        assert_eq!(to_vue_event("onSubmit"), "submit");
        assert_eq!(to_vue_event("onPress"), "click");
        assert_eq!(to_vue_event("onTap"), "click");
        assert_eq!(to_vue_event("onFocus"), "focus");
        assert_eq!(to_vue_event("onBlur"), "blur");
        assert_eq!(to_vue_event("onKeydown"), "keydown");
        assert_eq!(to_vue_event("onInput"), "input");
        // Custom events get lowercased from their stem
        assert_eq!(to_vue_event("onCustomEvent"), "customevent");
    }

    #[test]
    fn test_binary_op_to_vue() {
        assert_eq!(
            crate::codegen::binary_op_to_str(&BinaryOpIR::Add, true, false),
            "+"
        );
        assert_eq!(
            crate::codegen::binary_op_to_str(&BinaryOpIR::Eq, true, false),
            "==="
        );
        assert_eq!(
            crate::codegen::binary_op_to_str(&BinaryOpIR::And, true, false),
            "&&"
        );
        assert_eq!(
            crate::codegen::binary_op_to_str(&BinaryOpIR::Or, true, false),
            "||"
        );
        assert_eq!(
            crate::codegen::binary_op_to_str(&BinaryOpIR::Neq, true, false),
            "!=="
        );
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
            type_ann: None,
        };
        assert_eq!(format_expr_vue(&expr), "count");
    }

    #[test]
    fn test_format_expr_binary() {
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
        assert_eq!(format_expr_vue(&expr), "a + b");
    }

    #[test]
    fn test_format_expr_ternary() {
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
        assert_eq!(format_expr_vue(&expr), "x ? 1 : 0");
    }

    #[test]
    fn test_format_expr_field_access() {
        let expr = ExprIR::FieldAccessExpr {
            object: Box::new(ExprIR::IdentifierExpr {
                name: "obj".into(),
                type_ann: None,
            }),
            field: "prop".into(),
        };
        assert_eq!(format_expr_vue(&expr), "obj.prop");
    }

    #[test]
    fn test_format_expr_unary_not() {
        let expr = ExprIR::UnaryOpExpr {
            op: UnaryOpIR::Not,
            operand: Box::new(ExprIR::IdentifierExpr {
                name: "flag".into(),
                type_ann: None,
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

    #[test]
    fn test_vue_class_map_bindings() {
        let codegen = VueCodegen;
        let mut class_map: FxHashMap<String, ExprIR> = FxHashMap::default();
        class_map.insert(
            "active".into(),
            ExprIR::IdentifierExpr {
                name: "isOpen".into(),
                type_ann: None,
            },
        );
        class_map.insert(
            "highlighted".into(),
            ExprIR::IdentifierExpr {
                name: "isSelected".into(),
                type_ann: None,
            },
        );
        let mut components = FxHashMap::default();
        components.insert(
            "Card".into(),
            ComponentIR {
                form_intent: None,
                load_intent: None,
                name: "Card".into(),
                props: FxHashMap::default(),
                prop_defaults: FxHashMap::default(),
                signals: vec![],
                computeds: vec![],
                effects: vec![],
                lifecycles: vec![],
                fns: vec![],
                store_refs: vec![],
                refs: vec![],
                events: vec![],
                view: Some(ViewTree::Element(Box::new(ElementIR {
                    tag: "div".into(),
                    props: FxHashMap::default(),
                    bindings: FxHashMap::default(),
                    events: FxHashMap::default(),
                    style_classes: None,
                    inline_style: None,
                    children: vec![],
                    class_map_bindings: class_map,
                    motion: None,
                    ref_binding: None,
                    construction_kind: ConstructionKind::Static,
                    source_span: None,
                    semantic_bindings: FxHashMap::default(),
                    a11y: None,
                    drag_binding: None,
                }))),
                styles: vec![],
                ..Default::default()
            },
        );
        let ir = make_ir(components);
        let comp_ref = ir
            .modules
            .get("main")
            .unwrap()
            .components
            .get("Card")
            .unwrap();
        let sfc =
            codegen.gen_component_sfc("Card", comp_ref, &ir, &Rc::new(RefCell::new(Vec::new())));
        // Should contain :class with object syntax
        assert!(
            sfc.contains(":class=\"{ active: isOpen, highlighted: isSelected }\"")
                || sfc.contains(":class=\"{ highlighted: isSelected, active: isOpen }\"")
        );
    }

    #[test]
    fn test_vue_class_map_bindings_with_static_class() {
        let codegen = VueCodegen;
        let mut class_map: FxHashMap<String, ExprIR> = FxHashMap::default();
        class_map.insert(
            "active".into(),
            ExprIR::IdentifierExpr {
                name: "isOpen".into(),
                type_ann: None,
            },
        );
        let mut components = FxHashMap::default();
        components.insert(
            "Card".into(),
            ComponentIR {
                form_intent: None,
                load_intent: None,
                name: "Card".into(),
                props: FxHashMap::default(),
                prop_defaults: FxHashMap::default(),
                signals: vec![],
                computeds: vec![],
                effects: vec![],
                lifecycles: vec![],
                fns: vec![],
                store_refs: vec![],
                refs: vec![],
                events: vec![],
                view: Some(ViewTree::Element(Box::new(ElementIR {
                    tag: "div".into(),
                    props: FxHashMap::from_iter([(
                        "class".into(),
                        ExprIR::LiteralExpr {
                            value: serde_json::json!("base"),
                        },
                    )]),
                    bindings: FxHashMap::default(),
                    events: FxHashMap::default(),
                    style_classes: None,
                    inline_style: None,
                    children: vec![],
                    class_map_bindings: class_map,
                    motion: None,
                    ref_binding: None,
                    construction_kind: ConstructionKind::Static,
                    source_span: None,
                    semantic_bindings: FxHashMap::default(),
                    a11y: None,
                    drag_binding: None,
                }))),
                styles: vec![],
                ..Default::default()
            },
        );
        let ir = make_ir(components);
        let comp_ref = ir
            .modules
            .get("main")
            .unwrap()
            .components
            .get("Card")
            .unwrap();
        let sfc =
            codegen.gen_component_sfc("Card", comp_ref, &ir, &Rc::new(RefCell::new(Vec::new())));
        // Should contain both static class and :class binding
        assert!(sfc.contains("class=\"base\""));
        assert!(sfc.contains(":class=\"{ active: isOpen }\""));
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
            is_mut: false,
        };
        generate_statement_vue(&mut w, &stmt, &Rc::new(RefCell::new(Vec::new())));
        assert_eq!(w.into_string(), "const x = 42;\n");
    }

    /// Task 5.1 coordination fix: Vue previously emitted `const` for EVERY
    /// binding (`use_const_for_immutable` was unconditionally true), so a
    /// `let mut` var became `const` — the same wrong-code bug as the React
    /// BUG-A. The shared trait now branches on the IR `is_mut` flag.
    #[test]
    fn test_vue_stmt_let_mut() {
        let mut w = CodeWriter::new();
        let stmt = StatementIR::LetStmtIR {
            name: "x".into(),
            value: Some(ExprIR::LiteralExpr {
                value: serde_json::json!(42),
            }),
            r#type: None,
            is_mut: true,
        };
        generate_statement_vue(&mut w, &stmt, &Rc::new(RefCell::new(Vec::new())));
        assert_eq!(w.into_string(), "let x = 42;\n");
    }

    #[test]
    fn test_vue_stmt_return() {
        let mut w = CodeWriter::new();
        let stmt = StatementIR::ReturnStmtIR {
            value: Some(ExprIR::IdentifierExpr {
                name: "result".into(),
                type_ann: None,
            }),
        };
        generate_statement_vue(&mut w, &stmt, &Rc::new(RefCell::new(Vec::new())));
        assert_eq!(w.into_string(), "return result;\n");
    }

    #[test]
    fn test_vue_stmt_mutation_increment() {
        let mut w = CodeWriter::new();
        let stmt = StatementIR::MutationStmt {
            intent: Some(MutationIntent::Increment {
                target: SignalRefIR {
                    name: "count".into(),
                },

                amount: Some(Box::new(ExprIR::LiteralExpr {
                    value: serde_json::json!(1),
                })),
            }),
        };
        generate_statement_vue(&mut w, &stmt, &Rc::new(RefCell::new(Vec::new())));
        assert_eq!(w.into_string(), "count.value += 1;\n");
    }

    #[test]
    fn test_vue_stmt_mutation_assign() {
        let mut w = CodeWriter::new();
        let stmt = StatementIR::MutationStmt {
            intent: Some(MutationIntent::DirectAssign {
                target: SignalRefIR {
                    name: "name".into(),
                },

                value: Some(Box::new(ExprIR::LiteralExpr {
                    value: serde_json::json!("alice"),
                })),
                field_object: None,
            }),
        };
        generate_statement_vue(&mut w, &stmt, &Rc::new(RefCell::new(Vec::new())));
        assert_eq!(w.into_string(), "name.value = \"alice\";\n");
    }

    #[test]
    fn test_vue_stmt_if() {
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
                    is_mut: false,
                }],
            },
            else_ifs: vec![],
            else_body: None,
        };
        generate_statement_vue(&mut w, &stmt, &Rc::new(RefCell::new(Vec::new())));
        let out = w.into_string();
        assert!(out.contains("if (cond) {"));
        assert!(out.contains("const x = 1;"));
        assert!(out.contains("}"));
    }

    // ── gen_component_sfc tests ──────────────────────────────

    /// Build IRProgram with a single module containing the given components.
    fn make_ir(components: FxHashMap<String, ComponentIR>) -> IRProgram {
        IRProgram {
            version: "1.0".into(),
            modules: FxHashMap::from_iter([(
                "main".to_string(),
                ModuleIR {
                    name: "main".to_string(),
                    components,
                    ..Default::default()
                },
            )]),
            stdlib_imports: Default::default(),
            source_map: FxHashMap::default(),
            motion_index: FxHashMap::default(),
            source_span: None,
        }
    }

    #[test]
    fn test_vue_empty_component() {
        let codegen = VueCodegen;
        let mut components = FxHashMap::default();
        components.insert(
            "App".into(),
            ComponentIR {
                form_intent: None,
                load_intent: None,
                name: "App".into(),
                props: FxHashMap::default(),
                prop_defaults: FxHashMap::default(),
                signals: vec![],
                computeds: vec![],
                effects: vec![],
                lifecycles: vec![],
                fns: vec![],
                store_refs: vec![],
                refs: vec![],
                events: vec![],
                view: None,
                styles: vec![],
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
        let sfc =
            codegen.gen_component_sfc("App", comp_ref, &ir, &Rc::new(RefCell::new(Vec::new())));
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
        let state = SignalIntent {
            name: "count".into(),
            r#type: TypeIR {
                kind: "i32".into(),
                type_args: vec![],
                is_nullable: false,
            },
            initial_value: ExprIR::LiteralExpr {
                value: serde_json::json!(0),
            },
        };
        let mut components = FxHashMap::default();
        components.insert(
            "Counter".into(),
            ComponentIR {
                form_intent: None,
                load_intent: None,
                name: "Counter".into(),
                props: FxHashMap::default(),
                prop_defaults: FxHashMap::default(),
                signals: vec![state],
                computeds: vec![],
                effects: vec![],
                lifecycles: vec![],
                fns: vec![],
                store_refs: vec![],
                refs: vec![],
                events: vec![],
                view: None,
                styles: vec![],
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
        let sfc =
            codegen.gen_component_sfc("Counter", comp_ref, &ir, &Rc::new(RefCell::new(Vec::new())));
        assert!(sfc.contains("const count = ref<number>(0);"));
        assert!(sfc.contains("<template>"));
        assert!(sfc.contains("</template>"));
        assert!(sfc.contains("<script setup lang=\"ts\">"));
        assert!(sfc.contains("</script>"));
    }

    #[test]
    fn test_vue_component_with_props() {
        let codegen = VueCodegen;
        let mut props = FxHashMap::default();
        props.insert(
            "name".into(),
            TypeIR {
                kind: "string".into(),
                type_args: vec![],
                is_nullable: false,
            },
        );
        props.insert(
            "age".into(),
            TypeIR {
                kind: "i32".into(),
                type_args: vec![],
                is_nullable: false,
            },
        );
        let mut components = FxHashMap::default();
        components.insert(
            "Greeting".into(),
            ComponentIR {
                form_intent: None,
                load_intent: None,
                name: "Greeting".into(),
                props,
                prop_defaults: FxHashMap::default(),
                signals: vec![],
                computeds: vec![],
                effects: vec![],
                lifecycles: vec![],
                fns: vec![],
                store_refs: vec![],
                refs: vec![],
                events: vec![],
                view: None,
                styles: vec![],
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
        let sfc = codegen.gen_component_sfc(
            "Greeting",
            comp_ref,
            &ir,
            &Rc::new(RefCell::new(Vec::new())),
        );
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
                        type_ann: None,
                    },
                }],
            },
        };
        let mut components = FxHashMap::default();
        components.insert(
            "Watcher".into(),
            ComponentIR {
                form_intent: None,
                load_intent: None,
                name: "Watcher".into(),
                props: FxHashMap::default(),
                prop_defaults: FxHashMap::default(),
                signals: vec![],
                computeds: vec![],
                effects: vec![effect],
                lifecycles: vec![],
                fns: vec![],
                store_refs: vec![],
                refs: vec![],
                events: vec![],
                view: None,
                styles: vec![],
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
        let sfc =
            codegen.gen_component_sfc("Watcher", comp_ref, &ir, &Rc::new(RefCell::new(Vec::new())));
        assert!(sfc.contains("watch([count], () => {"));
        assert!(sfc.contains("import { ref, watch } from 'vue';"));
    }

    #[test]
    fn test_vue_component_with_lifecycle_mount() {
        let codegen = VueCodegen;
        let lc = LifecycleIntent {
            kind: LifecycleKind::Mount,
            body: BlockIR { statements: vec![] },
        };
        let mut components = FxHashMap::default();
        components.insert(
            "App".into(),
            ComponentIR {
                form_intent: None,
                load_intent: None,
                name: "App".into(),
                props: FxHashMap::default(),
                prop_defaults: FxHashMap::default(),
                signals: vec![],
                computeds: vec![],
                effects: vec![],
                lifecycles: vec![lc],
                fns: vec![],
                store_refs: vec![],
                refs: vec![],
                events: vec![],
                view: None,
                styles: vec![],
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
        let sfc =
            codegen.gen_component_sfc("App", comp_ref, &ir, &Rc::new(RefCell::new(Vec::new())));
        assert!(sfc.contains("onMounted(() => {"));
        assert!(sfc.contains("import { ref, onMounted } from 'vue';"));
    }

    // ── gen_store_hook tests ─────────────────────────────────

    #[test]
    fn test_vue_store_hook() {
        let codegen = VueCodegen;
        let store = StoreIR {
            name: "AppStore".into(),
            signals: FxHashMap::default(),
            signal_defaults: FxHashMap::default(),
            getters: FxHashMap::default(),
            actions: vec![],
            ..Default::default()
        };
        let hook = codegen.gen_store_hook("AppStore", &store, &Rc::new(RefCell::new(Vec::new())));
        assert!(hook.contains("export function useAppStore() {"));
        assert!(hook.contains("import { ref, computed } from 'vue';"));
    }

    #[test]
    fn test_vue_store_hook_with_signals() {
        let codegen = VueCodegen;
        let mut signals = FxHashMap::default();
        signals.insert(
            "count".into(),
            TypeIR {
                kind: "i32".into(),
                type_args: vec![],
                is_nullable: false,
            },
        );
        signals.insert(
            "name".into(),
            TypeIR {
                kind: "string".into(),
                type_args: vec![],
                is_nullable: false,
            },
        );
        let mut defaults = FxHashMap::default();
        defaults.insert("count".into(), serde_json::json!(0));
        defaults.insert("name".into(), serde_json::json!("world"));
        let store = StoreIR {
            name: "AppStore".into(),
            signals,
            signal_defaults: defaults,
            getters: FxHashMap::default(),
            actions: vec![],
            ..Default::default()
        };
        let hook = codegen.gen_store_hook("AppStore", &store, &Rc::new(RefCell::new(Vec::new())));
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
        let mut getters = FxHashMap::default();
        getters.insert(
            "doubleCount".into(),
            ExprIR::BinaryOpExpr {
                op: BinaryOpIR::Mul,
                left: Box::new(ExprIR::IdentifierExpr {
                    name: "count".into(),
                    type_ann: None,
                }),
                right: Box::new(ExprIR::LiteralExpr {
                    value: serde_json::json!(2),
                }),
            },
        );
        let mut signals = FxHashMap::default();
        signals.insert(
            "count".into(),
            TypeIR {
                kind: "i32".into(),
                type_args: vec![],
                is_nullable: false,
            },
        );
        let mut defaults = FxHashMap::default();
        defaults.insert("count".into(), serde_json::json!(0));
        let store = StoreIR {
            name: "CalcStore".into(),
            signals,
            signal_defaults: defaults,
            getters,
            actions: vec![],
            ..Default::default()
        };
        let hook = codegen.gen_store_hook("CalcStore", &store, &Rc::new(RefCell::new(Vec::new())));
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
            modules: FxHashMap::from_iter([(
                "main".to_string(),
                ModuleIR {
                    name: "main".to_string(),
                    components: FxHashMap::default(),
                    stores: FxHashMap::default(),
                    ..Default::default()
                },
            )]),
            source_map: FxHashMap::default(),
            motion_index: FxHashMap::default(),
            stdlib_imports: Default::default(),
            source_span: None,
        };
        let sources = FxHashMap::from_iter([("main.omni".into(), "".into())]);
        let output = codegen.generate(&ir, &sources);
        assert!(output.errors.is_empty());
        assert!(output.files.is_empty());
    }

    #[test]
    fn test_full_pipeline_component() {
        let codegen = VueCodegen;
        let comp = ComponentIR {
            form_intent: None,
            load_intent: None,
            name: "App".into(),
            props: FxHashMap::default(),
            prop_defaults: FxHashMap::default(),
            signals: vec![SignalIntent {
                name: "count".into(),
                r#type: TypeIR {
                    kind: "i32".into(),
                    type_args: vec![],
                    is_nullable: false,
                },
                initial_value: ExprIR::LiteralExpr {
                    value: serde_json::json!(0),
                },
            }],
            computeds: vec![],
            effects: vec![],
            lifecycles: vec![],
            fns: vec![],
            store_refs: vec![],
            refs: vec![],
            events: vec![],
            view: None,
            styles: vec![],
            ..Default::default()
        };
        let ir = IRProgram {
            version: "0.1".into(),
            modules: FxHashMap::from_iter([(
                "main".to_string(),
                ModuleIR {
                    name: "main".to_string(),
                    components: FxHashMap::from_iter([("App".into(), comp)]),
                    stores: FxHashMap::default(),
                    ..Default::default()
                },
            )]),
            source_map: FxHashMap::default(),
            motion_index: FxHashMap::default(),
            stdlib_imports: Default::default(),
            source_span: None,
        };
        let sources = FxHashMap::from_iter([("main.omni".into(), "".into())]);
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
            signals: FxHashMap::from_iter([(
                "count".into(),
                TypeIR {
                    kind: "i32".into(),
                    type_args: vec![],
                    is_nullable: false,
                },
            )]),
            signal_defaults: FxHashMap::from_iter([("count".into(), serde_json::json!(0))]),
            getters: FxHashMap::default(),
            actions: vec![],
            ..Default::default()
        };
        let ir = IRProgram {
            version: "0.1".into(),
            modules: FxHashMap::from_iter([(
                "main".to_string(),
                ModuleIR {
                    name: "main".to_string(),
                    components: FxHashMap::default(),
                    stores: FxHashMap::from_iter([("AppStore".into(), store)]),
                    ..Default::default()
                },
            )]),
            source_map: FxHashMap::default(),
            motion_index: FxHashMap::default(),
            stdlib_imports: Default::default(),
            source_span: None,
        };
        let sources = FxHashMap::from_iter([("main.omni".into(), "".into())]);
        let output = codegen.generate(&ir, &sources);
        assert!(output.errors.is_empty());
        assert_eq!(output.files.len(), 1); // just useAppStore.ts
        let hook = &output.files[0];
        assert_eq!(hook.path, "useAppStore.ts");
        insta::with_settings!({snapshot_path => "snapshots"}, {
            insta::assert_snapshot!("test_full_pipeline_with_store", &hook.content);
        });
    }

    // ── @for list rendering tests ──────────────────────────

    #[test]
    fn test_vue_list_rendering_basic() {
        let mut w = CodeWriter::new();
        let fmt = VueFormatExpr::default();
        let list = ListRenderingIntent {
            iterable: ExprIR::IdentifierExpr {
                name: "items".into(),
                type_ann: None,
            },
            item_binding: BindingIR {
                name: "item".into(),
                r#type: None,
            },
            body: Some(ViewTree::Text("Item".into())),
            key_expr: None,
            empty_body: None,
            scroll_hint: false,
        };
        crate::codegen::vue::gen_template_node(
            &mut w,
            &ViewTree::ListRendering(Box::new(list)),
            &fmt,
        );
        let out = w.into_string();
        // Should emit v-for without v-if guard
        assert!(out.contains("v-for=\"(item, index) in items\""));
        assert!(!out.contains("v-if"));
        assert!(!out.contains("v-else"));
        assert!(out.contains("Item"));
    }

    #[test]
    fn test_vue_list_rendering_with_empty_body() {
        let mut w = CodeWriter::new();
        let fmt = VueFormatExpr::default();
        let list = ListRenderingIntent {
            iterable: ExprIR::IdentifierExpr {
                name: "items".into(),
                type_ann: None,
            },
            item_binding: BindingIR {
                name: "item".into(),
                r#type: None,
            },
            body: Some(ViewTree::Text("Item".into())),
            key_expr: None,
            empty_body: Some(ViewTree::Text("Nothing".into())),
            scroll_hint: false,
        };
        crate::codegen::vue::gen_template_node(
            &mut w,
            &ViewTree::ListRendering(Box::new(list)),
            &fmt,
        );
        let out = w.into_string();
        // Should wrap v-for in v-if/v-else conditional
        assert!(out.contains("v-if=\"items.length > 0\""));
        assert!(out.contains("v-for=\"(item, index) in items\""));
        assert!(out.contains("v-else"));
        assert!(out.contains("Nothing"));
        assert!(out.contains("Item"));
    }

    // ── Dangerous URL sanitization tests ──────────────────────

    #[test]
    fn test_vue_dangerous_url_neutralized() {
        let mut w = CodeWriter::new();
        let fmt = VueFormatExpr::default();
        let el = ElementIR {
            tag: "a".into(),
            props: FxHashMap::from_iter([(
                "href".into(),
                ExprIR::LiteralExpr {
                    value: serde_json::json!("javascript:alert(1)"),
                },
            )]),
            bindings: FxHashMap::default(),
            events: FxHashMap::default(),
            style_classes: None,
            inline_style: None,
            children: vec![],
            class_map_bindings: FxHashMap::default(),
            motion: None,
            ref_binding: None,
            construction_kind: ConstructionKind::Static,
            source_span: None,
            semantic_bindings: FxHashMap::default(),
            a11y: None,
            drag_binding: None,
        };
        crate::codegen::vue::gen_template_node(&mut w, &ViewTree::Element(Box::new(el)), &fmt);
        let out = w.into_string();
        // Dangerous javascript: URL should be neutralized to empty string
        assert!(out.contains(r#"href="""#));
        assert!(!out.contains("javascript"));
    }

    #[test]
    fn test_vue_safe_url_passes_through() {
        let mut w = CodeWriter::new();
        let fmt = VueFormatExpr::default();
        let el = ElementIR {
            tag: "a".into(),
            props: FxHashMap::from_iter([(
                "href".into(),
                ExprIR::LiteralExpr {
                    value: serde_json::json!("https://example.com"),
                },
            )]),
            bindings: FxHashMap::default(),
            events: FxHashMap::default(),
            style_classes: None,
            inline_style: None,
            children: vec![],
            class_map_bindings: FxHashMap::default(),
            motion: None,
            ref_binding: None,
            construction_kind: ConstructionKind::Static,
            source_span: None,
            semantic_bindings: FxHashMap::default(),
            a11y: None,
            drag_binding: None,
        };
        crate::codegen::vue::gen_template_node(&mut w, &ViewTree::Element(Box::new(el)), &fmt);
        let out = w.into_string();
        // Safe URL should pass through unchanged
        assert!(out.contains(r#"href="https://example.com""#));
    }

    #[test]
    fn test_vue_src_attr_sanitized() {
        let mut w = CodeWriter::new();
        let fmt = VueFormatExpr::default();
        let el = ElementIR {
            tag: "img".into(),
            props: FxHashMap::from_iter([(
                "src".into(),
                ExprIR::LiteralExpr {
                    value: serde_json::json!("data:text/html,<script>alert(1)</script>"),
                },
            )]),
            bindings: FxHashMap::default(),
            events: FxHashMap::default(),
            style_classes: None,
            inline_style: None,
            children: vec![],
            class_map_bindings: FxHashMap::default(),
            motion: None,
            ref_binding: None,
            construction_kind: ConstructionKind::Static,
            source_span: None,
            semantic_bindings: FxHashMap::default(),
            a11y: None,
            drag_binding: None,
        };
        crate::codegen::vue::gen_template_node(&mut w, &ViewTree::Element(Box::new(el)), &fmt);
        let out = w.into_string();
        // data: URL in src should be neutralized
        assert!(out.contains(r#"src="""#));
        assert!(!out.contains("data:"));
    }

    #[test]
    fn test_vue_non_url_attr_not_affected() {
        let mut w = CodeWriter::new();
        let fmt = VueFormatExpr::default();
        let el = ElementIR {
            tag: "div".into(),
            props: FxHashMap::from_iter([(
                "title".into(),
                ExprIR::LiteralExpr {
                    value: serde_json::json!("javascript:not-a-url"),
                },
            )]),
            bindings: FxHashMap::default(),
            events: FxHashMap::default(),
            style_classes: None,
            inline_style: None,
            children: vec![],
            class_map_bindings: FxHashMap::default(),
            motion: None,
            ref_binding: None,
            construction_kind: ConstructionKind::Static,
            source_span: None,
            semantic_bindings: FxHashMap::default(),
            a11y: None,
            drag_binding: None,
        };
        crate::codegen::vue::gen_template_node(&mut w, &ViewTree::Element(Box::new(el)), &fmt);
        let out = w.into_string();
        // Non-URL attributes should not be sanitized
        assert!(out.contains(r#"title="javascript:not-a-url""#));
    }

    #[test]
    fn test_full_pipeline_component_with_store_ref() {
        let codegen = VueCodegen;
        let store = StoreIR {
            name: "AppStore".into(),
            signals: FxHashMap::from_iter([(
                "count".into(),
                TypeIR {
                    kind: "i32".into(),
                    type_args: vec![],
                    is_nullable: false,
                },
            )]),
            signal_defaults: FxHashMap::from_iter([("count".into(), serde_json::json!(0))]),
            getters: FxHashMap::default(),
            actions: vec![],
            ..Default::default()
        };
        let comp = ComponentIR {
            form_intent: None,
            load_intent: None,
            name: "App".into(),
            props: FxHashMap::default(),
            prop_defaults: FxHashMap::default(),
            signals: vec![],
            computeds: vec![],
            effects: vec![],
            lifecycles: vec![],
            fns: vec![],
            store_refs: vec!["AppStore".into()],
            refs: vec![],
            events: vec![],
            view: None,
            styles: vec![],
            ..Default::default()
        };
        let ir = IRProgram {
            version: "0.1".into(),
            modules: FxHashMap::from_iter([(
                "main".to_string(),
                ModuleIR {
                    name: "main".to_string(),
                    components: FxHashMap::from_iter([("App".into(), comp)]),
                    stores: FxHashMap::from_iter([("AppStore".into(), store)]),
                    ..Default::default()
                },
            )]),
            source_map: FxHashMap::default(),
            motion_index: FxHashMap::default(),
            stdlib_imports: Default::default(),
            source_span: None,
        };
        let sources = FxHashMap::from_iter([("main.omni".into(), "".into())]);
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
