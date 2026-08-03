// ═══════════════════════════════════════════════════════════
// Unit tests
// ═══════════════════════════════════════════════════════════

use rustc_hash::FxHashMap;
use rustc_hash::FxHashSet;
use std::fmt::Write;

use super::expr::*;
#[allow(unused_imports)]
use super::utils::*;
use super::ReactCodegen;
use crate::codegen::shared::build_component_map;
use crate::codegen::{
    binary_op_to_str, camel_to_kebab, capitalize_first, format_json_value, CodeWriter, Codegen,
    JSON_REACT,
};
use crate::error::DiagnosticKind;
use crate::ir::*;
use crate::ir::{ConstructionKind, ModuleIR};
use crate::path_mapping;

// ── Helper tests ─────────────────────────────────────────

#[test]
fn test_capitalize_first() {
    assert_eq!(capitalize_first("hello"), "Hello");
    assert_eq!(capitalize_first(""), "");
    assert_eq!(capitalize_first("a"), "A");
    assert_eq!(capitalize_first("ABC"), "ABC");
}

#[test]
fn test_format_json_value_tsx() {
    assert_eq!(
        format_json_value(&serde_json::Value::Null, &JSON_REACT),
        "null"
    );
    assert_eq!(
        format_json_value(&serde_json::json!(true), &JSON_REACT),
        "true"
    );
    assert_eq!(
        format_json_value(&serde_json::json!(false), &JSON_REACT),
        "false"
    );
    assert_eq!(format_json_value(&serde_json::json!(42), &JSON_REACT), "42");
    assert_eq!(
        format_json_value(&serde_json::json!(2.71), &JSON_REACT),
        "2.71"
    );
    assert_eq!(
        format_json_value(&serde_json::json!("hello"), &JSON_REACT),
        "\"hello\""
    );
    assert_eq!(
        format_json_value(&serde_json::json!("he\"llo"), &JSON_REACT),
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
fn test_to_jsx_attr() {
    assert_eq!(to_jsx_attr("class"), "className");
    assert_eq!(to_jsx_attr("for"), "htmlFor");
    assert_eq!(to_jsx_attr("tabindex"), "tabIndex");
    assert_eq!(to_jsx_attr("onclick"), "onclick");
    assert_eq!(to_jsx_attr("autocomplete"), "autoComplete");
}

#[test]
fn test_to_react_event() {
    assert_eq!(to_react_event("click"), "onClick");
    assert_eq!(to_react_event("change"), "onChange");
    assert_eq!(to_react_event("submit"), "onSubmit");
    assert_eq!(to_react_event("focus"), "onFocus");
    assert_eq!(to_react_event("dblclick"), "onDoubleClick");
    assert_eq!(to_react_event("keydown"), "onKeyDown");
    assert_eq!(to_react_event("customEvent"), "onCustomEvent");
}

#[test]
fn test_binary_op_to_str() {
    assert_eq!(binary_op_to_str(&BinaryOpIR::Add, true, false), "+");
    assert_eq!(binary_op_to_str(&BinaryOpIR::Eq, true, false), "===");
    assert_eq!(binary_op_to_str(&BinaryOpIR::And, true, false), "&&");
    assert_eq!(binary_op_to_str(&BinaryOpIR::Or, true, false), "||");
    assert_eq!(binary_op_to_str(&BinaryOpIR::Neq, true, false), "!==");
    // C-loose eq
    assert_eq!(binary_op_to_str(&BinaryOpIR::Eq, false, false), "==");
    assert_eq!(binary_op_to_str(&BinaryOpIR::Neq, false, false), "!=");
    // Python textual bool
    assert_eq!(binary_op_to_str(&BinaryOpIR::And, false, true), "and");
    assert_eq!(binary_op_to_str(&BinaryOpIR::Or, false, true), "or");
}

// ── format_expr_ir_tsx tests ────────────────────────────

#[test]
fn test_format_expr_literal() {
    let expr = ExprIR::LiteralExpr {
        value: serde_json::json!(42),
    };
    assert_eq!(format_expr_ir_tsx(&expr, &FxHashSet::default()), "42");
}

#[test]
fn test_format_expr_identifier() {
    let expr = ExprIR::IdentifierExpr {
        name: "count".into(),
        type_ann: None,
    };
    assert_eq!(format_expr_ir_tsx(&expr, &FxHashSet::default()), "count");
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
    assert_eq!(format_expr_ir_tsx(&expr, &FxHashSet::default()), "a + b");
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
    assert_eq!(
        format_expr_ir_tsx(&expr, &FxHashSet::default()),
        "x ? 1 : 0"
    );
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
    assert_eq!(format_expr_ir_tsx(&expr, &FxHashSet::default()), "obj.prop");
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
    assert_eq!(format_expr_ir_tsx(&expr, &FxHashSet::default()), "!flag");
}

#[test]
fn test_format_expr_store_field() {
    let expr = ExprIR::StoreFieldAccessExpr {
        store_alias: "store".into(),
        field_name: "getCount".into(),
        call_args: vec![],
    };
    assert_eq!(
        format_expr_ir_tsx(&expr, &FxHashSet::default()),
        "store.getCount"
    );
}

// ── format_match tests ─────────────────────────────────

/// Literal arms must compare the subject value itself (`_v === 0`), not
/// `_v._variant === 0` — a raw primitive (const `_v = n`) has no `_variant`
/// field, so the old shape never matched at runtime. Variant arms keep the
/// `_variant` dispatch and bindings still destructure from the subject.
#[test]
fn test_format_match_literal_arms_compare_subject_value() {
    let expr = ExprIR::MatchExpr {
        matched: Box::new(ExprIR::IdentifierExpr {
            name: "n".into(),
            type_ann: None,
        }),
        arms: vec![
            MatchArmIR {
                pattern: MatchPatternIR {
                    kind: MatchPatternKindIR::Literal {
                        value: serde_json::json!(0),
                    },
                },
                body: BlockIR {
                    statements: vec![StatementIR::ReturnStmtIR {
                        value: Some(ExprIR::LiteralExpr {
                            value: serde_json::json!("zero"),
                        }),
                    }],
                },
                union_type: None,
            },
            MatchArmIR {
                pattern: MatchPatternIR {
                    kind: MatchPatternKindIR::Literal {
                        value: serde_json::json!("seven"),
                    },
                },
                body: BlockIR {
                    statements: vec![StatementIR::ReturnStmtIR {
                        value: Some(ExprIR::LiteralExpr {
                            value: serde_json::json!("seven"),
                        }),
                    }],
                },
                union_type: None,
            },
            MatchArmIR {
                pattern: MatchPatternIR {
                    kind: MatchPatternKindIR::Variant {
                        name: "Star".into(),
                        bindings: vec!["level".into()],
                    },
                },
                body: BlockIR {
                    statements: vec![StatementIR::ReturnStmtIR {
                        value: Some(ExprIR::IdentifierExpr {
                            name: "level".into(),
                            type_ann: None,
                        }),
                    }],
                },
                union_type: None,
            },
            MatchArmIR {
                pattern: MatchPatternIR {
                    kind: MatchPatternKindIR::Wildcard,
                },
                body: BlockIR {
                    statements: vec![StatementIR::ReturnStmtIR {
                        value: Some(ExprIR::LiteralExpr {
                            value: serde_json::json!("many"),
                        }),
                    }],
                },
                union_type: None,
            },
        ],
    };
    let out = format_expr_ir_tsx(&expr, &FxHashSet::default());
    assert!(
        out.contains("_v === 0"),
        "int literal arm should compare the subject value. Output: {}",
        out
    );
    assert!(
        out.contains("_v === \"seven\""),
        "string literal arm should compare the subject value. Output: {}",
        out
    );
    assert!(
        out.contains("_v._variant === \"Star\""),
        "variant arm should keep _variant dispatch. Output: {}",
        out
    );
    assert!(
        !out.contains("_v._variant === 0"),
        "literal arm must not dispatch on _variant. Output: {}",
        out
    );
    assert!(
        out.contains("const { level } = _v;"),
        "variant bindings should still destructure the subject. Output: {}",
        out
    );
}

// ── generate_statement_tsx tests ─────────────────────────

#[test]
fn test_tsx_stmt_let() {
    let mut w = CodeWriter::new();
    let stmt = StatementIR::LetStmtIR {
        name: "x".into(),
        value: Some(ExprIR::LiteralExpr {
            value: serde_json::json!(42),
        }),
        r#type: None,
    };
    generate_statement_tsx(
        &mut w,
        &stmt,
        &FxHashSet::default(),
        &FxHashMap::default(),
        &FxHashSet::default(),
        &FxHashMap::default(),
    );
    assert_eq!(w.into_string(), "let x = 42;\n");
}

#[test]
fn test_tsx_stmt_return() {
    let mut w = CodeWriter::new();
    let stmt = StatementIR::ReturnStmtIR {
        value: Some(ExprIR::IdentifierExpr {
            name: "result".into(),
            type_ann: None,
        }),
    };
    generate_statement_tsx(
        &mut w,
        &stmt,
        &FxHashSet::default(),
        &FxHashMap::default(),
        &FxHashSet::default(),
        &FxHashMap::default(),
    );
    assert_eq!(w.into_string(), "return result;\n");
}

#[test]
fn test_tsx_stmt_mutation_increment_local() {
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
    // owner=None + not in signal_names → direct assignment
    generate_statement_tsx(
        &mut w,
        &stmt,
        &FxHashSet::default(),
        &FxHashMap::default(),
        &FxHashSet::default(),
        &FxHashMap::default(),
    );
    assert_eq!(w.into_string(), "count = count + 1;\n");
}

#[test]
fn test_tsx_stmt_mutation_increment_signal() {
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
    // owner=None but "count" IS in signal_names → setter call
    let mut sig_names = FxHashSet::default();
    sig_names.insert("count".to_string());
    generate_statement_tsx(
        &mut w,
        &stmt,
        &sig_names,
        &FxHashMap::default(),
        &FxHashSet::default(),
        &FxHashMap::default(),
    );
    assert_eq!(w.into_string(), "setCount(prev => prev + 1);\n");
}

#[test]
fn test_tsx_stmt_mutation_assign_local() {
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
    // owner=None + not in signal_names → direct assignment
    generate_statement_tsx(
        &mut w,
        &stmt,
        &FxHashSet::default(),
        &FxHashMap::default(),
        &FxHashSet::default(),
        &FxHashMap::default(),
    );
    assert_eq!(w.into_string(), "name = \"alice\";\n");
}

#[test]
fn test_tsx_stmt_mutation_assign_store() {
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
    // Note: `owner` field removed from SignalRefIR — store signal
    // detection via target.owner is no longer available, so this
    // falls through to direct assignment.
    generate_statement_tsx(
        &mut w,
        &stmt,
        &FxHashSet::default(),
        &FxHashMap::default(),
        &FxHashSet::default(),
        &FxHashMap::default(),
    );
    assert_eq!(w.into_string(), "name = \"alice\";\n");
}

// ── Task 3.5 M2: functional updaters (setCount((c) => c + 1)) ──

/// `count = count + 1` on a component signal must emit the functional
/// updater form — the direct `setCount(count + 1)` closes over a snapshot
/// and loses updates under React batching / StrictMode double-invoke.
#[test]
fn test_tsx_stmt_mutation_assign_signal_self_read_functional_updater() {
    let mut w = CodeWriter::new();
    let stmt = StatementIR::MutationStmt {
        intent: Some(MutationIntent::DirectAssign {
            target: SignalRefIR {
                name: "count".into(),
            },
            value: Some(Box::new(ExprIR::BinaryOpExpr {
                op: BinaryOpIR::Add,
                left: Box::new(ExprIR::ident("count")),
                right: Box::new(ExprIR::LiteralExpr {
                    value: serde_json::json!(1),
                }),
            })),
            field_object: None,
        }),
    };
    let mut sig_names = FxHashSet::default();
    sig_names.insert("count".to_string());
    generate_statement_tsx(
        &mut w,
        &stmt,
        &sig_names,
        &FxHashMap::default(),
        &FxHashSet::default(),
        &FxHashMap::default(),
    );
    assert_eq!(w.into_string(), "setCount((c) => c + 1);\n");
}

/// The same-signal read must only trigger the functional form — a value
/// that does NOT read the target keeps the direct setter form.
#[test]
fn test_tsx_stmt_mutation_assign_signal_external_value_keeps_direct() {
    let mut w = CodeWriter::new();
    let stmt = StatementIR::MutationStmt {
        intent: Some(MutationIntent::DirectAssign {
            target: SignalRefIR {
                name: "count".into(),
            },
            value: Some(Box::new(ExprIR::ident("step"))),
            field_object: None,
        }),
    };
    let mut sig_names = FxHashSet::default();
    sig_names.insert("count".to_string());
    generate_statement_tsx(
        &mut w,
        &stmt,
        &sig_names,
        &FxHashMap::default(),
        &FxHashSet::default(),
        &FxHashMap::default(),
    );
    assert_eq!(w.into_string(), "setCount(step);\n");
}

/// `AppStore.count = AppStore.count + 1` on a store signal must emit the
/// functional `useAppStore.setState((s) => ({ count: s.count + 1 }))` —
/// the object form `setState({ count: AppStore.count + 1 })` reads a stale
/// slice/snapshot under batching.
#[test]
fn test_tsx_stmt_mutation_assign_store_signal_self_read_functional() {
    let mut w = CodeWriter::new();
    let stmt = StatementIR::MutationStmt {
        intent: Some(MutationIntent::DirectAssign {
            target: SignalRefIR {
                name: "count".into(),
            },
            value: Some(Box::new(ExprIR::BinaryOpExpr {
                op: BinaryOpIR::Add,
                left: Box::new(ExprIR::FieldAccessExpr {
                    object: Box::new(ExprIR::ident("AppStore")),
                    field: "count".into(),
                }),
                right: Box::new(ExprIR::LiteralExpr {
                    value: serde_json::json!(1),
                }),
            })),
            field_object: None,
        }),
    };
    let mut owners = FxHashMap::default();
    owners.insert("count".to_string(), "AppStore".to_string());
    generate_statement_tsx(
        &mut w,
        &stmt,
        &FxHashSet::default(),
        &owners,
        &FxHashSet::default(),
        &FxHashMap::default(),
    );
    assert_eq!(
        w.into_string(),
        "useAppStore.setState((s) => ({ count: s.count + 1 }));\n"
    );
}

/// The functional-updater parameter must not collide with identifiers the
/// value expression keeps (`count = count + c` → param `c` is taken → `c1`).
#[test]
fn test_tsx_stmt_mutation_assign_signal_updater_param_avoids_collision() {
    let mut w = CodeWriter::new();
    let stmt = StatementIR::MutationStmt {
        intent: Some(MutationIntent::DirectAssign {
            target: SignalRefIR {
                name: "count".into(),
            },
            value: Some(Box::new(ExprIR::BinaryOpExpr {
                op: BinaryOpIR::Add,
                left: Box::new(ExprIR::ident("count")),
                right: Box::new(ExprIR::ident("c")),
            })),
            field_object: None,
        }),
    };
    let mut sig_names = FxHashSet::default();
    sig_names.insert("count".to_string());
    generate_statement_tsx(
        &mut w,
        &stmt,
        &sig_names,
        &FxHashMap::default(),
        &FxHashSet::default(),
        &FxHashMap::default(),
    );
    assert_eq!(w.into_string(), "setCount((c1) => c1 + c);\n");
}

#[test]
fn test_tsx_stmt_if() {
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
            }],
        },
        else_ifs: vec![],
        else_body: None,
    };
    generate_statement_tsx(
        &mut w,
        &stmt,
        &FxHashSet::default(),
        &FxHashMap::default(),
        &FxHashSet::default(),
        &FxHashMap::default(),
    );
    let out = w.into_string();
    assert!(out.contains("if (cond) {"));
    assert!(out.contains("let x = 1;"));
    assert!(out.contains("}"));
}

