//! Unit tests for OmniView codegen.
//!
//! Tests cover expression formatting, statement generation, component/store emission,
//! HTML generation, and full pipeline integration. The OmniView codegen produces
//! HTML, CSS, and JS from an IRProgram.

use super::*;
use rustc_hash::FxHashMap;

// ── Helpers ─────────────────────────────────────────

fn make_element(tag: &str) -> ElementIR {
    ElementIR {
        tag: tag.into(),
        props: FxHashMap::default(),
        bindings: FxHashMap::default(),
        events: FxHashMap::default(),
        style_classes: None,
        inline_style: None,
        children: vec![],
        test_id: None,
        ref_name: None,
        construction_kind: ConstructionKind::Static,
        class_map_bindings: FxHashMap::default(),
        source_span: None,
        motion: None,
        scroll: None,
        drag: None,
        hero: None,
    }
}

fn make_module(name: &str) -> ModuleIR {
    ModuleIR {
        name: name.into(),
        functions: FxHashMap::default(),
        structs: FxHashMap::default(),
        enums: FxHashMap::default(),
        type_aliases: FxHashMap::default(),
        components: FxHashMap::default(),
        stores: FxHashMap::default(),
        themes: vec![],
        imports: vec![],
        exports: vec![],
        designs: vec![],
        motions: vec![],
        timelines: vec![],
        source_span: None,
    }
}

fn make_component(name: &str) -> ComponentIR {
    ComponentIR {
        name: name.into(),
        source_file: None,
        generic_params: vec![],
        props: FxHashMap::default(),
        prop_defaults: FxHashMap::default(),
        signals: vec![],
        computeds: vec![],
        effects: vec![],
        lifecycles: vec![],
        fns: vec![],
        store_refs: vec![],
        refs: vec![],
        event_names: vec![],
        view: None,
        styles: vec![],
        if_mode_default: None,
        hero_defs: vec![],
        source_span: None,
    }
}

fn make_program(modules: FxHashMap<String, ModuleIR>) -> IRProgram {
    IRProgram {
        version: "0.1".into(),
        modules,
        source_map: FxHashMap::default(),
        stdlib_imports: Default::default(),
        source_span: None,
    }
}

// ── wrap_debug_html ──────────────────────────────────

#[test]
fn test_wrap_debug_html() {
    let html = wrap_debug_html(
        "/// OmniView Native Runtime v2 — reactive signal system\nconsole.log('hi');",
        "body { color: red; }",
        "<div>Hello</div>",
    );
    assert!(html.contains("<!DOCTYPE html>"));
    assert!(html.contains("OmniView Debug"));
    assert!(html.contains("console.log('hi');"));
    assert!(html.contains("body { color: red; }"));
    assert!(html.contains("<div>Hello</div>"));
    assert!(html.contains("OmniView Native Runtime v2"));
}

// ── Store Generation ────────────────────────────────

#[test]
fn test_store_signals_with_defaults() {
    let mut module = make_module("main");
    module.stores.insert(
        "AppStore".to_string(),
        StoreIR {
            name: "AppStore".to_string(),
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
        },
    );
    let ir = make_program(FxHashMap::from_iter([("main".into(), module)]));
    let output = generate(&ir, false, false, false);
    assert!(output.js.contains("AppStore_count"));
    assert!(output.js.contains("__omni_signal"));
    assert!(output.js.contains("0"));
    assert!(output.js.contains("world"));
}

#[test]
fn test_store_getters() {
    let mut module = make_module("main");
    module.stores.insert(
        "CalcStore".to_string(),
        StoreIR {
            name: "CalcStore".to_string(),
            signals: FxHashMap::from_iter([(
                "x".into(),
                TypeIR {
                    kind: "i32".into(),
                    type_args: vec![],
                    is_nullable: false,
                },
            )]),
            signal_defaults: FxHashMap::from_iter([("x".into(), serde_json::json!(10))]),
            getters: FxHashMap::from_iter([(
                "doubled".into(),
                ExprIR::BinaryOpExpr {
                    op: BinaryOpIR::Mul,
                    left: Box::new(ExprIR::IdentifierExpr {
                        name: "x".into(),
                        type_ann: None,
                    }),
                    right: Box::new(ExprIR::LiteralExpr {
                        value: serde_json::json!(2),
                    }),
                },
            )]),
            actions: vec![],
            ..Default::default()
        },
    );
    let ir = make_program(FxHashMap::from_iter([("main".into(), module)]));
    let output = generate(&ir, false, false, false);
    assert!(output.js.contains("CalcStore_doubled"));
    assert!(output.js.contains("__omni_computed"));
    assert!(output.js.contains("CalcStore_x.value * 2"));
}

