//! Lowering for statements and blocks.
//!
//! Includes: `lower_stmt`, `lower_block`, `try_lower_mutation`, `mutation_for_method`.

use crate::ir::types::*;
use crate::parser::ast::*;
use std::collections::HashMap;

use super::expr::lower_expr;

// ═══════════════════════════════════════════════════════
// Statement lowering
// ═══════════════════════════════════════════════════════

pub(super) fn lower_stmt(stmt: &Stmt, variant_map: &HashMap<String, String>) -> StatementIR {
    match stmt {
        Stmt::VarDecl { name, init, .. } => StatementIR::LetStmtIR {
            name: name.clone(),
            value: init.as_ref().map(|e| lower_expr(e, variant_map)),
            r#type: None,
        },
        Stmt::Return { value, .. } => StatementIR::ReturnStmtIR {
            value: value.as_ref().map(|e| lower_expr(e, variant_map)),
        },
        Stmt::ExprStmt { expr, .. } => {
            // Bare block in statement position: emit a scoped block
            if let Expr::Block { body, .. } = expr {
                return StatementIR::BlockStmtIR {
                    body: lower_block(body, variant_map),
                };
            }
            if let Some(mutation) = try_lower_mutation(expr, variant_map) {
                StatementIR::MutationStmt {
                    intent: Some(mutation),
                }
            } else {
                StatementIR::ExprStmtIR {
                    expr: lower_expr(expr, variant_map),
                }
            }
        }
        Stmt::If {
            condition,
            then_body,
            else_ifs,
            else_body,
            ..
        } => StatementIR::IfStmtIR {
            condition: lower_expr(condition, variant_map),
            then_body: lower_block(then_body, variant_map),
            else_ifs: else_ifs
                .iter()
                .map(|(cond, body)| {
                    (
                        lower_expr(cond, variant_map),
                        lower_block(body, variant_map),
                    )
                })
                .collect(),
            else_body: else_body.as_ref().map(|b| lower_block(b, variant_map)),
        },
        Stmt::Print { newline, arg, .. } => StatementIR::ExprStmtIR {
            expr: ExprIR::CallExpr {
                name: if *newline { "println" } else { "print" }.into(),
                args: vec![lower_expr(arg, variant_map)],
            },
        },
        Stmt::While {
            condition, body, ..
        } => StatementIR::WhileStmtIR {
            condition: lower_expr(condition, variant_map),
            body: lower_block(body, variant_map),
        },
        Stmt::ForIn {
            var_name,
            iterable,
            body,
            ..
        } => StatementIR::ForInStmtIR {
            var_name: var_name.clone(),
            iterable: lower_expr(iterable, variant_map),
            body: lower_block(body, variant_map),
        },
        Stmt::Assign { target, value, .. } => {
            let (name, owner) = match target {
                AssignTarget::Name(n) => (n.clone(), None),
                AssignTarget::Field { object, field } => (field.clone(), Some(object.clone())),
                AssignTarget::Index { object, .. } => (object.clone(), None),
            };
            StatementIR::MutationStmt {
                intent: Some(MutationIntent {
                    target: SignalRefIR { owner, name },
                    operation: "direct_assign".into(),
                    amount: Some(lower_expr(value, variant_map)),
                    item: None,
                }),
            }
        }
        Stmt::Break(_) => StatementIR::BreakStmtIR,
        Stmt::Continue(_) => StatementIR::ContinueStmtIR,
        Stmt::Throw { expr, .. } => StatementIR::ThrowStmtIR {
            expr: lower_expr(expr, variant_map),
        },
        Stmt::TryCatch {
            try_body,
            catch_var,
            catch_body,
            finally_body,
            ..
        } => StatementIR::TryCatchStmtIR {
            try_body: lower_block(try_body, variant_map),
            catch_var: catch_var.clone(),
            catch_body: catch_body.as_ref().map(|b| lower_block(b, variant_map)),
            finally_body: finally_body.as_ref().map(|b| lower_block(b, variant_map)),
        },
        _ => StatementIR::ExprStmtIR {
            expr: ExprIR::LiteralExpr {
                value: serde_json::Value::Null,
            },
        },
    }
}