// ── gen_component_css tests ──────────────────────────────

#[test]
fn test_css_empty_component() {
    let codegen = ReactCodegen::default();
    let comp = ComponentIR {
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
    };
    let css = codegen.gen_component_css("App", &comp);
    assert!(css.contains(".container"));
    assert!(css.contains("/* Styles for App */"));
}

#[test]
fn test_css_with_styles() {
    let codegen = ReactCodegen::default();
    let mut props = FxHashMap::default();
    props.insert(
        "backgroundColor".into(),
        ExprIR::LiteralExpr {
            value: serde_json::json!("red"),
        },
    );
    props.insert(
        "fontSize".into(),
        ExprIR::LiteralExpr {
            value: serde_json::json!(16),
        },
    );
    let style = StyleRuleIR {
        class_name: "header".into(),
        properties: props,
        output_mode: "static".into(),
        is_tag_selector: false,
    };
    let comp = ComponentIR {
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
        styles: vec![style],
        ..Default::default()
    };
    let css = codegen.gen_component_css("App", &comp);
    assert!(css.contains(".header"));
    assert!(css.contains("background-color: red;"));
    assert!(css.contains("font-size: 16;"));
}

// ── gen_component_tsx tests ──────────────────────────────

fn make_test_component(name: &str, view: ViewTree) -> ComponentIR {
    ComponentIR {
        name: name.into(),
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
        view: Some(view),
        styles: vec![],
        ..Default::default()
    }
}

/// Build IRProgram with a single module containing the given components.
fn make_ir(components: FxHashMap<String, ComponentIR>) -> IRProgram {
    IRProgram {
        version: "1.0".into(),
        modules: vec![(
            "main".to_string(),
            ModuleIR {
                name: "main".to_string(),
                components,
                ..Default::default()
            },
        )]
        .into_iter()
        .collect::<FxHashMap<_, _>>(),
        stdlib_imports: Default::default(),
        source_map: FxHashMap::default(),
        source_span: None,
        motion_index: FxHashMap::default(),
    }
}

#[test]
fn test_tsx_interpolation_renders_expr() {
    let _codegen = ReactCodegen::default();
    let comp = make_test_component(
        "Test",
        ViewTree::Element(Box::new(ElementIR {
            tag: "div".into(),
            props: FxHashMap::default(),
            bindings: FxHashMap::default(),
            events: FxHashMap::default(),
            style_classes: None,
            inline_style: None,
            children: vec![ViewTree::Interpolation(Box::new(ExprIR::IdentifierExpr {
                name: "count".into(),
                type_ann: None,
            }))],
            class_map_bindings: FxHashMap::default(),
            motion: None,
            ref_binding: None,
            semantic_bindings: FxHashMap::default(),
            a11y: None,
            construction_kind: ConstructionKind::Static,
            source_span: None,
        })),
    );
    let ir = make_ir(
        vec![("Test".into(), comp)]
            .into_iter()
            .collect::<FxHashMap<_, _>>(),
    );
    let comp_ref = ir
        .modules
        .get("main")
        .unwrap()
        .components
        .get("Test")
        .unwrap();
    let component_map = build_component_map(&ir);
    let tsx = ReactCodegen::default().gen_component_tsx("Test", comp_ref, &ir, &component_map);
    assert!(
        !tsx.contains("/* interpolation */"),
        "Interpolation must NOT be a comment placeholder. Got: {}",
        tsx
    );
    assert!(
        tsx.contains("{count}"),
        "Should contain {{count}} JSX expression. Got: {}",
        tsx
    );
}

#[test]
fn test_tsx_empty_component() {
    let codegen = ReactCodegen::default();
    let mut components = FxHashMap::default();
    components.insert(
        "App".into(),
        ComponentIR {
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
    let tsx = codegen.gen_component_tsx("App", comp_ref, &ir, &FxHashMap::default());
    assert!(tsx.contains("export function App() {"));
}

#[test]
fn test_tsx_component_with_state() {
    let codegen = ReactCodegen::default();
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
    let tsx = codegen.gen_component_tsx("Counter", comp_ref, &ir, &FxHashMap::default());
    assert!(tsx.contains("export function Counter() {"));
}

#[test]
fn test_tsx_component_with_props() {
    let codegen = ReactCodegen::default();
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
    let tsx = codegen.gen_component_tsx("Greeting", comp_ref, &ir, &FxHashMap::default());
    assert!(tsx.contains("interface GreetingProps {"));
    assert!(tsx.contains("name: string;"));
    assert!(tsx.contains("age: number;"));
    assert!(tsx.contains("export function Greeting("));
    assert!(tsx.contains("}: GreetingProps) {"));
}

#[test]
fn test_tsx_component_with_effect() {
    let codegen = ReactCodegen::default();
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
    let tsx = codegen.gen_component_tsx("Watcher", comp_ref, &ir, &FxHashMap::default());
    assert!(tsx.contains("useEffect(() => {"));
    assert!(tsx.contains("}, [count]);"));
}

#[test]
fn test_tsx_component_with_lifecycle_mount() {
    let codegen = ReactCodegen::default();
    let lc = LifecycleIntent {
        kind: LifecycleKind::Mount,
        body: BlockIR { statements: vec![] },
    };
    let mut components = FxHashMap::default();
    components.insert(
        "App".into(),
        ComponentIR {
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
    let tsx = codegen.gen_component_tsx("App", comp_ref, &ir, &FxHashMap::default());
    assert!(tsx.contains("useEffect(() => {"));
    assert!(tsx.contains("}, []);"));
}

// ── Zustand store codegen tests ───────────────────────────

#[test]
fn test_zustand_store_empty() {
    let codegen = ReactCodegen::default();
    let store = StoreIR {
        name: "AppStore".into(),
        signals: FxHashMap::default(),
        signal_defaults: FxHashMap::default(),
        getters: FxHashMap::default(),
        actions: vec![],
        ..Default::default()
    };
    let result = codegen.gen_zustand_store("AppStore", &store);
    assert!(result.contains("import { create } from 'zustand';"));
    assert!(result.contains("interface AppStoreState {"));
    assert!(result.contains("export const useAppStore"));
}

#[test]
fn test_zustand_store_with_signals_and_getters() {
    let codegen = ReactCodegen::default();
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
    let mut getter_types = FxHashMap::default();
    getter_types.insert(
        "doubleCount".into(),
        TypeIR {
            kind: "i32".into(),
            type_args: vec![],
            is_nullable: false,
        },
    );
    let store = StoreIR {
        name: "CalcStore".into(),
        signals,
        signal_defaults: defaults,
        getters,
        getter_types,
        actions: vec![],
        ..Default::default()
    };
    let result = codegen.gen_zustand_store("CalcStore", &store);
    assert!(
        result.contains("count: 0,"),
        "should emit signal default: {}",
        result
    );
    assert!(
        result.contains("name: \"world\","),
        "should emit string default: {}",
        result
    );
    assert!(
        result.contains("get doubleCount()"),
        "should emit getter accessor: {}",
        result
    );
    assert!(
        result.contains("readonly doubleCount: number;"),
        "should emit typed getter return: {}",
        result
    );
    assert!(
        result.contains("export function useCount() {"),
        "should emit standalone signal selector hook: {}",
        result
    );
    assert!(
        result.contains("export function useDoubleCount() {"),
        "should emit standalone getter selector hook: {}",
        result
    );
    assert!(
        result.contains("return useCalcStore((s) => s.count);"),
        "selector hook should subscribe via the store hook: {}",
        result
    );
}

#[test]
fn test_zustand_store_mutation_multiply_in_action() {
    let codegen = ReactCodegen::default();
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
    defaults.insert("count".into(), serde_json::json!(1));
    let store = StoreIR {
        name: "CalcStore".into(),
        signals,
        signal_defaults: defaults,
        getters: FxHashMap::default(),
        actions: vec![FunctionIR {
            name: "multiply".into(),
            visibility: Visibility::Public,
            params: vec![ParamIR {
                name: "factor".into(),
                r#type: Some(TypeIR {
                    kind: "i32".into(),
                    type_args: vec![],
                    is_nullable: false,
                }),
            }],
            return_type: None,
            body: BlockIR {
                statements: vec![StatementIR::MutationStmt {
                    intent: Some(MutationIntent::Multiply {
                        target: SignalRefIR {
                            name: "count".into(),
                        },
                        amount: Some(Box::new(ExprIR::IdentifierExpr {
                            name: "factor".into(),
                            type_ann: None,
                        })),
                    }),
                }],
            },
            is_async: false,
            source_span: None,
        }],
        ..Default::default()
    };
    let result = codegen.gen_zustand_store("CalcStore", &store);
    assert!(
        result.contains("set({ count: state.count * factor })") || result.contains("count *= "),
        "should emit multiply mutation, got: {}",
        result
    );
    assert!(
        !result.contains("TODO"),
        "should NOT contain TODO comment, got: {}",
        result
    );
    assert!(
        result.contains("multiply: (factor: number) => void;"),
        "should emit typed action param, got: {}",
        result
    );
    assert!(
        result.contains("export function useMultiply() {"),
        "should emit standalone action selector hook, got: {}",
        result
    );
}

/// Regression (CRITICAL): `items = [...items, {...}]` must emit the
/// `const state = get();` prologue and rename `items` to `state.items`.
/// The prologue walker must recurse into list/object literals.
#[test]
fn test_zustand_store_array_literal_mutation_emits_prologue() {
    let codegen = ReactCodegen::default();
    let mut signals = FxHashMap::default();
    signals.insert(
        "items".into(),
        TypeIR {
            kind: "Vec".into(),
            type_args: vec![TypeIR {
                kind: "string".into(),
                type_args: vec![],
                is_nullable: false,
            }],
            is_nullable: false,
        },
    );
    let mut defaults = FxHashMap::default();
    defaults.insert("items".into(), serde_json::json!([]));
    let store = StoreIR {
        name: "TodoStore".into(),
        signals,
        signal_defaults: defaults,
        getters: FxHashMap::default(),
        actions: vec![FunctionIR {
            name: "add".into(),
            visibility: Visibility::Public,
            params: vec![ParamIR {
                name: "text".into(),
                r#type: Some(TypeIR {
                    kind: "string".into(),
                    type_args: vec![],
                    is_nullable: false,
                }),
            }],
            return_type: None,
            body: BlockIR {
                statements: vec![StatementIR::MutationStmt {
                    intent: Some(MutationIntent::DirectAssign {
                        target: SignalRefIR {
                            name: "items".into(),
                        },
                        value: Some(Box::new(ExprIR::ArrayExpr {
                            elements: vec![
                                ExprIR::SpreadExpr {
                                    operand: Box::new(ExprIR::IdentifierExpr {
                                        name: "items".into(),
                                        type_ann: None,
                                    }),
                                },
                                ExprIR::MapExpr {
                                    entries: vec![
                                        (
                                            ExprIR::LiteralExpr {
                                                value: serde_json::json!("text"),
                                            },
                                            ExprIR::IdentifierExpr {
                                                name: "text".into(),
                                                type_ann: None,
                                            },
                                        ),
                                        (
                                            ExprIR::LiteralExpr {
                                                value: serde_json::json!("done"),
                                            },
                                            ExprIR::LiteralExpr {
                                                value: serde_json::json!(false),
                                            },
                                        ),
                                    ],
                                },
                            ],
                        })),
                        field_object: None,
                    }),
                }],
            },
            is_async: false,
            source_span: None,
        }],
        ..Default::default()
    };
    let result = codegen.gen_zustand_store("TodoStore", &store);
    assert!(
        result.contains("const state = get();"),
        "array-literal mutation must emit the prologue, got: {}",
        result
    );
    assert!(
        result.contains("set({ items: [...state.items,"),
        "items must be renamed to state.items inside the array literal, got: {}",
        result
    );
    assert!(
        result.contains("\"text\": text"),
        "object literal values must keep local identifiers, got: {}",
        result
    );
}

// ── Task 3.5 M4: consecutive set() calls coalesce into one object ──

/// Three consecutive store-signal mutations in one action body must
/// coalesce into a SINGLE `set({ k1: v1, k2: v2, k3: v3 });` call — three
/// sequential sets trigger three subscriber notifications for one logical
/// update (review M4).
#[test]
fn test_zustand_store_consecutive_mutations_coalesce_into_single_set() {
    let codegen = ReactCodegen::default();
    let mut signals = FxHashMap::default();
    signals.insert(
        "items".into(),
        TypeIR {
            kind: "Vec".into(),
            type_args: vec![TypeIR {
                kind: "string".into(),
                type_args: vec![],
                is_nullable: false,
            }],
            is_nullable: false,
        },
    );
    signals.insert(
        "nextId".into(),
        TypeIR {
            kind: "i32".into(),
            type_args: vec![],
            is_nullable: false,
        },
    );
    signals.insert(
        "lastAction".into(),
        TypeIR {
            kind: "string".into(),
            type_args: vec![],
            is_nullable: false,
        },
    );
    let mut defaults = FxHashMap::default();
    defaults.insert("items".into(), serde_json::json!([]));
    defaults.insert("nextId".into(), serde_json::json!(0));
    defaults.insert("lastAction".into(), serde_json::json!("none"));
    let store = StoreIR {
        name: "TodoStore".into(),
        signals,
        signal_defaults: defaults,
        getters: FxHashMap::default(),
        actions: vec![FunctionIR {
            name: "add".into(),
            visibility: Visibility::Public,
            params: vec![],
            return_type: None,
            body: BlockIR {
                statements: vec![
                    StatementIR::MutationStmt {
                        intent: Some(MutationIntent::DirectAssign {
                            target: SignalRefIR {
                                name: "items".into(),
                            },
                            value: Some(Box::new(ExprIR::ArrayExpr {
                                elements: vec![
                                    ExprIR::SpreadExpr {
                                        operand: Box::new(ExprIR::ident("items")),
                                    },
                                    ExprIR::LiteralExpr {
                                        value: serde_json::json!("new"),
                                    },
                                ],
                            })),
                            field_object: None,
                        }),
                    },
                    StatementIR::MutationStmt {
                        intent: Some(MutationIntent::Increment {
                            target: SignalRefIR {
                                name: "nextId".into(),
                            },
                            amount: Some(Box::new(ExprIR::LiteralExpr {
                                value: serde_json::json!(1),
                            })),
                        }),
                    },
                    StatementIR::MutationStmt {
                        intent: Some(MutationIntent::DirectAssign {
                            target: SignalRefIR {
                                name: "lastAction".into(),
                            },
                            value: Some(Box::new(ExprIR::LiteralExpr {
                                value: serde_json::json!("added"),
                            })),
                            field_object: None,
                        }),
                    },
                ],
            },
            is_async: false,
            source_span: None,
        }],
        ..Default::default()
    };
    let result = codegen.gen_zustand_store("TodoStore", &store);
    let add_idx = result.find("add: () => {").expect("action emitted");
    let add_block = &result[add_idx..];
    assert!(
        add_block.contains("set({ items: [...state.items, \"new\"], nextId: state.nextId + 1, lastAction: \"added\" });"),
        "three consecutive mutations must coalesce into ONE set() with all keys, got:\n{}",
        add_block
    );
    assert_eq!(
        add_block.matches("set(").count(),
        1,
        "exactly one set() call expected after coalescing, got:\n{}",
        add_block
    );
}

