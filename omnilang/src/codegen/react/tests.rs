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
    assert_eq!(to_react_event("click"), Some("onClick".into()));
    assert_eq!(to_react_event("change"), Some("onChange".into()));
    assert_eq!(to_react_event("submit"), Some("onSubmit".into()));
    assert_eq!(to_react_event("focus"), Some("onFocus".into()));
    assert_eq!(to_react_event("dblclick"), Some("onDoubleClick".into()));
    assert_eq!(to_react_event("keydown"), Some("onKeyDown".into()));
    assert_eq!(to_react_event("customEvent"), Some("onCustomEvent".into()));
    // v4 3.3 (compiler HIGH-3): Flutter-only conveniences have no React
    // synthetic event — None triggers warn + skip (never emit the dropped
    // onLongpress prop). Any spelling canonicalizes to the same None.
    assert_eq!(to_react_event("longpress"), None);
    assert_eq!(to_react_event("onLongpress"), None);
    assert_eq!(to_react_event("hover"), None);
    // Task 6.1b (gate FAIL-2): toggle IS a React synthetic event — but the
    // DOM-element call site gates it to `<details>` (other elements warn +
    // skip, see view.rs). Any spelling canonicalizes to the same Some.
    assert_eq!(to_react_event("toggle"), Some("onToggle".into()));
    assert_eq!(to_react_event("onToggle"), Some("onToggle".into()));
    // Real React synthetic events the old fallback arm served by accident
    // stay mapped (explicit arms, v4 3.3).
    assert_eq!(to_react_event("copy"), Some("onCopy".into()));
    assert_eq!(to_react_event("select"), Some("onSelect".into()));
    assert_eq!(to_react_event("load"), Some("onLoad".into()));
    assert_eq!(to_react_event("resize"), Some("onResize".into()));
    // Component-instance events keep the legacy spelling (declared event
    // props still wire).
    assert_eq!(to_react_event_or_legacy("onLongpress"), "onLongpress");
    assert_eq!(to_react_event_or_legacy("customEvent"), "onCustomEvent");
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
fn test_format_method_call_splice() {
    // Task 4.10 review I-1 — `.splice(start, deleteCount, ...items)` is a
    // cross-backend language method: React slice-combination (pinned here),
    // Dart sublist-combination (shared table), OmniView native
    // Array.prototype.splice.
    let expr = ExprIR::MethodCallExpr {
        object: Box::new(ExprIR::IdentifierExpr {
            name: "items".into(),
            type_ann: Some("Array<i32>".into()),
        }),
        method: "splice".into(),
        args: vec![
            ExprIR::LiteralExpr {
                value: serde_json::json!(1),
            },
            ExprIR::LiteralExpr {
                value: serde_json::json!(2),
            },
        ],
        is_optional: false,
    };
    assert_eq!(
        format_expr_ir_tsx(&expr, &FxHashSet::default()),
        "[...items.slice(0, 1), ...items.slice(1 + 2)]"
    );
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
        is_mut: false,
    };
    generate_statement_tsx(
        &mut w,
        &stmt,
        &FxHashSet::default(),
        &FxHashMap::default(),
        &FxHashSet::default(),
        &FxHashMap::default(),
        &FxHashSet::default(),
        &FxHashSet::default(),
    );
    assert_eq!(w.into_string(), "const x = 42;\n");
}

#[test]
fn test_tsx_stmt_let_mut() {
    let mut w = CodeWriter::new();
    let stmt = StatementIR::LetStmtIR {
        name: "x".into(),
        value: Some(ExprIR::LiteralExpr {
            value: serde_json::json!(42),
        }),
        r#type: None,
        is_mut: true,
    };
    generate_statement_tsx(
        &mut w,
        &stmt,
        &FxHashSet::default(),
        &FxHashMap::default(),
        &FxHashSet::default(),
        &FxHashMap::default(),
        &FxHashSet::default(),
        &FxHashSet::default(),
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
        &FxHashSet::default(),
        &FxHashSet::default(),
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
        &FxHashSet::default(),
        &FxHashSet::default(),
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
        &FxHashSet::default(),
        &FxHashSet::default(),
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
        &FxHashSet::default(),
        &FxHashSet::default(),
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
        &FxHashSet::default(),
        &FxHashSet::default(),
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
        &FxHashSet::default(),
        &FxHashSet::default(),
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
        &FxHashSet::default(),
        &FxHashSet::default(),
    );
    assert_eq!(w.into_string(), "setCount(step);\n");
}

/// M2 struct-literal regression: `stats = Stats { name: stats.name, ... }`
/// (self-read through a struct literal) must wrap the object so TS does not
/// parse it as an arrow block body — `setStats((s) => ({ ... }))`, not
/// `setStats((s) => { ... })` (TS1005).
#[test]
fn test_tsx_stmt_mutation_assign_signal_self_read_struct_literal_wrapped() {
    let mut w = CodeWriter::new();
    let stmt = StatementIR::MutationStmt {
        intent: Some(MutationIntent::DirectAssign {
            target: SignalRefIR {
                name: "stats".into(),
            },
            value: Some(Box::new(ExprIR::StructExpr {
                type_name: "StatisticsData".into(),
                fields: vec![
                    ("name".into(), ExprIR::FieldAccessExpr {
                        object: Box::new(ExprIR::ident("stats")),
                        field: "name".into(),
                    }),
                    ("count".into(), ExprIR::BinaryOpExpr {
                        op: BinaryOpIR::Add,
                        left: Box::new(ExprIR::FieldAccessExpr {
                            object: Box::new(ExprIR::ident("stats")),
                            field: "count".into(),
                        }),
                        right: Box::new(ExprIR::LiteralExpr {
                            value: serde_json::json!(1),
                        }),
                    }),
                ],
            })),
            field_object: None,
        }),
    };
    let mut sig_names = FxHashSet::default();
    sig_names.insert("stats".to_string());
    generate_statement_tsx(
        &mut w,
        &stmt,
        &sig_names,
        &FxHashMap::default(),
        &FxHashSet::default(),
        &FxHashMap::default(),
        &FxHashSet::default(),
        &FxHashSet::default(),
    );
    assert_eq!(
        w.into_string(),
        "setStats((s) => ({ \"name\": s.name, \"count\": s.count + 1 }));\n"
    );
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
        &FxHashSet::default(),
        &FxHashSet::default(),
    );
    assert_eq!(
        w.into_string(),
        "useAppStore.setState((s) => ({ count: s.count + 1 }));\n"
    );
}

/// Review I1 (task 2.11): the write-only signal DCE prunes by BARE target
/// name — a component signal and a store field can share a name (`signal
/// count` + `AppStore.count`). A pruned component signal's mutation must be
/// skipped, but a STORE-FIELD mutation of the same bare name resolves
/// through `store_signal_owners` to `useStore.setState(...)` — the store
/// hook exists regardless of the DCE, so the store write must NEVER be
/// pruned (silent data loss otherwise).
#[test]
fn test_pruned_signal_dce_never_swallows_store_field_mutation() {
    let mut w = CodeWriter::new();
    let stmt = StatementIR::MutationStmt {
        intent: Some(MutationIntent::DirectAssign {
            target: SignalRefIR {
                name: "mode".into(),
            },
            value: Some(Box::new(ExprIR::LiteralExpr {
                value: serde_json::json!("dark"),
            })),
            field_object: Some("CollisionStore".to_string()),
        }),
    };
    let mut owners = FxHashMap::default();
    owners.insert("mode".to_string(), "CollisionStore".to_string());
    let mut pruned = FxHashSet::default();
    pruned.insert("mode".to_string());
    generate_statement_tsx(
        &mut w,
        &stmt,
        &FxHashSet::default(),
        &owners,
        &FxHashSet::default(),
        &FxHashMap::default(),
        &pruned,
        &FxHashSet::default(),
    );
    assert_eq!(
        w.into_string(),
        "useCollisionStore.setState({ mode: \"dark\" });\n",
        "a store-field mutation must survive the DCE even when a pruned \
         component signal shares its bare name (review I1)"
    );
}

/// Review I1 — the mirror case: the colliding COMPONENT signal's own write
/// (`mode = !mode`, NO field_object) must stay pruned even when the owners
/// map holds the same bare name — an owners-map-only guard would un-prune
/// it and misroute it through the owners map to the store setter. The
/// field_object discriminator prevents that: output must be empty.
#[test]
fn test_pruned_component_write_not_misrouted_to_store_by_collision() {
    let mut w = CodeWriter::new();
    let stmt = StatementIR::MutationStmt {
        intent: Some(MutationIntent::DirectAssign {
            target: SignalRefIR {
                name: "mode".into(),
            },
            value: Some(Box::new(ExprIR::UnaryOpExpr {
                op: UnaryOpIR::Not,
                operand: Box::new(ExprIR::ident("mode")),
            })),
            field_object: None,
        }),
    };
    let mut owners = FxHashMap::default();
    owners.insert("mode".to_string(), "CollisionStore".to_string());
    let mut pruned = FxHashSet::default();
    pruned.insert("mode".to_string());
    generate_statement_tsx(
        &mut w,
        &stmt,
        &FxHashSet::default(),
        &owners,
        &FxHashSet::default(),
        &FxHashMap::default(),
        &pruned,
        &FxHashSet::default(),
    );
    assert_eq!(
        w.into_string(),
        "\n", // empty writer output — nothing emitted
        "a plain component-signal write (no field_object) must stay pruned \
         even when a store field shares its bare name (review I1)"
    );
}

/// Review N1 companion — the NON-colliding bare store-field push: with an
/// empty pruned set (no write-only component signal sharing the name), the
/// array_push interception falls through to the owners branch and emits
/// the store's functional setState — the pre-existing bare store-field
/// contract. The analysis-time intersection (pruned ∩ comp.signals) is
/// what keeps a bare store field name OUT of the pruned set, so this shape
/// is exactly the store field's protection.
#[test]
fn test_bare_store_field_array_push_emits_store_set_state() {
    let mut w = CodeWriter::new();
    let stmt = StatementIR::ExprStmtIR {
        expr: ExprIR::CallExpr {
            name: "array_push".to_string(),
            args: vec![ExprIR::ident("mode")],
        },
    };
    let mut owners = FxHashMap::default();
    owners.insert("mode".to_string(), "CollisionStore".to_string());
    generate_statement_tsx(
        &mut w,
        &stmt,
        &FxHashSet::default(),
        &owners,
        &FxHashSet::default(),
        &FxHashMap::default(),
        &FxHashSet::default(),
        &FxHashSet::default(),
    );
    assert_eq!(
        w.into_string(),
        "useCollisionStore.setState((s) => ({ mode: [...s.mode, undefined] }));\n",
        "a bare store-field array_push (not in pruned) must emit the store setState"
    );
}

