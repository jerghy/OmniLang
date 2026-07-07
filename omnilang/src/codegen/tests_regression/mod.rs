//! Cross-backend regression test baseline.
//!
//! Provides shared fixture factories and helpers for per-backend test modules.
//! Each backend sub-module contains its own snapshot tests.

pub(crate) mod dart;
pub(crate) mod dioxus;
pub(crate) mod flutter;
pub(crate) mod javascript;
pub(crate) mod python;
pub(crate) mod react;
pub(crate) mod rust;
pub(crate) mod vue;

use std::collections::HashMap;

use crate::codegen::CompileOutput;
use crate::ir::*;

// ═════════════════════════════════════════════════════════════════════════════
// Fixture factories
// ═════════════════════════════════════════════════════════════════════════════

/// Empty IRProgram — no components, no stores, no modules.
pub(crate) fn fixture_empty() -> IRProgram {
    IRProgram {
        version: "0.1".into(),
        modules: HashMap::from([(
            "main".to_string(),
            ModuleIR {
                name: "main".to_string(),
                ..Default::default()
            },
        )]),
        source_map: HashMap::new(),
        stdlib_imports: Vec::new(),
    }
}

/// A simple component with a state, props, computed, effect, and a view tree.
pub(crate) fn fixture_component() -> IRProgram {
    let mut props = HashMap::new();
    props.insert(
        "name".into(),
        TypeIR {
            kind: "string".into(),
            type_args: vec![],
            is_nullable: false,
        },
    );
    let state = StateIntent {
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
        name: "Counter".into(),
        generic_params: vec![],
        props,
        prop_defaults: HashMap::new(),
        states: vec![state],
        computeds: vec![ComputedIntent {
            name: "doubled".into(),
            r#type: TypeIR {
                kind: "i32".into(),
                type_args: vec![],
                is_nullable: false,
            },
            body: ExprIR::BinaryOpExpr {
                op: "mul".into(),
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
        event_names: vec!["onUpdate".into()],
        view: Some(ViewTree::Element(ElementIR {
            tag: "div".into(),
            props: HashMap::new(),
            bindings: HashMap::new(),
            events: HashMap::new(),
            style_classes: None,
            inline_style: None,
            children: vec![
                ViewTree::Text("Hello".into()),
                ViewTree::Interpolation(Box::new(ExprIR::IdentifierExpr {
                    name: "count".into(),
                    type_ann: None,
                })),
            ],
            class_map_bindings: HashMap::new(),
            motion: None,
            scroll: None,
            drag: None,
            hero: None,
            test_id: None,
            ref_name: None,
            construction_kind: ConstructionKind::Static,
            source_line: 0,
            source_col: 0,
        })),
        styles: vec![],
        motions: vec![],
        if_mode_default: None,
        hero_defs: vec![],
        timelines: vec![],
        ..Default::default()
    };
    IRProgram {
        version: "0.1".into(),
        modules: HashMap::from([(
            "main".to_string(),
            ModuleIR {
                name: "main".to_string(),
                components: HashMap::from([("Counter".into(), comp)]),
                ..Default::default()
            },
        )]),
        source_map: HashMap::new(),
        stdlib_imports: Vec::new(),
    }
}

/// A store with signals, getters, and an action.
pub(crate) fn fixture_store() -> IRProgram {
    let store = StoreIR {
        name: "AppStore".into(),
        signals: HashMap::from([(
            "count".into(),
            TypeIR {
                kind: "i32".into(),
                type_args: vec![],
                is_nullable: false,
            },
        )]),
        signal_defaults: HashMap::from([("count".into(), serde_json::json!(0))]),
        getters: HashMap::from([(
            "doubleCount".into(),
            ExprIR::BinaryOpExpr {
                op: "mul".into(),
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
                    intent: Some(MutationIntent {
                        target: SignalRefIR {
                            owner: None,
                            name: "count".into(),
                        },
                        operation: MutationOp::Increment,
                        amount: Some(ExprIR::LiteralExpr {
                            value: serde_json::json!(1),
                        }),
                        item: None,
                    }),
                }],
            },
            is_async: false,
        }],
        ..Default::default()
    };
    IRProgram {
        version: "0.1".into(),
        modules: HashMap::from([(
            "main".to_string(),
            ModuleIR {
                name: "main".to_string(),
                stores: HashMap::from([("AppStore".into(), store)]),
                ..Default::default()
            },
        )]),
        source_map: HashMap::new(),
        stdlib_imports: Vec::new(),
    }
}

/// A component that references the AppStore.
pub(crate) fn fixture_component_with_store() -> IRProgram {
    let store = StoreIR {
        name: "AppStore".into(),
        signals: HashMap::from([(
            "count".into(),
            TypeIR {
                kind: "i32".into(),
                type_args: vec![],
                is_nullable: false,
            },
        )]),
        signal_defaults: HashMap::from([("count".into(), serde_json::json!(0))]),
        getters: HashMap::new(),
        actions: vec![FunctionIR {
            name: "increment".into(),
            visibility: Visibility::Public,
            params: vec![],
            return_type: None,
            body: BlockIR {
                statements: vec![StatementIR::MutationStmt {
                    intent: Some(MutationIntent {
                        target: SignalRefIR {
                            owner: None,
                            name: "count".into(),
                        },
                        operation: MutationOp::Increment,
                        amount: Some(ExprIR::LiteralExpr {
                            value: serde_json::json!(1),
                        }),
                        item: None,
                    }),
                }],
            },
            is_async: false,
        }],
        ..Default::default()
    };
    let comp = ComponentIR {
        name: "CounterWithStore".into(),
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
        view: Some(ViewTree::Element(ElementIR {
            tag: "div".into(),
            props: HashMap::new(),
            bindings: HashMap::new(),
            events: HashMap::from([(
                "onClick".into(),
                EventHandlerIR::StoreAction {
                    r#ref: "AppStore.increment".into(),
                    event_data: None,
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
            class_map_bindings: HashMap::new(),
            motion: None,
            scroll: None,
            drag: None,
            hero: None,
            test_id: None,
            ref_name: None,
            construction_kind: ConstructionKind::Static,
            source_line: 0,
            source_col: 0,
        })),
        styles: vec![],
        motions: vec![],
        if_mode_default: None,
        hero_defs: vec![],
        timelines: vec![],
        ..Default::default()
    };
    IRProgram {
        version: "0.1".into(),
        modules: HashMap::from([(
            "main".to_string(),
            ModuleIR {
                name: "main".to_string(),
                stores: HashMap::from([("AppStore".into(), store)]),
                components: HashMap::from([("CounterWithStore".into(), comp)]),
                ..Default::default()
            },
        )]),
        source_map: HashMap::new(),
        stdlib_imports: Vec::new(),
    }
}

/// A module with a public function that exercises all common ExprIR variants.
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
                },
                StatementIR::LetStmtIR {
                    name: "b".into(),
                    value: Some(ExprIR::LiteralExpr {
                        value: serde_json::json!(3),
                    }),
                    r#type: None,
                },
                StatementIR::LetStmtIR {
                    name: "flag".into(),
                    value: Some(ExprIR::LiteralExpr {
                        value: serde_json::json!(true),
                    }),
                    r#type: None,
                },
                StatementIR::LetStmtIR {
                    name: "name".into(),
                    value: Some(ExprIR::LiteralExpr {
                        value: serde_json::json!("hello"),
                    }),
                    r#type: None,
                },
                StatementIR::LetStmtIR {
                    name: "nothing".into(),
                    value: Some(ExprIR::LiteralExpr {
                        value: serde_json::Value::Null,
                    }),
                    r#type: None,
                },
                StatementIR::LetStmtIR {
                    name: "add".into(),
                    value: Some(ExprIR::BinaryOpExpr {
                        op: "add".into(),
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
                },
                StatementIR::LetStmtIR {
                    name: "sub".into(),
                    value: Some(ExprIR::BinaryOpExpr {
                        op: "sub".into(),
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
                },
                StatementIR::LetStmtIR {
                    name: "mul".into(),
                    value: Some(ExprIR::BinaryOpExpr {
                        op: "mul".into(),
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
                },
                StatementIR::LetStmtIR {
                    name: "div".into(),
                    value: Some(ExprIR::BinaryOpExpr {
                        op: "div".into(),
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
                },
                StatementIR::LetStmtIR {
                    name: "eq".into(),
                    value: Some(ExprIR::BinaryOpExpr {
                        op: "eq".into(),
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
                },
                StatementIR::LetStmtIR {
                    name: "neq".into(),
                    value: Some(ExprIR::BinaryOpExpr {
                        op: "neq".into(),
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
                },
                StatementIR::LetStmtIR {
                    name: "and".into(),
                    value: Some(ExprIR::BinaryOpExpr {
                        op: "and".into(),
                        left: Box::new(ExprIR::IdentifierExpr {
                            name: "flag".into(),
                            type_ann: None,
                        }),
                        right: Box::new(ExprIR::LiteralExpr {
                            value: serde_json::json!(true),
                        }),
                    }),
                    r#type: None,
                },
                StatementIR::LetStmtIR {
                    name: "or".into(),
                    value: Some(ExprIR::BinaryOpExpr {
                        op: "or".into(),
                        left: Box::new(ExprIR::IdentifierExpr {
                            name: "flag".into(),
                            type_ann: None,
                        }),
                        right: Box::new(ExprIR::LiteralExpr {
                            value: serde_json::json!(false),
                        }),
                    }),
                    r#type: None,
                },
                StatementIR::LetStmtIR {
                    name: "neg".into(),
                    value: Some(ExprIR::UnaryOpExpr {
                        op: "neg".into(),
                        operand: Box::new(ExprIR::IdentifierExpr {
                            name: "a".into(),
                            type_ann: None,
                        }),
                    }),
                    r#type: None,
                },
                StatementIR::LetStmtIR {
                    name: "not".into(),
                    value: Some(ExprIR::UnaryOpExpr {
                        op: "not".into(),
                        operand: Box::new(ExprIR::IdentifierExpr {
                            name: "flag".into(),
                            type_ann: None,
                        }),
                    }),
                    r#type: None,
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
                            op: "add".into(),
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
                            },
                        ],
                    }),
                    r#type: None,
                },
            ],
        },
        is_async: false,
    };
    IRProgram {
        version: "0.1".into(),
        modules: HashMap::from([(
            "main".to_string(),
            ModuleIR {
                name: "main".to_string(),
                functions: HashMap::from([("testExpressions".into(), func)]),
                ..Default::default()
            },
        )]),
        source_map: HashMap::new(),
        stdlib_imports: Vec::new(),
    }
}

// ═════════════════════════════════════════════════════════════════════════════
// Snapshot helper
// ═════════════════════════════════════════════════════════════════════════════

/// Concatenate all output files into a single snapshot string.
pub(crate) fn format_output(output: &CompileOutput) -> String {
    let mut s = String::new();
    for f in &output.files {
        s.push_str(&format!("=== {} ===\n", f.path));
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