#[test]
fn test_store_actions() {
    let mut module = make_module("main");
    module.stores.insert(
        "TodoStore".to_string(),
        StoreIR {
            name: "TodoStore".to_string(),
            signals: FxHashMap::from_iter([(
                "items".into(),
                TypeIR {
                    kind: "Vec".into(),
                    type_args: vec![],
                    is_nullable: false,
                },
            )]),
            signal_defaults: FxHashMap::from_iter([("items".into(), serde_json::json!([]))]),
            getters: FxHashMap::default(),
            actions: vec![FunctionIR {
                name: "addItem".into(),
                visibility: Visibility::Public,
                params: vec![ParamIR {
                    name: "item".into(),
                    r#type: None,
                }],
                return_type: None,
                body: BlockIR {
                    statements: vec![StatementIR::ExprStmtIR {
                        expr: ExprIR::LiteralExpr {
                            value: serde_json::json!("ok"),
                        },
                    }],
                },
                is_async: false,

                source_span: None,
            }],
            ..Default::default()
        },
    );
    let ir = make_program(FxHashMap::from_iter([("main".into(), module)]));
    let output = generate(&ir, false, false, false);
    assert!(output.js.contains("function TodoStore_addItem"));
    assert!(output.js.contains("item"));
}

// ── Expression Formatting ────────────────────────────

#[test]
fn test_expr_literal() {
    let gen = OmniViewGenerator::test_new();
    assert_eq!(
        gen.expr_to_js(&ExprIR::LiteralExpr {
            value: serde_json::json!(42)
        }),
        "42"
    );
}

#[test]
fn test_expr_literal_string() {
    let gen = OmniViewGenerator::test_new();
    assert_eq!(
        gen.expr_to_js(&ExprIR::LiteralExpr {
            value: serde_json::json!("hello")
        }),
        "\"hello\""
    );
    assert_eq!(
        gen.expr_to_js(&ExprIR::LiteralExpr {
            value: serde_json::Value::Null
        }),
        "null"
    );
    assert_eq!(
        gen.expr_to_js(&ExprIR::LiteralExpr {
            value: serde_json::json!(true)
        }),
        "true"
    );
}

#[test]
fn test_expr_identifier() {
    let gen = OmniViewGenerator::test_new();
    assert_eq!(
        gen.expr_to_js(&ExprIR::IdentifierExpr {
            name: "count".into(),
            type_ann: None,
        }),
        "count"
    );
}

#[test]
fn test_expr_binary() {
    let gen = OmniViewGenerator::test_new();
    assert_eq!(
        gen.expr_to_js(&ExprIR::BinaryOpExpr {
            op: BinaryOpIR::Add,
            left: Box::new(ExprIR::IdentifierExpr {
                name: "a".into(),
                type_ann: None
            }),
            right: Box::new(ExprIR::IdentifierExpr {
                name: "b".into(),
                type_ann: None
            })
        }),
        "a + b"
    );
    assert_eq!(
        gen.expr_to_js(&ExprIR::BinaryOpExpr {
            op: BinaryOpIR::Eq,
            left: Box::new(ExprIR::IdentifierExpr {
                name: "x".into(),
                type_ann: None
            }),
            right: Box::new(ExprIR::IdentifierExpr {
                name: "y".into(),
                type_ann: None
            })
        }),
        "x === y"
    );
    assert_eq!(
        gen.expr_to_js(&ExprIR::BinaryOpExpr {
            op: BinaryOpIR::And,
            left: Box::new(ExprIR::IdentifierExpr {
                name: "a".into(),
                type_ann: None
            }),
            right: Box::new(ExprIR::IdentifierExpr {
                name: "b".into(),
                type_ann: None
            })
        }),
        "a && b"
    );
}

#[test]
fn test_expr_unary() {
    let gen = OmniViewGenerator::test_new();
    assert_eq!(
        gen.expr_to_js(&ExprIR::UnaryOpExpr {
            op: UnaryOpIR::Not,
            operand: Box::new(ExprIR::IdentifierExpr {
                name: "flag".into(),
                type_ann: None,
            })
        }),
        "!flag"
    );
    assert_eq!(
        gen.expr_to_js(&ExprIR::UnaryOpExpr {
            op: UnaryOpIR::Neg,
            operand: Box::new(ExprIR::IdentifierExpr {
                name: "x".into(),
                type_ann: None
            })
        }),
        "-x"
    );
}