/// A repeated target in a run must NOT be coalesced — two keys of the same
/// name in one object literal would silently drop the earlier write.
#[test]
fn test_zustand_store_duplicate_target_mutations_not_coalesced() {
    let codegen = ReactCodegen::default();
    let mut signals = FxHashMap::default();
    signals.insert(
        "items".into(),
        TypeIR {
            kind: "Vec".into(),
            type_args: vec![TypeIR {
                kind: "string".into(),
                type_args: vec![],
                is_nullable: false,
            }],
            is_nullable: false,
        },
    );
    let mut defaults = FxHashMap::default();
    defaults.insert("items".into(), serde_json::json!([]));
    let store = StoreIR {
        name: "TodoStore".into(),
        signals,
        signal_defaults: defaults,
        getters: FxHashMap::default(),
        actions: vec![FunctionIR {
            name: "addTwo".into(),
            visibility: Visibility::Public,
            params: vec![],
            return_type: None,
            body: BlockIR {
                statements: vec![
                    StatementIR::MutationStmt {
                        intent: Some(MutationIntent::Push {
                            target: SignalRefIR {
                                name: "items".into(),
                            },
                            item: Some(Box::new(ExprIR::LiteralExpr {
                                value: serde_json::json!("a"),
                            })),
                        }),
                    },
                    StatementIR::MutationStmt {
                        intent: Some(MutationIntent::Push {
                            target: SignalRefIR {
                                name: "items".into(),
                            },
                            item: Some(Box::new(ExprIR::LiteralExpr {
                                value: serde_json::json!("b"),
                            })),
                        }),
                    },
                ],
            },
            is_async: false,
            source_span: None,
        }],
        ..Default::default()
    };
    let result = codegen.gen_zustand_store("TodoStore", &store);
    let add_idx = result.find("addTwo: () => {").expect("action emitted");
    let add_block = &result[add_idx..];
    assert_eq!(
        add_block.matches("set(").count(),
        2,
        "duplicate-target mutations must stay separate set() calls, got:\n{}",
        add_block
    );
    assert!(
        add_block.contains("set({ items: [...state.items, \"a\"] });")
            && add_block.contains("set({ items: [...state.items, \"b\"] });"),
        "each push keeps its own set() call, got:\n{}",
        add_block
    );
}

/// Regression (HIGH #1): getter references inside action bodies must be
/// renamed to `state.<getter>` and trigger the prologue.
#[test]
fn test_zustand_store_getter_ref_renamed_in_action() {
    let codegen = ReactCodegen::default();
    let mut signals = FxHashMap::default();
    signals.insert(
        "notifications".into(),
        TypeIR {
            kind: "Vec".into(),
            type_args: vec![TypeIR {
                kind: "string".into(),
                type_args: vec![],
                is_nullable: false,
            }],
            is_nullable: false,
        },
    );
    let mut defaults = FxHashMap::default();
    defaults.insert("notifications".into(), serde_json::json!([]));
    let mut getters = FxHashMap::default();
    getters.insert(
        "unreadCount".into(),
        ExprIR::MethodCallExpr {
            object: Box::new(ExprIR::IdentifierExpr {
                name: "notifications".into(),
                type_ann: None,
            }),
            method: "len".into(),
            args: vec![],
            is_optional: false,
        },
    );
    let mut getter_types = FxHashMap::default();
    getter_types.insert(
        "unreadCount".into(),
        TypeIR {
            kind: "i32".into(),
            type_args: vec![],
            is_nullable: false,
        },
    );
    let store = StoreIR {
        name: "NotificationStore".into(),
        signals,
        signal_defaults: defaults,
        getters,
        getter_types,
        actions: vec![FunctionIR {
            name: "logUnread".into(),
            visibility: Visibility::Public,
            params: vec![],
            return_type: None,
            body: BlockIR {
                statements: vec![StatementIR::LetStmtIR {
                    name: "summary".into(),
                    value: Some(ExprIR::MethodCallExpr {
                        object: Box::new(ExprIR::IdentifierExpr {
                            name: "unreadCount".into(),
                            type_ann: None,
                        }),
                        method: "to_string".into(),
                        args: vec![],
                        is_optional: false,
                    }),
                    r#type: None,
                }],
            },
            is_async: false,
            source_span: None,
        }],
        ..Default::default()
    };
    let result = codegen.gen_zustand_store("NotificationStore", &store);
    assert!(
        result.contains("const state = get();"),
        "getter read in action must emit the prologue, got: {}",
        result
    );
    assert!(
        result.contains("state.unreadCount.toString()"),
        "getter reference must be renamed to state.unreadCount, got: {}",
        result
    );
}

/// Regression (HIGH #2): string literals containing signal names must NOT be
/// renamed — the rename happens at the AST identifier level only.
#[test]
fn test_zustand_store_string_literal_not_renamed() {
    let codegen = ReactCodegen::default();
    let mut signals = FxHashMap::default();
    signals.insert(
        "items".into(),
        TypeIR {
            kind: "Vec".into(),
            type_args: vec![TypeIR {
                kind: "string".into(),
                type_args: vec![],
                is_nullable: false,
            }],
            is_nullable: false,
        },
    );
    let mut defaults = FxHashMap::default();
    defaults.insert("items".into(), serde_json::json!([]));
    let store = StoreIR {
        name: "CartStore".into(),
        signals,
        signal_defaults: defaults,
        getters: FxHashMap::default(),
        actions: vec![FunctionIR {
            name: "logCart".into(),
            visibility: Visibility::Public,
            params: vec![],
            return_type: None,
            body: BlockIR {
                statements: vec![StatementIR::ExprStmtIR {
                    expr: ExprIR::CallExpr {
                        name: "println".into(),
                        args: vec![ExprIR::LiteralExpr {
                            value: serde_json::json!("items in cart"),
                        }],
                    },
                }],
            },
            is_async: false,
            source_span: None,
        }],
        ..Default::default()
    };
    let result = codegen.gen_zustand_store("CartStore", &store);
    assert!(
        result.contains("console.log(\"items in cart\")"),
        "string literal containing a signal name must stay intact, got: {}",
        result
    );
    assert!(
        !result.contains("state.items in cart"),
        "string literal must NOT be renamed, got: {}",
        result
    );
}

// ── Full pipeline through Codegen trait ──────────────────

#[test]
fn test_full_pipeline_empty() {
    let codegen = ReactCodegen::default();
    let ir = IRProgram {
        version: "0.1".into(),
        modules: vec![(
            "main".to_string(),
            ModuleIR {
                name: "main".to_string(),
                components: FxHashMap::default(),
                stores: FxHashMap::default(),
                ..Default::default()
            },
        )]
        .into_iter()
        .collect::<FxHashMap<_, _>>(),
        source_map: FxHashMap::default(),
        stdlib_imports: Default::default(),
        source_span: None,
        motion_index: FxHashMap::default(),
    };
    let sources = vec![("main.omni".into(), "".into())]
        .into_iter()
        .collect::<FxHashMap<_, _>>();
    let output = codegen.generate(&ir, &sources);
    assert!(output.errors.is_empty());
    assert!(output.files.is_empty());
}

#[test]
fn test_full_pipeline_component() {
    let codegen = ReactCodegen::default();
    let comp = ComponentIR {
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
        modules: vec![(
            "main".to_string(),
            ModuleIR {
                name: "main".to_string(),
                components: vec![("App".into(), comp)]
                    .into_iter()
                    .collect::<FxHashMap<_, _>>(),
                stores: FxHashMap::default(),
                ..Default::default()
            },
        )]
        .into_iter()
        .collect::<FxHashMap<_, _>>(),
        source_map: FxHashMap::default(),
        stdlib_imports: Default::default(),
        source_span: None,
        motion_index: FxHashMap::default(),
    };
    let sources = vec![("main.omni".into(), "".into())]
        .into_iter()
        .collect::<FxHashMap<_, _>>();
    let output = codegen.generate(&ir, &sources);
    assert!(output.errors.is_empty());
    assert_eq!(output.files.len(), 2); // .tsx + .module.css

    insta::with_settings!({snapshot_path => "snapshots"}, {
        let mut snapshot_content = String::new();
        for f in &output.files {
            let _ = write!(snapshot_content, "=== {} ===\n", f.path);
            snapshot_content.push_str(&f.content);
            snapshot_content.push('\n');
        }
        insta::assert_snapshot!("test_full_pipeline_component", &snapshot_content);
    });
}

#[test]
fn test_full_pipeline_with_store() {
    let codegen = ReactCodegen::default();
    let store = StoreIR {
        name: "AppStore".into(),
        signals: vec![(
            "count".into(),
            TypeIR {
                kind: "i32".into(),
                type_args: vec![],
                is_nullable: false,
            },
        )]
        .into_iter()
        .collect::<FxHashMap<_, _>>(),
        signal_defaults: vec![("count".into(), serde_json::json!(0))]
            .into_iter()
            .collect::<FxHashMap<_, _>>(),
        getters: FxHashMap::default(),
        actions: vec![],
        ..Default::default()
    };
    let ir = IRProgram {
        version: "0.1".into(),
        modules: vec![(
            "main".to_string(),
            ModuleIR {
                name: "main".to_string(),
                components: FxHashMap::default(),
                stores: vec![("AppStore".into(), store)]
                    .into_iter()
                    .collect::<FxHashMap<_, _>>(),
                ..Default::default()
            },
        )]
        .into_iter()
        .collect::<FxHashMap<_, _>>(),
        source_map: FxHashMap::default(),
        stdlib_imports: Default::default(),
        source_span: None,
        motion_index: FxHashMap::default(),
    };
    let sources = vec![("main.omni".into(), "".into())]
        .into_iter()
        .collect::<FxHashMap<_, _>>();
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
    let codegen = ReactCodegen::default();
    let store = StoreIR {
        name: "AppStore".into(),
        signals: vec![(
            "count".into(),
            TypeIR {
                kind: "i32".into(),
                type_args: vec![],
                is_nullable: false,
            },
        )]
        .into_iter()
        .collect::<FxHashMap<_, _>>(),
        signal_defaults: vec![("count".into(), serde_json::json!(0))]
            .into_iter()
            .collect::<FxHashMap<_, _>>(),
        getters: FxHashMap::default(),
        actions: vec![],
        ..Default::default()
    };
    let comp = ComponentIR {
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
        modules: vec![(
            "main".to_string(),
            ModuleIR {
                name: "main".to_string(),
                components: vec![("App".into(), comp)]
                    .into_iter()
                    .collect::<FxHashMap<_, _>>(),
                stores: vec![("AppStore".into(), store)]
                    .into_iter()
                    .collect::<FxHashMap<_, _>>(),
                ..Default::default()
            },
        )]
        .into_iter()
        .collect::<FxHashMap<_, _>>(),
        source_map: FxHashMap::default(),
        stdlib_imports: Default::default(),
        source_span: None,
        motion_index: FxHashMap::default(),
    };
    let sources = vec![("main.omni".into(), "".into())]
        .into_iter()
        .collect::<FxHashMap<_, _>>();
    let output = codegen.generate(&ir, &sources);
    assert!(output.errors.is_empty());
    assert_eq!(output.files.len(), 3); // .tsx + .module.css + useAppStore.ts
    let tsx = output.files.iter().find(|f| f.path == "App.tsx").unwrap();
    insta::with_settings!({snapshot_path => "snapshots"}, {
        insta::assert_snapshot!("test_full_pipeline_component_with_store_ref", &tsx.content);
    });
}

// ═══════════════════════════════════════════════════════════
// Integration tests (full pipeline)
// ═══════════════════════════════════════════════════════════

/// Compile a single .omni source string to React, returning the InternalCompileResult.
fn compile_react(source: &str) -> Option<crate::InternalCompileResult> {
    let mut sources = FxHashMap::default();
    sources.insert("input.omni".to_string(), source.to_string());
    let pm = path_mapping::derive_path_mapping(&sources);
    let result = crate::compile_multi_internal(
        sources,
        crate::codegen::Target::React,
        pm,
        false,
        false,
        None,
    );
    if result.success {
        Some(result)
    } else {
        None
    }
}

// ── Class-name injection defense (security regression, Task 3.1) ───

/// Compile helper returning the full result (success + errors) so tests can
/// assert on diagnostics.
fn compile_react_full(source: &str) -> crate::InternalCompileResult {
    let mut sources = FxHashMap::default();
    sources.insert("input.omni".to_string(), source.to_string());
    let pm = path_mapping::derive_path_mapping(&sources);
    crate::compile_multi_internal(
        sources,
        crate::codegen::Target::React,
        pm,
        false,
        false,
        None,
    )
}

// ── BUG-C hard diagnostic (Task 2.2): inline stores on React ──

/// Compile helper returning (result, diagnostics) so hard-diagnostic tests
/// can assert on the emitted E_CODEGEN errors.
fn compile_react_with_diags(
    source: &str,
) -> (crate::InternalCompileResult, Vec<crate::error::Diagnostic>) {
    let result = compile_react_full(source);
    let diags = result.errors.clone();
    (result, diags)
}

#[test]
fn inline_store_react_hard_diagnosed() {
    let src =
        r#"component Inline { store Cart { signal items: Array<String> = []; } view { <div/> } }"#;
    let (_, diags) = compile_react_with_diags(src);
    let d = diags
        .iter()
        .find(|d| d.kind == DiagnosticKind::ECodegen)
        .expect("inline store must diagnose on react");
    assert!(
        d.message.contains("inline store"),
        "expected migration hint, got: {}",
        d.message
    );
    assert!(
        d.message.contains("顶层 store"),
        "expected migration hint, got: {}",
        d.message
    );
}

// ── @for cross-target keyed contract (Task 5.5) ─────────────

/// T5.5: key_expr → `key={expr}`; a key-less @for falls back to the index
/// `key={_i}` with a generated warning comment — NEVER `key={String(item)}`
/// (which renders "[object Object]" for object items and silently collapses
/// keyed reconciliation, review CRITICAL).
#[test]
fn react_for_index_key_fallback_with_warning_comment() {
    let source = r#"
component ListPage {
    signal items: Array<String> = [];
    view {
        @for item in items { <div>{item}</div> }
    }
}
"#;
    let result = compile_react_full(source);
    assert!(result.success, "errors: {:?}", result.errors);
    let content: String = result
        .files
        .values()
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        content.contains("key={_i}"),
        "key-less @for must fall back to the index key, got:\n{content}"
    );
    assert!(
        content.contains("omni:"),
        "key-less @for must carry a generated warning comment, got:\n{content}"
    );
    assert!(
        !content.contains("key={String("),
        "the String(item) -> \"[object Object]\" key must be gone, got:\n{content}"
    );
}

/// T5.5: keyed @for keeps `key={expr}` on the item fragment.
#[test]
fn react_for_key_expr_emits_expr_key() {
    let source = r#"
component ListPage {
    signal items: Array<String> = [];
    view {
        @for item in items key={item} { <div>{item}</div> }
    }
}
"#;
    let result = compile_react_full(source);
    assert!(result.success, "errors: {:?}", result.errors);
    let content: String = result
        .files
        .values()
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        content.contains("key={item}"),
        "key_expr must emit key={{expr}}, got:\n{content}"
    );
    assert!(
        !content.contains("key={_i}"),
        "keyed @for must not use the index fallback, got:\n{content}"
    );
}

