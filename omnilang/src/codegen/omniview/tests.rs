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
        construction_kind: ConstructionKind::Static,
        class_map_bindings: FxHashMap::default(),
        source_span: None,
        motion: None,
        ref_binding: None,
        semantic_bindings: FxHashMap::default(),
        a11y: None,
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
        motions: vec![],
        imports: vec![],
        exports: vec![],
        designs: vec![],
        source_span: None,
    }
}

fn make_component(name: &str) -> ComponentIR {
    ComponentIR {
        name: name.into(),
        source_file: None,
        props: FxHashMap::default(),
        prop_defaults: FxHashMap::default(),
        signals: vec![],
        computeds: vec![],
        effects: vec![],
        lifecycles: vec![],
        fns: vec![],
        store_refs: vec![],
        inline_stores: vec![],
        refs: vec![],
        events: vec![],
        slots: vec![],
        view: None,
        styles: vec![],
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
        motion_index: FxHashMap::default(),
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

// ── Prop Emission ─────────────────────────────────────

#[test]
fn props_are_emitted_as_readonly_wrappers() {
    // Build a component with `prop count: i32 = 0` and a signal
    let mut module = make_module("main");
    let comp = ComponentIR {
        name: "PropComponent".into(),
        source_file: None,
        props: FxHashMap::from_iter([(
            "count".into(),
            TypeIR {
                kind: "i32".into(),
                type_args: vec![],
                is_nullable: false,
            },
        )]),
        prop_defaults: FxHashMap::from_iter([(
            "count".into(),
            ExprIR::LiteralExpr {
                value: serde_json::json!(0),
            },
        )]),
        signals: vec![SignalIntent {
            name: "internal".into(),
            r#type: TypeIR {
                kind: "string".into(),
                type_args: vec![],
                is_nullable: false,
            },
            initial_value: ExprIR::LiteralExpr {
                value: serde_json::json!("hello"),
            },
        }],
        computeds: vec![],
        effects: vec![],
        lifecycles: vec![],
        fns: vec![],
        store_refs: vec![],
        inline_stores: vec![],
        refs: vec![],
        events: vec![],
        slots: vec![],
        view: None,
        styles: vec![],
        source_span: None,
    };
    module.components.insert("PropComponent".to_string(), comp);
    let ir = make_program(FxHashMap::from_iter([("main".into(), module)]));
    let output = generate(&ir, false, false, false);

    // The prop default must use __omni_readonlySignal
    assert!(
        output
            .js
            .contains("__omni_readonlySignal(\"PropComponent_count\", 0)"),
        "prop should be emitted via __omni_readonlySignal"
    );
    // The internal signal must still use __omni_signal
    assert!(
        output
            .js
            .contains("__omni_signal(\"PropComponent_internal\", \"hello\")"),
        "internal signal should still use __omni_signal"
    );
    // The runtime itself has __omni_readonlySignal function definition
    assert!(
        output.js.contains("function __omni_readonlySignal"),
        "runtime should define __omni_readonlySignal"
    );
}

// ── Expression Formatting ────────────────────────────

#[test]
fn test_expr_literal() {
    let mut gen = OmniViewGenerator::default();
    assert_eq!(
        gen.expr_to_js(&ExprIR::LiteralExpr {
            value: serde_json::json!(42)
        }),
        "42"
    );
}

#[test]
fn test_expr_literal_string() {
    let mut gen = OmniViewGenerator::default();
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
    let mut gen = OmniViewGenerator::default();
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
    let mut gen = OmniViewGenerator::default();
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
    let mut gen = OmniViewGenerator::default();
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
    let mut gen = OmniViewGenerator::default();
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
    let mut gen = OmniViewGenerator::default();
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
    let mut gen = OmniViewGenerator::default();
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
    let mut gen = OmniViewGenerator::default();
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
    let mut gen = OmniViewGenerator::default();
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
    let mut gen = OmniViewGenerator::default();
    gen.name_resolver.store_names.insert("AppStore".into());
    let js = gen.expr_to_js(&ExprIR::StoreFieldAccessExpr {
        store_alias: "AppStore".into(),
        field_name: "count".into(),
        call_args: vec![],
    });
    assert!(js.contains("AppStore_count"));
}

#[test]
fn test_expr_block() {
    let mut gen = OmniViewGenerator::default();
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
    let mut gen = OmniViewGenerator::default();
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
    let mut gen = OmniViewGenerator::default();
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
    let mut gen = OmniViewGenerator::default();
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
    let mut gen = OmniViewGenerator::default();
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
    let mut gen = OmniViewGenerator::default();
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
    let mut gen = OmniViewGenerator::default();
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
    let mut gen = OmniViewGenerator::default();
    gen.emit_statement(&StatementIR::MutationStmt {
        intent: Some(MutationIntent::DirectAssign {
            target: SignalRefIR {
                name: "count".into(),
            },

            value: Some(Box::new(ExprIR::LiteralExpr {
                value: serde_json::json!(42),
            })),
            field_object: None,
        }),
    });
    assert!(gen.js.contains("count = 42"));
}

#[test]
fn test_stmt_mutation_increment() {
    let mut gen = OmniViewGenerator::default();
    gen.emit_statement(&StatementIR::MutationStmt {
        intent: Some(MutationIntent::Increment {
            target: SignalRefIR {
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
    let mut gen = OmniViewGenerator::default();
    gen.name_resolver.signal_names.insert("App_count".into());
    gen.name_resolver
        .name_renames
        .insert("count".into(), "App_count".into());
    gen.emit_statement(&StatementIR::MutationStmt {
        intent: Some(MutationIntent::DirectAssign {
            target: SignalRefIR {
                name: "count".into(),
            },

            value: Some(Box::new(ExprIR::LiteralExpr {
                value: serde_json::json!(99),
            })),
            field_object: None,
        }),
    });
    assert!(gen.js.contains(".set(99)"));
}

#[test]
fn test_stmt_try_catch() {
    let mut gen = OmniViewGenerator::default();
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
    let mut gen = OmniViewGenerator::default();
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
    let mut gen = OmniViewGenerator::default();
    gen.emit_statement(&StatementIR::ContinueStmtIR);
    assert!(gen.js.contains("continue"));
}

#[test]
fn test_block_stmt_in_function() {
    let source = r#"import omniview;
component main {
    signal x: i32 = 0;
    fn increment() {
        {
            x = x + 1;
        }
    }
    view { <div>{x}</div> }
}
"#;
    let result = crate::compile_omniview(source, false, false);
    let js = result.files.expect("Expected JS output").js;
    // The block's inner statement should be emitted, not "unhandled stmt"
    assert!(
        !js.contains("unhandled stmt"),
        "BlockStmtIR should not fall through to wildcard. JS:\n{}",
        js
    );
    // The increment logic should appear in the output
    assert!(
        js.contains("__omni_subs.get"),
        "Signal mutation should be emitted"
    );
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
            view: Some(ViewTree::Element(Box::new(make_element("div")))),
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
    el.children = vec![ViewTree::Element(Box::new(child))];
    module.components.insert(
        "App".to_string(),
        ComponentIR {
            view: Some(ViewTree::Element(Box::new(el))),
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
            view: Some(ViewTree::Element(Box::new(make_element("button")))),
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
            }],
            view: Some(ViewTree::Element(Box::new(el))),
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
                    body: Some(ViewTree::Element(Box::new(visible_el))),
                }],
                fallback: Some(ViewTree::Element(Box::new(hidden_el))),
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

/// Build a single-branch + fallback conditional module for transition-mode
/// tests (mirrors test_html_conditional's shape).
fn make_conditional_module(transition_mode: Option<String>) -> IRProgram {
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
                    body: Some(ViewTree::Element(Box::new(visible_el))),
                }],
                fallback: Some(ViewTree::Element(Box::new(hidden_el))),
                transition_mode,
            }))),
            ..make_component("App")
        },
    );
    make_program(FxHashMap::from_iter([("main".into(), module)]))
}