/// Review N1 (round 2) — the array_push interception mirror: a pruned
/// COMPONENT signal's own `array_push(mode, x)` must be pruned EVEN WHEN a
/// store field shares its bare name (owners map contains `mode`). An
/// owners guard would fall through and emit the store's functional
/// `useStore.setState` push — a cross-write the user never wrote (same
/// defect class as I1's misroute). The pruned set is intersected with
/// comp.signals at analysis time, so this emission-level shape (pruned
/// name + owners entry) can only be a colliding component signal.
#[test]
fn test_pruned_component_array_push_not_misrouted_to_store_by_collision() {
    let mut w = CodeWriter::new();
    let stmt = StatementIR::ExprStmtIR {
        expr: ExprIR::CallExpr {
            name: "array_push".to_string(),
            args: vec![ExprIR::ident("mode")],
        },
    };
    let mut owners = FxHashMap::default();
    owners.insert("mode".to_string(), "CollisionStore".to_string());
    let mut pruned = FxHashSet::default();
    pruned.insert("mode".to_string());
    generate_statement_tsx(
        &mut w,
        &stmt,
        &FxHashSet::default(),
        &owners,
        &FxHashSet::default(),
        &FxHashMap::default(),
        &pruned,
        &FxHashSet::default(),
    );
    assert_eq!(
        w.into_string(),
        "\n", // empty writer output — nothing emitted
        "a pruned component signal's array_push must be pruned, never \\
         misrouted to the store setState (review N1)"
    );
}