#[test]
fn test_expr_ternary() {
    let gen = OmniViewGenerator::test_new();
    assert_eq!(
        gen.expr_to_js(&ExprIR::TernaryIR {
            cond: Box::new(ExprIR::IdentifierExpr {
                name: "x".into(),
                type_ann: None
            }),
            then_expr: Box::new(ExprIR::LiteralExpr {
                value: serde_json::json!(1)
            }),
            else_expr: Box::new(ExprIR::LiteralExpr {
                value: serde_json::json!(0)
            }),
        }),
        "x ? 1 : 0"
    );
}

#[test]
fn test_expr_field_access() {
    let gen = OmniViewGenerator::test_new();
    assert_eq!(
        gen.expr_to_js(&ExprIR::FieldAccessExpr {
            object: Box::new(ExprIR::IdentifierExpr {
                name: "obj".into(),
                type_ann: None
            }),
            field: "prop".into()
        }),
        "obj.prop"
    );
}

#[test]
fn test_expr_method_call() {
    let gen = OmniViewGenerator::test_new();
    assert_eq!(
        gen.expr_to_js(&ExprIR::MethodCallExpr {
            object: Box::new(ExprIR::IdentifierExpr {
                name: "arr".into(),
                type_ann: None
            }),
            method: "len".into(),
            args: vec![],
            is_optional: false
        }),
        "(arr || []).length"
    );
    assert_eq!(
        gen.expr_to_js(&ExprIR::MethodCallExpr {
            object: Box::new(ExprIR::IdentifierExpr {
                name: "val".into(),
                type_ann: None
            }),
            method: "to_string".into(),
            args: vec![],
            is_optional: false
        }),
        "String(val)"
    );
}

#[test]
fn test_expr_closure() {
    let gen = OmniViewGenerator::test_new();
    assert_eq!(
        gen.expr_to_js(&ExprIR::ClosureExpr {
            params: vec!["x".into()],
            body: Box::new(ExprIR::BinaryOpExpr {
                op: BinaryOpIR::Mul,
                left: Box::new(ExprIR::IdentifierExpr {
                    name: "x".into(),
                    type_ann: None
                }),
                right: Box::new(ExprIR::LiteralExpr {
                    value: serde_json::json!(2)
                }),
            }),
        }),
        "((x) => x * 2)"
    );
}

#[test]
fn test_expr_array() {
    let gen = OmniViewGenerator::test_new();
    assert_eq!(
        gen.expr_to_js(&ExprIR::ArrayExpr {
            elements: vec![
                ExprIR::LiteralExpr {
                    value: serde_json::json!(1)
                },
                ExprIR::LiteralExpr {
                    value: serde_json::json!(2)
                },
                ExprIR::LiteralExpr {
                    value: serde_json::json!(3)
                },
            ],
        }),
        "[1, 2, 3]"
    );
}

#[test]
fn test_expr_store_field_access() {
    let mut gen = OmniViewGenerator::test_new();
    gen.store_names.insert("AppStore".into());
    let js = gen.expr_to_js(&ExprIR::StoreFieldAccessExpr {
        store_alias: "AppStore".into(),
        field_name: "count".into(),
        call_args: vec![],
    });
    assert!(js.contains("AppStore_count"));
}

#[test]
fn test_expr_block() {
    let gen = OmniViewGenerator::test_new();
    let js = gen.expr_to_js(&ExprIR::BlockExpr {
        statements: vec![StatementIR::LetStmtIR {
            name: "x".into(),
            value: Some(ExprIR::LiteralExpr {
                value: serde_json::json!(42),
            }),
            r#type: None,
        }],
        tail: Box::new(ExprIR::IdentifierExpr {
            name: "x".into(),
            type_ann: None,
        }),
    });
    assert!(js.contains("function"));
    assert!(js.contains("let x = 42"));
    assert!(js.contains("return x"));
}

#[test]
fn test_expr_variant_literal() {
    let gen = OmniViewGenerator::test_new();
    let js = gen.expr_to_js(&ExprIR::VariantLiteral {
        union_type: "Result".into(),
        variant: "ok".into(),
        fields: vec![(
            "value".into(),
            ExprIR::LiteralExpr {
                value: serde_json::json!(42),
            },
        )],
    });
    assert!(js.contains("\"_variant\": \"ok\""));
    assert!(js.contains("\"value\": 42"));
}

// ── Statement Generation ────────────────────────────

#[test]
fn test_stmt_let() {
    let mut gen = OmniViewGenerator::test_new();
    gen.emit_statement(&StatementIR::LetStmtIR {
        name: "x".into(),
        value: Some(ExprIR::LiteralExpr {
            value: serde_json::json!(42),
        }),
        r#type: None,
    });
    assert!(gen.js.contains("let x = 42"));
}