/// `mode: "wait"` — nothing registers `_leave` animation entries (there is
/// no @leave mechanism in OmniLang), so the old "play leave then swap" path
/// could never fire. It degrades to an instant swap (same timing as
/// mode: "none") with the degradation stated in the generated JS — never a
/// silent no-op.
#[test]
fn test_html_conditional_wait_mode_is_honest_instant_swap() {
    let output = generate(
        &make_conditional_module(Some("wait".into())),
        true,
        false,
        false,
    );
    assert!(
        output
            .js
            .contains("no leave animation exists in OmniLang; degrades to an instant swap"),
        "Expected the degradation comment in generated JS, got:\n{}",
        output.js
    );
    assert!(
        !output.js.contains("_leave"),
        "Dead _leave lookup must be gone, got:\n{}",
        output.js
    );
    assert!(
        output.js.contains("_show("),
        "Wait mode must still swap branches (via the sync _show path), got:\n{}",
        output.js
    );
}

/// `mode: "fade"` and `mode: "none"` keep the plain sync output — no
/// degradation comment, no dead _leave code.
#[test]
fn test_html_conditional_fade_and_none_modes_unchanged() {
    for mode in [Some("fade".to_string()), Some("none".to_string()), None] {
        let output = generate(&make_conditional_module(mode.clone()), true, false, false);
        assert!(
            !output.js.contains("no leave animation exists"),
            "Unexpected degradation comment for mode {:?}:\n{}",
            mode,
            output.js
        );
        assert!(
            !output.js.contains("_leave"),
            "Dead _leave lookup must be gone for mode {:?}:\n{}",
            mode,
            output.js
        );
        assert!(
            output.js.contains("_show("),
            "Expected sync _show path for mode {:?}, got:\n{}",
            mode,
            output.js
        );
    }
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
            }],
            view: Some(ViewTree::Element(Box::new(el))),
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
            view: Some(ViewTree::Element(Box::new(make_element("div")))),
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
            view: Some(ViewTree::Element(Box::new(el))),
            ..make_component("App")
        },
    );
    let ir = make_program(FxHashMap::from_iter([("main".into(), module)]));
    let output = generate(&ir, true, false, false);
    assert!(output.js.contains("click"));
    assert!(output.js.contains("App_handleClick"));
    assert!(output.js.contains("addEventListener"));
}