/// Review I1 + N1 — full-pipeline collision shape: a component with a
/// write-only signal `mode` (pruned by the DCE) AND a mutation of a store
/// field `mode` (`CollisionStore.mode = "dark"` — same bare target name,
/// the store wired by a view read of the same field). The store write must
/// survive (field_object discriminator, I1); the component signal's own
/// array_push must be pruned and never misrouted to the store's functional
/// setState (comp.signals intersection, N1).
#[test]
fn test_pruned_component_signal_keeps_colliding_store_field_mutation() {
    let out = compile_react(
        r#"
        import omniview;
        import { array_push } from "std/array";

        store CollisionStore {
            signal mode: string = "light";
        }

        component CollisionDemo {
            @store CollisionStore;
            signal mode: Array<String> = [];
            fn setDark() { CollisionStore.mode = "dark"; }
            fn pushMode() { array_push(mode, "x"); }
            view {
                <button @click={setDark}>dark</button>
                <button @click={pushMode}>push</button>
                <span>{CollisionStore.mode}</span>
            }
        }
    "#,
    );
    let out = out.expect("collision fixture must compile");
    let tsx = out
        .files
        .get("CollisionDemo.tsx")
        .expect("CollisionDemo.tsx emitted");
    assert!(
        tsx.contains("useCollisionStore.setState({ mode: \"dark\" });"),
        "the STORE write must be preserved (review I1 — the pruned component \
         signal shares its bare name). Got:\n{}",
        tsx
    );
    assert!(
        !tsx.contains("setMode"),
        "the write-only component signal must stay pruned. Got:\n{}",
        tsx
    );
    assert!(
        !tsx.contains("const [mode, setMode] = useState"),
        "no state hook for the write-only component signal. Got:\n{}",
        tsx
    );
    assert!(
        !tsx.contains("useCollisionStore.setState((s) => ({ mode: [..."),
        "the component signal's array_push must NOT misroute to the store's \
         functional setState (review N1). Got:\n{}",
        tsx
    );
    assert!(
        !tsx.contains("setMode(prev =>"),
        "the component signal's array_push must be pruned entirely. Got:\n{}",
        tsx
    );
    assert!(
        tsx.contains("const mode = useCollisionStore((s) => s.mode);")
            || tsx.contains("const CollisionStore = useCollisionStore();"),
        "the store must be subscribed (view read wiring). Got:\n{}",
        tsx
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
        &FxHashSet::default(),
        &FxHashSet::default(),
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
                is_mut: false,
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
        &FxHashSet::default(),
        &FxHashSet::default(),
    );
    let out = w.into_string();
    assert!(out.contains("if (cond) {"));
    assert!(out.contains("const x = 1;"));
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

// ── Task 2.6: @unmount cleanup reads enter deps + _M hoist ─

/// Timer class (ts/react M4): the `@unmount` cleanup body reads `count` but
/// the emitted useEffect deps were hardcoded `[]` — the cleanup closure ran
/// at unmount with the FIRST-render count (final count always 0). The
/// cleanup reads must enter the deps array: with `}, [count]);` React
/// re-registers the cleanup on every count change, so the unmount cleanup
/// closes over the LATEST count.
///
/// (The signal is initialized from a PROP, not a literal — a literal
/// initializer would be hoisted to a module-scope const by perf M3 and a
/// constant correctly needs no deps. A prop-backed signal is reactive and
/// must keep its dep.)
#[test]
fn effect_cleanup_reads_are_deps() {
    let src = r#"
        component Timer {
            prop initial: i32 = 0;
            signal count: i32 = initial;
            @unmount {
                println("Timer unmounted, final count: " + count.to_string());
            }
            view { <div/> }
        }
    "#;
    let (result, diags) = compile_react_with_diags(src);
    assert!(
        diags
            .iter()
            .all(|d| d.kind != DiagnosticKind::EEffectDepsMissing),
        "cleanup read must be accepted as dep — got: {:?}",
        diags
            .iter()
            .map(|d| (d.kind.code_str(), d.message.clone()))
            .collect::<Vec<_>>()
    );
    let tsx = result.files.get("Timer.tsx").expect("Timer.tsx emitted");
    assert!(
        tsx.contains("useEffect(() => {") && tsx.contains("return () => {"),
        "unmount must remain a cleanup closure. Got:\n{}",
        tsx
    );
    assert!(
        tsx.contains("}, [count]);"),
        "cleanup read `count` must enter the deps array (stale-closure fix). Got:\n{}",
        tsx
    );
}

/// Store-field read inside an `@unmount` cleanup must resolve through the
/// T3.3 slice renames exactly like `@effect` deps — the slice local enters
/// the deps array (`[level]`), never the undefined whole-store alias.
#[test]
fn unmount_store_field_read_resolves_deps_via_slice() {
    let src = r#"
        store LogStore {
            signal level: i32 = 0;
        }
        component T {
            @store LogStore;
            @unmount {
                println("level: " + LogStore.level.to_string());
            }
            view { <div/> }
        }
    "#;
    let out = compile_react(src).expect("compile must succeed");
    let tsx = out.files.get("T.tsx").expect("T.tsx emitted");
    assert!(
        tsx.contains("const level = useLogStore((s) => s.level);"),
        "unmount read must subscribe via slice selector. Got:\n{}",
        tsx
    );
    assert!(
        tsx.contains("}, [level]);"),
        "store-field cleanup read must enter deps via the slice local. Got:\n{}",
        tsx
    );
}

/// perf M4 (task 2.8): a component fn passed as a child component's prop
/// must be wrapped in useCallback (stable identity across renders), and a
/// child instance receiving no children/slots must be React.memo-wrapped —
/// together they kill list-cascade re-renders.
#[test]
fn prop_fn_wrapped_in_use_callback() {
    let out = compile_react(
        r#"
        store TodoStore {
            signal items: Array<String> = [];
            action add(item: String) {
                items.push(item);
            }
        }
        component AddTodo {
            event onAdd: String;
            view { <button/> }
        }
        component TodoList {
            @store TodoStore;
            fn handleAdd(x) { TodoStore.add(x) }
            view { <AddTodo onAdd={handleAdd}/> }
        }
    "#,
    )
    .expect("compile must succeed");
    let tsx = out.files.get("TodoList.tsx").expect("TodoList.tsx emitted");
    assert!(
        tsx.contains("useCallback"),
        "prop fn must be memoized: {}",
        tsx
    );
    assert!(
        tsx.contains("React.memo(AddTodo)") || tsx.contains("memo(AddTodo)"),
        "child must be memoized: {}",
        tsx
    );
}

/// Task 5.5 regression pin: `emit onX(...)` inside an expression-level
/// block body (computed with a block body) must still count `onX` as a
/// read — the shared-walker migration of `collect_expr_identifiers`
/// (d112b68d) surfaced only `ExprIR` nodes, and `EventEmitStmtIR.event_name`
/// is a `String` invisible to the `&ExprIR` walker callback. The old
/// hand-rolled walker delegated BlockExpr/MatchExpr statements to
/// `collect_stmt_identifiers`, which collects the event name; without it,
/// the generated `onNotify?.("computed")` call inside the `useMemo` body
/// referenced an event prop that was never destructured (TS2304 — event
/// props are destructured only when `read_signals` contains the name).
/// A targeted emit-name scanner (see `collect_expr_identifiers`) restores
/// the behavior; this test pins the generated code.
#[test]
fn emit_inside_computed_block_body_golden() {
    let out = compile_react(
        r#"
        component App {
            event onNotify: String;
            signal base: i32 = 1;
            computed doubled: i32 = {
                emit onNotify("computed");
                base * 2;
            };
            view {
                <div>{doubled}</div>
            }
        }
    "#,
    )
    .expect("compile must succeed");
    let tsx = out.files.get("App.tsx").expect("App.tsx emitted");
    assert!(
        tsx.contains("useMemo"),
        "block-form computed must be a useMemo: {}",
        tsx
    );
    assert!(
        tsx.contains("onNotify?.(\"computed\")"),
        "emit call must use the optional-call form (event props are optional, TS2722): {}",
        tsx
    );
    // The event prop must be destructured: expression-level emit counts as
    // a read of the event prop name (statement-level emit in fns/effects
    // does the same via collect_stmt_identifiers).
    assert!(
        tsx.contains("export function App({ onNotify }"),
        "expression-level emit must destructure the event prop (TS2304 guard): {}",
        tsx
    );
}

/// Task 5.5 pin, nested variants: emit names nested deeper inside
/// expression-level blocks (inside an `if` statement in the block body, and
/// inside a `match` arm) must also count as reads — the targeted scanner
/// mirrors the old walker's full statement descent.
#[test]
fn emit_inside_computed_block_body_nested_golden() {
    let out = compile_react(
        r#"
        component App {
            event onNested: String;
            event onMatched: String;
            signal flag: Bool = false;
            signal base: i32 = 1;
            computed doubled: i32 = {
                if flag {
                    emit onNested("in-if");
                }
                match base {
                    1 => { emit onMatched("in-arm"); 10 }
                    _ => 20
                }
            };
            view {
                <div>{doubled}</div>
            }
        }
    "#,
    )
    .expect("compile must succeed");
    let tsx = out.files.get("App.tsx").expect("App.tsx emitted");
    assert!(
        tsx.contains("export function App({ onNested, onMatched }"),
        "nested expression-level emits must destructure both event props (TS2304 guard): {}",
        tsx
    );
}

/// Task 5.6 (deferral #61): event-handler call sites must emit the
/// optional-call form `f?.()` when the handler may be undefined — event
/// props are optional (`onX?: (args) => void`), so a bare `f(...)` is a
/// TS2722 possibly-undefined invocation (25 instances across the learn
/// project at the 0.6 snapshot).
#[test]
fn event_handler_call_sites_use_optional_call() {
    let out = compile_react(
        r#"
        component V {
            signal f: fn(String) -> Void = null;
            view { <button @onClick={f}/> }
        }
    "#,
    )
    .expect("compile must succeed");
    let tsx = out.files.get("V.tsx").expect("V.tsx emitted");
    assert!(
        tsx.contains("f?.("),
        "must guard possibly-undefined handler with optional call: {}",
        tsx
    );
}

/// Task 5.6 (deferral #61): a DIRECT call to an event prop — the corpus
/// form (`onSwitch("overview")` as an expression statement, no `emit`
/// keyword) — must emit the optional-call form too (TS2722).
#[test]
fn event_prop_direct_calls_use_optional_call() {
    let out = compile_react(
        r#"
        component V {
            event onGo: String;
            fn send() {
                onGo("x");
            }
            view { <button @onPress={send}/> }
        }
    "#,
    )
    .expect("compile must succeed");
    let tsx = out.files.get("V.tsx").expect("V.tsx emitted");
    assert!(
        tsx.contains("onGo?.(\"x\")"),
        "direct event-prop call must be optional-called: {}",
        tsx
    );
}

/// Task 5.6 (deferral #61): LOCAL fns are always defined — their call
/// sites must stay bare `f(...)` (no `?.()` noise).
#[test]
fn local_fn_calls_stay_bare() {
    let out = compile_react(
        r#"
        component V {
            fn helper() {}
            fn useIt() {
                helper();
            }
            view { <button @onPress={useIt}/> }
        }
    "#,
    )
    .expect("compile must succeed");
    let tsx = out.files.get("V.tsx").expect("V.tsx emitted");
    assert!(
        tsx.contains("helper();"),
        "local fn call must stay bare: {}",
        tsx
    );
}

/// perf L1: the `_M` motion table is a compile-time constant (motion defs
/// are literals) — it was re-allocated inside the component function on
/// EVERY render, and each re-allocation made every `_M["x"]` use site a
/// fresh object identity (exhaustive-deps errors at every reference).
/// Module scope: one allocation per module, stable identity.
#[test]
fn motion_table_hoisted_to_module_scope() {
    let out = compile_react(
        r#"
        @motion fadeIn {
            from { opacity: 0 }
            to   { opacity: 1 }
            over: 300ms
        }
        component V {
            view { <div @hover={fadeIn}/> }
        }
    "#,
    )
    .expect("compile must succeed");
    let tsx = out.files.get("V.tsx").expect("V.tsx emitted");
    let fn_start = tsx
        .find("export function V")
        .expect("component function emitted");
    let table = tsx.find("const _M").expect("_M table emitted");
    assert!(
        table < fn_start,
        "_M must be module-scoped (before `export function V`), not per-render. Got:\n{}",
        tsx
    );
    assert_eq!(
        tsx.matches("const _M").count(),
        1,
        "_M must be emitted exactly once (module scope). Got:\n{}",
        tsx
    );
}

/// perf M1 (task 2.10): a store reference inside a MOTION VALUE used to
/// force the whole-store subscription (`const S = useS()`) — motion value
/// formatting (keyframes.rs) carried no slice renames, so the qualified
/// `Store.field` read needed the whole-store object. The motion refs must
/// resolve through the same `store_field_renames` slice channel as view/body
/// expressions (subscription granularity: a store write re-renders only when
/// the CHANGED field feeds the motion).
///
/// (The brief's `view V {...}` shorthand, element-level `@motion="m"`
/// attribute and store reference in the `style` prop do not parse or do not
/// force whole-store today — the store reference moves into the `@motion`
/// def body, the shape that actually exercises the fallback; the component
/// must declare `@store S;` or no subscription is emitted at all. Test name,
/// assertions and failure messages are kept verbatim.)
#[test]
fn motion_store_value_uses_slice() {
    let out = compile_react(
        r#"
        store S { signal pos: i32 = 0; }
        @motion m {
            from { translateX: 0 }
            to   { translateX: S.pos }
            over: 300ms
        }
        component V {
            @store S;
            view { <div @scroll={m}/> }
        }
    "#,
    )
    .expect("compile must succeed");
    let tsx = out.files.get("V.tsx").expect("V.tsx emitted");
    assert!(
        !tsx.contains("const S = useS()"),
        "motion store ref must not force whole-store: {}",
        tsx
    );
    assert!(
        tsx.contains("(s) => s.pos"),
        "expected slice selector: {}",
        tsx
    );
    // The slice local must reach the MOTION VALUE itself — the keyframe
    // transform embeds the renamed field (also pins the task 2.6 deferred
    // case: a store-ref motion keeps the `_M` table in-function — slice
    // locals are component-scope, a module-level table cannot see them).
    assert!(
        tsx.contains("translateX(${pos}px)"),
        "motion value must reference the slice local: {}",
        tsx
    );
    assert!(
        tsx.contains("const _M"),
        "_M table must be emitted (in-function for the store-ref motion): {}",
        tsx
    );
}

/// perf M5 (task 2.10): @scroll progress handling recomputes
/// `Number(anim.effect?.getTiming()?.duration)` and writes `anim.currentTime`
/// on EVERY scroll event, even when the progress value did not change —
/// the seek must be gated by an equality guard. (The brief's disjunction
/// `scrubCache || lastProgress` accidentally matches the pre-existing
/// `scrubCache` smoothing variable, so the assertion is narrowed to the
/// guard's own variable; test name and failure message are kept verbatim.)
#[test]
fn scroll_progress_has_equality_guard() {
    let out = compile_react(
        r#"
        @motion m {
            from { opacity: 0 }
            to   { opacity: 1 }
        }
        component V {
            view { <div @scroll={m}/> }
        }
    "#,
    )
    .expect("compile must succeed");
    let tsx = out.files.get("V.tsx").expect("V.tsx emitted");
    assert!(
        tsx.contains("lastProgress"),
        "missing equality guard: {}",
        tsx
    );
}

// ── v4 H1: write-then-read threading (task 2.1) ────────────

/// v4 H1: `agreed = !agreed; statusText = if agreed {...}` must generate
/// sequential reads — setX 后读新值（const next + setX(next) + next 续用）。
/// React setX 是异步的，直接读 `agreed` 会拿到陈旧闭包值。
#[test]
fn handler_write_then_read_threads_next_value() {
    let out = compile_react(
        "component App { signal agreed: Bool = false; signal statusText: String = \"\"; \
         view { <input bind:checked={agreed} @onChange={() => { agreed = !agreed; \
         statusText = if agreed { \"Agreed\" } else { \"Not agreed\" }; }} /> } }",
    )
    .expect("compile must succeed");
    let tsx = out.files.get("App.tsx").expect("App.tsx emitted");
    assert!(
        tsx.contains("setAgreed(nextAgreed)") && tsx.contains("setStatusText(nextAgreed ?"),
        "写后读未线程化：\n{}",
        tsx
    );
    assert!(
        !tsx.contains("setStatusText(agreed ?"),
        "陈旧闭包仍存在：\n{}",
        tsx
    );
}

/// v4 H1: the component-fn handler path (the H1 browser-test host —
/// FormControlsDemo's `fn handleAgree()`) must thread the same way.
#[test]
fn fn_handler_write_then_read_threads_next_value() {
    let out = compile_react(
        "component App { signal agreed: Bool = false; signal statusText: String = \"\"; \
         fn handleAgree() { agreed = !agreed; statusText = if agreed { \"Agreed\" } \
         else { \"Not agreed\" }; } \
         view { <input type=\"checkbox\" checked={agreed} @onChange={handleAgree} /> } }",
    )
    .expect("compile must succeed");
    let tsx = out.files.get("App.tsx").expect("App.tsx emitted");
    assert!(
        tsx.contains("setAgreed(nextAgreed)") && tsx.contains("setStatusText(nextAgreed ?"),
        "fn 处理器写后读未线程化：\n{}",
        tsx
    );
    assert!(
        !tsx.contains("setStatusText(agreed ?"),
        "fn 处理器陈旧闭包仍存在：\n{}",
        tsx
    );
}

/// v4 评审 Important 1: `count += 1; count += 1; use(count)` 必须逐次
/// 线程化——Increment 的隐式 target 读（下值 = target + amount）不能漏判。
/// 漏判则第一轮保持 functional updater、第二轮分裂读陈旧闭包值，
/// React 队列终态 old+1 而非 old+2（状态值回归）。
#[test]
fn increment_chain_write_then_read_threads_in_order() {
    let out = compile_react(
        "component App { signal count: i32 = 0; signal text: String = \"\"; \
         fn bump() { count += 1; count += 1; text = \"n=\" + count; } \
         view { <button @onPress={bump}>bump</button> } }",
    )
    .expect("compile must succeed");
    let tsx = out.files.get("App.tsx").expect("App.tsx emitted");
    assert!(
        tsx.contains("setCount(nextCount)") && tsx.contains("setCount(nextCount1)"),
        "+= 链未逐次线程化（隐式 target 读漏判）：\n{}",
        tsx
    );
    assert!(
        tsx.contains("setText(\"n=\" + nextCount1)"),
        "+= 链尾读取未指向两轮后的值：\n{}",
        tsx
    );
}

/// v4 评审修复轮（新 Important）：读在嵌套写「之前」时线程化精确正确——
/// `x = 1; use(x); if (c) { x = 5; }` 的 use(nextX) 读 1（写→读→条件重置
/// 形态）；位置不敏感的放弃会把读取打回陈旧闭包初值（静默值回归）。
#[test]
fn read_before_nested_write_still_threads() {
    let out = compile_react(
        "component App { signal x: i32 = 0; signal c: Bool = false; signal text: String = \"\"; \
         fn go() { x = 1; text = \"v=\" + x; if (c) { x = 5; } } \
         view { <button @onPress={go}>go</button> } }",
    )
    .expect("compile must succeed");
    let tsx = out.files.get("App.tsx").expect("App.tsx emitted");
    assert!(
        tsx.contains("setText(\"v=\" + nextX)"),
        "读在嵌套写前应线程化（use(nextX) 读 1）：\n{}",
        tsx
    );
    assert!(
        !tsx.contains("setText(\"v=\" + x)"),
        "读在嵌套写前不应回退到陈旧闭包初值：\n{}",
        tsx
    );
}

/// v4 评审修复轮：顶层非三变体写（`x *= 2` 类）不可线程化——它的值无法
/// 局部化，与嵌套写同级切断 local 链；断点之后的读取保持原状（不 rename
/// 到过期 local，functional updater 链安全）。
#[test]
fn unthreadable_write_breaks_chain() {
    let out = compile_react(
        "component App { signal x: i32 = 2; signal text: String = \"\"; \
         fn go() { x = 1; x *= 2; text = \"v=\" + x; } \
         view { <button @onPress={go}>go</button> } }",
    )
    .expect("compile must succeed");
    let tsx = out.files.get("App.tsx").expect("App.tsx emitted");
    assert!(!tsx.contains("nextX"), "非三变体写后不应线程化：\n{}", tsx);
    assert!(
        tsx.contains("setText(\"v=\" + x)"),
        "链断点后的读取应保持原状：\n{}",
        tsx
    );
}

/// v4 评审 Important 2: 区间内出现同信号「嵌套」写（if 体内）时区间模型
/// 失效——保守放弃线程化：不引入 local、不 rename 后续读取（保持既有
/// 发射路径形状）。
#[test]
fn nested_block_write_skips_threading() {
    let out = compile_react(
        "component App { signal x: i32 = 0; signal c: Bool = false; signal text: String = \"\"; \
         fn go() { x = 1; if (c) { x = 5; } text = \"v=\" + x; } \
         view { <button @onPress={go}>go</button> } }",
    )
    .expect("compile must succeed");
    let tsx = out.files.get("App.tsx").expect("App.tsx emitted");
    assert!(!tsx.contains("nextX"), "嵌套写存在时不应线程化：\n{}", tsx);
    assert!(
        tsx.contains("setText(\"v=\" + x)"),
        "嵌套写存在时后续读取不应 rename 到 local：\n{}",
        tsx
    );
}

/// v4 评审 Important 2 传播: 嵌套写使前一写放弃线程化——其后的自读写
/// （count = count + 1）也必须放弃（否则 RHS 读陈旧闭包值），保持
/// functional updater 形状（React 队列链安全）。
#[test]
fn nested_block_write_keeps_chain_functional_updater() {
    let out = compile_react(
        "component App { signal count: i32 = 0; signal c: Bool = false; signal text: String = \"\"; \
         fn go() { count = 1; if (c) { count = 5; } count = count + 1; text = \"v=\" + count; } \
         view { <button @onPress={go}>go</button> } }",
    )
    .expect("compile must succeed");
    let tsx = out.files.get("App.tsx").expect("App.tsx emitted");
    assert!(
        !tsx.contains("nextCount"),
        "嵌套写后的自读写不应线程化：\n{}",
        tsx
    );
    assert!(
        tsx.contains("setCount((c) => c + 1)"),
        "嵌套写后的自读写应保持 functional updater（链安全）：\n{}",
        tsx
    );
}

/// v4 H1: 链式写（同信号多次写）按 DSL 顺序语义逐次线程化——每次「下值」
/// 读当前可见值（前一 local），读取重写指向最后一次写的 local。
#[test]
fn write_then_read_chain_threads_in_order() {
    let out = compile_react(
        "component App { signal count: i32 = 0; signal text: String = \"\"; \
         fn bump() { count = count + 1; count = count + 1; text = \"n=\" + count; } \
         view { <button @onPress={bump}>bump</button> } }",
    )
    .expect("compile must succeed");
    let tsx = out.files.get("App.tsx").expect("App.tsx emitted");
    assert!(
        tsx.contains("setCount(nextCount)") && tsx.contains("setCount(nextCount1)"),
        "链式写未逐次线程化：\n{}",
        tsx
    );
    assert!(
        tsx.contains("setText(\"n=\" + nextCount1)"),
        "链尾读取未指向最后一次写：\n{}",
        tsx
    );
    assert!(
        !tsx.contains("setText(\"n=\" + count)"),
        "链尾读取仍是陈旧闭包：\n{}",
        tsx
    );
}

/// perf M3: a never-mutated signal initialized with a literal is hoisted to
/// module scope — it was re-allocated inside the component function on EVERY
/// render, and a computed/memo depending on it recomputed every render (the
/// fresh array identity defeats the memo deps). Module scope: one allocation
/// per module, stable identity across renders. (The brief's `view V {...}`
/// shorthand does not parse — the standard `component V { ... }` form is
/// used; test name, assertion and failure message are kept verbatim.)
#[test]
fn literal_never_mutated_signal_hoisted() {
    let out = compile_react(
        r#"
        component V {
            signal items: Array<String> = ["a", "b"];
            view { <div>{items.length}</div> }
        }
        "#,
    )
    .expect("compile must succeed");
    let tsx = out.files.get("V.tsx").expect("V.tsx emitted");
    let fn_start = tsx.find("function V").expect("component function emitted");
    let decl = tsx.find("const items =").expect("signal const emitted");
    assert!(
        decl < fn_start,
        "literal signal must be module-scoped: {}",
        tsx
    );
}

/// MapOperationsDemo gate regression: a hoisted Map literal const must carry
/// its `Record<K,V>` annotation — a bare object literal infers a narrow type
/// whose keys cannot be indexed by `string` (TS7053). Scalars keep their
/// literal inference (no annotation).
#[test]
fn hoisted_map_signal_const_annotated_scalar_untouched() {
    let out = compile_react(
        r#"
        component V {
            signal endpoints: Map<String, String> = { "dev": "http://localhost:3000", "prod": "https://api.example.com" };
            signal env: String = "staging";
            view {
                <div>{endpoints.get(env)}</div>
            }
        }
        "#,
    )
    .expect("compile must succeed");
    let tsx = out.files.get("V.tsx").expect("V.tsx emitted");
    assert!(
        tsx.contains("const endpoints: Record<string, string> = {"),
        "hoisted Map const must be annotated with Record: {}",
        tsx
    );
    assert!(
        !tsx.contains("const env: string"),
        "scalar hoisted const keeps literal inference (no annotation): {}",
        tsx
    );
}

/// `.len()` on component-signal receivers currently emits the RUNTIME
/// dispatch form (`Array.isArray(...) ? .length : Object.keys(...).length`)
/// — semantically identical for arrays/strings/objects, just noisier.
/// The typed static form (`items.length`, `Object.keys(counts).length`) is
/// the design target (see the typed arms in `translate_method_call` and
/// `format_method_call_with_ir`) but is DEAD CODE today: `lower_expr` reads
/// `IdentifierExpr.type_ann` from the semantic symbol table, and lowering
/// runs against FRESH scopes (`ir/lower/mod.rs` pushes new `component:` /
/// `fn:` scopes) whose chain never reaches the scopes where `collect.rs`
/// declared the component symbols — so `type_ann` is always `None` on
/// identifier IR (verified experimentally, 2026-08-06). Deferred: attach
/// `type_ann` in lowering (core fix; would activate typed dispatch on
/// React/Dart/Flutter simultaneously — needs a cross-target regression
/// pass before landing).
#[test]
fn react_len_runtime_dispatch_until_type_ann_lands() {
    let out = compile_react(
        r#"
        component V {
            signal items: Array<String> = ["a", "b"];
            signal label: String = "hi";
            signal counts: Map<String, i32> = { "a": 1 };
            computed n: i32 = items.len() + label.len() + counts.len();
            view {
                <div>{n}</div>
            }
        }
        "#,
    )
    .expect("compile must succeed");
    let tsx = out.files.get("V.tsx").expect("V.tsx emitted");
    assert!(
        tsx.contains("((_c) => Array.isArray(_c)"),
        "component-signal .len() keeps the runtime dispatch until type_ann \
         attachment lands (never a crash — semantically identical): {}",
        tsx
    );
    assert!(
        !tsx.contains("items.length + label.length"),
        "no stale typed-form assertion: {}",
        tsx
    );
    // The three receivers must ALL dispatch — nothing is statically typed
    // through the IR today.
    assert_eq!(
        tsx.matches("Array.isArray(_c)").count(),
        3,
        "one dispatch per .len() receiver: {}",
        tsx
    );
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
    // Phase 5.2 (getter freeze fix): getters are COMPUTED VALUES in a
    // `__getters` helper — never accessor properties (Zustand's
    // Object.assign merge flattens accessors into stale data props on the
    // first set()).
    assert!(
        result.contains("const __getters = (s: CalcStoreSignals) => {"),
        "should emit the getter helper: {}",
        result
    );
    assert!(
        result.contains("type CalcStoreSignals = Pick<CalcStoreState, \"count\" | \"name\">;"),
        "should emit the signals-only Pick type for contextual typing: {}",
        result
    );
    assert!(
        result.contains("const doubleCount = s.count * 2;"),
        "getter body must be computed from the state param: {}",
        result
    );
    assert!(
        result.contains("return { ...s, doubleCount };"),
        "getters must be materialized as data properties: {}",
        result
    );
    assert!(
        result.contains("const __initial = __getters({"),
        "initial state must go through the same getter path: {}",
        result
    );
    assert!(
        !result.contains("get doubleCount()"),
        "accessor getters are gone (freeze bug): {}",
        result
    );
    assert!(
        result.contains("readonly doubleCount: number;"),
        "should emit typed getter return: {}",
        result
    );
    // Task 2.7 (perf M2): the standalone per-field selector hooks were dead
    // code — components and the design-tokens hook subscribe inline with
    // `useStore((s) => s.x)` — so the store file must NOT emit them.
    assert!(
        !result.contains("export function useCount() {"),
        "standalone per-field selector hooks must not be emitted: {}",
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
    // Task 2.7 (perf M2): no standalone action selector hook either.
    assert!(
        !result.contains("export function useMultiply() {"),
        "standalone action selector hook must not be emitted: {}",
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
                    is_mut: false,
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

/// maintainability HIGH-1 (Phase 5.4): an action param that collides with a
/// store signal name must stay BARE — `validate_all(items)` iterates the
/// ARGUMENT, never the store signal. Port of the OmniView fn_locals pattern
/// (omniview/store.rs): the old emission rewrote the param to
/// `validate_all(state.items)` — silently iterating the store (wrong-code
/// class, verified in-browser on TodoStore's validate_all).
#[test]
fn store_action_param_shadows_signal_react() {
    let result = compile_react_full(
        r#"import omniview;
import { console_log } from "std/console";
store S {
    signal items: Array<i32> = [1];
    action validate_all(items) { console_log(items.length) }
}
"#,
    );
    assert!(
        result.success,
        "store action param shadowing must compile: {:?}",
        result.errors
    );
    let out: String = result
        .files
        .values()
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        !out.contains("state.items"),
        "param must not be rewritten to a store ref: {}",
        out
    );
    assert!(
        out.contains("console.log(items.length)"),
        "param must stay bare inside the call arg: {}",
        out
    );
    // The only body reference is the shadowed param — no `const state =
    // get();` may be emitted (an unused `state` trips the eslint
    // no-unused-vars gate on generated stores).
    assert!(
        !out.contains("const state = get()"),
        "shadowed body must not emit the unused prologue: {}",
        out
    );
}

/// Review follow-up (5.4 Important-1): the prologue pruning must also cover
/// params colliding with GETTER names — the old code pruned the signal
/// derived set and extended getters afterwards, re-adding the param
/// (`action check(total)` with `getter total` emitted an unused
/// `const state = get();` → eslint no-unused-vars gate failure).
#[test]
fn store_action_param_shadows_getter_react() {
    let result = compile_react_full(
        r#"import omniview;
import { console_log } from "std/console";
store S {
    signal count: i32 = 1;
    getter total: i32 = count * 2;
    action check(total) { console_log(total) }
}
"#,
    );
    assert!(
        result.success,
        "getter-shadowing store action must compile: {:?}",
        result.errors
    );
    let out: String = result
        .files
        .values()
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        !out.contains("state.total"),
        "getter-named param must not be rewritten to a store ref: {}",
        out
    );
    assert!(
        out.contains("console.log(total)"),
        "getter-named param must stay bare inside the call arg: {}",
        out
    );
    assert!(
        !out.contains("const state = get()"),
        "getter-shadowed body must not emit the unused prologue: {}",
        out
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
            let _ = writeln!(snapshot_content, "=== {} ===", f.path);
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

/// Compile a single .omni source string to React and join all emitted file
/// contents — the shape-assertion shorthand for single-component sources.
fn compile_react_src(source: &str) -> String {
    compile_react(source)
        .expect("compile should succeed")
        .files
        .values()
        .cloned()
        .collect::<Vec<_>>()
        .join("\n")
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

// ── useDesignTokens slice subscription (Task 2.7, ts/react M2 + perf M2) ──

/// Compile a minimal design program: a `DynamicTokenStore` (signal
/// `liveAccent`), the given design block, and an `App` root component.
/// Returns the emitted `__design/useDesignTokens.ts` hook — the file this
/// task rewires to per-field subscriptions (returning the whole output
/// would let the store file's own dead selector hooks mask the shape).
///
/// The design block must be complete (`design Name { ... }`) and carry a
/// dynamic token referencing `DynamicTokenStore.liveAccent` — the
/// `useDesignTokens.ts` hook is only emitted when a token is dynamic.
fn compile_react_with_design(design_src: &str) -> String {
    let source = format!(
        "store DynamicTokenStore {{\n    signal liveAccent: String = \"#f00\";\n}}\n{}\ncomponent App {{\n    view {{ <div/> }}\n}}\n",
        design_src
    );
    let result = compile_react_full(&source);
    assert!(result.success, "design compile failed: {:?}", result.errors);
    result
        .files
        .get("__design/useDesignTokens.ts")
        .expect("design block with a dynamic token must emit __design/useDesignTokens.ts")
        .clone()
}

#[test]
fn design_tokens_hook_uses_slices() {
    let out = compile_react_with_design(
        r#"design AppDesign { token accent: color = DynamicTokenStore.liveAccent; }"#,
    );
    // useDesignTokens 必须用 per-field selectors
    assert!(
        out.contains("useDynamicTokenStore((s) => s."),
        "whole-store subscription in design hook: {}",
        out
    );
}

/// An inline store (`store Cart { ... }` in a component body) must lower to
/// a real module-level store: React emits a useCart Zustand store file and
/// the component subscribes through it — no bare undefined `Cart.*`
/// references (BUG-C / deferral #37; replaces the task-2.2 E_CODEGEN gate).
#[test]
fn inline_store_emits_working_store() {
    let result = compile_react_full(
        r#"import omniview; component V { store Cart { signal items: Array<String> = []; action addItem(x: String) { items = [...items, x]; } } fn doAdd() { Cart.addItem("Apple"); } view { <div>{Cart.items.len()}</div> } }"#,
    );
    assert!(
        result.success,
        "inline store must compile without errors: {:?}",
        result.errors
    );
    let out: String = result
        .files
        .values()
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        result.files.contains_key("useCart.ts"),
        "must emit the store file: {}",
        out
    );
    assert!(
        out.contains("createStore") || out.contains("useCart"),
        "must emit real store: {}",
        out
    );
    assert!(
        !out.contains("Cart.addItem") || out.contains("useCart"),
        "bare-name calls must resolve: {}",
        out
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
        result
            .warnings
            .iter()
            .all(|d| !d.message.contains("对象项列表")),
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
    // Task 2.7 (ts/react M2 + perf M2): per-field slice subscriptions —
    // one selector per referenced signal, whole-store object gone.
    assert!(
        hook.contains("const liveColor = useDynamicTokenStore((s) => s.liveColor);"),
        "store must be subscribed per field. Got:\n{}",
        hook
    );
    assert!(
        hook.contains("const liveBg = useDynamicTokenStore((s) => s.liveBg);"),
        "one slice per referenced signal. Got:\n{}",
        hook
    );
    assert!(
        !hook.contains("const DynamicTokenStore = useDynamicTokenStore();"),
        "whole-store subscription must be gone. Got:\n{}",
        hook
    );
    assert!(
        hook.contains(
            "document.documentElement.style.setProperty('--dynColor', sanitizeCss(liveColor));"
        ),
        "dynamic token must set the CSS custom property at runtime. Got:\n{}",
        hook
    );
    assert!(
        hook.contains("}, [liveColor]);"),
        "effect deps must be the exact slice local. Got:\n{}",
        hook
    );
    assert!(
        hook.contains("}, [liveBg]);"),
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
  prop defaultActive: Bool = false
  prop defaultPos: Float = 0
  signal active: Bool = defaultActive
  signal pos: Float = defaultPos
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

/// Test: numeric signals' bind:value must coerce the raw string
/// (`e.target.value`) through `Number(...)` — feeding the string directly
/// into a `useState<number>` setter is a type lie with runtime consequences
/// (`age + 1` → `"301"` concatenation, v4 H3).
#[test]
fn numeric_bind_value_emits_number_coercion() {
    // v4 H3: age: i32 的 bind:value 必须 Number(e.target.value)——裸
    // e.target.value 把字符串喂进 number 状态（age+1 → "301"）。
    let tsx = compile_react_src(
        "component App { signal age: i32 = 0; \
         view { <input type=\"number\" bind:value={age} /> } }",
    );
    assert!(
        tsx.contains("setAge(Number(e.target.value))")
            || tsx.contains("setAge(e.target.valueAsNumber)"),
        "数字 bind:value 无强制转换：\n{}",
        tsx
    );
    // 字符串 signal 保持原样（不破坏既有形状）
    let tsx_str = compile_react_src(
        "component App { signal q: String = \"\"; \
         view { <input bind:value={q} /> } }",
    );
    assert!(tsx_str.contains("setQ(e.target.value)"));
}

/// Test: `bind:value` + `@onChange={handleChange}`（0 参 fn）时 setter 分支
/// 不得调 `handleChange({})`——与无 setter 分支的空事件特判一致（TS2554）。
#[test]
fn bind_value_handler_with_zero_arity_fn_skips_empty_obj() {
    // v4 M3 (task 2.4): 空事件对象仍被构造 → 0 参 fn 被传 1 参（TS2554）。
    let tsx = compile_react_src(
        "component App { signal q: String = \"\"; fn handleChange() {} \
         view { <input bind:value={q} @onChange={handleChange} /> } }",
    );
    assert!(
        !tsx.contains("handleChange({})") && tsx.contains("onChange={(e) => {"),
        "空事件对象仍被构造：\n{}",
        tsx
    );
}

/// Test: `bind:value` + 内联 0 参 handler 的 setter 分支同样不构造空事件
/// 对象（InlineFn 分支对齐——`Object.assign(e, {})` 是纯噪音；setter
/// 仍经 `e` 写回，形状保持 `(e) => { setQ(e.target.value); ... }`）。
#[test]
fn bind_value_inline_handler_skips_empty_event_assign() {
    let tsx = compile_react_src(
        "component App { signal q: String = \"\"; signal n: i32 = 0; \
         view { <input bind:value={q} @onChange={() => { n = n + 1; }} /> } }",
    );
    assert!(
        !tsx.contains("Object.assign(e, {})") && tsx.contains("setQ(e.target.value)"),
        "InlineFn setter 分支仍构造空事件对象：\n{}",
        tsx
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
/// (Prop-backed signal — a literal-initialized one is hoisted to a
/// module-scope const by perf M3 and a constant correctly needs no deps.)
#[test]
fn test_react_semantic_binding_scroll_x() {
    let source = r#"
component Carousel {
  prop defaultPos: Float = 0
  signal pos: Float = defaultPos
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
            .any(|e| e.message.contains("bind:value") && e.message.contains("单向显示")),
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
        "boxRef.current?.getBoundingClientRect() ?? { left: 0, top: 0, right: 0, bottom: 0, x: 0, y: 0, width: 0, height: 0 };",
        "boxRef.current?.getBoundingClientRect().width ?? 0;",
        "boxRef.current?.getBoundingClientRect().height ?? 0;",
        "boxRef.current?.getBoundingClientRect() ?? { left: 0, top: 0, right: 0, bottom: 0, x: 0, y: 0, width: 0, height: 0 };",
        "boxRef.current?.select();",
        "boxRef.current?.setSelectionRange(0, 3);",
        "boxRef.current?.value ?? '';",
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
    // The active-pointer tracker is emitted only because the component uses
    // the pointer-capture ops (D3 parameterless form). Task 2.11 (L1): the
    // STATE stays at module scope (ref ops reference it from fn bodies) but
    // the LISTENER registers in a mount effect with an unmount cleanup —
    // the old module-level `if (typeof window ...) addEventListener` was a
    // module-scope side effect.
    assert!(
        content.contains("let __omni_activePointerId: number | null = null;"),
        "pointer ops must emit the tracker state. Got:\n{}",
        content
    );
    assert!(
        content.contains("window.addEventListener(\"pointerdown\", onPointerDown, true);")
            && content
                .contains("window.removeEventListener(\"pointerdown\", onPointerDown, true);")
            && content.contains("useEffect(() => {"),
        "pointer listener must register in a mount effect with unmount cleanup. Got:\n{}",
        content
    );
    assert!(
        !content.contains("if (typeof window !== \"undefined\")"),
        "the module-level guarded addEventListener must be gone. Got:\n{}",
        content
    );
}

/// Test: FIELD ACCESS on a ref-op RESULT (`ref.measure().y`) parenthesizes
/// the coalesced output — `A ?? {...}.y` would bind the field to the
/// zero-rect FALLBACK literal (member access binds tighter than `??`), so
/// the result is `(A ?? {...}).y`. Covers both the fn-body path (shared
/// FormatExpr::format_ref_op_field_access hook) and the renamed-locals path
/// (computed/state-init expressions).
#[test]
fn test_react_ref_op_field_access_parenthesizes_coalesced_result() {
    let source = r#"
component RefFieldAccess {
  signal info: String = ""
  ref rectBox: Element
  ref scrollArea: Element
  fn query() {
    let pos = rectBox.measure().y
    scrollArea.scrollTo(0, rectBox.measure().y)
    let w = rectBox.measure().width
    info = w
  }
  view {
    <div ref:rectBox @onClick={query}>box</div>
    <div ref:scrollArea>scroll</div>
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
    let zero_rect =
        "{ left: 0, top: 0, right: 0, bottom: 0, x: 0, y: 0, width: 0, height: 0 }";
    // Field access must land on the PARENTHESIZED coalesced result, not on
    // the fallback literal (`.y` on the fresh zero-rect is always undefined).
    assert!(
        content.contains(&format!(
            "const pos = (rectBoxRef.current?.getBoundingClientRect() ?? {zero_rect}).y;"
        )),
        "let-bound measure().y must parenthesize. Got:\n{}",
        content
    );
    assert!(
        content.contains(&format!(
            "scrollAreaRef.current?.scrollTo({{ left: 0, top: (rectBoxRef.current?.getBoundingClientRect() ?? {zero_rect}).y }});"
        )),
        "nested measure().y inside scrollTo args must parenthesize. Got:\n{}",
        content
    );
    assert!(
        content.contains(&format!(
            "const w = (rectBoxRef.current?.getBoundingClientRect() ?? {zero_rect}).width;"
        )),
        "measure().width field access must parenthesize. Got:\n{}",
        content
    );
    // The fallback literal must never sit bare next to the field (`?? {...}.y`).
    assert!(
        !content.contains(&format!(
            "getBoundingClientRect() ?? {zero_rect}.y"
        )),
        "bare `?? {{...}}.y` must never be emitted (binds to the fallback). Got:\n{}",
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

/// Task 2.11 — write-only signal DCE: a signal whose VALUE is never consumed
/// (its only reads are its own mutation RHS, which emit as functional
/// updaters `setPlayAll((p) => !p)` that never reference the state) gets no
/// useState hook, its mutations are pruned, and the toggling fn becomes a
/// no-op. The button binding stays valid (the fn still exists).
#[test]
fn test_write_only_signal_is_pruned() {
    let source = r#"
component ToggleDemo {
    signal playAll: bool = false;
    fn toggleAll() { playAll = !playAll; }
    view {
        <button @click={toggleAll}>Replay</button>
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
        !content.contains("playAll"),
        "write-only signal must be pruned entirely. Got:\n{}",
        content
    );
    assert!(
        !content.contains("useState"),
        "no state hook for a value-unconsumed signal. Got:\n{}",
        content
    );
    assert!(
        content.contains("const toggleAll = () => {") && content.contains("};"),
        "the toggle fn must still exist (button binding) as a no-op. Got:\n{}",
        content
    );
}

/// v4 H2 — a signal that is never READ but mutated inside a called fn keeps
/// its useState hook: the emitted `setVariant(...)` must have a setter to
/// reference, or the click crashes with `setVariant is not defined`
/// (ReferenceError, AdvancedPseudoDemo「Dark Variant」button). The value
/// name is dropped — `const [, setVariant] = useState(...)` — so no unused
/// variable lint fires either.
#[test]
fn write_only_signal_keeps_state_hook() {
    let source = r#"
component App {
    signal variant: String = "light";
    fn setDark() { variant = "dark"; }
    view {
        <button @onClick={setDark}>Dark</button>
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
        content.contains("const [, setVariant] = useState"),
        "write-only signal must keep a setter-only hook. Got:\n{}",
        content
    );
    assert!(
        !content.contains("setVariant is not defined"),
        "no dangling setter. Got:\n{}",
        content
    );
}

/// Task 2.11 — write-only signal DCE keeps VALUE-CONSUMED mutations intact:
/// `x = playAll + 1` (target x, value reads playAll) emits the direct setter
/// form and DOES read playAll — the state must be emitted and the mutation
/// must stay.
#[test]
fn test_signal_mutated_from_other_value_is_kept() {
    let source = r#"
component KeepDemo {
    signal count: i32 = 0;
    signal other: i32 = 5;
    fn bump() { count = other + 1; }
    view {
        <button @click={bump}>Go</button>
        <span>{count}</span>
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
        content.contains("const [count, setCount] = useState"),
        "count is read in the view — state must be emitted. Got:\n{}",
        content
    );
    // `other` is read by count's mutation but never itself mutated — the
    // perf M3 hoist moves it to a module-scope const (still a real read).
    assert!(
        content.contains("const other = 5;"),
        "other is read by count's mutation — it must stay emitted (hoisted const). Got:\n{}",
        content
    );
    assert!(
        content.contains("setCount(other + 1)"),
        "a mutation reading ANOTHER signal must stay. Got:\n{}",
        content
    );
}

/// Task 2.11 — match arms whose variant bindings are all unused skip the
/// destructure entirely (previously `const { _data } = _v;` linted as an
/// unused variable — `_`-prefixing does not exempt destructured vars).
#[test]
fn test_match_arm_unused_bindings_skip_destructure() {
    // `data` is bound by the arm but never referenced by the body — the
    // old emission prefixed it `_data` (still flagged by
    // @typescript-eslint/no-unused-vars, task 2.11); the arm now skips the
    // destructure entirely. The `error` arm USES its binding and must keep
    // `const { error } = _v;`.
    let expr = ExprIR::MatchExpr {
        matched: Box::new(ExprIR::IdentifierExpr {
            name: "result".into(),
            type_ann: None,
        }),
        arms: vec![
            MatchArmIR {
                pattern: MatchPatternIR {
                    kind: MatchPatternKindIR::Variant {
                        name: "Ok".into(),
                        bindings: vec!["data".into()],
                    },
                },
                body: BlockIR {
                    statements: vec![StatementIR::ReturnStmtIR {
                        value: Some(ExprIR::LiteralExpr {
                            value: serde_json::json!(""),
                        }),
                    }],
                },
                union_type: None,
            },
            MatchArmIR {
                pattern: MatchPatternIR {
                    kind: MatchPatternKindIR::Variant {
                        name: "Err".into(),
                        bindings: vec!["error".into()],
                    },
                },
                body: BlockIR {
                    statements: vec![StatementIR::ReturnStmtIR {
                        value: Some(ExprIR::IdentifierExpr {
                            name: "error".into(),
                            type_ann: None,
                        }),
                    }],
                },
                union_type: None,
            },
        ],
    };
    let out = format_expr_ir_tsx(&expr, &FxHashSet::default());
    assert!(
        !out.contains("_data") && !out.contains("const {  } = _v;"),
        "an unused variant binding must not emit a destructured const. Output: {}",
        out
    );
    assert!(
        out.contains("if (_v._variant === \"Ok\") { return \"\"; }"),
        "the arm must skip the empty destructure, keeping the guard. Output: {}",
        out
    );
    assert!(
        out.contains("const { error } = _v;"),
        "USED bindings must still destructure. Output: {}",
        out
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

/// I2 (view-root block expressions): a component whose view ROOT is a
/// conditional (`@if/@else` with no wrapper element — the PageSwitcherDemo
/// shape) used to emit `return ( { ... } )` — JSX expression-container
/// braces around the ternary — which is a TS syntax error (TS1005/TS1136/
/// TS1128). Container braces are only valid in JSX-children position; at
/// the root of `return ( ... )` the ternary must be emitted bare.
#[test]
fn react_root_conditional_emits_bare_ternary() {
    let source = r#"
component PageSwitcher {
    signal page: String = "home";
    view {
        @if page == "home" {
            <p>Home</p>
        }
        @else {
            <p>About</p>
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
    assert!(
        content.contains("return (\n        page === \"home\" ? ("),
        "the root ternary must sit directly in the return parens, got:\n{}",
        content
    );
    assert!(
        !content.contains("return (\n    {"),
        "no JSX container braces may wrap the root ternary, got:\n{}",
        content
    );
}

/// I2: the same for a root-level `@for ... @empty` (the ScrollStoreDemo
/// shape) — the IIFE must sit directly in the return parens, and the
/// closing `); })()` must NOT carry the JSX-container `}`.
#[test]
fn react_root_for_empty_emits_bare_iife() {
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
    assert!(
        content.contains("return (\n        (() => { const __omni_iterable = items;"),
        "the single-eval IIFE must sit directly in the return parens, got:\n{}",
        content
    );
    assert!(
        content.contains("); })()\n);"),
        "the IIFE must close WITHOUT a trailing JSX-container brace, got:\n{}",
        content
    );
    assert!(
        !content.contains("); })()}"),
        "the JSX-container `}}` must not be emitted at the view root, got:\n{}",
        content
    );
    assert!(
        !content.contains("return (\n    {"),
        "no JSX container braces may wrap the root list, got:\n{}",
        content
    );
}

/// I2: a root-level `@for` without `@empty` also drops the container braces
/// (`{items.map(...)}` → `items.map(...)`).
#[test]
fn react_root_for_without_empty_emits_bare_map() {
    let source = r#"
component ListPage {
    signal items: Array<String> = [];
    view {
        @for item in items {
            <div>{item}</div>
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
    assert!(
        content.contains("return (\n        items.map((item, _i) => ("),
        "the bare .map must sit directly in the return parens, got:\n{}",
        content
    );
    assert!(
        !content.contains("return (\n    {"),
        "no JSX container braces may wrap the root list, got:\n{}",
        content
    );
}

/// I2: the container braces stay for expression nodes in JSX-CHILDREN
/// position — a conditional inside a wrapper element is unchanged.
#[test]
fn react_conditional_in_children_keeps_container_braces() {
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
    let result = compile_react_full(source);
    assert!(result.success, "errors: {:?}", result.errors);
    let content: String = result
        .files
        .values()
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        content.contains("{\n                open && ("),
        "a conditional in children position still needs the container, got:\n{}",
        content
    );
    assert!(
        content.contains("return (\n        <div>"),
        "the element root is unchanged, got:\n{}",
        content
    );
}

/// I2 review fix (Important-1): a conditional INSIDE a `@for` body renders
/// between the `<React.Fragment>` tags — JSX-children position — so its
/// `{...}` container must stay even though the map callback wraps the
/// fragment in parens. A bare `cond && (<div/>)` there renders as literal
/// text (silent mis-render).
#[test]
fn react_list_body_conditional_keeps_container_braces() {
    let source = r#"
component ListPage {
    signal items: Array<String> = [];
    view {
        @for item in items {
            @if item == "x" {
                <div>{item}</div>
            }
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
    assert!(
        content.contains("{\n                    item === \"x\" && ("),
        "the conditional inside the @for body must keep its container braces, got:\n{}",
        content
    );
    assert!(
        content.contains("<React.Fragment") && content.contains("</React.Fragment>"),
        "the fragment wrapper must be present, got:\n{}",
        content
    );
}

/// I2: a root conditional with `mode: "fade"` emits the fade wrapper divs
/// directly in the return parens — no container braces, but the branch
/// bodies (now inside the wrapper divs) still get them.
#[test]
fn react_root_conditional_fade_mode_bare_ternary() {
    let source = r#"
component FadePanel {
    signal open: bool = false
    view {
        @if open mode: "fade" {
            <p>Panel</p>
        }
        @else {
            <p>Other</p>
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
    assert!(
        content.contains("return (\n        open ? ("),
        "the root ternary must sit directly in the return parens, got:\n{}",
        content
    );
    assert!(
        !content.contains("return (\n    {"),
        "no JSX container braces may wrap the root ternary, got:\n{}",
        content
    );
    assert!(
        content.contains("styles['omni-if-fade']"),
        "the fade wrapper class must still be emitted, got:\n{}",
        content
    );
}

/// I2: a root conditional with `mode: "wait"` degrades its stated-comment to
/// a `//` line comment — a `{/* */}` container comment at the root of
/// `return ( ... )` would be a syntax error.
#[test]
fn react_root_conditional_wait_mode_uses_line_comment() {
    let source = r#"
component WaitPanel {
    signal open: bool = false
    view {
        @if open mode: "wait" {
            <p>Panel</p>
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
    assert!(
        content.contains("return (\n        // @if mode: \"wait\""),
        "the degradation note must be a line comment at the root, got:\n{}",
        content
    );
    assert!(
        !content.contains("{/* @if mode: \"wait\""),
        "no JSX container comment at the view root, got:\n{}",
        content
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
    let _content: String = output
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
        app.contains("<MemoizedModal />"),
        "Expected bare instantiation to keep declared fallbacks (task 2.8: \
         children-free instances instantiate the React.memo alias), got:\n{}",
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
    // Read sites are rewritten to the slice locals. Store fields carry no
    // static type through the IR, so `.len()` on one falls back to the
    // runtime dispatch (`Array.isArray(...) ? .length : Object.keys(...)`)
    // — semantically identical for arrays, zero static-noise for the
    // typed-local path (see react_typed_len_uses_static_length).
    assert!(
        content.contains("))(items)}"),
        "view read must use the slice local via runtime len dispatch. Got:\n{}",
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
        content.contains("))(CartStore.items)}"),
        "qualified read preserved on collision (runtime len dispatch). Got:\n{}",
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
        content.contains("const s = AliasStore;"),
        "bare alias reference must stay qualified. Got:\n{}",
        content
    );
    assert!(
        content.contains("))(AliasStore.items)}"),
        "qualified reads preserved with the whole-store object (runtime len dispatch). Got:\n{}",
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
        content.contains("const a = s?.items;"),
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

// ═══════════════════════════════════════════════════════════
// H1 (ts/react): fn tail expression return
// ── match/block tail values were silently dropped — the trailing
// ExprStmtIR emitted as a bare `{expr};` statement, the fn returned
// undefined (DataFetching `error_message` → `msg === ""` always false,
// success branch unreachable). Task 2.3.
// ═══════════════════════════════════════════════════════════

/// T2.3: a fn whose body ENDS in a match expression must emit the match
/// as the fn's return value — `return (() => { const _v = ...; ... })();`.
/// Before the fix the match IIFE ran as a bare statement and the fn
/// returned undefined.
#[test]
fn fn_tail_match_emits_return() {
    let src = r#"
import omniview;
import { console_log } from "std/console";
component V {
    fn score(item) { match item { 1 => "a", _ => "b" } }
    view {
        <button @click={() => { console_log(score(1)); }}>go</button>
    }
}
"#;
    let out = compile_react_full(src);
    if !out.success {
        panic!("compile errors: {:?}", out.errors);
    }
    let content: String = out.files.values().cloned().collect::<Vec<_>>().join("\n");
    assert!(
        content.contains("return (() => { const _v = item;"),
        "tail match must emit return of the match IIFE, got:\n{}",
        content
    );
}

/// T2.3: a fn whose body ENDS in a bare tail expression must emit
/// `return <expr>;` — `fn f(x) { let y = x + 1; y }` must return y,
/// not drop it.
#[test]
fn fn_tail_block_emits_return() {
    let src = r#"
import omniview;
component V {
    fn f(x) { let y = x + 1; y }
    view {
        <button @click={() => { f(1); }}>go</button>
    }
}
"#;
    let out = compile_react_full(src);
    if !out.success {
        panic!("compile errors: {:?}", out.errors);
    }
    let content: String = out.files.values().cloned().collect::<Vec<_>>().join("\n");
    assert!(
        content.contains("return y;"),
        "tail block value lost: fn must return the tail expression, got:\n{}",
        content
    );
}

// ── Task 2.4: JS string literal entity-escape separation ───────────

/// JSX string literals (`{'...'}`) must NOT be HTML-entity-escaped — React
/// renders JS string content in expression containers literally (no entity
/// decoding), so escaping produced visible `&amp;amp;` text (ts/react M1).
/// Only JS quote/backslash escaping applies inside the string; entity
/// escaping remains on the JSXText/HTML paths.
#[test]
fn jsx_string_literal_not_entity_escaped() {
    // Raw text node (the buggy path — Text emission wraps in `{'...'}`, a JS
    // string literal, where HTML entity escaping is a visible double-escape:
    // `Signal & Store` rendered `Signal &amp; Store`; a source entity became
    // `&amp;amp;` — ts/react M1). The brief's `view V { <div>{"Signal &
    // Store"}</div> }` source does not parse (no `import omniview;
    // component`), and the `{"..."}` interpolation form already emits
    // unescaped via format_expr_ir_tsx_renamed — the raw-text form is the
    // shape that reproduced `&amp;amp;` in the demos.
    let src = r#"
import omniview;
component V {
    view {
        <div>Signal & Store</div>
    }
}
"#;
    let out = compile_react_full(src);
    if !out.success {
        panic!("compile errors: {:?}", out.errors);
    }
    let content: String = out.files.values().cloned().collect::<Vec<_>>().join("\n");
    assert!(
        !content.contains("&amp;"),
        "string literal must not be entity-escaped: {}",
        content
    );
    assert!(
        content.contains("Signal & Store"),
        "literal text changed: {}",
        content
    );
}

// ── Task 2.5: style key camelCase + React synthetic event names ─────

/// Inline style keys must be camelCased (`font-size` → `fontSize`) — React's
/// style object only accepts camelCase keys; kebab-case keys emit an
/// "Unsupported style property" console error and never apply (ts/react M3,
/// TodoList.tsx). CSS custom properties (`--*`) are NOT camelCased — they are
/// case-sensitive and React accepts them quoted verbatim.
#[test]
fn style_keys_are_camel_cased() {
    let src = r#"
import omniview;
component V {
    view {
        <div style: { font-size: "16px" }/>
    }
}
"#;
    let output = compile_react(src).expect("style map must compile");
    let out: String = output
        .files
        .values()
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        !out.contains("font-size"),
        "kebab style key leaked: {}",
        out
    );
    assert!(out.contains("fontSize"), "expected camelCase: {}", out);
}

/// Animation/transition events must emit React's synthetic handler names
/// (`onAnimationEnd`, not the lowercase `onAnimationend`) — non-synthetic
/// prop names are dropped by React (no listener attached — silent dead
/// event) and warn "React does not recognize" (ts/react M3, EventsDemo.tsx).
#[test]
fn animation_events_use_synthetic_names() {
    let src = r#"
import omniview;
component V {
    fn f() {}
    view {
        <div @onAnimationEnd={f}/>
    }
}
"#;
    let output = compile_react(src).expect("animation event must compile");
    let out: String = output
        .files
        .values()
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        !out.contains("onAnimationend"),
        "must emit React synthetic name: {}",
        out
    );
    assert!(
        out.contains("onAnimationEnd"),
        "expected onAnimationEnd: {}",
        out
    );
}

// ── Task 6.1b (gate FAIL-2): toggle — <details>-only element condition ──

/// toggle is a real DOM event on `<details>` (React synthetic onToggle) —
/// the handler must emit with zero warnings.
#[test]
fn toggle_on_details_emits_on_react() {
    let result = compile_react_full(
        r#"
import omniview;
component V {
    fn f() {}
    view { <details @onToggle={f}><summary>S</summary></details> }
}
"#,
    );
    assert!(
        result.success,
        "details+toggle must compile: {:?}",
        result.errors
    );
    assert!(
        result
            .warnings
            .iter()
            .all(|w| !w.message.contains("toggle")),
        "details+toggle 不应产生 toggle 警告：{:?}",
        result
            .warnings
            .iter()
            .map(|w| &w.message)
            .collect::<Vec<_>>()
    );
    let out: String = result
        .files
        .values()
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        out.contains("onToggle="),
        "expected onToggle handler on details: {}",
        out
    );
}

/// toggle on any element other than `<details>` keeps the v4 3.3 family
/// behavior — warn + skip (never emit the dropped onToggle prop).
#[test]
fn toggle_on_non_details_warns_and_skips_on_react() {
    let result = compile_react_full(
        r#"
import omniview;
component V {
    fn f() {}
    view { <div @onToggle={f}/> }
}
"#,
    );
    assert!(
        result.success,
        "div+toggle must compile: {:?}",
        result.errors
    );
    assert!(
        result
            .warnings
            .iter()
            .any(|w| w.message.contains("toggle")),
        "缺 toggle 警告：{:?}",
        result
            .warnings
            .iter()
            .map(|w| &w.message)
            .collect::<Vec<_>>()
    );
    let out: String = result
        .files
        .values()
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        !out.contains("onToggle"),
        "非 details 元素上不得发射 onToggle：{}",
        out
    );
}

// ── Task 5.3 (deferral #49): user struct/union type emission ──────────

/// User structs (`struct Todo { ... }`) must emit TypeScript type
/// declarations into the generated files that reference them — the React
/// target previously declared nothing, so every annotation (`items: Todo[]`)
/// failed tsc with TS2304 (Cannot find name 'Todo') and the useState-generic
/// guard suppressed typed state for struct-typed signals (never[] cascade).
///
/// NOTE: the brief's literal source `struct Todo { ... } view V { }` does
/// not parse — `view` is not a top-level declaration (views exist only
/// inside components; same documented deviation as task 5.2 and
/// design_class_ref_counts_as_use). A store whose state interface
/// references the struct keeps the assertion identical.
#[test]
fn user_struct_emits_ts_type() {
    let out = compile_react(
        r#"
        struct Todo { id: int, text: String, done: bool }
        store TodoStore {
            signal items: Todo[] = [];
        }
        component V {
            view { <div/> }
        }
    "#,
    );
    let out = out.expect("compile must succeed");
    let all: String = out.files.values().cloned().collect::<Vec<_>>().join("\n");
    assert!(
        all.contains("type Todo ="),
        "missing TS type declaration for the struct (note: the store's own \
         'interface TodoStoreState' is not a struct type — the assertion \
         requires the emitted struct declaration): {}",
        all
    );
}

/// Tagged unions (`type Rating = Star { level: int } | Blank`) must emit a
/// discriminated-union TS type — variant values emit
/// `{ "_variant": "Star", "level": 5 }` and match arms dispatch on
/// `_v._variant === "Star"`, so the literal discriminant is the union
/// discriminator (burn-down: MatchBreadthDemo TS2304).
#[test]
fn union_type_emits_discriminated_union_ts() {
    let out = compile_react(
        r#"
        type Rating = Star { level: int } | Blank
        store RatingStore {
            signal rating: Rating = Blank;
        }
        component V {
            view { <div/> }
        }
    "#,
    );
    let out = out.expect("compile must succeed");
    let all: String = out.files.values().cloned().collect::<Vec<_>>().join("\n");
    assert!(all.contains("type Rating ="), "missing union type: {}", all);
    assert!(
        all.contains("_variant: \"Star\"") && all.contains("_variant: \"Blank\""),
        "union variants must carry the literal _variant discriminator: {}",
        all
    );
}

/// Variant literals must keep the `_variant` DISCRIMINANT literal-typed
/// (`"_variant": "Err" as const`) — the declared union type discriminates
/// on `_variant: "Err"`, so a constructor that widens the discriminant to
/// plain `string` is not assignable to the union (TS2345 — DataFetching's
/// `fetch_demo` return passed into `error_message(result: ApiResult)`).
#[test]
fn variant_literal_keeps_literal_discriminant() {
    let out = compile_react(
        r#"
        type ApiResult = Ok { data: String } | Err { error: String } | Loading
        component V {
            signal result: ApiResult = Loading;
            fn load() {
                let e = Err { error: "url is empty" };
                let n = result;
                result = e;
            }
            view {
                <div @onClick={load}/>
            }
        }
    "#,
    );
    let out = out.expect("compile must succeed");
    let all: String = out.files.values().cloned().collect::<Vec<_>>().join("\n");
    assert!(
        all.contains("\"_variant\": \"Err\" as const"),
        "variant literal must keep the discriminant literal-typed: {}",
        all
    );
}

/// DataTable never[] regression (ts/react M8 + review LOW-1): a mutated
/// struct-typed signal (`signal rows: RowItem[] = []`) must emit
/// `useState<RowItem[]>([])` — the bare `useState([])` inferred `never[]`,
/// and every setter call / field read failed TS2322/TS2339 (deferral #49).
#[test]
fn struct_typed_state_emits_typed_use_state() {
    let out = compile_react(
        r#"
        struct RowItem { id: int, name: String }
        component V {
            signal rows: RowItem[] = [];
            fn load() {
                let n = rows.len();
                rows = [RowItem { id: 1, name: "a" }];
            }
            view {
                <div @onClick={load}/>
            }
        }
    "#,
    );
    let out = out.expect("compile must succeed");
    let tsx = out.files.get("V.tsx").expect("V.tsx emitted");
    assert!(
        tsx.contains("useState<RowItem[]>([])"),
        "struct-typed signal must emit a typed useState generic (never[] fix): {}",
        tsx
    );
    assert!(
        tsx.contains("type RowItem ="),
        "the struct type must be declared in the component file: {}",
        tsx
    );
}

// ── Direct-write channel (Phase 1) ──────────────────────

/// Compile a single .omni source through the pipeline and return its
/// first component IR. Panics on pipeline errors — tests are contract.
fn compile_component_ir(source: &str) -> ComponentIR {
    let out = omnilang_core::run_pipeline(source);
    assert!(!out.has_errors, "pipeline errors: {:?}", out.diagnostics);
    // run_pipeline keys the single-file module as "input.omni" (parser
    // module naming), not "main".
    let mut comps = out
        .ir
        .modules
        .get("input.omni")
        .unwrap()
        .components
        .values();
    comps.next().unwrap().clone()
}

use super::component::direct_write::{classify_direct_write, DirectWriteSets};

#[test]
fn test_classify_direct_write_drag_demo() {
    // 拖拽三态：dragX/dragY 仅 style 消费（D1）；dragOriginX 仅 mutation RHS
    // 自读（D2）；dragging 有 class 绑定（B2 → 不在 D1/D2）。
    let comp = compile_component_ir(
        r#"
component Drag {
  signal dragging: Bool = false
  signal dragX: f64 = 0
  signal dragY: f64 = 0
  signal dragOriginX: f64 = 0
  signal dragOriginY: f64 = 0
  ref box: Element
  fn start(e) { dragging = true; dragOriginX = e.x; dragOriginY = e.y }
  fn move(e) { if dragging { dragX = e.x - dragOriginX; dragY = e.y - dragOriginY } }
  fn end(e) { dragging = false }
  view {
    <div @ref={box} @onPointerDown={start} @onPointerMove={move} @onPointerUp={end}
         style: { transform: "translate(" + dragX + "px, " + dragY + "px)" }
         class: { Active: dragging } />
  }
}
"#,
    );
    let sets = classify_direct_write(&comp);
    assert!(
        sets.d1.contains("dragX"),
        "dragX should be D1: {:?}",
        sets.d1
    );
    assert!(
        sets.d1.contains("dragY"),
        "dragY should be D1: {:?}",
        sets.d1
    );
    assert!(
        sets.d2.contains("dragOriginX"),
        "dragOriginX should be D2: {:?}",
        sets.d2
    );
    assert!(
        sets.d2.contains("dragOriginY"),
        "dragOriginY should be D2: {:?}",
        sets.d2
    );
    assert!(!sets.d1.contains("dragging"), "dragging must NOT be D1");
    assert!(!sets.d2.contains("dragging"), "dragging must NOT be D2");
    assert_eq!(sets.entries.len(), 1, "one active entry (transform)");
    assert_eq!(sets.entries[0].ref_name, "box");
    assert_eq!(sets.entries[0].prop, "transform");
}

#[test]
fn test_classify_direct_write_no_ref_is_inactive() {
    // 无 @ref → 不激活（激活条件 1）。
    let comp = compile_component_ir(
        r#"
component NoRef {
  signal x: f64 = 0
  view { <div style: { transform: "translate(" + x + "px, 0px)" } /> }
}
"#,
    );
    let sets = classify_direct_write(&comp);
    assert!(sets.entries.is_empty());
    assert!(sets.d1.is_empty() && sets.d2.is_empty());
}

#[test]
fn test_classify_direct_write_computed_stays_b2() {
    // computed 信号永远不进 D1/D2（保持 useMemo 渲染路径）。
    let comp = compile_component_ir(
        r#"
component SizeBox {
  signal w: f64 = 200
  computed wPx: String = w.to_string() + "px"
  ref sb: Element
  view { <div @ref={sb} style: { width: wPx } /> }
}
"#,
    );
    let sets = classify_direct_write(&comp);
    assert!(sets.d1.is_empty(), "computed must not be D1: {:?}", sets.d1);
    assert_eq!(sets.b2_entries.len(), 1, "computed-driven entry is B2");
}

#[test]
fn test_classify_direct_write_empty_read_concat_is_b2() {
    // 纯字面量 $concat（"a" + "b"，无 $signal/$ident 读）→ read_signals
    // 为空：空读条目不得留在 D1 通道（Task 3 apply 不覆盖 → 属性静默永不
    // 生效），必须入 B2（useEffect([deps]) 路径）——fix round F1。
    let comp = compile_component_ir(
        r#"
component StaticConcat {
  ref box: Element
  view { <div @ref={box} style: { transform: "a" + "b" } /> }
}
"#,
    );
    let sets = classify_direct_write(&comp);
    assert_eq!(sets.entries.len(), 1, "concat entry still activates");
    assert!(
        sets.d1.is_empty(),
        "empty read_signals must not produce D1: {:?}",
        sets.d1
    );
    assert_eq!(sets.b2_entries.len(), 1, "empty-read entry is B2");
    assert_eq!(sets.b2_entries[0].ref_name, "box");
    assert_eq!(sets.b2_entries[0].prop, "transform");
    assert!(
        sets.b2_entries[0].read_signals.is_empty(),
        "literal concat reads no signals: {:?}",
        sets.b2_entries[0].read_signals
    );
}
