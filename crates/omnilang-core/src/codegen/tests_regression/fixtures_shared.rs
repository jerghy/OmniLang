// Shared regression-test fixtures. This file is ALSO included verbatim into
// the omnilang crate's tests (`include!` in omnilang/src/codegen/tests_regression/mod.rs),
// where the fixture factories below are heavily used — so most of them look
// dead when compiled inside omnilang-core's own test build (hence the per-fn
// `#[allow(dead_code)]`s; a module-level inner attribute is impossible here
// because `include!` rejects inner attributes).
use rustc_hash::FxHashMap;

use crate::codegen::CompileOutput;
use crate::ir::*;
use std::fmt::Write;

// ═════════════════════════════════════════════════════════════════════════════
// Fixture factories
// ═════════════════════════════════════════════════════════════════════════════

/// Empty IRProgram — no components, no stores, no modules.
#[allow(dead_code)] // used by the omnilang facade crate's test build via include!
pub(crate) fn fixture_empty() -> IRProgram {
    IRProgram {
        version: "0.1".into(),
        modules: FxHashMap::from_iter([(
            "main".to_string(),
            ModuleIR {
                name: "main".to_string(),
                ..Default::default()
            },
        )]),
        source_map: FxHashMap::default(),
        stdlib_imports: Default::default(),
        source_span: None,
        motion_index: FxHashMap::default(),
    }
}

/// A simple component with a state, props, computed, effect, and a view tree.
#[allow(dead_code)] // used by the omnilang facade crate's test build via include!
pub(crate) fn fixture_component() -> IRProgram {
    let mut props = FxHashMap::default();
    props.insert(
        "name".into(),
        TypeIR {
            kind: "string".into(),
            type_args: vec![],
            is_nullable: false,
        },
    );
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
    let effect = EffectIntent {
        deps: vec!["count".into()],
        body: BlockIR {
            statements: vec![StatementIR::ExprStmtIR {
                expr: ExprIR::CallExpr {
                    name: "console.log".into(),
                    args: vec![ExprIR::IdentifierExpr {
                        name: "count".into(),
                        type_ann: None,
                    }],
                },
            }],
        },
    };
    let comp = ComponentIR {
        form_intent: None,
        load_intent: None,
        name: "Counter".into(),
        props,
        prop_defaults: FxHashMap::default(),
        signals: vec![state],
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
                    name: "count".into(),
                    type_ann: None,
                }),
                right: Box::new(ExprIR::LiteralExpr {
                    value: serde_json::json!(2),
                }),
            },
            deps: vec!["count".into()],
        }],
        effects: vec![effect],
        lifecycles: vec![],
        fns: vec![],
        store_refs: vec![],
        refs: vec![],
        events: vec![EventDeclIR {
            name: "onUpdate".into(),
            param_type: String::new(),
        }],
        view: Some(ViewTree::Element(Box::new(ElementIR {
            tag: "div".into(),
            props: FxHashMap::default(),
            bindings: FxHashMap::default(),
            events: FxHashMap::default(),
            style_classes: None,
            inline_style: None,
            children: vec![
                ViewTree::Text("Hello".into()),
                ViewTree::Interpolation(Box::new(ExprIR::IdentifierExpr {
                    name: "count".into(),
                    type_ann: None,
                })),
            ],
            class_map_bindings: FxHashMap::default(),
            motion: None,
            ref_binding: None,
            semantic_bindings: FxHashMap::default(),
            a11y: None,
            drag_binding: None,
            modal: None,
            skeleton: None,
            error_boundary: None,
            construction_kind: ConstructionKind::Static,
            source_span: None,
        }))),
        styles: vec![],
        ..Default::default()
    };
    IRProgram {
        version: "0.1".into(),
        modules: FxHashMap::from_iter([(
            "main".to_string(),
            ModuleIR {
                name: "main".to_string(),
                components: FxHashMap::from_iter([("Counter".into(), comp)]),
                ..Default::default()
            },
        )]),
        source_map: FxHashMap::default(),
        stdlib_imports: Default::default(),
        source_span: None,
        motion_index: FxHashMap::default(),
    }
}