/// Try to lower an expression statement as a mutation intent.
/// Handles: MethodCall, FuncCall(FieldAccess), and Assignment to signals.
pub(super) fn try_lower_mutation(
    expr: &Expr,
    variant_map: &HashMap<String, String>,
) -> Option<MutationIntent> {
    // Pattern 1: Expr::MethodCall { object: VarRef("count"), method: "set", args: [value] }
    if let Expr::MethodCall {
        object,
        method,
        args,
        ..
    } = expr
    {
        if let Expr::VarRef(target_name, _) = object.as_ref() {
            return mutation_for_method(target_name, method, args.first(), variant_map);
        }
    }
    // Pattern 2: Expr::FuncCall { func: FieldAccess { VarRef("count"), "set" }, args: [value] }
    if let Expr::FuncCall { func, args, .. } = expr {
        if let Expr::FieldAccess { object, field, .. } = func.as_ref() {
            if let Expr::VarRef(target_name, _) = object.as_ref() {
                return mutation_for_method(target_name, field, args.first(), variant_map);
            }
        }
    }
    // Pattern 3: Assignment to signal: count = value or count += 1
    if let Expr::BinaryOp {
        left, op, right, ..
    } = expr
    {
        if let Expr::VarRef(target_name, _) = left.as_ref() {
            match op {
                BinaryOpKind::Assign => {
                    return Some(MutationIntent {
                        target: SignalRefIR {
                            owner: None,
                            name: target_name.clone(),
                        },
                        operation: "direct_assign".into(),
                        amount: Some(lower_expr(right, variant_map)),
                        item: None,
                    });
                }
                BinaryOpKind::AddAssign => {
                    return Some(MutationIntent {
                        target: SignalRefIR {
                            owner: None,
                            name: target_name.clone(),
                        },
                        operation: "increment".into(),
                        amount: Some(lower_expr(right, variant_map)),
                        item: None,
                    });
                }
                BinaryOpKind::SubAssign => {
                    return Some(MutationIntent {
                        target: SignalRefIR {
                            owner: None,
                            name: target_name.clone(),
                        },
                        operation: "decrement".into(),
                        amount: Some(lower_expr(right, variant_map)),
                        item: None,
                    });
                }
                BinaryOpKind::MulAssign => {
                    return Some(MutationIntent {
                        target: SignalRefIR {
                            owner: None,
                            name: target_name.clone(),
                        },
                        operation: "multiply".into(),
                        amount: Some(lower_expr(right, variant_map)),
                        item: None,
                    });
                }
                BinaryOpKind::DivAssign => {
                    return Some(MutationIntent {
                        target: SignalRefIR {
                            owner: None,
                            name: target_name.clone(),
                        },
                        operation: "divide".into(),
                        amount: Some(lower_expr(right, variant_map)),
                        item: None,
                    });
                }
                BinaryOpKind::ModAssign => {
                    return Some(MutationIntent {
                        target: SignalRefIR {
                            owner: None,
                            name: target_name.clone(),
                        },
                        operation: "modulo".into(),
                        amount: Some(lower_expr(right, variant_map)),
                        item: None,
                    });
                }
                _ => {}
            }
        }
        // Pattern 4: Assignment to store field: MyStore.count = value or MyStore.count += 1
        if let Expr::FieldAccess { object, field, .. } = left.as_ref() {
            if let Expr::VarRef(owner_name, _) = object.as_ref() {
                match op {
                    BinaryOpKind::Assign => {
                        return Some(MutationIntent {
                            target: SignalRefIR {
                                owner: Some(owner_name.clone()),
                                name: field.clone(),
                            },
                            operation: "direct_assign".into(),
                            amount: Some(lower_expr(right, variant_map)),
                            item: None,
                        });
                    }
                    BinaryOpKind::AddAssign => {
                        return Some(MutationIntent {
                            target: SignalRefIR {
                                owner: Some(owner_name.clone()),
                                name: field.clone(),
                            },
                            operation: "increment".into(),
                            amount: Some(lower_expr(right, variant_map)),
                            item: None,
                        });
                    }
                    BinaryOpKind::SubAssign => {
                        return Some(MutationIntent {
                            target: SignalRefIR {
                                owner: Some(owner_name.clone()),
                                name: field.clone(),
                            },
                            operation: "decrement".into(),
                            amount: Some(lower_expr(right, variant_map)),
                            item: None,
                        });
                    }
                    _ => {}
                }
            }
        }
    }
    None
}

pub(super) fn mutation_for_method(
    target_name: &str,
    method: &str,
    amount: Option<&Expr>,
    variant_map: &HashMap<String, String>,
) -> Option<MutationIntent> {
    let target = SignalRefIR {
        owner: None,
        name: target_name.into(),
    };
    let amount = amount.map(|e| lower_expr(e, variant_map));
    match method {
        "set" | "setValue" => Some(MutationIntent {
            target,
            operation: "direct_assign".into(),
            amount,
            item: None,
        }),
        "inc" | "increment" => Some(MutationIntent {
            target,
            operation: "increment".into(),
            amount,
            item: None,
        }),
        "dec" | "decrement" => Some(MutationIntent {
            target,
            operation: "decrement".into(),
            amount,
            item: None,
        }),
        "push" => Some(MutationIntent {
            target,
            operation: "push".into(),
            amount,
            item: None,
        }),
        "pop" => Some(MutationIntent {
            target,
            operation: "pop".into(),
            amount,
            item: None,
        }),
        _ => None,
    }
}

// ═══════════════════════════════════════════════════════
// Block lowering
// ═══════════════════════════════════════════════════════

pub(crate) fn lower_block(block: &Block, variant_map: &HashMap<String, String>) -> BlockIR {
    let mut statements: Vec<StatementIR> = block
        .stmts
        .iter()
        .map(|s| lower_stmt(s, variant_map))
        .collect();

    // Include trailing expression as an expression statement
    if let Some(ref expr) = block.trailing_expr {
        if let Some(mutation) = try_lower_mutation(expr, variant_map) {
            statements.push(StatementIR::MutationStmt {
                intent: Some(mutation),
            });
        } else {
            statements.push(StatementIR::ExprStmtIR {
                expr: lower_expr(expr, variant_map),
            });
        }
    }

    BlockIR {
        statements,
        execution_mode: "client-only".into(),
    }
}