// ── Ref Bindings ─────────────────────────────────────

#[test]
fn test_ref_wrapper_generated_for_ref_declaration() {
    let source = r#"
component TestRef {
  ref box: Element
  view {
    <div ref:box>content</div>
  }
}
"#;
    let result = crate::compile_omniview(source, false, false);
    assert!(result.success, "compilation failed: {:?}", result.errors);
    let js = result
        .files
        .as_ref()
        .expect("Expected JS output")
        .js
        .clone();
    let html = result
        .files
        .as_ref()
        .expect("Expected HTML output")
        .html
        .clone();

    // Verify the wrapper object is generated
    assert!(
        js.contains("const box = (function()"),
        "Expected ref wrapper 'box' in generated JS, got: {}",
        js
    );
    // Verify __omni_el is used (the lazy element getter)
    assert!(
        js.contains("__omni_el("),
        "Expected __omni_el call in wrapper: {}",
        js
    );
    // Verify the ref name does NOT get .value suffix (refs are not signals)
    assert!(
        !js.contains("box.value"),
        "Ref name 'box' should NOT have .value suffix. JS:\n{}",
        js
    );
    // Verify NO data-ref attribute in HTML
    assert!(
        !html.contains("data-ref=\"box\""),
        "data-ref attribute should NOT be emitted for ref bindings. HTML:\n{}",
        html
    );
}

// ── Component-level @motion ─────────────────────────