#[test]
fn test_stmt_return() {
    let mut gen = OmniViewGenerator::test_new();
    gen.emit_statement(&StatementIR::ReturnStmtIR {
        value: Some(ExprIR::IdentifierExpr {
            name: "result".into(),
            type_ann: None,
        }),
    });
    assert!(gen.js.contains("return result"));
}

#[test]
fn test_stmt_if_else() {
    let mut gen = OmniViewGenerator::test_new();
    gen.emit_statement(&StatementIR::IfStmtIR {
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
                name: "y".into(),
                value: Some(ExprIR::LiteralExpr {
                    value: serde_json::json!(2),
                }),
                r#type: None,
            }],
        }),
    });
    assert!(gen.js.contains("if (cond)"));
    assert!(gen.js.contains("let x = 1"));
    assert!(gen.js.contains("} else {"));
    assert!(gen.js.contains("let y = 2"));
}

#[test]
fn test_stmt_while() {
    let mut gen = OmniViewGenerator::test_new();
    gen.emit_statement(&StatementIR::WhileStmtIR {
        condition: ExprIR::IdentifierExpr {
            name: "running".into(),
            type_ann: None,
        },
        body: BlockIR {
            statements: vec![StatementIR::BreakStmtIR],
        },
    });
    assert!(gen.js.contains("while (running)"));
    assert!(gen.js.contains("break"));
}

#[test]
fn test_stmt_for_in() {
    let mut gen = OmniViewGenerator::test_new();
    gen.emit_statement(&StatementIR::ForInStmtIR {
        var_name: "item".into(),
        iterable: ExprIR::IdentifierExpr {
            name: "items".into(),
            type_ann: None,
        },
        body: BlockIR {
            statements: vec![StatementIR::ExprStmtIR {
                expr: ExprIR::CallExpr {
                    name: "println".into(),
                    args: vec![ExprIR::IdentifierExpr {
                        name: "item".into(),
                        type_ann: None,
                    }],
                },
            }],
        },
    });
    assert!(gen.js.contains("for (const item of items)"));
    assert!(gen.js.contains("console.log(item)"));
}

#[test]
fn test_stmt_mutation_assign() {
    let mut gen = OmniViewGenerator::test_new();
    gen.emit_statement(&StatementIR::MutationStmt {
        intent: Some(MutationIntent::DirectAssign {
            target: SignalRefIR {
                owner: None,
                name: "count".into(),
            },

            value: Some(Box::new(ExprIR::LiteralExpr {
                value: serde_json::json!(42),
            })),
        }),
    });
    assert!(gen.js.contains("count = 42"));
}

#[test]
fn test_stmt_mutation_increment() {
    let mut gen = OmniViewGenerator::test_new();
    gen.emit_statement(&StatementIR::MutationStmt {
        intent: Some(MutationIntent::Increment {
            target: SignalRefIR {
                owner: None,
                name: "count".into(),
            },

            amount: Some(Box::new(ExprIR::LiteralExpr {
                value: serde_json::json!(1),
            })),
        }),
    });
    assert!(gen.js.contains("count += 1"));
}

#[test]
fn test_stmt_mutation_signal_set() {
    let mut gen = OmniViewGenerator::test_new();
    gen.signal_names.insert("App_count".into());
    gen.name_renames.insert("count".into(), "App_count".into());
    gen.emit_statement(&StatementIR::MutationStmt {
        intent: Some(MutationIntent::DirectAssign {
            target: SignalRefIR {
                owner: None,
                name: "count".into(),
            },

            value: Some(Box::new(ExprIR::LiteralExpr {
                value: serde_json::json!(99),
            })),
        }),
    });
    assert!(gen.js.contains(".set(99)"));
}

#[test]
fn test_stmt_try_catch() {
    let mut gen = OmniViewGenerator::test_new();
    gen.emit_statement(&StatementIR::TryCatchStmtIR {
        try_body: BlockIR {
            statements: vec![StatementIR::ExprStmtIR {
                expr: ExprIR::CallExpr {
                    name: "riskyOp".into(),
                    args: vec![],
                },
            }],
        },
        catch_var: Some("err".into()),
        catch_body: Some(BlockIR {
            statements: vec![StatementIR::ExprStmtIR {
                expr: ExprIR::CallExpr {
                    name: "println".into(),
                    args: vec![ExprIR::IdentifierExpr {
                        name: "err".into(),
                        type_ann: None,
                    }],
                },
            }],
        }),
        finally_body: None,
    });
    assert!(gen.js.contains("try {"));
    assert!(gen.js.contains("riskyOp()"));
    assert!(gen.js.contains("catch (err)"));
    assert!(gen.js.contains("console.log(err)"));
}