/// T5.5: the empty-body iterable is evaluated ONCE (render-scope const in
/// an IIFE), not twice (`({} && {}.length === 0) ? ... : ({}.map(...))`
/// re-evaluated the iterable per render — side effects / allocations ran
/// twice, review CRITICAL).
#[test]
fn react_for_empty_body_single_eval() {
    let source = r#"
component ListPage {
    signal items: Array<String> = [];
    view {
        @for item in items {
            <div>{item}</div>
        } @empty {
            <p>No items</p>
        }
    }
}
"#;
    let result = compile_react_full(source);
    assert!(result.success, "errors: {:?}", result.errors);
    let content: String = result
        .files
        .values()
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");
    assert_eq!(
        content.matches("const __omni_iterable = items;").count(),
        1,
        "the iterable const must be declared exactly once, got:\n{content}"
    );
    assert_eq!(
        content.matches("__omni_iterable.map").count(),
        1,
        "the iterable must be mapped exactly once, got:\n{content}"
    );
    assert!(
        !content.contains("items && items"),
        "the double-evaluation shape must be gone, got:\n{content}"
    );
    assert!(
        content.contains("__omni_iterable.length === 0"),
        "the empty check must read the single-eval const, got:\n{content}"
    );
}

/// T5.5: a @for over OBJECT items (record/struct element type) without
/// key= must produce the semantic for-key WARNING (design doc 域 6 诊断表:
/// 「对象项列表建议提供 key=；未提供时使用索引，重排将丢失状态」) while
/// still compiling.
#[test]
fn react_for_object_items_no_key_warns() {
    let source = r#"
struct Todo { id: i32, title: String }

component App {
    signal todos: Array<Todo> = [];
    view {
        @for t in todos { <div>{t.title}</div> }
    }
}
"#;
    let result = compile_react_full(source);
    assert!(result.success, "errors: {:?}", result.errors);
    let kinds: Vec<String> = result
        .warnings
        .iter()
        .map(|d| d.kind.code_str().to_string())
        .collect();
    assert!(
        result
            .warnings
            .iter()
            .any(|d| d.message.contains("对象项列表建议提供 key=")),
        "object-item @for without key must warn; warnings: {kinds:?}"
    );
}

/// T5.5: primitive-item @for without key= must NOT warn.
#[test]
fn react_for_primitive_items_no_key_does_not_warn() {
    let source = r#"
component ListPage {
    signal items: Array<String> = [];
    view {
        @for item in items { <div>{item}</div> }
    }
}
"#;
    let result = compile_react_full(source);
    assert!(result.success, "errors: {:?}", result.errors);
    assert!(
        result.warnings.iter().all(|d| !d.message.contains("对象项列表")),
        "primitive-item @for must not warn; warnings: {:?}",
        result
            .warnings
            .iter()
            .map(|d| d.message.clone())
            .collect::<Vec<_>>()
    );
}

/// Review-verified exploit: `class: "x',alert(document.cookie),'"` used to
/// compile to valid TSX (`styles['x',alert(document.cookie),'']` — comma
/// operator) that executes arbitrary JS on render. It must now be a compile
/// ERROR (not merely "compiles but warned").
#[test]
fn class_name_injection_rejected() {
    let source = r##"
component App {
    view {
        <div class: "x',alert(document.cookie),'"/>
    }
}
"##;
    let result = compile_react_full(source);
    assert!(
        !result.success,
        "injected class name must fail compilation. Output:\n{:?}",
        result.files
    );
    let messages: Vec<&str> = result.errors.iter().map(|d| d.message.as_str()).collect();
    assert!(
        messages
            .iter()
            .any(|m| m.contains("类名仅允许字母数字下划线连字符")),
        "expected charset-validation error, got: {:?}",
        messages
    );
    // Defense in depth: even the (failed) codegen output must not carry the
    // executable payload.
    let all_output: String = result
        .files
        .values()
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        !all_output.contains("alert("),
        "generated output must not contain the payload even on failed compiles:\n{}",
        all_output
    );
}

/// Simple (non-design) class names keep the CSS-module lookup emission
/// `styles['name']` — the legit path is unchanged by the escaping layer.
#[test]
fn react_class_legit_path_unchanged() {
    let source = r##"
component App {
    style {
        .btn { padding: 4px; }
        .btn-primary { color: red; }
    }
    view {
        <div class: "btn btn-primary"/>
    }
}
"##;
    let output = compile_react(source).unwrap();
    let tsx = output
        .files
        .get("App.tsx")
        .expect("App.tsx should be emitted");
    assert!(
        tsx.contains("styles['btn'] + ' ' + styles['btn-primary']"),
        "legit class names must keep CSS-module lookups. Got:\n{}",
        tsx
    );
}

/// `class: AppDesign.Card` design references keep the literal class-name
/// emission (global rules in design.css) — unchanged by the fix.
#[test]
fn react_design_class_legit_path_unchanged() {
    let source = r##"
design AppDesign {
    class Card { display: "block"; }
}
component App {
    view {
        <div class: AppDesign.Card/>
    }
}
"##;
    let output = compile_react(source).unwrap();
    let tsx = output
        .files
        .get("App.tsx")
        .expect("App.tsx should be emitted");
    assert!(
        tsx.contains("className={\"Card\"}"),
        "design class must stay a literal class name. Got:\n{}",
        tsx
    );
    assert!(
        !tsx.contains("alert("),
        "design class emission must not carry payloads. Got:\n{}",
        tsx
    );
}

/// Layer 2 of the double-insurance: `format_class_ref` must escape quotes
/// even when a malicious class name reaches codegen through a hand-built IR
/// (bypassing the .omni parser/lowering charset validation).
#[test]
fn react_format_class_ref_escapes_quotes_defense_in_depth() {
    use super::design::DesignCtx;
    use crate::ir::design::*;

    fn ir_with_design_class(name: &str) -> IRProgram {
        let design = DesignIR {
            name: "D".into(),
            tokens: FxHashMap::default(),
            classes: vec![ClassIR {
                name: name.into(),
                properties: FxHashMap::default(),
                pseudo_states: vec![],
                source_span: None,
            }],
            variants: vec![],
            default_theme: None,
            themes: vec![],
            theme_signal: None,
            source_span: None,
            source_file: None,
        };
        let module = ModuleIR {
            name: "main".into(),
            functions: FxHashMap::default(),
            structs: FxHashMap::default(),
            enums: FxHashMap::default(),
            type_aliases: FxHashMap::default(),
            components: FxHashMap::default(),
            stores: FxHashMap::default(),
            motions: vec![],
            imports: vec![],
            exports: vec![],
            designs: vec![design],
            source_span: None,
        };
        IRProgram {
            version: "0.1".into(),
            modules: FxHashMap::from_iter([("main".into(), module)]),
            source_map: FxHashMap::default(),
            stdlib_imports: Default::default(),
            source_span: None,
            motion_index: FxHashMap::default(),
        }
    }

    // CSS-module branch: single-quoted literal — `'` must be escaped so the
    // payload cannot break out into executable code.
    let ir = ir_with_design_class("unused");
    let ctx = DesignCtx::new(&ir);
    assert_eq!(
        ctx.format_class_ref("x',alert(1),'"),
        "styles['x\\',alert(1),\\'']"
    );
    assert_eq!(
        ctx.format_class_ref("x\\';alert(1);//"),
        "styles['x\\\\\\';alert(1);//']"
    );
    // Legit names are untouched (no escaping churn on the happy path).
    assert_eq!(ctx.format_class_ref("btn"), "styles['btn']");
    assert_eq!(ctx.format_class_ref("btn-primary"), "styles['btn-primary']");

    // Design-class branch: double-quoted literal — `"` must be escaped.
    let ir = ir_with_design_class("x\",alert(1),\"");
    let ctx = DesignCtx::new(&ir);
    assert_eq!(
        ctx.format_class_ref("x\",alert(1),\""),
        "\"x\\\",alert(1),\\\"\""
    );
    // A legit design class name keeps the literal emission.
    let ir = ir_with_design_class("Card");
    let ctx = DesignCtx::new(&ir);
    assert_eq!(ctx.format_class_ref("Card"), "\"Card\"");
}

// ── Component-level @motion codegen integration tests ───

#[test]
fn react_component_level_motion_emits_motion_factory() {
    // Component-level `@motion` defs lower into ModuleIR.motions (module-
    // scoped); codegen must emit the motion factory and wire the trigger.
    let source = r##"
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
"##;
    let output = compile_react(source).unwrap();
    let tsx = output
        .files
        .get("App.tsx")
        .expect("App.tsx should be emitted");
    assert!(
        tsx.contains("slide:"),
        "motion factory must contain the component motion. Got:
{}",
        tsx
    );
    assert!(
        tsx.contains("_M[\"slide\"]"),
        "enter trigger must resolve the component motion by name. Got:
{}",
        tsx
    );
}

// ── Design codegen integration tests ─────────────────────

/// Task 4.4: the `@native` lexer token is REMOVED — the platform-layer
/// escape hatch (spec §1.2) no longer exists, and `@native("dart") { ... }`
/// fails with a clean parse error instead of lexing as a directive.
#[test]
fn native_escape_hatch_is_rejected() {
    let source = r##"
@native("dart") { ... }
component App {
    view { <div>hi</div> }
}
"##;
    let mut sources = FxHashMap::default();
    sources.insert("input.omni".to_string(), source.to_string());
    let pm = path_mapping::derive_path_mapping(&sources);
    let result = crate::compile_multi_internal(
        sources,
        crate::codegen::Target::React,
        pm,
        false,
        false,
        None,
    );
    assert!(
        !result.success,
        "@native must be rejected — the token was removed (Task 4.4)"
    );
    assert!(
        result
            .errors
            .iter()
            .any(|e| e.message.contains("Ident(\"@native\")")),
        "the failure must be a clean parse error naming the @native identifier. Got:\n{:?}",
        result.errors
    );
}

// ── Design codegen integration tests ─────────────────────

/// Test: a design block compiles to __design/design.css (unscoped, plain
/// HTML) plus __design/useThemeStore.ts when @themes is declared (Task 4.4
/// naming isolation — the runtime files live in the dedicated __design/
/// subdirectory).
#[test]
fn react_design_emits_css_and_theme_store() {
    let source = r##"
design AppDesign {
    @themes { light, dark }
    @default-theme light;

    token primary: color = "#6366f1" dark "#a5b4fc";
    token surface: color = "#ffffff" dark "#111827";

    class Card {
        background: token(surface);
        border-radius: "8px";
    }

    variant Button {
        @dim color: primary | danger;
        @default { color: primary; }
        base {
            display: inline-flex;
        }
        color.primary { background: token(primary); }
    }
}
"##;
    let output = compile_react(source).unwrap();
    // design.css + useThemeStore.ts
    let css = output
        .files
        .get("__design/design.css")
        .expect("design.css should be emitted");
    assert!(
        css.contains(":root {"),
        "tokens must be declared on :root. Got:\n{}",
        css
    );
    assert!(
        css.contains("--primary: #6366f1;"),
        "static token value must be emitted. Got:\n{}",
        css
    );
    assert!(
        css.contains("[data-theme=\"dark\"] {"),
        "theme overrides must be emitted. Got:\n{}",
        css
    );
    assert!(
        css.contains("--primary: #a5b4fc;"),
        "dark theme token override must be emitted. Got:\n{}",
        css
    );
    assert!(
        css.contains("\n.Card {") && css.contains("background: var(--surface);"),
        "class rules must be emitted unscoped. Got:\n{}",
        css
    );
    assert!(
        css.contains("\n.Button {") && css.contains("\n.Button.primary {"),
        "variant base + branch rules must be emitted. Got:\n{}",
        css
    );
    assert!(
        !css.contains("[data-design~="),
        "React design CSS must be unscoped (plain HTML, no data-design wrapper)"
    );

    let theme_store = output
        .files
        .get("__design/useThemeStore.ts")
        .expect("useThemeStore.ts should be emitted when @themes is declared");
    assert!(
        theme_store.contains("theme: 'light',"),
        "default theme must come from @default-theme. Got:\n{}",
        theme_store
    );
    assert!(
        theme_store.contains("setTheme: (theme) => set({ theme }),"),
        "setTheme action must be emitted. Got:\n{}",
        theme_store
    );
    assert!(
        theme_store.contains("document.documentElement.setAttribute('data-theme', state.theme);"),
        "data-theme subscription must be emitted. Got:\n{}",
        theme_store
    );

    // No dynamic tokens → no useDesignTokens.ts
    assert!(
        !output.files.contains_key("__design/useDesignTokens.ts"),
        "no hook without dynamic tokens"
    );
}

/// Task 4.2 (review HIGH-2): with a `@theme`-marked store signal the
/// generated useThemeStore.ts subscribes to the USER's store (slice selector)
/// and writes `data-theme` in a useEffect; the root component imports the
/// hook (aliased to avoid colliding with the store hook) and calls it.
///
/// Multi-file (the real demo shape): the store, design, and App component
/// live in separate source files so the store hook
/// (`stores/useThemeStore.ts`) and the design runtime
/// (`__design/useThemeStore.ts`) coexist without a basename collision
/// (Task 4.4 naming isolation).
#[test]
fn react_design_theme_signal_wires_subscription() {
    let store_source = r##"
store ThemeStore {
    signal mode: String @theme = "light";
}
"##;
    let design_source = r##"
design AppDesign {
    @themes { light, dark }
    @default-theme light;
    token primary: color = "#6366f1" dark "#a5b4fc";
}
"##;
    let comp_source = r#"
import { ThemeStore } from "./stores/ThemeStore";

component App {
    @store ThemeStore;
    view {
        <div>{ThemeStore.mode}</div>
    }
}
"#;
    let mut sources = FxHashMap::default();
    sources.insert(
        "src/stores/ThemeStore.omni".to_string(),
        store_source.to_string(),
    );
    sources.insert("src/design.omni".to_string(), design_source.to_string());
    sources.insert("src/main.omni".to_string(), comp_source.to_string());
    let pm = path_mapping::derive_path_mapping(&sources);
    let output = crate::compile_multi_internal(
        sources,
        crate::codegen::Target::React,
        pm,
        false,
        false,
        None,
    );
    assert!(
        output.success,
        "theme-signal compilation must succeed. Errors: {:?}",
        output.errors
    );
    let theme_file = output
        .files
        .get("__design/useThemeStore.ts")
        .expect("useThemeStore.ts should be emitted when @themes is declared");
    assert!(
        theme_file.contains("import { useEffect } from 'react';"),
        "the wired theme runtime must use useEffect. Got:\n{}",
        theme_file
    );
    assert!(
        theme_file.contains("(s) => s.mode"),
        "the wired theme runtime must subscribe with a slice selector. Got:\n{}",
        theme_file
    );
    assert!(
        theme_file.contains("document.documentElement.setAttribute('data-theme', mode);"),
        "the wired theme runtime must write data-theme. Got:\n{}",
        theme_file
    );
    assert!(
        theme_file.contains(
            "import { useThemeStore as useAppThemeStore } from '../stores/useThemeStore';"
        ),
        "the wired theme runtime must import the user's store hook. Got:\n{}",
        theme_file
    );
    assert!(
        !theme_file.contains("create"),
        "the orphan Zustand store must not be emitted when a @theme signal is wired. Got:\n{}",
        theme_file
    );
    let app = output
        .files
        .get("App.tsx")
        .expect("App.tsx should be emitted");
    assert!(
        app.contains(
            "import { useThemeStore as useThemeStoreDesign } from './__design/useThemeStore';"
        ),
        "root component must import the wired theme hook (aliased). Got:\n{}",
        app
    );
    assert!(
        app.contains("useThemeStoreDesign();"),
        "root component must CALL the wired theme hook. Got:\n{}",
        app
    );
    // The user's own store hook must still be imported for the component.
    assert!(
        app.contains("import { useThemeStore } from './stores/useThemeStore';"),
        "the store hook import must stay. Got:\n{}",
        app
    );
}

