//! Lowering for statements and blocks.
//!
//! Includes: `lower_stmt`, `lower_block`, `try_lower_mutation`, `mutation_for_method`.

use crate::ir::types::*;
use crate::parser::ast::*;
use crate::semantic::SemanticResult;
use rustc_hash::FxHashMap;
use rustc_hash::FxHashSet;
use super::expr::lower_expr;

// ═══════════════════════════════════════════════════════
// Statement lowering
// ═══════════════════════════════════════════════════════

pub(super) fn lower_stmt(
    stmt: &Stmt,
    variant_map: &FxHashMap<String, String>,
    mut event_fields: Option<&mut FxHashSet<String>>,
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
                intent: Some(MutationIntent::DirectAssign {
                    target: SignalRefIR { owner, name },
                    value: Some(Box::new(lower_expr(
                        value,
                        variant_map,
                        event_fields.as_deref_mut(),
                        semantics,
                    ))),
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
    variant_map: &FxHashMap<String, String>,
    mut event_fields: Option<&mut FxHashSet<String>>,
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
                    return Some(MutationIntent::DirectAssign {
                        target: SignalRefIR {
                            owner: None,
                            name: target_name.clone(),
                        },
                        value: Some(Box::new(lower_expr(
                            right,
                            variant_map,
                            event_fields.as_deref_mut(),
                            semantics,
                        ))),
                    });
                }
                BinaryOpKind::AddAssign => {
                    return Some(MutationIntent::Increment {
                        target: SignalRefIR {
                            owner: None,
                            name: target_name.clone(),
                        },
                        amount: Some(Box::new(lower_expr(
                            right,
                            variant_map,
                            event_fields.as_deref_mut(),
                            semantics,
                        ))),
                    });
                }
                BinaryOpKind::SubAssign => {
                    return Some(MutationIntent::Decrement {
                        target: SignalRefIR {
                            owner: None,
                            name: target_name.clone(),
                        },
                        amount: Some(Box::new(lower_expr(
                            right,
                            variant_map,
                            event_fields.as_deref_mut(),
                            semantics,
                        ))),
                    });
                }
                BinaryOpKind::MulAssign => {
                    return Some(MutationIntent::Multiply {
                        target: SignalRefIR {
                            owner: None,
                            name: target_name.clone(),
                        },
                        amount: Some(Box::new(lower_expr(
                            right,
                            variant_map,
                            event_fields.as_deref_mut(),
                            semantics,
                        ))),
                    });
                }
                BinaryOpKind::DivAssign => {
                    return Some(MutationIntent::Divide {
                        target: SignalRefIR {
                            owner: None,
                            name: target_name.clone(),
                        },
                        amount: Some(Box::new(lower_expr(
                            right,
                            variant_map,
                            event_fields.as_deref_mut(),
                            semantics,
                        ))),
                    });
                }
                BinaryOpKind::ModAssign => {
                    return Some(MutationIntent::Modulo {
                        target: SignalRefIR {
                            owner: None,
                            name: target_name.clone(),
                        },
                        amount: Some(Box::new(lower_expr(
                            right,
                            variant_map,
                            event_fields.as_deref_mut(),
                            semantics,
                        ))),
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
                        return Some(MutationIntent::DirectAssign {
                            target: SignalRefIR {
                                owner: Some(owner_name.clone()),
                                name: field.clone(),
                            },
                            value: Some(Box::new(lower_expr(
                                right,
                                variant_map,
                                event_fields.as_deref_mut(),
                                semantics,
                            ))),
                        });
                    }
                    BinaryOpKind::AddAssign => {
                        return Some(MutationIntent::Increment {
                            target: SignalRefIR {
                                owner: Some(owner_name.clone()),
                                name: field.clone(),
                            },
                            amount: Some(Box::new(lower_expr(
                                right,
                                variant_map,
                                event_fields.as_deref_mut(),
                                semantics,
                            ))),
                        });
                    }
                    BinaryOpKind::SubAssign => {
                        return Some(MutationIntent::Decrement {
                            target: SignalRefIR {
                                owner: Some(owner_name.clone()),
                                name: field.clone(),
                            },
                            amount: Some(Box::new(lower_expr(right, variant_map, event_fields, semantics))),
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
    variant_map: &FxHashMap<String, String>,
    event_fields: Option<&mut FxHashSet<String>>,
    semantics: &mut SemanticResult,
) -> Option<MutationIntent> {
    let target = SignalRefIR {
        owner: None,
        name: target_name.into(),
    };
    let amount = amount.map(|e| Box::new(lower_expr(e, variant_map, event_fields, semantics)));
    match method {
        "set" | "setValue" => Some(MutationIntent::DirectAssign {
            target,
            value: amount,
        }),
        "inc" | "increment" => Some(MutationIntent::Increment {
            target,
            amount,
        }),
        "dec" | "decrement" => Some(MutationIntent::Decrement {
            target,
            amount,
        }),
        "push" => Some(MutationIntent::Push {
            target,
            item: amount,
        }),
        "pop" => Some(MutationIntent::Pop { target }),
        _ => None,
    }
}

// ═══════════════════════════════════════════════════════
// Block lowering
// ═══════════════════════════════════════════════════════

pub(crate) fn lower_block(
    block: &Block,
    variant_map: &FxHashMap<String, String>,
    mut event_fields: Option<&mut FxHashSet<String>>,
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