#[test]
fn test_stmt_throw() {
    let mut gen = OmniViewGenerator::test_new();
    gen.emit_statement(&StatementIR::ThrowStmtIR {
        expr: ExprIR::LiteralExpr {
            value: serde_json::json!("error!"),
        },
    });
    assert!(gen.js.contains("throw"));
    assert!(gen.js.contains("error!"));
}

#[test]
fn test_stmt_continue() {
    let mut gen = OmniViewGenerator::test_new();
    gen.emit_statement(&StatementIR::ContinueStmtIR);
    assert!(gen.js.contains("continue"));
}

// ── Component Generation ────────────────────────────

#[test]
fn test_component_with_states() {
    let mut module = make_module("main");
    module.components.insert(
        "Counter".to_string(),
        ComponentIR {
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
                is_copy: false,
            }],
            ..make_component("Counter")
        },
    );
    let ir = make_program(FxHashMap::from_iter([("main".into(), module)]));
    let output = generate(&ir, false, false, false);
    assert!(output.js.contains("Counter_count"));
    assert!(output.js.contains("__omni_signal"));
}

#[test]
fn test_component_with_computed() {
    let mut module = make_module("main");
    module.components.insert(
        "Calc".to_string(),
        ComponentIR {
            signals: vec![SignalIntent {
                name: "x".into(),
                r#type: TypeIR {
                    kind: "i32".into(),
                    type_args: vec![],
                    is_nullable: false,
                },
                initial_value: ExprIR::LiteralExpr {
                    value: serde_json::json!(10),
                },
                is_copy: false,
            }],
            computeds: vec![ComputedIntent {
                name: "doubled".into(),
                r#type: TypeIR {
                    kind: "i32".into(),
                    type_args: vec![],
                    is_nullable: false,
                },
                body: ExprIR::BinaryOpExpr {
                    op: BinaryOpIR::Mul,
                    left: Box::new(ExprIR::IdentifierExpr {
                        name: "x".into(),
                        type_ann: None,
                    }),
                    right: Box::new(ExprIR::LiteralExpr {
                        value: serde_json::json!(2),
                    }),
                },
                deps: vec!["x".into()],
            }],
            ..make_component("Calc")
        },
    );
    let ir = make_program(FxHashMap::from_iter([("main".into(), module)]));
    let output = generate(&ir, false, false, false);
    assert!(output.js.contains("Calc_doubled"));
    assert!(output.js.contains("__omni_computed"));
}

#[test]
fn test_component_with_functions() {
    let mut module = make_module("main");
    module.components.insert(
        "App".to_string(),
        ComponentIR {
            fns: vec![FunctionIR {
                name: "greet".into(),
                visibility: Visibility::Public,
                params: vec![ParamIR {
                    name: "name".into(),
                    r#type: None,
                }],
                return_type: None,
                body: BlockIR {
                    statements: vec![StatementIR::ExprStmtIR {
                        expr: ExprIR::CallExpr {
                            name: "println".into(),
                            args: vec![ExprIR::BinaryOpExpr {
                                op: BinaryOpIR::Add,
                                left: Box::new(ExprIR::LiteralExpr {
                                    value: serde_json::json!("Hello "),
                                }),
                                right: Box::new(ExprIR::IdentifierExpr {
                                    name: "name".into(),
                                    type_ann: None,
                                }),
                            }],
                        },
                    }],
                },
                is_async: false,

                source_span: None,
            }],
            ..make_component("App")
        },
    );
    let ir = make_program(FxHashMap::from_iter([("main".into(), module)]));
    let output = generate(&ir, false, false, false);
    assert!(output.js.contains("function App_greet"));
    assert!(output.js.contains("name"));
    assert!(output.js.contains("console.log"));
}

#[test]
fn test_component_with_lifecycle_mount() {
    let mut module = make_module("main");
    module.components.insert(
        "App".to_string(),
        ComponentIR {
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
                is_copy: false,
            }],
            lifecycles: vec![LifecycleIntent {
                kind: LifecycleKind::Mount,
                body: BlockIR {
                    statements: vec![StatementIR::ExprStmtIR {
                        expr: ExprIR::CallExpr {
                            name: "println".into(),
                            args: vec![ExprIR::LiteralExpr {
                                value: serde_json::json!("mounted"),
                            }],
                        },
                    }],
                },
            }],
            view: Some(ViewTree::Element(make_element("div"))),
            ..make_component("App")
        },
    );
    let ir = make_program(FxHashMap::from_iter([("main".into(), module)]));
    let output = generate(&ir, false, false, false);
    assert!(output.js.contains("@mount App"));
    assert!(output.js.contains("console.log"));
}