/// Test: dynamic tokens referencing store signals emit a useDesignTokens.ts
/// hook with per-token useEffect setProperty calls and the store hook import.
#[test]
fn react_design_dynamic_tokens_emit_hook() {
    let source = r##"
store DynamicTokenStore {
    signal liveColor: String = "#6366f1";
    signal liveBg: String = "#eef2ff";
}

design AppDesign {
    token dynColor: color = DynamicTokenStore.liveColor;
    token dynBg: color = DynamicTokenStore.liveBg;
    token staticColor: color = "#ffffff";
}
"##;
    let output = compile_react(source).unwrap();
    let hook = output
        .files
        .get("__design/useDesignTokens.ts")
        .expect("useDesignTokens.ts should be emitted for dynamic tokens");
    assert!(
        hook.contains("import { useEffect } from 'react';"),
        "useEffect must be imported. Got:\n{}",
        hook
    );
    assert!(
        hook.contains("const DynamicTokenStore = useDynamicTokenStore();"),
        "store hook must be instantiated. Got:\n{}",
        hook
    );
    assert!(
        hook.contains(
            "document.documentElement.style.setProperty('--dynColor', sanitizeCss(DynamicTokenStore.liveColor));"
        ),
        "dynamic token must set the CSS custom property at runtime. Got:\n{}",
        hook
    );
    assert!(
        hook.contains("}, [DynamicTokenStore.liveColor]);"),
        "effect deps must be the exact store signal expression. Got:\n{}",
        hook
    );
    assert!(
        hook.contains("}, [DynamicTokenStore.liveBg]);"),
        "one effect per dynamic token. Got:\n{}",
        hook
    );
    // The store hook file itself must also be generated
    assert!(
        output.files.contains_key("useDynamicTokenStore.ts"),
        "referenced store hook must be generated"
    );
}

/// Test: design outputs land in the directory mirroring the design's source
/// file, and multiple design blocks in one file share one set of outputs.
#[test]
fn react_design_files_follow_source_directory() {
    let store_source = r##"
store DynamicTokenStore {
    signal liveColor: String = "#6366f1";
}
"##;
    let design_source = r##"
design ADesign {
    token a: color = "#111111";
}
design BDesign {
    token b: color = "#222222";
}
"##;
    let comp_source = r#"
component App {
    view {
        <div>hello</div>
    }
}
"#;
    let mut sources = FxHashMap::default();
    sources.insert(
        "src/stores/DynamicTokenStore.omni".to_string(),
        store_source.to_string(),
    );
    sources.insert("src/design.omni".to_string(), design_source.to_string());
    sources.insert("src/main.omni".to_string(), comp_source.to_string());
    let pm = path_mapping::derive_path_mapping(&sources);
    let result = crate::compile_multi_internal(
        sources,
        crate::codegen::Target::React,
        pm,
        false,
        false,
        None,
    );
    assert!(
        result.success,
        "design compilation must succeed. Errors: {:?}",
        result.errors
    );
    // __design/design.css at the design file's level (src/ → output root)
    let css = result
        .files
        .get("__design/design.css")
        .expect("__design/design.css should be emitted at output root");
    assert!(
        css.contains("Design: ADesign") && css.contains("Design: BDesign"),
        "both design blocks must be concatenated into one design.css. Got:\n{}",
        css
    );
    assert!(css.contains("--a: #111111;") && css.contains("--b: #222222;"));
    // Store hook generated in stores/
    assert!(result.files.contains_key("stores/useDynamicTokenStore.ts"));
    // Component at root (src/main.omni → output root)
    assert!(result.files.contains_key("App.tsx"));
}

/// Test (HIGH #1): design classes and variant combination classes are
/// emitted as literal class names so the global rules in design.css match;
/// component-local style classes keep CSS-module lookups.
#[test]
fn react_design_classes_emit_literal_class_names() {
    let source = r##"
design LearnDesign {
    @themes { light, dark }
    token primary: color = "#6366f1";

    class Page {
        background: token(primary);
    }
    class Card {
        border-radius: "8px";
    }

    variant Button {
        @dim color: primary | danger;
        @default { color: primary; }
        base { display: inline-flex; }
        color.primary { background: token(primary); }
    }
}

component App {
    signal isActive: bool = true
    style {
        .box { padding: 8px; }
    }
    view {
        <div>
            <div class: LearnDesign.Page>page</div>
            <button variant: LearnDesign.Button(color=primary)>go</button>
            <div class: { Card: true, box: isActive }>card</div>
        </div>
    }
}
"##;
    let output = compile_react(source).unwrap();
    let app = output
        .files
        .get("App.tsx")
        .expect("App.tsx should be emitted");

    // `class: LearnDesign.Page` → literal class name (global .Page rule)
    assert!(
        app.contains("className={\"Page\"}"),
        "design class must be a literal class name. Got:\n{}",
        app
    );
    assert!(
        !app.contains("styles['Page']"),
        "design class must NOT go through the CSS module. Got:\n{}",
        app
    );
    // `variant: LearnDesign.Button(color=primary)` → "Button primary"
    assert!(
        app.contains("className={\"Button primary\"}"),
        "variant binding must emit combination classes as literals. Got:\n{}",
        app
    );
    assert!(
        !app.contains("styles['Button']"),
        "variant classes must NOT go through the CSS module. Got:\n{}",
        app
    );
    // class map: design class literal + local class CSS-module lookup
    assert!(
        app.contains("\"Card\"") && app.contains("styles['box']"),
        "class map must mix design literals and module lookups. Got:\n{}",
        app
    );
    // design.css still emits the global rules the literals match
    let css = output
        .files
        .get("__design/design.css")
        .expect("design.css should be emitted");
    assert!(css.contains("\n.Page {"), "global .Page rule missing");
    assert!(
        css.contains("\n.Button.primary {"),
        "global .Button.primary rule missing"
    );
}

/// Test (HIGH #1): a component using ONLY design classes does not import a
/// CSS module (nothing local to look up).
#[test]
fn react_design_only_classes_skip_css_module_import() {
    let source = r##"
design LearnDesign {
    token primary: color = "#6366f1";
    class Page {
        background: token(primary);
    }
}

component App {
    view {
        <div class: LearnDesign.Page>page</div>
    }
}
"##;
    let output = compile_react(source).unwrap();
    let app = output
        .files
        .get("App.tsx")
        .expect("App.tsx should be emitted");
    assert!(
        !app.contains("import styles from './App.module.css';"),
        "no CSS module import when only design classes are used. Got:\n{}",
        app
    );
    assert!(
        app.contains("className={\"Page\"}"),
        "design class must still be a literal. Got:\n{}",
        app
    );
}

/// Test (HIGH #2): the root component imports and calls `useDesignTokens`
/// and side-effect-imports `useThemeStore`, so the emitted design runtime
/// actually activates.
#[test]
fn react_root_component_wires_design_runtime() {
    let source = r##"
store DynamicTokenStore {
    signal liveColor: String = "#6366f1";
}

design AppDesign {
    @themes { light, dark }
    @default-theme light;
    token dynColor: color = DynamicTokenStore.liveColor;
    token staticColor: color = "#ffffff";
}

component App {
    view {
        <div>hello</div>
    }
}
"##;
    let output = compile_react(source).unwrap();
    let app = output
        .files
        .get("App.tsx")
        .expect("App.tsx should be emitted");
    assert!(
        app.contains("import { useDesignTokens } from './__design/useDesignTokens';"),
        "root component must import the design tokens hook. Got:\n{}",
        app
    );
    assert!(
        app.contains("useDesignTokens();"),
        "root component must call the design tokens hook. Got:\n{}",
        app
    );
    assert!(
        app.contains("import './__design/useThemeStore';"),
        "root component must import the theme store from __design/ (side effect). Got:\n{}",
        app
    );
    // Non-root components must not import the wiring
    let non_root = r##"
store DynamicTokenStore {
    signal liveColor: String = "#6366f1";
}

design AppDesign {
    @themes { light, dark }
    token dynColor: color = DynamicTokenStore.liveColor;
}

component App {
    view {
        <div>app</div>
    }
}

component Sidebar {
    view {
        <div>side</div>
    }
}
"##;
    let output = compile_react(non_root).unwrap();
    let sidebar = output
        .files
        .get("Sidebar.tsx")
        .expect("Sidebar.tsx should be emitted");
    assert!(
        !sidebar.contains("useDesignTokens"),
        "non-root components must not import design hooks. Got:\n{}",
        sidebar
    );
    let app = output
        .files
        .get("App.tsx")
        .expect("App.tsx should be emitted");
    assert!(
        app.contains("useDesignTokens();"),
        "App must still be wired as root. Got:\n{}",
        app
    );
}

/// Test (HIGH #2): the root component wires hooks from a design file in a
/// different source directory via a correct relative import path.
#[test]
fn react_root_component_wires_design_across_directories() {
    let store_source = r##"
store DynamicTokenStore {
    signal liveColor: String = "#6366f1";
}
"##;
    let design_source = r##"
import { DynamicTokenStore } from "./stores/DynamicTokenStore";

design AppDesign {
    @themes { light, dark }
    token dynColor: color = DynamicTokenStore.liveColor;
}
"##;
    let comp_source = r#"
component App {
    view {
        <div>app</div>
    }
}
"#;
    let mut sources = FxHashMap::default();
    sources.insert(
        "src/stores/DynamicTokenStore.omni".to_string(),
        store_source.to_string(),
    );
    sources.insert(
        "src/designs/theme.omni".to_string(),
        design_source.to_string(),
    );
    sources.insert("src/main.omni".to_string(), comp_source.to_string());
    let pm = path_mapping::derive_path_mapping(&sources);
    let result = crate::compile_multi_internal(
        sources,
        crate::codegen::Target::React,
        pm,
        false,
        false,
        None,
    );
    assert!(
        result.success,
        "design compilation must succeed. Errors: {:?}",
        result.errors
    );
    let app = result
        .files
        .get("App.tsx")
        .expect("App.tsx should be emitted");
    assert!(
        app.contains("import { useDesignTokens } from './designs/__design/useDesignTokens';"),
        "hook import must resolve across directories. Got:\n{}",
        app
    );
    assert!(
        app.contains("useDesignTokens();"),
        "hook must be called. Got:\n{}",
        app
    );
    assert!(
        result
            .files
            .contains_key("designs/__design/useDesignTokens.ts"),
        "hook file must be emitted next to the design source"
    );
}

// ── Ref codegen integration tests ────────────────────────

/// Test: ref declaration generates useRef + ref={} JSX attribute
#[test]
fn test_react_ref_basic() {
    let source = r#"
component TestRef {
  ref box: Element
  view {
    <div ref:box>content</div>
  }
}
"#;
    let output = compile_react(source).unwrap();
    let content: String = output
        .files
        .values()
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");
    // useRef import
    assert!(
        content.contains("useRef"),
        "Import should include useRef. Got:\n{}",
        content
    );
    // useRef declaration — typed by the bound element's tag (div → HTMLDivElement)
    assert!(
        content.contains("const boxRef = useRef<HTMLDivElement>(null)"),
        "Should generate useRef declaration. Got:\n{}",
        content
    );
    // ref={} JSX attribute
    assert!(
        content.contains("ref={boxRef}"),
        "JSX should have ref={{boxRef}}. Got:\n{}",
        content
    );
    // NOT data-ref (legacy lowering artifact)
    assert!(
        !content.contains("data-ref"),
        "Should not emit data-ref attribute. Got:\n{}",
        content
    );
}

/// Test: ref declaration + JSX ref binding in event handler component
#[test]
fn test_react_ref_in_handler() {
    let source = r#"
component DragRef {
  signal x: Float = 0
  ref box: Element
  fn onDrag(e) {
    box.focus()
  }
  view {
    <div ref:box @onPointerMove={onDrag}>drag</div>
  }
}
"#;
    let output = compile_react(source).unwrap();
    let content: String = output
        .files
        .values()
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");
    // useRef imported
    assert!(
        content.contains("useRef"),
        "useRef should be imported. Got:\n{}",
        content
    );
    // useRef declaration — typed by the bound element's tag (div → HTMLDivElement)
    assert!(
        content.contains("const boxRef = useRef<HTMLDivElement>(null)"),
        "Should generate useRef declaration for 'box'. Got:\n{}",
        content
    );
    // ref={boxRef} in JSX
    assert!(
        content.contains("ref={boxRef}"),
        "JSX should have ref={{boxRef}}. Got:\n{}",
        content
    );
    // Legal ref ops are rewritten to ExprIR::RefOpExpr by validate_ref_ops
    // and codegen emits the native DOM call on the ref's .current (ADR-036).
    // (ref.style.* was removed by the semantic DSL — ADR-052/053 — and now
    // produces a compile error instead.)
    assert!(
        content.contains("boxRef.current?.focus()"),
        "Handler body should emit the native focus() call. Got:\n{}",
        content
    );
}

/// Test: component without refs does NOT import useRef
#[test]
fn test_react_no_ref_no_use_ref_import() {
    let source = r#"
component NoRef {
  signal count: Float = 0
  view {
    <div>{count}</div>
  }
}
"#;
    let output = compile_react(source).unwrap();
    let content: String = output
        .files
        .values()
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        !content.contains("useRef"),
        "useRef should NOT be imported when no refs. Got:\n{}",
        content
    );
}

// ── Ref semantic binding + RefOpExpr codegen tests ────────

/// Test: useRef is typed by the bound element's tag — input → HTMLInputElement,
/// textarea → HTMLTextAreaElement, canvas → HTMLCanvasElement, div → HTMLDivElement.
#[test]
fn test_react_ref_typed_by_element_tag() {
    let source = r#"
component TypedRefs {
  ref field: Element
  ref area: Element
  ref canvasEl: Element
  ref box: Element
  view {
    <div>
      <input ref:field />
      <textarea ref:area></textarea>
      <canvas ref:canvasEl></canvas>
      <div ref:box />
    </div>
  }
}
"#;
    let output = compile_react(source).unwrap();
    let content: String = output
        .files
        .values()
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        content.contains("const fieldRef = useRef<HTMLInputElement>(null)"),
        "input should type useRef as HTMLInputElement. Got:\n{}",
        content
    );
    assert!(
        content.contains("const areaRef = useRef<HTMLTextAreaElement>(null)"),
        "textarea should type useRef as HTMLTextAreaElement. Got:\n{}",
        content
    );
    assert!(
        content.contains("const canvasElRef = useRef<HTMLCanvasElement>(null)"),
        "canvas should type useRef as HTMLCanvasElement. Got:\n{}",
        content
    );
    assert!(
        content.contains("const boxRef = useRef<HTMLDivElement>(null)"),
        "div should type useRef as HTMLDivElement. Got:\n{}",
        content
    );
}