#[test]
fn test_component_level_motion_emits_motion_factory() {
    // Component-level `@motion` defs lower into ModuleIR.motions (module-
    // scoped); OmniView codegen must emit the WAAPI motion factory and wire
    // the element trigger to it.
    let source = r#"
component App {
    @motion slide {
        from { translateX: 0 }
        to   { translateX: 100 }
        over: 300ms
    }
    view {
        <div @enter={slide}/>
    }
}
"#;
    let result = crate::compile_omniview(source, false, false);
    assert!(result.success, "compilation failed: {:?}", result.errors);
    let js = result
        .files
        .as_ref()
        .expect("Expected JS output")
        .js
        .clone();
    assert!(
        js.contains("function __omni_motion_slide"),
        "component motion factory must be emitted. JS:\n{}",
        js
    );
}

// ── Styles ──────────────────────────────────────────

#[test]
fn test_style_emission() {
    let mut module = make_module("main");
    module.components.insert(
        "App".to_string(),
        ComponentIR {
            view: Some(ViewTree::Element(Box::new(make_element("div")))),
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
    el.source_span = Some(crate::lexer::Span {
        start: 0,
        end: 0,
        line: 5,
        col: 3,
    });
    module.components.insert(
        "App".to_string(),
        ComponentIR {
            view: Some(ViewTree::Element(Box::new(el))),
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
            }],
            view: Some(ViewTree::Element(Box::new(el))),
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
        output.js.contains("try { e.run(); } finally"),
        "__omni_effect must use try/finally"
    );
    assert!(
        output
            .js
            .contains("__omni_reportError(\"warn:interpolation:text\""),
        "interpolation uses __omni_reportError"
    );
}

// ── Theme runtime ────────────────────────────────────

#[test]
fn test_theme_runtime_emitted_when_design_has_themes() {
    let mut tokens = FxHashMap::default();
    let mut theme_values_dark = FxHashMap::default();
    theme_values_dark.insert(
        "dark".to_string(),
        TokenValueIR::Static("#a5b4fc".to_string()),
    );
    tokens.insert(
        "primary".to_string(),
        TokenIR {
            value: TokenValueIR::Static("#6366f1".to_string()),
            token_type: TokenTypeIR::Color,
            theme_values: theme_values_dark,
            source_span: None,
        },
    );

    let design = DesignIR {
        name: "AppDesign".to_string(),
        themes: vec!["light".to_string(), "dark".to_string()],
        theme_signal: None,
        default_theme: Some("light".to_string()),
        tokens,
        classes: vec![],
        variants: vec![],
        source_span: None,
        source_file: None,
    };

    let mut module = make_module("main");
    module.designs = vec![design];
    module.components.insert(
        "App".to_string(),
        ComponentIR {
            view: Some(ViewTree::Element(Box::new(make_element("div")))),
            ..make_component("App")
        },
    );
    let ir = make_program(FxHashMap::from_iter([("main".into(), module)]));
    let output = generate(&ir, false, false, false);

    assert!(
        output.js.contains("__omni_theme"),
        "should emit __omni_theme when designs have themes"
    );
    assert!(
        output.js.contains("_themes: [\"light\", \"dark\"]"),
        "should list all theme names in _themes array"
    );
    assert!(
        output.js.contains("_storageKey: \"omni-theme\""),
        "should use 'omni-theme' as storage key"
    );
    assert!(
        output.js.contains("localStorage.setItem"),
        "should persist theme to localStorage"
    );
    assert!(
        output.js.contains("prefers-color-scheme: dark"),
        "should detect prefers-color-scheme when dark theme exists"
    );
    assert!(
        output.js.contains("__omni_theme.init();"),
        "should auto-initialize __omni_theme"
    );
}

#[test]
fn test_theme_runtime_not_emitted_without_themes() {
    let design = DesignIR {
        name: "SimpleDesign".to_string(),
        themes: vec![],
        theme_signal: None,
        default_theme: None,
        tokens: FxHashMap::default(),
        classes: vec![],
        variants: vec![],
        source_span: None,
        source_file: None,
    };

    let mut module = make_module("main");
    module.designs = vec![design];
    module.components.insert(
        "App".to_string(),
        ComponentIR {
            view: Some(ViewTree::Element(Box::new(make_element("div")))),
            ..make_component("App")
        },
    );
    let ir = make_program(FxHashMap::from_iter([("main".into(), module)]));
    let output = generate(&ir, false, false, false);

    assert!(
        !output.js.contains("__omni_theme"),
        "should NOT emit __omni_theme when no themes declared"
    );
}

// ── Theme signal bridge (Task 4.2, design D1) ──────────────

/// Build an IR program with a `ThemeStore` carrying a `mode` signal plus a
/// themed design. `theme_signal` controls whether the signal is marked
/// `@theme` (wired into DesignIR.theme_signal by validate_theme).
fn make_theme_signal_ir(theme_signal: Option<String>) -> IRProgram {
    let store = StoreIR {
        name: "ThemeStore".to_string(),
        signals: vec![(
            "mode".to_string(),
            TypeIR {
                kind: "String".into(),
                type_args: vec![],
                is_nullable: false,
            },
        )]
        .into_iter()
        .collect(),
        signal_defaults: vec![("mode".to_string(), serde_json::json!("light"))]
            .into_iter()
            .collect(),
        getters: FxHashMap::default(),
        actions: vec![],
        source_file: None,
        ..Default::default()
    };
    let design = DesignIR {
        name: "AppDesign".to_string(),
        themes: vec!["light".to_string(), "dark".to_string()],
        theme_signal,
        default_theme: Some("light".to_string()),
        tokens: FxHashMap::default(),
        classes: vec![],
        variants: vec![],
        source_span: None,
        source_file: None,
    };
    let mut module = make_module("main");
    module.designs = vec![design];
    module.stores.insert("ThemeStore".to_string(), store);
    module.components.insert(
        "App".to_string(),
        ComponentIR {
            view: Some(ViewTree::Element(Box::new(make_element("div")))),
            ..make_component("App")
        },
    );
    make_program(FxHashMap::from_iter([("main".into(), module)]))
}

/// Task 4.2 (design D1): the @theme-marked store signal syncs to
/// __omni_theme (which writes data-theme) — recognized by IR, not by name.
#[test]
fn theme_signal_drives_omni_theme_sync_bridge() {
    let ir = make_theme_signal_ir(Some("ThemeStore.mode".to_string()));
    let output = generate(&ir, false, false, false);
    assert!(
        output.js.contains("__omni_theme"),
        "theme runtime must be emitted"
    );
    assert!(
        output.js.contains("__omni_subs.get(\"ThemeStore_mode\")"),
        "the @theme signal must sync to __omni_theme. Got:\n{}",
        output.js
    );
    assert!(
        output
            .js
            .contains("__omni_theme.set(ThemeStore_mode.value)"),
        "store changes must propagate to __omni_theme (data-theme). Got:\n{}",
        output.js
    );
}

/// Task 4.2 (design D1): a store signal named "mode" WITHOUT the @theme
/// marker must NOT be bridged — the `mode` magic name is gone.
#[test]
fn mode_magic_name_bridge_removed_without_theme_signal() {
    let ir = make_theme_signal_ir(None);
    let output = generate(&ir, false, false, false);
    assert!(
        output.js.contains("__omni_theme"),
        "theme runtime must be emitted"
    );
    assert!(
        !output.js.contains("__omni_subs.get(\"ThemeStore_mode\")"),
        "no magic-name bridge without a declared @theme signal. Got:\n{}",
        output.js
    );
    assert!(
        !output.js.contains("Theme mode sync"),
        "no magic-name bridge without a declared @theme signal. Got:\n{}",
        output.js
    );
}

// ── Slot Projection (Content Injection) ─────────────────

#[test]
fn slot_projection_inlines_parent_children_into_default_slot() {
    let mut module = make_module("main");

    // Button component: <div><slot></slot></div>
    let mut btn_div = make_element("div");
    btn_div.children = vec![ViewTree::Slot {
        name: None,
        fallback: Vec::new(),
    }];
    module.components.insert(
        "Button".to_string(),
        ComponentIR {
            view: Some(ViewTree::Element(Box::new(btn_div))),
            ..make_component("Button")
        },
    );

    // App component: <Button>hello</Button>
    let mut app_el = make_element("Button");
    app_el.children = vec![ViewTree::Text("hello".into())];
    module.components.insert(
        "App".to_string(),
        ComponentIR {
            view: Some(ViewTree::Element(Box::new(app_el))),
            ..make_component("App")
        },
    );

    let ir = make_program(FxHashMap::from_iter([("main".into(), module)]));
    let output = generate(&ir, false, false, false);

    assert!(
        output.html.contains("hello"),
        "parent-submitted children should appear in child's rendered HTML.\nHTML:\n{}",
        output.html
    );
}

#[test]
fn slot_projection_maps_named_slots_correctly() {
    let mut module = make_module("main");

    // Panel component: <div><slot name="header"></slot><slot></slot></div>
    let mut panel_div = make_element("div");
    panel_div.children = vec![
        ViewTree::Slot {
            name: Some("header".into()),
            fallback: Vec::new(),
        },
        ViewTree::Slot {
            name: None,
            fallback: Vec::new(),
        },
    ];
    module.components.insert(
        "Panel".to_string(),
        ComponentIR {
            view: Some(ViewTree::Element(Box::new(panel_div))),
            ..make_component("Panel")
        },
    );

    // App component: <Panel><span slot="header">H</span>body</Panel>
    let mut span = make_element("span");
    span.props.insert(
        "slot".into(),
        ExprIR::LiteralExpr {
            value: serde_json::json!("header"),
        },
    );
    span.children = vec![ViewTree::Text("H".into())];

    let mut app_el = make_element("Panel");
    app_el.children = vec![
        ViewTree::Element(Box::new(span)),
        ViewTree::Text("body".into()),
    ];
    module.components.insert(
        "App".to_string(),
        ComponentIR {
            view: Some(ViewTree::Element(Box::new(app_el))),
            ..make_component("App")
        },
    );

    let ir = make_program(FxHashMap::from_iter([("main".into(), module)]));
    let output = generate(&ir, false, false, false);

    assert!(
        output.html.contains("H"),
        "named slot content should appear in output.\nHTML:\n{}",
        output.html
    );
    assert!(
        output.html.contains("body"),
        "default slot content should appear in output.\nHTML:\n{}",
        output.html
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

// ── TypeScript Declarations ──────────────────────────

#[test]
fn typescript_declarations_emit_props_interface() {
    let mut module = make_module("main");
    module.components.insert(
        "Counter".to_string(),
        ComponentIR {
            props: FxHashMap::from_iter([
                (
                    "count".into(),
                    TypeIR {
                        kind: TypeKind::Int,
                        type_args: vec![],
                        is_nullable: false,
                    },
                ),
                (
                    "label".into(),
                    TypeIR {
                        kind: TypeKind::String,
                        type_args: vec![],
                        is_nullable: false,
                    },
                ),
                (
                    "enabled".into(),
                    TypeIR {
                        kind: TypeKind::Bool,
                        type_args: vec![],
                        is_nullable: false,
                    },
                ),
            ]),
            ..make_component("Counter")
        },
    );
    let ir = make_program(FxHashMap::from_iter([("main".into(), module)]));
    let output = generate(&ir, false, false, false);
    assert!(
        output.types.is_empty(),
        "ADR-031 D6: TypeScript gen gated, types should be empty"
    );
}

#[test]
fn typescript_declarations_optional_for_defaulted_props() {
    let mut module = make_module("main");
    module.components.insert(
        "Widget".to_string(),
        ComponentIR {
            props: FxHashMap::from_iter([
                (
                    "required".into(),
                    TypeIR {
                        kind: TypeKind::String,
                        type_args: vec![],
                        is_nullable: false,
                    },
                ),
                (
                    "optional".into(),
                    TypeIR {
                        kind: TypeKind::Int,
                        type_args: vec![],
                        is_nullable: false,
                    },
                ),
            ]),
            prop_defaults: FxHashMap::from_iter([(
                "optional".into(),
                ExprIR::LiteralExpr {
                    value: serde_json::json!(42),
                },
            )]),
            ..make_component("Widget")
        },
    );
    let ir = make_program(FxHashMap::from_iter([("main".into(), module)]));
    let output = generate(&ir, false, false, false);
    assert!(
        output.types.is_empty(),
        "ADR-031 D6: TypeScript gen gated, types should be empty"
    );
}

#[test]
fn typescript_declarations_event_handler_types() {
    let mut module = make_module("main");
    module.components.insert(
        "Button".to_string(),
        ComponentIR {
            events: vec![
                EventDeclIR {
                    name: "onClick".into(),
                    param_type: "MouseEvent".into(),
                },
                EventDeclIR {
                    name: "onHover".into(),
                    param_type: "".into(),
                },
            ],
            ..make_component("Button")
        },
    );
    let ir = make_program(FxHashMap::from_iter([("main".into(), module)]));
    let output = generate(&ir, false, false, false);
    assert!(
        output.types.is_empty(),
        "ADR-031 D6: TypeScript gen gated, types should be empty"
    );
}

#[test]
fn typescript_declarations_empty_when_no_components() {
    let ir = IRProgram::default();
    let output = generate(&ir, false, false, false);
    assert!(
        output.types.is_empty(),
        "ADR-031 D6: TypeScript gen gated, types should be empty"
    );
}

// ── Body-level Slot Declarations (ADR-053 D5) ───────────

/// Body-level `@slot` declarations surface in the view as `{name}`
/// interpolations. Standalone (no consumer projection), the interpolation
/// position renders the declared fallback inside a `<slot name="...">`
/// element — exactly like view-level `@slot` directives.
#[test]
fn body_slot_declaration_renders_fallback_at_interpolation() {
    let mut module = make_module("main");

    // Panel: `@slot header { <h4>Default header</h4> }` + view `<div>{header}</div>`
    let mut h4 = make_element("h4");
    h4.children = vec![ViewTree::Text("Default header".into())];
    let mut panel_div = make_element("div");
    panel_div.children = vec![ViewTree::Interpolation(Box::new(ExprIR::ident("header")))];
    module.components.insert(
        "Panel".to_string(),
        ComponentIR {
            slots: vec![SlotDeclIR {
                name: Some("header".into()),
                fallback: vec![ViewTree::Element(Box::new(h4))],
            }],
            view: Some(ViewTree::Element(Box::new(panel_div))),
            ..make_component("Panel")
        },
    );

    let ir = make_program(FxHashMap::from_iter([("main".into(), module)]));
    let output = generate(&ir, false, false, false);

    assert!(
        output.html.contains("<slot name=\"header\">"),
        "interpolation position should render a named slot element.\nHTML:\n{}",
        output.html
    );
    assert!(
        output.html.contains("Default header"),
        "declared fallback should render at the interpolation position.\nHTML:\n{}",
        output.html
    );
}

/// Consumer named-prop projection (`header={<div/>}`, ADR-053 D5) injects
/// into the child's body-declared slot position; un-slotted children flow
/// into the unnamed slot. No projection → declared fallback renders.
#[test]
fn consumer_named_prop_projects_into_body_declared_slot() {
    let mut module = make_module("main");

    // Panel: `@slot header { ... }` + `@slot { ... }` + view
    // `<div>{header}{children}</div>`
    let mut h4 = make_element("h4");
    h4.children = vec![ViewTree::Text("Default header".into())];
    let mut p_default = make_element("p");
    p_default.children = vec![ViewTree::Text("Default body".into())];
    let mut panel_div = make_element("div");
    panel_div.children = vec![
        ViewTree::Interpolation(Box::new(ExprIR::ident("header"))),
        ViewTree::Interpolation(Box::new(ExprIR::ident("children"))),
    ];
    module.components.insert(
        "Panel".to_string(),
        ComponentIR {
            slots: vec![
                SlotDeclIR {
                    name: Some("header".into()),
                    fallback: vec![ViewTree::Element(Box::new(h4))],
                },
                SlotDeclIR {
                    name: None,
                    fallback: vec![ViewTree::Element(Box::new(p_default))],
                },
            ],
            view: Some(ViewTree::Element(Box::new(panel_div))),
            ..make_component("Panel")
        },
    );

    // App: `<Panel header={<div>H</div>}><p>body</p></Panel>` — the named
    // projection lowers to a ViewTree::Slot child (ADR-053 D5).
    let mut proj_div = make_element("div");
    proj_div.children = vec![ViewTree::Text("H".into())];
    let mut body_p = make_element("p");
    body_p.children = vec![ViewTree::Text("body".into())];

    let mut app_el = make_element("Panel");
    app_el.children = vec![
        ViewTree::Element(Box::new(body_p)),
        ViewTree::Slot {
            name: Some("header".into()),
            fallback: vec![ViewTree::Element(Box::new(proj_div))],
        },
    ];
    module.components.insert(
        "App".to_string(),
        ComponentIR {
            view: Some(ViewTree::Element(Box::new(app_el))),
            ..make_component("App")
        },
    );

    let ir = make_program(FxHashMap::from_iter([("main".into(), module)]));
    let output = generate(&ir, false, false, false);

    assert!(
        output.html.contains("H"),
        "named-prop projection should inject into the named slot.\nHTML:\n{}",
        output.html
    );
    assert!(
        output.html.contains("body"),
        "un-slotted children should inject into the unnamed slot.\nHTML:\n{}",
        output.html
    );
    assert!(
        !output.html.contains("Default header") && !output.html.contains("Default body"),
        "projected content must replace the declared fallbacks.\nHTML:\n{}",
        output.html
    );
}

/// Children of a nested component instantiation are THAT component's
/// projections, not slot positions of the current view — injection must
/// not descend into them (regression: the new consumer named-prop syntax
/// lowers to ViewTree::Slot children, which the old recursion would
/// wrongly match).
#[test]
fn slot_injection_skips_nested_component_children() {
    let mut module = make_module("main");

    // Outer: `@slot header { ... }` + view
    // `<div><Inner>@slot header { <p>Inner content</p> }</Inner><slot name="header"/></div>`
    let mut inner_proj_p = make_element("p");
    inner_proj_p.children = vec![ViewTree::Text("Inner content".into())];
    let mut inner_el = make_element("Inner");
    inner_el.children = vec![ViewTree::Slot {
        name: Some("header".into()),
        fallback: vec![ViewTree::Element(Box::new(inner_proj_p))],
    }];
    let mut outer_div = make_element("div");
    outer_div.children = vec![
        ViewTree::Element(Box::new(inner_el)),
        ViewTree::Slot {
            name: Some("header".into()),
            fallback: Vec::new(),
        },
    ];
    module.components.insert(
        "Outer".to_string(),
        ComponentIR {
            slots: vec![SlotDeclIR {
                name: Some("header".into()),
                fallback: Vec::new(),
            }],
            view: Some(ViewTree::Element(Box::new(outer_div))),
            ..make_component("Outer")
        },
    );
    // Inner registers as a component (no own view — renders as an element).
    module.components.insert(
        "Inner".to_string(),
        ComponentIR {
            ..make_component("Inner")
        },
    );

    // App: `<Outer header={<div>Outer header</div>} />`
    let mut outer_hdr_div = make_element("div");
    outer_hdr_div.children = vec![ViewTree::Text("Outer header".into())];
    let mut app_el = make_element("Outer");
    app_el.children = vec![ViewTree::Slot {
        name: Some("header".into()),
        fallback: vec![ViewTree::Element(Box::new(outer_hdr_div))],
    }];
    module.components.insert(
        "App".to_string(),
        ComponentIR {
            view: Some(ViewTree::Element(Box::new(app_el))),
            ..make_component("App")
        },
    );

    let ir = make_program(FxHashMap::from_iter([("main".into(), module)]));
    let output = generate(&ir, false, false, false);

    assert!(
        output.html.contains("Outer header"),
        "outer projection should inject into Outer's own slot.\nHTML:\n{}",
        output.html
    );
    assert!(
        output.html.contains("Inner content"),
        "nested component's projection must stay untouched by the outer injection.\nHTML:\n{}",
        output.html
    );
}