#[test]
fn test_component_with_store_ref() {
    let mut module = make_module("main");
    module.stores.insert(
        "AppStore".to_string(),
        StoreIR {
            name: "AppStore".to_string(),
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
        },
    );
    module.components.insert(
        "App".to_string(),
        ComponentIR {
            store_refs: vec!["AppStore".into()],
            ..make_component("App")
        },
    );
    let ir = make_program(FxHashMap::from_iter([("main".into(), module)]));
    let output = generate(&ir, false, false, false);
    assert!(output.js.contains("AppStore_count"));
}

#[test]
fn test_component_with_effect() {
    let mut module = make_module("main");
    module.components.insert(
        "App".to_string(),
        ComponentIR {
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
                is_copy: false,
            }],
            effects: vec![EffectIntent {
                deps: vec!["count".into()],
                body: BlockIR {
                    statements: vec![StatementIR::ExprStmtIR {
                        expr: ExprIR::CallExpr {
                            name: "println".into(),
                            args: vec![ExprIR::IdentifierExpr {
                                name: "count".into(),
                                type_ann: None,
                            }],
                        },
                    }],
                },
            }],
            ..make_component("App")
        },
    );
    let ir = make_program(FxHashMap::from_iter([("main".into(), module)]));
    let output = generate(&ir, false, false, false);
    assert!(output.js.contains("__omni_effect"));
}

// ── HTML Generation ─────────────────────────────────

#[test]
fn test_html_element_with_attributes() {
    let mut module = make_module("main");
    let mut el = make_element("div");
    el.props.insert(
        "class".into(),
        ExprIR::LiteralExpr {
            value: serde_json::json!("container"),
        },
    );
    let mut child = make_element("span");
    child.props.insert(
        "id".into(),
        ExprIR::LiteralExpr {
            value: serde_json::json!("label"),
        },
    );
    el.children = vec![ViewTree::Element(child)];
    module.components.insert(
        "App".to_string(),
        ComponentIR {
            view: Some(ViewTree::Element(el)),
            ..make_component("App")
        },
    );
    let ir = make_program(FxHashMap::from_iter([("main".into(), module)]));
    let output = generate(&ir, true, false, false);
    assert!(output.html.contains("class=\"container\""));
    assert!(output.html.contains("id=\"label\""));
    assert!(output.html.contains("data-nid="));
}

#[test]
fn test_html_data_nid_present() {
    let mut module = make_module("main");
    module.components.insert(
        "App".to_string(),
        ComponentIR {
            view: Some(ViewTree::Element(make_element("button"))),
            ..make_component("App")
        },
    );
    let ir = make_program(FxHashMap::from_iter([("main".into(), module)]));
    let output = generate(&ir, true, false, false);
    assert!(output.html.contains("data-nid=\"el-button-"));
}

#[test]
fn test_html_interpolation() {
    let mut module = make_module("main");
    let mut el = make_element("div");
    el.children = vec![ViewTree::Interpolation(Box::new(ExprIR::IdentifierExpr {
        name: "name".into(),
        type_ann: None,
    }))];
    module.components.insert(
        "App".to_string(),
        ComponentIR {
            signals: vec![SignalIntent {
                name: "name".into(),
                r#type: TypeIR {
                    kind: "string".into(),
                    type_args: vec![],
                    is_nullable: false,
                },
                initial_value: ExprIR::LiteralExpr {
                    value: serde_json::json!("World"),
                },
                is_copy: false,
            }],
            view: Some(ViewTree::Element(el)),
            ..make_component("App")
        },
    );
    let ir = make_program(FxHashMap::from_iter([("main".into(), module)]));
    let output = generate(&ir, true, false, false);
    assert!(output.html.contains("<span"));
    assert!(output.html.contains("data-nid="));
    assert!(output.js.contains("textContent"));
}

#[test]
fn test_html_conditional() {
    let mut module = make_module("main");
    let mut visible_el = make_element("div");
    visible_el.children = vec![ViewTree::Text("Visible".into())];
    let mut hidden_el = make_element("div");
    hidden_el.children = vec![ViewTree::Text("Hidden".into())];
    module.components.insert(
        "App".to_string(),
        ComponentIR {
            view: Some(ViewTree::Conditional(Box::new(ConditionalIntent {
                branches: vec![ConditionalBranch {
                    condition: ExprIR::IdentifierExpr {
                        name: "show".into(),
                        type_ann: None,
                    },
                    body: Some(ViewTree::Element(visible_el)),
                }],
                fallback: Some(ViewTree::Element(hidden_el)),
                transition_mode: None,
            }))),
            ..make_component("App")
        },
    );
    let ir = make_program(FxHashMap::from_iter([("main".into(), module)]));
    let output = generate(&ir, true, false, false);
    assert!(output.html.contains("Visible"));
    assert!(output.html.contains("Hidden"));
}