/// Test: semantic bindings (focus, scroll-y) on a ref-bound element generate
/// useEffect blocks driven by the bound signal. (`value={sig}` is removed —
/// design D2 — it hard-errors in the semantic pass instead.)
#[test]
fn test_react_semantic_bindings_generate_effects() {
    let source = r#"
component SearchBox {
  signal active: Bool = false
  signal pos: Float = 0
  ref field: Element
  view {
    <input ref:field focus={active} scroll-y={pos} />
  }
}
"#;
    let output = compile_react(source).unwrap();
    let content: String = output
        .files
        .values()
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");
    // useEffect must be imported for the binding effects
    assert!(
        content.contains("useEffect"),
        "useEffect should be imported for semantic bindings. Got:\n{}",
        content
    );
    // focus binding → focus()/blur() on the boolean signal
    assert!(
        content.contains("if (active) {")
            && content.contains("fieldRef.current?.focus();")
            && content.contains("fieldRef.current?.blur();"),
        "focus binding should emit focus/blur effect. Got:\n{}",
        content
    );
    assert!(
        content.contains("}, [active]);"),
        "focus effect deps should track the bound signal. Got:\n{}",
        content
    );
    // scroll-y binding → scrollTo({ top: pos })
    assert!(
        content.contains("fieldRef.current?.scrollTo({ top: pos });"),
        "scroll-y binding should emit scrollTo top. Got:\n{}",
        content
    );
    assert!(
        content.contains("}, [pos]);"),
        "scroll-y effect deps should track the bound signal. Got:\n{}",
        content
    );
}

/// Test: `bind:value` on React emits the idiomatic two-way form —
/// `value={sig}` + an `onChange` setter writing back through the same
/// mutation machinery as actions (design D2 方向一致性; the review HIGH-1
/// divergence was the removed `value={sig}` one-way effect).
#[test]
fn test_react_bind_value_emits_onchange_setter() {
    let source = r#"
component SearchBox {
  signal query: String = ""
  view {
    <input bind:value={query} />
  }
}
"#;
    let output = compile_react(source).unwrap();
    let content: String = output
        .files
        .values()
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        content.contains("value={query}") && content.contains("setQuery(e.target.value)"),
        "bind:value must emit value + onChange setter. Got:\n{}",
        content
    );
    assert!(
        !content.contains("queryRef") && !content.contains("useEffect"),
        "bind:value must not need a ref or a DOM-syncing effect. Got:\n{}",
        content
    );
}

/// Test: `bind:value={Store.sig}` writes back through the Zustand mutation
/// machinery (`useStore.setState`) — not an invalid `setStore.sig(...)` call.
#[test]
fn test_react_bind_value_store_signal_uses_setstate() {
    let store_source = r#"
store CartStore {
    signal text: String = "";
}
"#;
    let comp_source = r#"
import { CartStore } from "./store";

component StoreBinding {
    @store CartStore;
    view {
        <input bind:value={CartStore.text} />
    }
}
"#;
    let mut sources = FxHashMap::default();
    sources.insert("store.omni".to_string(), store_source.to_string());
    sources.insert("main.omni".to_string(), comp_source.to_string());
    let pm = path_mapping::derive_path_mapping(&sources);
    let result = crate::compile_multi_internal(
        sources,
        crate::codegen::Target::React,
        pm,
        false,
        false,
        None,
    );
    assert!(
        result.success,
        "store-backed bind:value must compile. Errors: {:?}",
        result.errors
    );
    let content: String = result
        .files
        .values()
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");
    // T3.3: the store is subscribed per field — the binding read uses the
    // slice local, not a whole-store hook object.
    assert!(
        content.contains("const text = useCartStore((s) => s.text);"),
        "store-backed binding must subscribe via slice selector. Got:\n{}",
        content
    );
    assert!(
        !content.contains("const CartStore = useCartStore();"),
        "no whole-store hook for a store-backed binding. Got:\n{}",
        content
    );
    // Read side: value={text} through the slice local.
    assert!(
        content.contains("value={text}"),
        "bind:value must emit the slice-local read. Got:\n{}",
        content
    );
    // Write side: the Zustand setState mutation machinery, like store
    // mutations in actions — never `setStore.sig(...)`.
    assert!(
        content.contains("useCartStore.setState({ text: e.target.value })"),
        "store write-back must use useStore.setState. Got:\n{}",
        content
    );
    assert!(
        !content.contains("setCartStore.text"),
        "no invalid setStore.sig call may be emitted. Got:\n{}",
        content
    );
}

/// Test: scroll-x semantic binding emits scrollTo({ left: sig }).
#[test]
fn test_react_semantic_binding_scroll_x() {
    let source = r#"
component Carousel {
  signal pos: Float = 0
  ref track: Element
  view {
    <div ref:track scroll-x={pos}>items</div>
  }
}
"#;
    let output = compile_react(source).unwrap();
    let content: String = output
        .files
        .values()
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        content.contains("trackRef.current?.scrollTo({ left: pos });"),
        "scroll-x binding should emit scrollTo left. Got:\n{}",
        content
    );
    assert!(
        content.contains("}, [pos]);"),
        "scroll-x effect deps should track the bound signal. Got:\n{}",
        content
    );
}

/// Test: the removed `value={Store.sig}` semantic-binding form must
/// hard-diagnose with the migration hint — `bind:value` is the sole two-way
/// binding (design D2), covered by `test_react_bind_value_store_signal_uses_setstate`.
#[test]
fn test_react_semantic_binding_store_signal_rejected() {
    let store_source = r#"
store CartStore {
    signal text: String = "";
}
"#;
    let comp_source = r#"
import { CartStore } from "./store";

component StoreBinding {
    @store CartStore;
    ref field: Element;
    view {
        <input ref:field value={CartStore.text} />
    }
}
"#;
    let mut sources = FxHashMap::default();
    sources.insert("store.omni".to_string(), store_source.to_string());
    sources.insert("main.omni".to_string(), comp_source.to_string());
    let pm = path_mapping::derive_path_mapping(&sources);
    let result = crate::compile_multi_internal(
        sources,
        crate::codegen::Target::React,
        pm,
        false,
        false,
        None,
    );
    assert!(
        !result.success,
        "value={{Store.sig}} must fail to compile. Errors: {:?}",
        result.errors
    );
    assert!(
        result
            .errors
            .iter()
            .any(|e| e.message.contains("bind:value") && e.message.contains("value prop")),
        "value={{Store.sig}} must diagnose with the migration hint. Got: {:?}",
        result.errors
    );
}

/// Test: every RefOpIR variant formats to a native DOM call on `nameRef.current`.
#[test]
fn test_react_ref_ops_format_native_dom_calls() {
    let source = r#"
component RefOps {
  signal x: Float = 0
  signal y: Float = 0
  signal text: String = ""
  ref box: Element
  fn runAll() {
    box.focus()
    box.blur()
    box.scrollTo(x, y)
    box.scrollBy(1, 2)
    box.measure()
    box.width
    box.height
    box.position
    box.select()
    box.setSelection(0, 3)
    box.value
    box.capturePointer()
    box.releasePointer()
  }
  view {
    <div ref:box @onClick={runAll}>click</div>
  }
}
"#;
    let output = compile_react(source).unwrap();
    let content: String = output
        .files
        .values()
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");
    let expectations = [
        "boxRef.current?.focus();",
        "boxRef.current?.blur();",
        "boxRef.current?.scrollTo({ left: x, top: y });",
        "boxRef.current?.scrollBy({ left: 1, top: 2 });",
        "boxRef.current?.getBoundingClientRect();",
        "boxRef.current?.getBoundingClientRect().width;",
        "boxRef.current?.getBoundingClientRect().height;",
        "boxRef.current?.getBoundingClientRect();",
        "boxRef.current?.select();",
        "boxRef.current?.setSelectionRange(0, 3);",
        "boxRef.current?.value;",
        // Design D3: parameterless ops capture the ACTIVE pointer via the
        // module-scope tracker (no event object is in scope inside fns).
        "boxRef.current?.setPointerCapture(__omni_activePointerId!);",
        "boxRef.current?.releasePointerCapture(__omni_activePointerId!);",
    ];
    for expected in expectations {
        assert!(
            content.contains(expected),
            "expected RefOp output to contain {:?}. Got:\n{}",
            expected,
            content
        );
    }
    // The module-scope active-pointer tracker is emitted only because the
    // component uses the pointer-capture ops (D3 parameterless form).
    assert!(
        content.contains("let __omni_activePointerId: number | null = null;")
            && content.contains("window.addEventListener(\"pointerdown\""),
        "pointer ops must emit the active-pointer tracker. Got:\n{}",
        content
    );
}

/// Test: a component WITHOUT pointer-capture ops gets no active-pointer
/// tracker (the emission is gated on actual usage).
#[test]
fn test_react_ref_ops_without_pointer_capture_emit_no_tracker() {
    let source = r#"
component RefOpsNoPtr {
  ref box: Element
  fn runAll() {
    box.focus()
  }
  view {
    <div ref:box @onClick={runAll}>click</div>
  }
}
"#;
    let output = compile_react(source).unwrap();
    let content: String = output
        .files
        .values()
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        !content.contains("__omni_activePointerId"),
        "no pointer ops \u{2192} no tracker expected. Got:\n{}",
        content
    );
}

/// Test: illegal DOM APIs on refs still produce compile errors with migration hints.
#[test]
fn test_react_ref_illegal_ops_compile_error() {
    let source = r#"
component BadRefOps {
  ref box: Element
  fn bad() {
    box.innerHTML = "x"
    box.getBoundingClientRect()
  }
  view {
    <div ref:box>content</div>
  }
}
"#;
    let mut sources = FxHashMap::default();
    sources.insert("input.omni".to_string(), source.to_string());
    let pm = path_mapping::derive_path_mapping(&sources);
    let result = crate::compile_multi_internal(
        sources,
        crate::codegen::Target::React,
        pm,
        false,
        false,
        None,
    );
    assert!(!result.success, "illegal ref ops must fail compilation");
    let messages: Vec<&str> = result.errors.iter().map(|e| e.message.as_str()).collect();
    assert!(
        messages
            .iter()
            .any(|m| m.contains("ref.innerHTML is not allowed")
                || m.contains("unknown ref field: ref.innerHTML")),
        "expected innerHTML migration error, got: {:?}",
        messages
    );
    assert!(
        messages
            .iter()
            .any(|m| m.contains("ref.getBoundingClientRect() is not allowed")),
        "expected getBoundingClientRect migration error, got: {:?}",
        messages
    );
}

/// Test: unnamed slot without fallback renders `{children}` and the Props
/// interface declares `children?: ReactNode`.
#[test]
fn slot_unnamed_renders_children_prop() {
    let source = r#"
component Card {
    view {
        <div class="card">@slot</div>
    }
}
"#;
    let output = compile_react(source).unwrap();
    let content: String = output
        .files
        .values()
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        content.contains("children?: ReactNode"),
        "Expected children slot prop in Props interface, got:\n{}",
        content
    );
    assert!(
        content.contains("export function Card({ children }: CardProps)"),
        "Expected destructured children prop, got:\n{}",
        content
    );
    assert!(
        content.contains("{children}"),
        "Expected {{children}} render, got:\n{}",
        content
    );
}

/// Test: named slot with fallback renders `{header ?? (fallback)}` and the
/// Props interface declares `header?: ReactNode`.
#[test]
fn slot_named_renders_prop_with_fallback() {
    let source = r#"
component Card {
    view {
        <div class="card">
            @slot header {
                <h4>Default Header</h4>
            }
        </div>
    }
}
"#;
    let output = compile_react(source).unwrap();
    let content: String = output
        .files
        .values()
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        content.contains("header?: ReactNode"),
        "Expected header slot prop in Props interface, got:\n{}",
        content
    );
    assert!(
        content.contains("export function Card({ header }: CardProps)"),
        "Expected destructured header prop, got:\n{}",
        content
    );
    assert!(
        content.contains("{header ?? ("),
        "Expected fallback-wrapped header render, got:\n{}",
        content
    );
    assert!(
        content.contains("Default Header"),
        "Expected fallback content in output, got:\n{}",
        content
    );
    assert!(
        !content.contains("WARNING:"),
        "Slot warning should be gone, got:\n{}",
        content
    );
}

/// Test: unnamed slot with fallback renders `{children ?? (fallback)}`.
#[test]
fn slot_unnamed_renders_children_with_fallback() {
    let source = r#"
component Panel {
    view {
        <div>
            @slot {
                <p>Default Body</p>
            }
        </div>
    }
}
"#;
    let output = compile_react(source).unwrap();
    let content: String = output
        .files
        .values()
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        content.contains("{children ?? ("),
        "Expected fallback-wrapped children render, got:\n{}",
        content
    );
    assert!(
        content.contains("children?: ReactNode"),
        "Expected children slot prop in Props interface, got:\n{}",
        content
    );
}

/// Test: `@slot header: String { ... }` — the type constraint is deleted
/// (design D4): the parser hard-errors with the migration hint, and the
/// constraint must NOT surface as a prop type anywhere.
#[test]
fn slot_type_constraint_hard_error() {
    let source = r#"
component Badge {
    view {
        <div>
            @slot header: String { <span>Fallback</span> }
        </div>
    }
}
"#;
    let output = compile_react_full(source);
    assert!(
        !output.success,
        "typed slot must be a hard error, got success"
    );
    assert!(
        output
            .errors
            .iter()
            .any(|e| e.message.contains("类型约束与数据参数已删除")),
        "Expected deleted-typed-slot error, got: {:?}",
        output.errors
    );
}

/// Test: a slot inside an `@if` body with sibling nodes is wrapped in a
/// React fragment so the emitted TSX stays valid (`{cond && (<>...)}`).
#[test]
fn slot_inside_conditional_wraps_in_fragment() {
    let source = r#"
component Panel {
    signal show: bool = true
    view {
        @if show {
            @slot
            <p>No data</p>
        }
    }
}
"#;
    let output = compile_react(source).unwrap();
    let content: String = output
        .files
        .values()
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        content.contains("&& (") && content.contains("<>") && content.contains("</>"),
        "Expected fragment-wrapped conditional body, got:\n{}",
        content
    );
    assert!(
        content.contains("{children}"),
        "Expected {{children}} slot render, got:\n{}",
        content
    );
}

/// Test: `@if mode: "fade"` is a real transition on React (ADR-052 floor —
/// transition_mode must produce behavior, never a silent no-op). The branch
/// renders inside a wrapper div carrying the CSS-module fade class, and the
/// component's `.module.css` defines the opacity keyframes. The wrapper
/// mounts with the branch, so the fade replays exactly when the shown
/// branch changes.
#[test]
fn conditional_fade_mode_emits_css_fade_transition() {
    let source = r#"
component FadePanel {
    signal open: bool = false
    view {
        <div>
            @if open mode: "fade" {
                <p>Panel</p>
            }
        </div>
    }
}
"#;
    let output = compile_react(source).unwrap();
    let tsx: String = output
        .files
        .iter()
        .filter(|(path, _)| path.ends_with(".tsx"))
        .map(|(_, content)| content.clone())
        .collect::<Vec<_>>()
        .join("\n");
    let css: String = output
        .files
        .iter()
        .filter(|(path, _)| path.ends_with(".module.css"))
        .map(|(_, content)| content.clone())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        tsx.contains("className={styles['omni-if-fade']}"),
        "Expected fade wrapper class in TSX, got:\n{}",
        tsx
    );
    assert!(
        tsx.contains("<div key=\"0\" className={styles['omni-if-fade']}>"),
        "Expected a keyed fade wrapper (key forces remount so the fade replays on branch changes), got:\n{}",
        tsx
    );
    assert!(
        tsx.contains("import styles from './FadePanel.module.css';"),
        "Fade forces the CSS-module import so the rules exist, got:\n{}",
        tsx
    );
    assert!(
        css.contains("@keyframes omni-if-fade")
            && css.contains("animation: omni-if-fade 300ms ease-out forwards;"),
        "Expected fade keyframes + animation in CSS module, got:\n{}",
        css
    );
    assert!(
        css.contains("@media (prefers-reduced-motion: reduce)")
            && css.contains("animation: none;")
            && css.contains("opacity: 1;"),
        "Expected prefers-reduced-motion override in CSS module, got:\n{}",
        css
    );
}