/// A store with signals, getters, and an action.
#[allow(dead_code)] // used by the omnilang facade crate's test build via include!
pub(crate) fn fixture_store() -> IRProgram {
    let store = StoreIR {
        name: "AppStore".into(),
        theme_signal: None,
        signals: FxHashMap::from_iter([(
            "count".into(),
            TypeIR {
                kind: "i32".into(),
                type_args: vec![],
                is_nullable: false,
            },
        )]),
        signal_defaults: FxHashMap::from_iter([("count".into(), serde_json::json!(0))]),
        getters: FxHashMap::from_iter([(
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
        )]),
        actions: vec![FunctionIR {
            name: "increment".into(),
            visibility: Visibility::Public,
            params: vec![],
            return_type: None,
            body: BlockIR {
                statements: vec![StatementIR::MutationStmt {
                    intent: Some(MutationIntent::Increment {
                        target: SignalRefIR {
                            name: "count".into(),
                        },

                        amount: Some(Box::new(ExprIR::LiteralExpr {
                            value: serde_json::json!(1),
                        })),
                    }),
                }],
            },
            is_async: false,
            source_span: None,
        }],
        ..Default::default()
    };
    IRProgram {
        version: "0.1".into(),
        modules: FxHashMap::from_iter([(
            "main".to_string(),
            ModuleIR {
                name: "main".to_string(),
                stores: FxHashMap::from_iter([("AppStore".into(), store)]),
                ..Default::default()
            },
        )]),
        source_map: FxHashMap::default(),
        stdlib_imports: Default::default(),
        source_span: None,
        motion_index: FxHashMap::default(),
    }
}

/// A component that references the AppStore.
#[allow(dead_code)] // used by the omnilang facade crate's test build via include!
pub(crate) fn fixture_component_with_store() -> IRProgram {
    let store = StoreIR {
        name: "AppStore".into(),
        theme_signal: None,
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
        actions: vec![FunctionIR {
            name: "increment".into(),
            visibility: Visibility::Public,
            params: vec![],
            return_type: None,
            body: BlockIR {
                statements: vec![StatementIR::MutationStmt {
                    intent: Some(MutationIntent::Increment {
                        target: SignalRefIR {
                            name: "count".into(),
                        },

                        amount: Some(Box::new(ExprIR::LiteralExpr {
                            value: serde_json::json!(1),
                        })),
                    }),
                }],
            },
            is_async: false,
            source_span: None,
        }],
        ..Default::default()
    };
    let comp = ComponentIR {
        form_intent: None,
        load_intent: None,
        name: "CounterWithStore".into(),
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
        view: Some(ViewTree::Element(Box::new(ElementIR {
            tag: "div".into(),
            props: FxHashMap::default(),
            bindings: FxHashMap::default(),
            events: FxHashMap::from_iter([(
                "onPress".into(),
                EventHandlerIR::StoreAction {
                    r#ref: "AppStore.increment".into(),
                },
            )]),
            style_classes: None,
            inline_style: None,
            children: vec![
                ViewTree::Text("Count: ".into()),
                ViewTree::Interpolation(Box::new(ExprIR::StoreFieldAccessExpr {
                    store_alias: "AppStore".into(),
                    field_name: "count".into(),
                    call_args: vec![],
                })),
            ],
            class_map_bindings: FxHashMap::default(),
            motion: None,
            ref_binding: None,
            semantic_bindings: FxHashMap::default(),
            a11y: None,
            drag_binding: None,
            modal: None,
            skeleton: None,
            error_boundary: None,
            construction_kind: ConstructionKind::Static,
            source_span: None,
        }))),
        styles: vec![],
        ..Default::default()
    };
    IRProgram {
        version: "0.1".into(),
        modules: FxHashMap::from_iter([(
            "main".to_string(),
            ModuleIR {
                name: "main".to_string(),
                stores: FxHashMap::from_iter([("AppStore".into(), store)]),
                components: FxHashMap::from_iter([("CounterWithStore".into(), comp)]),
                ..Default::default()
            },
        )]),
        source_map: FxHashMap::default(),
        stdlib_imports: Default::default(),
        source_span: None,
        motion_index: FxHashMap::default(),
    }
}

