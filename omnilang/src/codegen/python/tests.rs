//! Snapshot tests for Python expression formatting.
//!
//! Locks current output behavior BEFORE the FormatExpr trait migration
//! so that the refactor can be verified to produce byte-identical output.

#[allow(unused_imports)]
use super::*;
use crate::codegen::python::expr::PythonFormatExpr;
use crate::codegen::shared::expr::FormatExpr;
use crate::ir::*;

/// Helper: format an ExprIR using the PythonFormatExpr trait.
fn fmt(expr: &ExprIR) -> String {
    PythonFormatExpr.format_expr(expr)
}

#[test]
fn snapshot_ternary() {
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
    insta::with_settings!({snapshot_path => "snapshots"}, {
        insta::assert_snapshot!("python_ternary", fmt(&expr));
    });
}

#[test]
fn snapshot_closure_with_params() {
    let expr = ExprIR::ClosureExpr {
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
    };
    insta::with_settings!({snapshot_path => "snapshots"}, {
        insta::assert_snapshot!("python_closure_with_params", fmt(&expr));
    });
}

#[test]
fn snapshot_closure_no_params() {
    let expr = ExprIR::ClosureExpr {
        params: vec![],
        body: Box::new(ExprIR::LiteralExpr {
            value: serde_json::json!(42),
        }),
    };
    insta::with_settings!({snapshot_path => "snapshots"}, {
        insta::assert_snapshot!("python_closure_no_params", fmt(&expr));
    });
}

#[test]
fn snapshot_binary_and() {
    let expr = ExprIR::BinaryOpExpr {
        op: BinaryOpIR::And,
        left: Box::new(ExprIR::IdentifierExpr {
            name: "a".into(),
            type_ann: None,
        }),
        right: Box::new(ExprIR::IdentifierExpr {
            name: "b".into(),
            type_ann: None,
        }),
    };
    insta::with_settings!({snapshot_path => "snapshots"}, {
        insta::assert_snapshot!("python_binary_and", fmt(&expr));
    });
}

#[test]
fn snapshot_binary_or() {
    let expr = ExprIR::BinaryOpExpr {
        op: BinaryOpIR::Or,
        left: Box::new(ExprIR::IdentifierExpr {
            name: "a".into(),
            type_ann: None,
        }),
        right: Box::new(ExprIR::IdentifierExpr {
            name: "b".into(),
            type_ann: None,
        }),
    };
    insta::with_settings!({snapshot_path => "snapshots"}, {
        insta::assert_snapshot!("python_binary_or", fmt(&expr));
    });
}

#[test]
fn snapshot_unary_not() {
    let expr = ExprIR::UnaryOpExpr {
        op: UnaryOpIR::Not,
        operand: Box::new(ExprIR::IdentifierExpr {
            name: "flag".into(),
            type_ann: None,
        }),
    };
    insta::with_settings!({snapshot_path => "snapshots"}, {
        insta::assert_snapshot!("python_unary_not", fmt(&expr));
    });
}

#[test]
fn snapshot_unary_neg() {
    let expr = ExprIR::UnaryOpExpr {
        op: UnaryOpIR::Neg,
        operand: Box::new(ExprIR::IdentifierExpr {
            name: "x".into(),
            type_ann: None,
        }),
    };
    insta::with_settings!({snapshot_path => "snapshots"}, {
        insta::assert_snapshot!("python_unary_neg", fmt(&expr));
    });
}

#[test]
fn snapshot_struct_construction() {
    let expr = ExprIR::StructExpr {
        type_name: "Point".into(),
        fields: vec![
            (
                "x".into(),
                ExprIR::LiteralExpr {
                    value: serde_json::json!(1),
                },
            ),
            (
                "y".into(),
                ExprIR::LiteralExpr {
                    value: serde_json::json!(2),
                },
            ),
        ],
    };
    insta::with_settings!({snapshot_path => "snapshots"}, {
        insta::assert_snapshot!("python_struct", fmt(&expr));
    });
}

#[test]
fn snapshot_method_call_len() {
    let expr = ExprIR::MethodCallExpr {
        is_optional: false,
        object: Box::new(ExprIR::IdentifierExpr {
            name: "items".into(),
            type_ann: None,
        }),
        method: "len".into(),
        args: vec![],
    };
    insta::with_settings!({snapshot_path => "snapshots"}, {
        insta::assert_snapshot!("python_method_len", fmt(&expr));
    });
}

#[test]
fn snapshot_method_call_length() {
    let expr = ExprIR::MethodCallExpr {
        is_optional: false,
        object: Box::new(ExprIR::IdentifierExpr {
            name: "items".into(),
            type_ann: None,
        }),
        method: "length".into(),
        args: vec![],
    };
    insta::with_settings!({snapshot_path => "snapshots"}, {
        insta::assert_snapshot!("python_method_length", fmt(&expr));
    });
}

#[test]
fn snapshot_method_call_push() {
    let expr = ExprIR::MethodCallExpr {
        is_optional: false,
        object: Box::new(ExprIR::IdentifierExpr {
            name: "items".into(),
            type_ann: None,
        }),
        method: "push".into(),
        args: vec![ExprIR::LiteralExpr {
            value: serde_json::json!("new_item"),
        }],
    };
    insta::with_settings!({snapshot_path => "snapshots"}, {
        insta::assert_snapshot!("python_method_push", fmt(&expr));
    });
}

#[test]
fn snapshot_method_call_pop() {
    let expr = ExprIR::MethodCallExpr {
        is_optional: false,
        object: Box::new(ExprIR::IdentifierExpr {
            name: "items".into(),
            type_ann: None,
        }),
        method: "pop".into(),
        args: vec![],
    };
    insta::with_settings!({snapshot_path => "snapshots"}, {
        insta::assert_snapshot!("python_method_pop", fmt(&expr));
    });
}

#[test]
fn snapshot_method_call_is_empty() {
    let expr = ExprIR::MethodCallExpr {
        is_optional: false,
        object: Box::new(ExprIR::IdentifierExpr {
            name: "items".into(),
            type_ann: None,
        }),
        method: "is_empty".into(),
        args: vec![],
    };
    insta::with_settings!({snapshot_path => "snapshots"}, {
        insta::assert_snapshot!("python_method_is_empty", fmt(&expr));
    });
}

#[test]
fn snapshot_method_call_contains() {
    let expr = ExprIR::MethodCallExpr {
        is_optional: false,
        object: Box::new(ExprIR::IdentifierExpr {
            name: "items".into(),
            type_ann: None,
        }),
        method: "contains".into(),
        args: vec![ExprIR::IdentifierExpr {
            name: "target".into(),
            type_ann: None,
        }],
    };
    insta::with_settings!({snapshot_path => "snapshots"}, {
        insta::assert_snapshot!("python_method_contains", fmt(&expr));
    });
}

#[test]
fn snapshot_match_expr() {
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
                        bindings: vec!["value".into()],
                    },
                },
                body: BlockIR {
                    statements: vec![StatementIR::ReturnStmtIR {
                        value: Some(ExprIR::IdentifierExpr {
                            name: "value".into(),
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
                            value: serde_json::json!(-1),
                        }),
                    }],
                },
                union_type: None,
            },
        ],
    };
    insta::with_settings!({snapshot_path => "snapshots"}, {
        insta::assert_snapshot!("python_match_expr", fmt(&expr));
    });
}