/// Test: `mode: "fade"` wraps EVERY branch of a multi-branch conditional
/// (ternary), so each entering branch fades in when it becomes the shown
/// one. Each wrapper carries a per-branch `key` (`"0"`, `"1"`, ..., `"fb"`
/// for the fallback) — React reconciliation would otherwise reuse the
/// same-position wrapper node on branch swaps (animation already finished,
/// opacity stuck at 1) and the fade would not replay.
#[test]
fn conditional_fade_mode_wraps_every_ternary_branch() {
    let source = r#"
component FadeStatus {
    signal status: String = "loading"
    view {
        <div>
            @if status == "loading" mode: "fade" {
                <p>Loading</p>
            } @else if status == "error" {
                <p>Error</p>
            } @else {
                <p>Ready</p>
            }
        </div>
    }
}
"#;
    let output = compile_react(source).unwrap();
    let tsx: String = output
        .files
        .iter()
        .filter(|(path, _)| path.ends_with(".tsx"))
        .map(|(_, content)| content.clone())
        .collect::<Vec<_>>()
        .join("\n");
    let wrappers = tsx.matches("className={styles['omni-if-fade']}").count();
    assert_eq!(
        wrappers, 3,
        "Expected all 3 ternary branches wrapped in the fade div, got {}:\n{}",
        wrappers, tsx
    );
    assert!(
        tsx.contains("<div key=\"0\" className={styles['omni-if-fade']}>")
            && tsx.contains("<div key=\"1\" className={styles['omni-if-fade']}>")
            && tsx.contains("<div key=\"fb\" className={styles['omni-if-fade']}>"),
        "Expected per-branch keys (\"0\", \"1\", \"fb\") so React remounts the wrapper on branch swaps, got:\n{}",
        tsx
    );
}

/// Test: `mode: "fade"` inside a body-declared `@slot` fallback still forces
/// the CSS-module import. The import and the CSS rules are gated by the same
/// predicate (`component_has_fade_conditional` covers the view AND body-slot
/// fallbacks), so the generated TSX can never reference `styles` without
/// importing it.
#[test]
fn conditional_fade_in_slot_fallback_forces_styles_import() {
    let source = r#"
component FadeCard {
    @slot header { @if show mode: "fade" { <h4>Title</h4> } }
    signal show: bool = true
    view {
        <div>
            {header}
        </div>
    }
}
"#;
    let output = compile_react(source).unwrap();
    let tsx: String = output
        .files
        .iter()
        .filter(|(path, _)| path.ends_with(".tsx"))
        .map(|(_, content)| content.clone())
        .collect::<Vec<_>>()
        .join("\n");
    let css: String = output
        .files
        .iter()
        .filter(|(path, _)| path.ends_with(".module.css"))
        .map(|(_, content)| content.clone())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        tsx.contains("{header ?? ("),
        "Expected the slot fallback render, got:\n{}",
        tsx
    );
    assert!(
        tsx.contains("styles['omni-if-fade']"),
        "Expected the fade class in the slot fallback, got:\n{}",
        tsx
    );
    assert!(
        tsx.contains("import styles from './FadeCard.module.css';"),
        "A fade in a body-slot fallback must still force the styles import (HIGH-2), got:\n{}",
        tsx
    );
    assert!(
        css.contains("@keyframes omni-if-fade"),
        "Expected the fade keyframes in the CSS module, got:\n{}",
        css
    );
}

/// Test: `mode: "wait"` — OmniLang has no leave-animation mechanism to wait
/// for, so React degrades to an instant swap (same timing as mode: "none").
/// The degradation is stated in the generated TSX so it is never a silent
/// no-op.
#[test]
fn conditional_wait_mode_degrades_with_stated_comment() {
    let source = r#"
component WaitPanel {
    signal open: bool = false
    view {
        <div>
            @if open mode: "wait" {
                <p>Panel</p>
            }
        </div>
    }
}
"#;
    let output = compile_react(source).unwrap();
    let tsx: String = output
        .files
        .iter()
        .filter(|(path, _)| path.ends_with(".tsx"))
        .map(|(_, content)| content.clone())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        tsx.contains("no leave animation; degrades to an instant swap"),
        "Expected the degradation comment in TSX, got:\n{}",
        tsx
    );
    assert!(
        !tsx.contains("omni-if-fade"),
        "Wait mode must not emit fade wrappers, got:\n{}",
        tsx
    );
}

/// Test: an `@if` without `mode:` (or with explicit `mode: "none"`) keeps
/// the previous output exactly — no fade wrapper, no degradation comment.
#[test]
fn conditional_without_mode_output_unchanged() {
    let source = r#"
component PlainPanel {
    signal open: bool = false
    view {
        <div>
            @if open {
                <p>Panel</p>
            }
        </div>
    }
}
"#;
    let output = compile_react(source).unwrap();
    let tsx: String = output
        .files
        .iter()
        .filter(|(path, _)| path.ends_with(".tsx"))
        .map(|(_, content)| content.clone())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        tsx.contains("&& ("),
        "Expected the plain &&-conditional, got:\n{}",
        tsx
    );
    assert!(
        !tsx.contains("omni-if-fade") && !tsx.contains("degrades to an instant swap"),
        "No mode → unchanged output, got:\n{}",
        tsx
    );
}

/// Test: a multi-node slot fallback is wrapped in a React fragment inside
/// `{prop ?? ( ... )}` so the emitted TSX stays valid.
#[test]
fn slot_multi_node_fallback_wraps_in_fragment() {
    let source = r#"
component Card {
    view {
        <div class="card">
            @slot header {
                <h4>Title</h4>
                <p>Subtitle</p>
            }
        </div>
    }
}
"#;
    let output = compile_react(source).unwrap();
    let content: String = output
        .files
        .values()
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        content.contains("{header ?? (") && content.contains("<>") && content.contains("</>"),
        "Expected fragment-wrapped multi-node fallback, got:\n{}",
        content
    );
    assert!(
        content.contains("Title") && content.contains("Subtitle"),
        "Expected both fallback nodes, got:\n{}",
        content
    );
}

/// Test: consumer-position `@slot name { ... }` inside a component's
/// children projects into the child component's named prop — it is neither
/// rendered as a bare identifier nor added to the consumer's Props interface.
#[test]
fn slot_projection_directive_at_call_site() {
    let source = r#"
component SlotBox {
    view {
        <div class: box>
            <div>@slot header</div>
            <div>@slot body</div>
        </div>
    }
}

component Consumer {
    view {
        <SlotBox>
            @slot header {
                <h4>Custom Header</h4>
            }
            @slot {
                <p>Body content</p>
            }
            <span>Default child</span>
        </SlotBox>
    }
}
"#;
    let output = compile_react(source).unwrap();
    let content: String = output
        .files
        .values()
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");
    let consumer: String = output
        .files
        .iter()
        .filter(|(path, _)| path.contains("Consumer.tsx"))
        .map(|(_, c)| c.clone())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        content.contains("header={<h4>") || content.contains("header={<h4"),
        "Expected @slot directive projected to header prop, got:\n{}",
        content
    );
    assert!(
        content.contains("Custom Header"),
        "Expected projected header content, got:\n{}",
        content
    );
    assert!(
        content.contains("Body content"),
        "Expected unnamed @slot directive content as children, got:\n{}",
        content
    );
    assert!(
        !consumer.contains("header?: ReactNode"),
        "Consumer Props interface must not be polluted by projection slots, got:\n{}",
        consumer
    );
    assert!(
        !consumer.contains("interface ConsumerProps"),
        "Consumer without own props/declarations should have no Props interface, got:\n{}",
        consumer
    );
}

/// Test: consumer-side named-prop projections (`header={<h4/>}` — the only
/// consumer form since design D4) project into named props at the call
/// site.
#[test]
fn slot_projection_at_call_site() {
    let source = r#"
component SlotBox {
    view {
        <div class: box>
            <div>@slot header</div>
            <div>@slot body</div>
        </div>
    }
}

component Consumer {
    signal title: String = "T"
    view {
        <SlotBox
            header={<h4>Custom Header</h4>}
            body={<div><p>Body content</p></div>}
        >
            <span>Default child</span>
        </SlotBox>
    }
}
"#;
    let output = compile_react(source).unwrap();
    let content: String = output
        .files
        .values()
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        content.contains("header={"),
        "Expected named slot projected to header prop, got:\n{}",
        content
    );
    assert!(
        content.contains("body={"),
        "Expected named slot projected to body prop, got:\n{}",
        content
    );
    assert!(
        content.contains("Custom Header"),
        "Expected projected header content, got:\n{}",
        content
    );
    assert!(
        content.contains("Default child"),
        "Expected un-slotted child to remain in children, got:\n{}",
        content
    );
    assert!(
        !content.contains("slot=\""),
        "no slot attribute may survive in generated output, got:\n{}",
        content
    );
}

/// Test: ADR-053 D5 body-level `@slot` declarations become destructured
/// props (`children` for the unnamed slot, same-named props for named
/// slots) and their `{name}` interpolations render with the declared
/// fallback — `{header ?? (fallback)}`, mirroring view-level slots.
#[test]
fn body_slot_declarations_destructure_and_render_with_fallback() {
    let source = r#"
component Modal {
    @slot header { <h4>Default header</h4> }
    @slot { <p>Default body</p> }
    view {
        <div>
            {header}
            {children}
        </div>
    }
}
"#;
    let output = compile_react(source).unwrap();
    let content: String = output
        .files
        .values()
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        content.contains("export function Modal({ header, children }: ModalProps)"),
        "Expected body slots destructured as props, got:\n{}",
        content
    );
    assert!(
        content.contains("header?: ReactNode;") && content.contains("children?: ReactNode;"),
        "Expected slot props in the Props interface, got:\n{}",
        content
    );
    assert!(
        content.contains("{header ?? (") && content.contains("{children ?? ("),
        "Expected fallback-wrapped slot renders, got:\n{}",
        content
    );
    assert!(
        content.contains("Default header") && content.contains("Default body"),
        "Expected declared fallbacks in output, got:\n{}",
        content
    );
}

/// Test: a body-level `@slot name: Type` declaration is a deleted form
/// (design D4) — the parser hard-errors; the plain `@slot footer` form
/// types the prop as `ReactNode` (parent-provided content).
#[test]
fn body_slot_type_constraint_hard_error() {
    let source = r#"
component Badge {
    @slot footer: String
    view {
        <div>{footer}</div>
    }
}
"#;
    let output = compile_react_full(source);
    assert!(
        !output.success,
        "typed body slot must be a hard error, got success"
    );
    assert!(
        output
            .errors
            .iter()
            .any(|e| e.message.contains("类型约束与数据参数已删除")),
        "Expected deleted-typed-slot error, got: {:?}",
        output.errors
    );
}

/// Test: the plain `@slot name` form (no type) types the prop as ReactNode
/// and the interpolation renders the bare prop.
#[test]
fn body_slot_plain_types_prop_reactnode() {
    let source = r#"
component Badge {
    @slot footer
    view {
        <div>{footer}</div>
    }
}
"#;
    let output = compile_react(source).unwrap();
    let content: String = output
        .files
        .values()
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        content.contains("footer?: ReactNode;"),
        "Expected ReactNode-typed slot prop, got:\n{}",
        content
    );
    assert!(
        content.contains("{footer}"),
        "Expected bare slot interpolation render, got:\n{}",
        content
    );
}

/// Test: consumer named-prop projection (`header={<div/>}`, ADR-053 D5)
/// projects into the body-declared slot prop of the child component, and a
/// bare instantiation keeps the declared fallbacks.
#[test]
fn consumer_named_prop_projects_into_body_declared_slot() {
    let source = r#"
component Modal {
    @slot header { <h4>Default header</h4> }
    @slot { <p>Default body</p> }
    view {
        <div>
            {header}
            {children}
        </div>
    }
}

component App {
    view {
        <Modal header={<div>Custom title</div>}>
            <p>Custom body</p>
        </Modal>
        <Modal />
    }
}
"#;
    let output = compile_react(source).unwrap();
    let content: String = output
        .files
        .values()
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");
    let app: String = output
        .files
        .iter()
        .filter(|(path, _)| path.contains("App.tsx"))
        .map(|(_, c)| c.clone())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        app.contains("header={<div>"),
        "Expected named-prop projection in consumer JSX, got:\n{}",
        app
    );
    assert!(
        app.contains("Custom title") && app.contains("Custom body"),
        "Expected projected content, got:\n{}",
        app
    );
    assert!(
        app.contains("<Modal />"),
        "Expected bare instantiation to keep declared fallbacks, got:\n{}",
        app
    );
    // The consumer must not declare slots itself.
    assert!(
        !app.contains("interface AppProps"),
        "Consumer must not gain a Props interface from projections, got:\n{}",
        app
    );
}

/// Test: a slot whose name collides with a signal is NOT materialized —
/// `{header}` stays a signal read (no `header ?? (fallback)`), and the slot
/// prop is skipped. The semantic layer warns on the collision.
#[test]
fn body_slot_colliding_with_signal_skips_materialization() {
    let source = r#"
component Badge {
    signal header: String = "live"
    @slot header { <h4>Fallback</h4> }
    view {
        <div>{header}</div>
    }
}
"#;
    let output = compile_react(source).unwrap();
    let content: String = output
        .files
        .values()
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        !content.contains("header ??"),
        "colliding slot must not materialize into a fallback render, got:\n{}",
        content
    );
    assert!(
        !content.contains("header?: ReactNode"),
        "colliding slot prop must be skipped, got:\n{}",
        content
    );
    assert!(
        content.contains("const header ="),
        "the signal read must still be emitted, got:\n{}",
        content
    );
}

/// Test: body-slot interpolations inside `@if`/`@for` bodies materialize too
/// (the materializer recurses into Conditional/ListRendering arms).
#[test]
fn body_slot_materializes_inside_conditionals_and_lists() {
    let source = r#"
component Panel {
    signal show: bool = true
    signal items: List<String> = []
    @slot header { <h4>Default header</h4> }
    view {
        @if show { {header} }
        @for item in items { {header} }
    }
}
"#;
    let output = compile_react(source).unwrap();
    let content: String = output
        .files
        .values()
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");
    assert_eq!(
        content.matches("header ?? (").count(),
        2,
        "both @if and @for positions should render the materialized slot, got:\n{}",
        content
    );
}

/// Test: when the same slot name is declared at the body level AND as a
/// view-level `@slot`, the body declaration wins for `{name}` interpolation
/// materialization (its fallback is used).
#[test]
fn body_slot_declaration_wins_over_view_level_duplicate() {
    let source = r#"
component Modal {
    @slot header { <h4>Body fallback</h4> }
    view {
        @slot header { <h4>View fallback</h4> }
        <div>{header}</div>
    }
}
"#;
    let output = compile_react(source).unwrap();
    let content: String = output
        .files
        .values()
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        content.contains("Body fallback"),
        "the interpolation must materialize with the body declaration's fallback, got:\n{}",
        content
    );
    assert!(
        content.contains("View fallback"),
        "the view-level slot keeps its own fallback, got:\n{}",
        content
    );
}

