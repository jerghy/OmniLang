//! Lowering for statements and blocks.
//!
//! Includes: `lower_stmt`, `lower_block`, `try_lower_mutation`, `mutation_for_method`.

use crate::ir::types::*;
use crate::parser::ast::*;
use crate::semantic::SemanticResult;
use std::collections::HashMap;

use super::expr::lower_expr;

// ═══════════════════════════════════════════════════════
// Statement lowering
// ═══════════════════════════════════════════════════════

pub(super) fn lower_stmt(
    stmt: &Stmt,
    variant_map: &HashMap<String, String>,
    mut event_fields: Option<&mut Vec<String>>,
    semantics: &mut SemanticResult,
) -> StatementIR {
    match stmt {
        Stmt::VarDecl { name, init, .. } => StatementIR::LetStmtIR {
            name: name.clone(),
            value: init
                .as_ref()
                .map(|e| lower_expr(e, variant_map, event_fields.as_deref_mut(), semantics)),
            r#type: None,
        },
        Stmt::Return { value, .. } => StatementIR::ReturnStmtIR {
            value: value
                .as_ref()
                .map(|e| lower_expr(e, variant_map, event_fields.as_deref_mut(), semantics)),
        },
        Stmt::ExprStmt { expr, .. } => {
            // Bare block in statement position: emit a scoped block
            if let Expr::Block { body, .. } = expr {
                return StatementIR::BlockStmtIR {
                    body: lower_block(body, variant_map, event_fields.as_deref_mut(), semantics),
                };
            }
            if let Some(mutation) =
                try_lower_mutation(expr, variant_map, event_fields.as_deref_mut(), semantics)
            {
                StatementIR::MutationStmt {
                    intent: Some(mutation),
                }
            } else {
                StatementIR::ExprStmtIR {
                    expr: lower_expr(expr, variant_map, event_fields.as_deref_mut(), semantics),
                }
            }
        }
        Stmt::If {
            condition,
            then_body,
            else_ifs,
            else_body,
            ..
        } => {
            let mut ir_else_ifs = Vec::with_capacity(else_ifs.len());
            for (cond, body) in else_ifs {
                ir_else_ifs.push((
                    lower_expr(cond, variant_map, event_fields.as_deref_mut(), semantics),
                    lower_block(body, variant_map, event_fields.as_deref_mut(), semantics),
                ));
            }
            StatementIR::IfStmtIR {
                condition: lower_expr(
                    condition,
                    variant_map,
                    event_fields.as_deref_mut(),
                    semantics,
                ),
                then_body: lower_block(
                    then_body,
                    variant_map,
                    event_fields.as_deref_mut(),
                    semantics,
                ),
                else_ifs: ir_else_ifs,
                else_body: else_body
                    .as_ref()
                    .map(|b| lower_block(b, variant_map, event_fields.as_deref_mut(), semantics)),
            }
        }
        Stmt::Print { newline, arg, .. } => StatementIR::ExprStmtIR {
            expr: ExprIR::CallExpr {
                name: if *newline { "println" } else { "print" }.into(),
                args: vec![lower_expr(
                    arg,
                    variant_map,
                    event_fields.as_deref_mut(),
                    semantics,
                )],
            },
        },
        Stmt::While {
            condition, body, ..
        } => StatementIR::WhileStmtIR {
            condition: lower_expr(
                condition,
                variant_map,
                event_fields.as_deref_mut(),
                semantics,
            ),
            body: lower_block(body, variant_map, event_fields.as_deref_mut(), semantics),
        },
        Stmt::ForIn {
            var_name,
            iterable,
            body,
            ..
        } => StatementIR::ForInStmtIR {
            var_name: var_name.clone(),
            iterable: lower_expr(
                iterable,
                variant_map,
                event_fields.as_deref_mut(),
                semantics,
            ),
            body: lower_block(body, variant_map, event_fields.as_deref_mut(), semantics),
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
                    operation: MutationOp::DirectAssign,
                    amount: Some(lower_expr(
                        value,
                        variant_map,
                        event_fields.as_deref_mut(),
                        semantics,
                    )),
                    item: None,
                }),
            }
        }
        Stmt::Break(_) => StatementIR::BreakStmtIR,
        Stmt::Continue(_) => StatementIR::ContinueStmtIR,
        Stmt::Throw { expr, .. } => StatementIR::ThrowStmtIR {
            expr: lower_expr(expr, variant_map, event_fields.as_deref_mut(), semantics),
        },
        Stmt::TryCatch {
            try_body,
            catch_var,
            catch_body,
            finally_body,
            ..
        } => StatementIR::TryCatchStmtIR {
            try_body: lower_block(
                try_body,
                variant_map,
                event_fields.as_deref_mut(),
                semantics,
            ),
            catch_var: catch_var.clone(),
            catch_body: catch_body
                .as_ref()
                .map(|b| lower_block(b, variant_map, event_fields.as_deref_mut(), semantics)),
            finally_body: finally_body
                .as_ref()
                .map(|b| lower_block(b, variant_map, event_fields, semantics)),
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
    mut event_fields: Option<&mut Vec<String>>,
    semantics: &mut SemanticResult,
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
            return mutation_for_method(
                target_name,
                method,
                args.first(),
                variant_map,
                event_fields,
                semantics,
            );
        }
    }
    // Pattern 2: Expr::FuncCall { func: FieldAccess { VarRef("count"), "set" }, args: [value] }
    if let Expr::FuncCall { func, args, .. } = expr {
        if let Expr::FieldAccess { object, field, .. } = func.as_ref() {
            if let Expr::VarRef(target_name, _) = object.as_ref() {
                return mutation_for_method(
                    target_name,
                    field,
                    args.first(),
                    variant_map,
                    event_fields,
                    semantics,
                );
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
                        operation: MutationOp::DirectAssign,
                        amount: Some(lower_expr(
                            right,
                            variant_map,
                            event_fields.as_deref_mut(),
                            semantics,
                        )),
                        item: None,
                    });
                }
                BinaryOpKind::AddAssign => {
                    return Some(MutationIntent {
                        target: SignalRefIR {
                            owner: None,
                            name: target_name.clone(),
                        },
                        operation: MutationOp::Increment,
                        amount: Some(lower_expr(
                            right,
                            variant_map,
                            event_fields.as_deref_mut(),
                            semantics,
                        )),
                        item: None,
                    });
                }
                BinaryOpKind::SubAssign => {
                    return Some(MutationIntent {
                        target: SignalRefIR {
                            owner: None,
                            name: target_name.clone(),
                        },
                        operation: MutationOp::Decrement,
                        amount: Some(lower_expr(
                            right,
                            variant_map,
                            event_fields.as_deref_mut(),
                            semantics,
                        )),
                        item: None,
                    });
                }
                BinaryOpKind::MulAssign => {
                    return Some(MutationIntent {
                        target: SignalRefIR {
                            owner: None,
                            name: target_name.clone(),
                        },
                        operation: MutationOp::Multiply,
                        amount: Some(lower_expr(
                            right,
                            variant_map,
                            event_fields.as_deref_mut(),
                            semantics,
                        )),
                        item: None,
                    });
                }
                BinaryOpKind::DivAssign => {
                    return Some(MutationIntent {
                        target: SignalRefIR {
                            owner: None,
                            name: target_name.clone(),
                        },
                        operation: MutationOp::Divide,
                        amount: Some(lower_expr(
                            right,
                            variant_map,
                            event_fields.as_deref_mut(),
                            semantics,
                        )),
                        item: None,
                    });
                }
                BinaryOpKind::ModAssign => {
                    return Some(MutationIntent {
                        target: SignalRefIR {
                            owner: None,
                            name: target_name.clone(),
                        },
                        operation: MutationOp::Modulo,
                        amount: Some(lower_expr(
                            right,
                            variant_map,
                            event_fields.as_deref_mut(),
                            semantics,
                        )),
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
                            operation: MutationOp::DirectAssign,
                            amount: Some(lower_expr(
                                right,
                                variant_map,
                                event_fields.as_deref_mut(),
                                semantics,
                            )),
                            item: None,
                        });
                    }
                    BinaryOpKind::AddAssign => {
                        return Some(MutationIntent {
                            target: SignalRefIR {
                                owner: Some(owner_name.clone()),
                                name: field.clone(),
                            },
                            operation: MutationOp::Increment,
                            amount: Some(lower_expr(
                                right,
                                variant_map,
                                event_fields.as_deref_mut(),
                                semantics,
                            )),
                            item: None,
                        });
                    }
                    BinaryOpKind::SubAssign => {
                        return Some(MutationIntent {
                            target: SignalRefIR {
                                owner: Some(owner_name.clone()),
                                name: field.clone(),
                            },
                            operation: MutationOp::Decrement,
                            amount: Some(lower_expr(right, variant_map, event_fields, semantics)),
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
    event_fields: Option<&mut Vec<String>>,
    semantics: &mut SemanticResult,
) -> Option<MutationIntent> {
    let target = SignalRefIR {
        owner: None,
        name: target_name.into(),
    };
    let amount = amount.map(|e| lower_expr(e, variant_map, event_fields, semantics));
    match method {
        "set" | "setValue" => Some(MutationIntent {
            target,
            operation: MutationOp::DirectAssign,
            amount,
            item: None,
        }),
        "inc" | "increment" => Some(MutationIntent {
            target,
            operation: MutationOp::Increment,
            amount,
            item: None,
        }),
        "dec" | "decrement" => Some(MutationIntent {
            target,
            operation: MutationOp::Decrement,
            amount,
            item: None,
        }),
        "push" => Some(MutationIntent {
            target,
            operation: MutationOp::Push,
            amount,
            item: None,
        }),
        "pop" => Some(MutationIntent {
            target,
            operation: MutationOp::Pop,
            amount,
            item: None,
        }),
        _ => None,
    }
}

// ═══════════════════════════════════════════════════════
// Block lowering
// ═══════════════════════════════════════════════════════

pub(crate) fn lower_block(
    block: &Block,
    variant_map: &HashMap<String, String>,
    mut event_fields: Option<&mut Vec<String>>,
    semantics: &mut SemanticResult,
) -> BlockIR {
    let mut statements: Vec<StatementIR> = block
        .stmts
        .iter()
        .map(|s| lower_stmt(s, variant_map, event_fields.as_deref_mut(), semantics))
        .collect();

    // Include trailing expression as an expression statement
    if let Some(ref expr) = block.trailing_expr {
        if let Some(mutation) =
            try_lower_mutation(expr, variant_map, event_fields.as_deref_mut(), semantics)
        {
            statements.push(StatementIR::MutationStmt {
                intent: Some(mutation),
            });
        } else {
            statements.push(StatementIR::ExprStmtIR {
                expr: lower_expr(expr, variant_map, event_fields, semantics),
            });
        }
    }

    BlockIR { statements }
}