// ── Full Pipeline ───────────────────────────────────

#[test]
fn test_full_pipeline_empty() {
    let ir = IRProgram::default();
    let output = generate(&ir, false, false, false);
    assert!(output.js.contains("__omni_signal"));
}

#[test]
fn test_full_pipeline_component_only() {
    let mut module = make_module("main");
    let mut el = make_element("div");
    el.children = vec![ViewTree::Text("Hello".into())];
    module.components.insert(
        "App".to_string(),
        ComponentIR {
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
                is_copy: false,
            }],
            view: Some(ViewTree::Element(el)),
            ..make_component("App")
        },
    );
    let ir = make_program(FxHashMap::from_iter([("main".into(), module)]));
    let output = generate(&ir, true, false, false);
    assert!(output.js.contains("App_count"));
    assert!(output.js.contains("__omni_signal"));
    assert!(output.html.contains("Hello"));
}

#[test]
fn test_full_pipeline_store_only() {
    let mut module = make_module("main");
    module.stores.insert(
        "DataStore".to_string(),
        StoreIR {
            name: "DataStore".to_string(),
            signals: FxHashMap::from_iter([(
                "value".into(),
                TypeIR {
                    kind: "i32".into(),
                    type_args: vec![],
                    is_nullable: false,
                },
            )]),
            signal_defaults: FxHashMap::from_iter([("value".into(), serde_json::json!(5))]),
            getters: FxHashMap::default(),
            actions: vec![],
            ..Default::default()
        },
    );
    let ir = make_program(FxHashMap::from_iter([("main".into(), module)]));
    let output = generate(&ir, false, false, false);
    assert!(output.js.contains("DataStore_value"));
    assert!(output.js.contains("__omni_signal(\"DataStore_value\", 5)"));
}

#[test]
fn test_full_pipeline_component_with_store() {
    let mut module = make_module("main");
    module.stores.insert(
        "AppStore".to_string(),
        StoreIR {
            name: "AppStore".to_string(),
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
        },
    );
    module.components.insert(
        "App".to_string(),
        ComponentIR {
            store_refs: vec!["AppStore".into()],
            view: Some(ViewTree::Element(make_element("div"))),
            ..make_component("App")
        },
    );
    let ir = make_program(FxHashMap::from_iter([("main".into(), module)]));
    let output = generate(&ir, true, false, false);
    assert!(output.js.contains("AppStore_count"));
}

// ── Event Bindings ──────────────────────────────────

#[test]
fn test_event_binding_click() {
    let mut module = make_module("main");
    let mut el = make_element("button");
    el.events.insert(
        "onClick".into(),
        EventHandlerIR::FnRef {
            r#ref: "handleClick".into(),
            params: vec![],
            event_data: None,
            used_fields: None,
            needs_prevent_default: false,
            needs_stop_propagation: false,
        },
    );
    module.components.insert(
        "App".to_string(),
        ComponentIR {
            fns: vec![FunctionIR {
                name: "handleClick".into(),
                visibility: Visibility::Public,
                params: vec![],
                return_type: None,
                body: BlockIR { statements: vec![] },
                is_async: false,

                source_span: None,
            }],
            view: Some(ViewTree::Element(el)),
            ..make_component("App")
        },
    );
    let ir = make_program(FxHashMap::from_iter([("main".into(), module)]));
    let output = generate(&ir, true, false, false);
    assert!(output.js.contains("click"));
    assert!(output.js.contains("App_handleClick"));
    assert!(output.js.contains("addEventListener"));
}

// ── Styles ──────────────────────────────────────────

#[test]
fn test_style_emission() {
    let mut module = make_module("main");
    module.components.insert(
        "App".to_string(),
        ComponentIR {
            view: Some(ViewTree::Element(make_element("div"))),
            styles: vec![StyleRuleIR {
                class_name: "myClass".into(),
                properties: FxHashMap::from_iter([(
                    "color".into(),
                    ExprIR::LiteralExpr {
                        value: serde_json::json!("red"),
                    },
                )]),
                output_mode: "css".into(),
                is_tag_selector: false,
            }],
            ..make_component("App")
        },
    );
    let ir = make_program(FxHashMap::from_iter([("main".into(), module)]));
    let output = generate(&ir, true, false, false);
    assert!(output.css.contains("myClass"));
    assert!(output.css.contains("color: red"));
    assert!(output.css.contains("data-component"));
}