// ═══════════════════════════════════════════════════════════
// Task 3.3: slice-selector store subscription (perf HIGH-1/H2)
// ═══════════════════════════════════════════════════════════

/// Module-level store + a component that reads fields, calls actions and
/// computes over store fields — the canonical T3.3 fixture.
const STORE_SLICE_STORE_SRC: &str = r#"
store TodoStore {
    signal items: Array<String> = [];
    getter totalCount: i32 = items.len();
    action add(text: String) { }
}
"#;

const STORE_SLICE_COMP_SRC: &str = r#"
import { TodoStore } from "./store";

component TodoView {
    @store TodoStore;
    computed filtered: i32 = TodoStore.totalCount + 0;
    fn doAdd() {
        TodoStore.add("x");
    }
    view {
        <p>{TodoStore.items.len()}</p>
        <p>{filtered}</p>
        <button @onClick={doAdd}>add</button>
    }
}
"#;

/// Compile a store + component pair to React, returning the joined output.
fn compile_react_store_pair(store_src: &str, comp_src: &str) -> String {
    let mut sources = FxHashMap::default();
    sources.insert("store.omni".to_string(), store_src.to_string());
    sources.insert("main.omni".to_string(), comp_src.to_string());
    let pm = crate::path_mapping::derive_path_mapping(&sources);
    let result = crate::compile_multi_internal(
        sources,
        crate::codegen::Target::React,
        pm,
        false,
        false,
        None,
    );
    assert!(
        result.success,
        "store pair must compile. Errors: {:?}",
        result.errors
    );
    result
        .files
        .values()
        .cloned()
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn react_store_reads_use_slice_selectors() {
    let content = compile_react_store_pair(STORE_SLICE_STORE_SRC, STORE_SLICE_COMP_SRC);
    // Per-field slice selectors replace the whole-store hook subscription.
    assert!(
        content.contains("const items = useTodoStore((s) => s.items);"),
        "signal field must subscribe via slice selector. Got:\n{}",
        content
    );
    assert!(
        content.contains("const totalCount = useTodoStore((s) => s.totalCount);"),
        "getter field must subscribe via slice selector. Got:\n{}",
        content
    );
    assert!(
        content.contains("const add = useTodoStore((s) => s.add);"),
        "action field must subscribe via slice selector. Got:\n{}",
        content
    );
    assert!(
        !content.contains("const TodoStore = useTodoStore()"),
        "no whole-store subscription may be emitted. Got:\n{}",
        content
    );
    // Read sites are rewritten to the slice locals.
    assert!(
        content.contains("{items.length}"),
        "view read must use the slice local. Got:\n{}",
        content
    );
    assert!(
        !content.contains("TodoStore.items"),
        "qualified signal read must be rewritten. Got:\n{}",
        content
    );
    assert!(
        !content.contains("TodoStore.totalCount"),
        "qualified getter read must be rewritten. Got:\n{}",
        content
    );
    // Action calls still work — through the action slice.
    assert!(
        content.contains("add(\"x\");"),
        "action call must go through the action slice. Got:\n{}",
        content
    );
    assert!(
        !content.contains("TodoStore.add"),
        "qualified action call must be rewritten. Got:\n{}",
        content
    );
    // useMemo deps are the slice values, never the store object.
    assert!(
        content.contains("}, [totalCount]);"),
        "computed deps must reference the slice local. Got:\n{}",
        content
    );
    assert!(
        !content.contains("[TodoStore]"),
        "computed deps must not depend on the whole store object. Got:\n{}",
        content
    );
}

#[test]
fn react_store_slice_view_handler_and_effect_deps() {
    let store_src = r#"
store NoteStore {
    signal notes: Array<String> = [];
    action clear() { }
}
"#;
    let comp_src = r#"
import { NoteStore } from "./store";

component NoteView {
    @store NoteStore;
    @effect(notes) {
        let n = NoteStore.notes.len();
    }
    view {
        <button @onClick={NoteStore.clear}>clear</button>
        <p>{NoteStore.notes.len()}</p>
    }
}
"#;
    let content = compile_react_store_pair(store_src, comp_src);
    // Handler action calls (inline-handler form) are rewritten to the
    // action slice; the qualified name may never survive.
    assert!(
        content.contains("clear();"),
        "handler action must call the action slice. Got:\n{}",
        content
    );
    assert!(
        !content.contains("NoteStore.clear"),
        "qualified handler action must be rewritten. Got:\n{}",
        content
    );
    // Effect deps resolve through the slice local, not the qualified name.
    assert!(
        content.contains("}, [notes]);"),
        "effect deps must reference the slice local. Got:\n{}",
        content
    );
    assert!(
        !content.contains("[NoteStore]"),
        "effect deps must not depend on the whole store. Got:\n{}",
        content
    );
}

#[test]
fn react_store_whole_store_fallback_on_local_collision() {
    // A component signal named like a store field must NOT be shadowed by
    // the slice local — the store falls back to the whole-store object and
    // qualified reads stay qualified (identical to pre-T3.3 behavior).
    let store_src = r#"
store CartStore {
    signal items: Array<String> = [];
}
"#;
    let comp_src = r#"
import { CartStore } from "./store";

component CartView {
    @store CartStore;
    signal items: i32 = 0;
    view {
        <p>{CartStore.items.len()}</p>
        <p>{items}</p>
    }
}
"#;
    let content = compile_react_store_pair(store_src, comp_src);
    assert!(
        content.contains("const CartStore = useCartStore();"),
        "colliding store keeps the whole-store object. Got:\n{}",
        content
    );
    assert!(
        content.contains("{CartStore.items.length}"),
        "qualified read preserved on collision. Got:\n{}",
        content
    );
    assert!(
        content.contains("const items = 0;"),
        "component signal untouched. Got:\n{}",
        content
    );
}

/// Direct-IR fixture: StoreFieldAccessExpr reads + StoreAction handlers
/// (the semantic-normalized forms) must also emit slice selectors.
#[test]
fn react_store_field_access_expr_uses_slice_selectors() {
    let codegen = ReactCodegen::default();
    let store = StoreIR {
        name: "AppStore".into(),
        signals: vec![(
            "count".into(),
            TypeIR {
                kind: "i32".into(),
                type_args: vec![],
                is_nullable: false,
            },
        )]
        .into_iter()
        .collect::<FxHashMap<_, _>>(),
        signal_defaults: vec![("count".into(), serde_json::json!(0))]
            .into_iter()
            .collect::<FxHashMap<_, _>>(),
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
                "onClick".into(),
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
            construction_kind: ConstructionKind::Static,
            source_span: None,
        }))),
        styles: vec![],
        ..Default::default()
    };
    let ir = IRProgram {
        version: "0.1".into(),
        modules: vec![(
            "main".to_string(),
            ModuleIR {
                name: "main".to_string(),
                components: vec![("CounterWithStore".into(), comp)]
                    .into_iter()
                    .collect::<FxHashMap<_, _>>(),
                stores: vec![("AppStore".into(), store)]
                    .into_iter()
                    .collect::<FxHashMap<_, _>>(),
                ..Default::default()
            },
        )]
        .into_iter()
        .collect::<FxHashMap<_, _>>(),
        source_map: FxHashMap::default(),
        stdlib_imports: Default::default(),
        source_span: None,
        motion_index: FxHashMap::default(),
    };
    let sources = vec![("main.omni".into(), "".into())]
        .into_iter()
        .collect::<FxHashMap<_, _>>();
    let output = codegen.generate(&ir, &sources);
    assert!(output.errors.is_empty());
    let tsx = output
        .files
        .iter()
        .find(|f| f.path == "CounterWithStore.tsx")
        .unwrap()
        .content
        .clone();
    assert!(
        tsx.contains("const count = useAppStore((s) => s.count);"),
        "StoreFieldAccessExpr reads must subscribe via slice selector. Got:\n{}",
        tsx
    );
    assert!(
        tsx.contains("const increment = useAppStore((s) => s.increment);"),
        "StoreAction refs must subscribe via action slice. Got:\n{}",
        tsx
    );
    assert!(
        !tsx.contains("const AppStore = useAppStore()"),
        "no whole-store subscription. Got:\n{}",
        tsx
    );
    assert!(
        tsx.contains("{count}"),
        "StoreFieldAccessExpr read must be rewritten to the slice local. Got:\n{}",
        tsx
    );
    assert!(
        tsx.contains("onClick={() => { increment(); }}"),
        "StoreAction handler must call the action slice. Got:\n{}",
        tsx
    );
}

// ═══════════════════════════════════════════════════════════
// Task 3.3 review fixes: MEDIUM-1 + LOW-1/3/4 regression coverage
// ═══════════════════════════════════════════════════════════

/// MEDIUM-1: `array_push(list, TodoStore.field)` — the pushed item must go
/// through the store-field renames (`TodoStore.totalCount` → `totalCount`),
/// never emit an undeclared qualified reference. The interception fires on
/// bare signal names (component signals here; the shared item-formatting
/// line is the same for the store setState branch).
#[test]
fn react_store_array_push_item_uses_slice_renames() {
    let store_src = r#"
store PushStore {
    signal items: Array<String> = [];
    getter totalCount: i32 = items.len();
}
"#;
    let comp_src = r#"
import { PushStore } from "./store";
import { array_push } from "std/array";

component PushView {
    @store PushStore;
    signal local: Array<String> = [];
    fn doPush() {
        array_push(local, PushStore.totalCount.to_string());
    }
    view {
        <p>{PushStore.items.len()}</p>
        <p>{local.len()}</p>
        <button @onClick={doPush}>push</button>
    }
}
"#;
    let content = compile_react_store_pair(store_src, comp_src);
    assert!(
        content.contains("setLocal(prev => [...prev, totalCount.toString()]);"),
        "array_push item must reference the slice local. Got:\n{}",
        content
    );
    assert!(
        !content.contains("PushStore.totalCount"),
        "no qualified store reference may survive in the push item. Got:\n{}",
        content
    );
}

/// LOW-1: a component signal initialized from a store field must use the
/// slice local in the emitted initializer.
#[test]
fn react_store_signal_initializer_uses_slice_renames() {
    let store_src = r#"
store InitStore {
    signal base: i32 = 10;
}
"#;
    let comp_src = r#"
import { InitStore } from "./store";

component InitView {
    @store InitStore;
    signal copy: i32 = InitStore.base + 1;
    view {
        <p>{copy}</p>
        <p>{InitStore.base}</p>
    }
}
"#;
    let content = compile_react_store_pair(store_src, comp_src);
    assert!(
        content.contains("const copy = base + 1;"),
        "signal initializer must reference the slice local. Got:\n{}",
        content
    );
    assert!(
        !content.contains("InitStore.base"),
        "no qualified store reference may survive in the initializer. Got:\n{}",
        content
    );
}

/// LOW-3: a BARE store-alias reference (`let s = TodoStore;`) forces the
/// whole-store object — the alias must stay declared and usable.
#[test]
fn react_store_bare_alias_keeps_whole_store() {
    let store_src = r#"
store AliasStore {
    signal items: Array<String> = [];
}
"#;
    let comp_src = r#"
import { AliasStore } from "./store";

component AliasView {
    @store AliasStore;
    fn getLen() {
        let s = AliasStore;
    }
    view {
        <p>{AliasStore.items.len()}</p>
        <button @onClick={getLen}>go</button>
    }
}
"#;
    let content = compile_react_store_pair(store_src, comp_src);
    assert!(
        content.contains("const AliasStore = useAliasStore();"),
        "bare store-alias usage keeps the whole-store object. Got:\n{}",
        content
    );
    assert!(
        content.contains("let s = AliasStore;"),
        "bare alias reference must stay qualified. Got:\n{}",
        content
    );
    assert!(
        content.contains("{AliasStore.items.length}"),
        "qualified reads preserved with the whole-store object. Got:\n{}",
        content
    );
}

/// LOW-4: library functions are emitted at MODULE scope — neither the slice
/// locals nor a whole-store const exist there, so their store references
/// keep the pre-T3.3 qualified output (pre-existing limitation, unchanged).
#[test]
fn react_store_lib_fn_keeps_qualified_output() {
    let store_src = r#"
store LibStore {
    signal items: Array<String> = [];
    getter totalCount: i32 = items.len();
}
"#;
    let comp_src = r#"
import { LibStore } from "./store";
pub fn describeCount() -> i32 {
    return LibStore.totalCount;
}
component LibView {
    @store LibStore;
    view {
        <p>{describeCount()}</p>
        <p>{LibStore.items.len()}</p>
    }
}
"#;
    let content = compile_react_store_pair(store_src, comp_src);
    assert!(
        content.contains("return LibStore.totalCount;"),
        "lib fn body must keep the qualified store read (module scope). Got:\n{}",
        content
    );
    assert!(
        content.contains("const items = useLibStore((s) => s.items);"),
        "the component itself still slices. Got:\n{}",
        content
    );
    assert!(
        !content.contains("return totalCount;"),
        "lib fn must NOT reference a component-scope slice local. Got:\n{}",
        content
    );
}

// ═══════════════════════════════════════════════════════════
// Task 3.3 review round 2: NEW-1 OptionalChainExpr collection
// ═══════════════════════════════════════════════════════════

/// NEW-1: `OptionalChainExpr` (`s?.items`) was missing from the store-field
/// collection walker — any `?.` in a component body hit the `unreachable!`
/// arm and PANICKED the React backend. The optional-chain must compile, and
/// a store optional chain (`TodoStore?.items`) must slice like a plain read.
#[test]
fn react_store_optional_chain_no_panic_and_slices() {
    let store_src = r#"
store OptStore {
    signal items: Array<String> = [];
}
"#;
    let comp_src = r#"
import { OptStore } from "./store";

component OptView {
    @store OptStore;
    fn peek(s: String) {
        let a = s?.items;
    }
    fn peekStore() {
        let b = OptStore?.items;
    }
    view {
        <p>{OptStore.items.len()}</p>
        <button @onClick={peek}>go</button>
        <button @onClick={peekStore}>store</button>
    }
}
"#;
    // Must not panic — the compile itself is the regression assertion.
    let content = compile_react_store_pair(store_src, comp_src);
    assert!(
        content.contains("const items = useOptStore((s) => s.items);"),
        "store field must subscribe via slice selector. Got:\n{}",
        content
    );
    assert!(
        content.contains("let a = s?.items;"),
        "non-store optional chain must stay qualified. Got:\n{}",
        content
    );
    assert!(
        !content.contains("OptStore?.items"),
        "store optional chain must be rewritten to the slice local. Got:\n{}",
        content
    );
}

/// Task 2.1 side-fix: optional-chain METHOD calls must run through the
/// method table — `user?.len()` must emit `user?.length`. The shared
/// default emits `?.len()` verbatim, which fails tsc (TS2339 'len' on
/// string) — exposed by the fresh learn-omniview regen during the 2.1
/// verification (OptionalChainDemo computed `nameLen`).
#[test]
fn react_optional_chain_method_len_translates_to_length() {
    let source = r#"
import omniview;
component OptLen {
    signal s: String = "abc";
    computed n: i32 = s?.len() ?? 0;
    view {
        <p>{n}</p>
    }
}
"#;
    let output = compile_react(source).unwrap();
    let content: String = output
        .files
        .values()
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        content.contains("s?.length"),
        "optional-chain len() must translate to ?.length, got:\n{}",
        content
    );
    assert!(
        !content.contains("s?.len()"),
        "no verbatim ?.len() may survive, got:\n{}",
        content
    );
}