/// A module with a public function that exercises all common ExprIR variants.
#[allow(dead_code)] // used by the omnilang facade crate's test build via include!
pub(crate) fn fixture_expressions() -> IRProgram {
    let func = FunctionIR {
        name: "testExpressions".into(),
        visibility: Visibility::Public,
        params: vec![],
        return_type: None,
        body: BlockIR {
            statements: vec![
                StatementIR::LetStmtIR {
                    name: "a".into(),
                    value: Some(ExprIR::LiteralExpr {
                        value: serde_json::json!(10),
                    }),
                    r#type: None,
                    is_mut: false,
                },
                StatementIR::LetStmtIR {
                    name: "b".into(),
                    value: Some(ExprIR::LiteralExpr {
                        value: serde_json::json!(3),
                    }),
                    r#type: None,
                    is_mut: false,
                },
                StatementIR::LetStmtIR {
                    name: "flag".into(),
                    value: Some(ExprIR::LiteralExpr {
                        value: serde_json::json!(true),
                    }),
                    r#type: None,
                    is_mut: false,
                },
                StatementIR::LetStmtIR {
                    name: "name".into(),
                    value: Some(ExprIR::LiteralExpr {
                        value: serde_json::json!("hello"),
                    }),
                    r#type: None,
                    is_mut: false,
                },
                StatementIR::LetStmtIR {
                    name: "nothing".into(),
                    value: Some(ExprIR::LiteralExpr {
                        value: serde_json::Value::Null,
                    }),
                    r#type: None,
                    is_mut: false,
                },
                StatementIR::LetStmtIR {
                    name: "add".into(),
                    value: Some(ExprIR::BinaryOpExpr {
                        op: BinaryOpIR::Add,
                        left: Box::new(ExprIR::IdentifierExpr {
                            name: "a".into(),
                            type_ann: None,
                        }),
                        right: Box::new(ExprIR::IdentifierExpr {
                            name: "b".into(),
                            type_ann: None,
                        }),
                    }),
                    r#type: None,
                    is_mut: false,
                },
                StatementIR::LetStmtIR {
                    name: "sub".into(),
                    value: Some(ExprIR::BinaryOpExpr {
                        op: BinaryOpIR::Sub,
                        left: Box::new(ExprIR::IdentifierExpr {
                            name: "a".into(),
                            type_ann: None,
                        }),
                        right: Box::new(ExprIR::IdentifierExpr {
                            name: "b".into(),
                            type_ann: None,
                        }),
                    }),
                    r#type: None,
                    is_mut: false,
                },
                StatementIR::LetStmtIR {
                    name: "mul".into(),
                    value: Some(ExprIR::BinaryOpExpr {
                        op: BinaryOpIR::Mul,
                        left: Box::new(ExprIR::IdentifierExpr {
                            name: "a".into(),
                            type_ann: None,
                        }),
                        right: Box::new(ExprIR::IdentifierExpr {
                            name: "b".into(),
                            type_ann: None,
                        }),
                    }),
                    r#type: None,
                    is_mut: false,
                },
                StatementIR::LetStmtIR {
                    name: "div".into(),
                    value: Some(ExprIR::BinaryOpExpr {
                        op: BinaryOpIR::Div,
                        left: Box::new(ExprIR::IdentifierExpr {
                            name: "a".into(),
                            type_ann: None,
                        }),
                        right: Box::new(ExprIR::IdentifierExpr {
                            name: "b".into(),
                            type_ann: None,
                        }),
                    }),
                    r#type: None,
                    is_mut: false,
                },
                StatementIR::LetStmtIR {
                    name: "eq".into(),
                    value: Some(ExprIR::BinaryOpExpr {
                        op: BinaryOpIR::Eq,
                        left: Box::new(ExprIR::IdentifierExpr {
                            name: "a".into(),
                            type_ann: None,
                        }),
                        right: Box::new(ExprIR::IdentifierExpr {
                            name: "b".into(),
                            type_ann: None,
                        }),
                    }),
                    r#type: None,
                    is_mut: false,
                },
                StatementIR::LetStmtIR {
                    name: "neq".into(),
                    value: Some(ExprIR::BinaryOpExpr {
                        op: BinaryOpIR::Neq,
                        left: Box::new(ExprIR::IdentifierExpr {
                            name: "a".into(),
                            type_ann: None,
                        }),
                        right: Box::new(ExprIR::IdentifierExpr {
                            name: "b".into(),
                            type_ann: None,
                        }),
                    }),
                    r#type: None,
                    is_mut: false,
                },
                StatementIR::LetStmtIR {
                    name: "and".into(),
                    value: Some(ExprIR::BinaryOpExpr {
                        op: BinaryOpIR::And,
                        left: Box::new(ExprIR::IdentifierExpr {
                            name: "flag".into(),
                            type_ann: None,
                        }),
                        right: Box::new(ExprIR::LiteralExpr {
                            value: serde_json::json!(true),
                        }),
                    }),
                    r#type: None,
                    is_mut: false,
                },
                StatementIR::LetStmtIR {
                    name: "or".into(),
                    value: Some(ExprIR::BinaryOpExpr {
                        op: BinaryOpIR::Or,
                        left: Box::new(ExprIR::IdentifierExpr {
                            name: "flag".into(),
                            type_ann: None,
                        }),
                        right: Box::new(ExprIR::LiteralExpr {
                            value: serde_json::json!(false),
                        }),
                    }),
                    r#type: None,
                    is_mut: false,
                },
                StatementIR::LetStmtIR {
                    name: "neg".into(),
                    value: Some(ExprIR::UnaryOpExpr {
                        op: UnaryOpIR::Neg,
                        operand: Box::new(ExprIR::IdentifierExpr {
                            name: "a".into(),
                            type_ann: None,
                        }),
                    }),
                    r#type: None,
                    is_mut: false,
                },
                StatementIR::LetStmtIR {
                    name: "not".into(),
                    value: Some(ExprIR::UnaryOpExpr {
                        op: UnaryOpIR::Not,
                        operand: Box::new(ExprIR::IdentifierExpr {
                            name: "flag".into(),
                            type_ann: None,
                        }),
                    }),
                    r#type: None,
                    is_mut: false,
                },
                StatementIR::LetStmtIR {
                    name: "ternary".into(),
                    value: Some(ExprIR::TernaryIR {
                        cond: Box::new(ExprIR::IdentifierExpr {
                            name: "flag".into(),
                            type_ann: None,
                        }),
                        then_expr: Box::new(ExprIR::LiteralExpr {
                            value: serde_json::json!("yes"),
                        }),
                        else_expr: Box::new(ExprIR::LiteralExpr {
                            value: serde_json::json!("no"),
                        }),
                    }),
                    r#type: None,
                    is_mut: false,
                },
                StatementIR::LetStmtIR {
                    name: "arr".into(),
                    value: Some(ExprIR::ArrayExpr {
                        elements: vec![
                            ExprIR::LiteralExpr {
                                value: serde_json::json!(1),
                            },
                            ExprIR::LiteralExpr {
                                value: serde_json::json!(2),
                            },
                            ExprIR::LiteralExpr {
                                value: serde_json::json!(3),
                            },
                        ],
                    }),
                    r#type: None,
                    is_mut: false,
                },
                StatementIR::LetStmtIR {
                    name: "first".into(),
                    value: Some(ExprIR::IndexExpr {
                        object: Box::new(ExprIR::IdentifierExpr {
                            name: "arr".into(),
                            type_ann: None,
                        }),
                        index: Box::new(ExprIR::LiteralExpr {
                            value: serde_json::json!(0),
                        }),
                    }),
                    r#type: None,
                    is_mut: false,
                },
                StatementIR::LetStmtIR {
                    name: "len".into(),
                    value: Some(ExprIR::FieldAccessExpr {
                        object: Box::new(ExprIR::IdentifierExpr {
                            name: "name".into(),
                            type_ann: None,
                        }),
                        field: "length".into(),
                    }),
                    r#type: None,
                    is_mut: false,
                },
                StatementIR::ExprStmtIR {
                    expr: ExprIR::MethodCallExpr {
                        object: Box::new(ExprIR::IdentifierExpr {
                            name: "name".into(),
                            type_ann: None,
                        }),
                        method: "toUpperCase".into(),
                        args: vec![],
                        is_optional: false,
                    },
                },
                StatementIR::LetStmtIR {
                    name: "closure".into(),
                    value: Some(ExprIR::ClosureExpr {
                        params: vec!["x".into(), "y".into()],
                        body: Box::new(ExprIR::BinaryOpExpr {
                            op: BinaryOpIR::Add,
                            left: Box::new(ExprIR::IdentifierExpr {
                                name: "x".into(),
                                type_ann: None,
                            }),
                            right: Box::new(ExprIR::IdentifierExpr {
                                name: "y".into(),
                                type_ann: None,
                            }),
                        }),
                    }),
                    r#type: None,
                    is_mut: false,
                },
                StatementIR::ExprStmtIR {
                    expr: ExprIR::CallExpr {
                        name: "console.log".into(),
                        args: vec![ExprIR::IdentifierExpr {
                            name: "a".into(),
                            type_ann: None,
                        }],
                    },
                },
                StatementIR::LetStmtIR {
                    name: "matchResult".into(),
                    value: Some(ExprIR::MatchExpr {
                        matched: Box::new(ExprIR::IdentifierExpr {
                            name: "a".into(),
                            type_ann: None,
                        }),
                        arms: vec![
                            MatchArmIR {
                                pattern: MatchPatternIR {
                                    kind: MatchPatternKindIR::Variant {
                                        name: "Some".into(),
                                        bindings: vec!["val".into()],
                                    },
                                },
                                body: BlockIR {
                                    statements: vec![StatementIR::ExprStmtIR {
                                        expr: ExprIR::IdentifierExpr {
                                            name: "val".into(),
                                            type_ann: None,
                                        },
                                    }],
                                },
                                union_type: None,
                            },
                            MatchArmIR {
                                pattern: MatchPatternIR {
                                    kind: MatchPatternKindIR::Wildcard,
                                },
                                body: BlockIR {
                                    statements: vec![StatementIR::ExprStmtIR {
                                        expr: ExprIR::LiteralExpr {
                                            value: serde_json::json!(0),
                                        },
                                    }],
                                },
                                union_type: None,
                            },
                        ],
                    }),
                    r#type: None,
                    is_mut: false,
                },
            ],
        },
        is_async: false,
        source_span: None,
    };
    IRProgram {
        version: "0.1".into(),
        modules: FxHashMap::from_iter([(
            "main".to_string(),
            ModuleIR {
                name: "main".to_string(),
                functions: FxHashMap::from_iter([("testExpressions".into(), func)]),
                ..Default::default()
            },
        )]),
        source_map: FxHashMap::default(),
        stdlib_imports: Default::default(),
        source_span: None,
        motion_index: FxHashMap::default(),
    }
}

// ═════════════════════════════════════════════════════════════════════════════
// Snapshot helper
// ═════════════════════════════════════════════════════════════════════════════

/// Concatenate all output files into a single snapshot string.
#[allow(dead_code)] // used by the omnilang facade crate's test build via include!
pub(crate) fn format_output(output: &CompileOutput) -> String {
    let mut s = String::new();
    for f in &output.files {
        let _ = writeln!(s, "=== {} ===", f.path);
        s.push_str(&f.content);
        if !f.content.ends_with('\n') {
            s.push('\n');
        }
    }
    if s.is_empty() {
        s.push_str("(no output files)\n");
    }
    s
}