// ── Source Map ──────────────────────────────────────

#[test]
fn test_source_map_entries() {
    let mut module = make_module("main");
    let mut el = make_element("div");
    el.source_span = Some(omnilang_core::lexer::Span {
        start: 0,
        end: 0,
        line: 5,
        col: 3,
    });
    module.components.insert(
        "App".to_string(),
        ComponentIR {
            view: Some(ViewTree::Element(el)),
            ..make_component("App")
        },
    );
    let ir = make_program(FxHashMap::from_iter([("main".into(), module)]));
    let output = generate(&ir, true, false, false);
    assert!(
        !output.source_map.is_empty(),
        "should have source map entries"
    );
    assert!(output.source_map[0].node_id.starts_with("el-div-"));
    assert_eq!(output.source_map[0].omni_span.line, 5);
    assert_eq!(output.source_map[0].omni_span.col, 3);
}

#[test]
fn test_runtime_error_reporting() {
    let mut module = make_module("main");
    let mut el = make_element("div");
    el.children = vec![ViewTree::Interpolation(Box::new(ExprIR::IdentifierExpr {
        name: "name".into(),
        type_ann: None,
    }))];
    module.components.insert(
        "App".to_string(),
        ComponentIR {
            signals: vec![SignalIntent {
                name: "name".into(),
                r#type: TypeIR {
                    kind: "string".into(),
                    type_args: vec![],
                    is_nullable: false,
                },
                initial_value: ExprIR::LiteralExpr {
                    value: serde_json::json!("World"),
                },
                is_copy: false,
            }],
            view: Some(ViewTree::Element(el)),
            ..make_component("App")
        },
    );
    let ir = make_program(FxHashMap::from_iter([("main".into(), module)]));
    let output = generate(&ir, true, false, false);
    assert!(
        output.js.contains("__omni_reportError"),
        "runtime must include __omni_reportError"
    );
    assert!(
        output.js.contains("try { fn(); } finally"),
        "__omni_effect must use try/finally"
    );
    assert!(
        output
            .js
            .contains("__omni_reportError(\"interpolation:text\""),
        "interpolation uses __omni_reportError"
    );
}

// ── Proptest: backtick escaping for innerHTML ───────

#[cfg(test)]
mod proptests {
    use proptest::prelude::*;

    /// The escape function from view.rs:49-51 that protects innerHTML
    /// template literal insertion: backslash -> \\, backtick -> \`, dollar -> \$.
    fn escape_for_inner_html(s: &str) -> String {
        s.replace('\\', "\\\\")
            .replace('`', "\\`")
            .replace("$", "\\$")
    }

    proptest! {
        /// After escaping, no raw backtick must remain — an unescaped backtick
        /// would close the template literal and allow JS injection.
        #[test]
        fn no_unescaped_backtick(s in "\\PC*") {
            let escaped = escape_for_inner_html(&s);
            // Find all backtick positions in the escaped string
            for (i, ch) in escaped.char_indices() {
                if ch == '`' {
                    // Every backtick must be preceded by a backslash
                    assert!(i > 0, "backtick at position 0 is unescaped: {escaped:?}");
                    assert_eq!(
                        &escaped[i-1..i], "\\",
                        "backtick at position {i} is not escaped: {escaped:?}"
                    );
                }
            }
        }

        /// After escaping, `${` must not appear — that would be a JS template
        /// literal interpolation that could execute arbitrary code.
        #[test]
        fn no_interpolation_breakout(s in "\\PC*") {
            let escaped = escape_for_inner_html(&s);
            // The only `$` in the output must be preceded by `\`
            for (i, ch) in escaped.char_indices() {
                if ch == '$' {
                    assert!(i > 0, "$ at position 0 is unescaped: {escaped:?}");
                    assert_eq!(
                        &escaped[i-1..i], "\\",
                        "$ at position {i} is not escaped: {escaped:?}"
                    );
                }
            }
        }

        /// Verify that the three escape replacements are applied correctly:
        /// `\` → `\\`, `` ` `` → `` \` ``, `$` → `\$`.
        #[test]
        fn specific_chars_are_escaped(s in "[`\\\\$]{1,10}") {
            let escaped = escape_for_inner_html(&s);
            // All three dangerous chars are doubled/escaped in the output
            for ch in s.chars() {
                match ch {
                    '\\' => prop_assert!(escaped.contains("\\\\"), "backslash not doubled"),
                    '`' => prop_assert!(escaped.contains("\\`"), "backtick not escaped"),
                    '$' => prop_assert!(escaped.contains("\\$"), "dollar not escaped"),
                    _ => {}
                }
            }
        }
    }
}
