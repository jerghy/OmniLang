//! Lowering for expressions.
//!
//! Includes: `lower_expr`, `lower_expr_to_json`, `lower_expr_to_json_from_ir`.

use crate::ir::types::*;
use crate::parser::ast::*;
use std::collections::HashMap;

use super::stmt::lower_block;

// ═══════════════════════════════════════════════════════
// Expression lowering
// ═══════════════════════════════════════════════════════

pub(crate) fn lower_expr(
    expr: &Expr,
    variant_map: &HashMap<String, String>,
    mut event_fields: Option<&mut Vec<String>>,
) -> ExprIR {
    match expr {
        Expr::Literal(lit, _) => {
            let value = match lit {
                Literal::Int(n) => serde_json::Value::Number((*n).into()),
                Literal::Float(f) => serde_json::json!(f),
                Literal::String(s) => serde_json::Value::String(s.clone()),
                Literal::Bool(b) => serde_json::Value::Bool(*b),
                Literal::Null => serde_json::Value::Null,
            };
            ExprIR::LiteralExpr { value }
        }
        Expr::VarRef(name, _) => {
            // Check if this bare identifier is a tagged union variant (no fields)
            if let Some(union_name) = variant_map.get(name) {
                ExprIR::VariantLiteral {
                    union_type: union_name.clone(),
                    variant: name.clone(),
                    fields: vec![],
                }
            } else {
                ExprIR::IdentifierExpr { name: name.clone() }
            }
        }
        Expr::BinaryOp {
            left, op, right, ..
        } => ExprIR::BinaryOpExpr {
            op: match op {
                BinaryOpKind::NullishCoalesce => "??".into(),
                _ => format!("{:?}", op).to_lowercase(),
            },
            left: Box::new(lower_expr(left, variant_map, event_fields.as_deref_mut())),
            right: Box::new(lower_expr(right, variant_map, event_fields.as_deref_mut())),
        },
        Expr::UnaryOp { op, operand, .. } => ExprIR::UnaryOpExpr {
            op: match op {
                UnaryOpKind::Not => "!".into(),
                UnaryOpKind::Neg => "-".into(),
                UnaryOpKind::Await => "await ".into(),
            },
            operand: Box::new(lower_expr(
                operand,
                variant_map,
                event_fields.as_deref_mut(),
            )),
        },
        Expr::FuncCall { func, args, .. } => {
            // Pattern: signal.get() → signal.value
            if let Expr::FieldAccess { object, field, .. } = func.as_ref() {
                if let Expr::VarRef(obj_name, _) = object.as_ref() {
                    // Signal method calls
                    if field == "get" {
                        return ExprIR::FieldAccessExpr {
                            object: Box::new(ExprIR::IdentifierExpr {
                                name: obj_name.clone(),
                            }),
                            field: "value".into(),
                        };
                    }
                    if field == "set" || field == "inc" || field == "dec" {
                        // Mutation — handled at statement level by try_lower_mutation
                        return ExprIR::IdentifierExpr {
                            name: obj_name.clone(),
                        };
                    }
                    // Store field access
                    return ExprIR::StoreFieldAccessExpr {
                        store_alias: obj_name.clone(),
                        field_name: field.clone(),
                        call_args: args
                            .iter()
                            .map(|e| lower_expr(e, variant_map, event_fields.as_deref_mut()))
                            .collect(),
                    };
                }
            }
            // Regular function call
            if let Expr::VarRef(name, _) = func.as_ref() {
                ExprIR::CallExpr {
                    name: name.clone(),
                    args: args
                        .iter()
                        .map(|e| lower_expr(e, variant_map, event_fields.as_deref_mut()))
                        .collect(),
                }
            } else if let Expr::OptionalChain { object, field, .. } = func.as_ref() {
                // user?.len() — treat as optional method call
                ExprIR::MethodCallExpr {
                    object: Box::new(lower_expr(object, variant_map, event_fields.as_deref_mut())),
                    method: field.clone(),
                    args: args
                        .iter()
                        .map(|e| lower_expr(e, variant_map, event_fields.as_deref_mut()))
                        .collect(),
                    is_optional: true,
                }
            } else {
                ExprIR::LiteralExpr {
                    value: serde_json::Value::Null,
                }
            }
        }
        Expr::FieldAccess { object, field, .. } => {
            // Collect event field names during lowering (e.field → push field).
            if let Some(ref mut fields) = event_fields {
                if let Expr::VarRef(name, _) = object.as_ref() {
                    if name == "e" && !fields.contains(field) {
                        fields.push(field.clone());
                    }
                }
            }
            ExprIR::FieldAccessExpr {
                object: Box::new(lower_expr(object, variant_map, event_fields)),
                field: field.clone(),
            }
        }
        Expr::OptionalChain { object, field, .. } => ExprIR::OptionalChainExpr {
            object: Box::new(lower_expr(object, variant_map, event_fields.as_deref_mut())),
            field: field.clone(),
        },
        Expr::MethodCall {
            object,
            method,
            args,
            ..
        } => {
            // get() with no args → signal access (.value)
            // get(i) with args → array/map index access
            if method == "get" && args.is_empty() {
                ExprIR::FieldAccessExpr {
                    object: Box::new(lower_expr(object, variant_map, event_fields.as_deref_mut())),
                    field: "value".into(),
                }
            } else if method == "set" {
                // signal.set(value) → handled at statement level by try_lower_mutation
                ExprIR::IdentifierExpr {
                    name: method.clone(),
                }
            } else {
                // General method call: to_string, len, get(i), push, pop, contains, insert, etc.
                ExprIR::MethodCallExpr {
                    object: Box::new(lower_expr(object, variant_map, event_fields.as_deref_mut())),
                    method: method.clone(),
                    args: args
                        .iter()
                        .map(|e| lower_expr(e, variant_map, event_fields.as_deref_mut()))
                        .collect(),
                    is_optional: false,
                }
            }
        }
        Expr::StoreFieldAccess {
            store_alias,
            field_name,
            ..
        } => ExprIR::StoreFieldAccessExpr {
            store_alias: store_alias.clone(),
            field_name: field_name.clone(),
            call_args: vec![],
        },
        Expr::ArrayLiteral { elements, .. } => ExprIR::ArrayExpr {
            elements: elements
                .iter()
                .map(|e| lower_expr(e, variant_map, event_fields.as_deref_mut()))
                .collect(),
        },
        Expr::MapLiteral { entries, .. } => {
            let map: serde_json::Map<String, serde_json::Value> = entries
                .iter()
                .map(|(k, v)| {
                    let key = match lower_expr(k, variant_map, event_fields.as_deref_mut()) {
                        ExprIR::LiteralExpr { value } => match value {
                            serde_json::Value::String(s) => s,
                            other => format!("{}", other),
                        },
                        other => {
                            // For non-string literal keys, use the identifiers' name or Debug as fallback
                            match other {
                                ExprIR::IdentifierExpr { name } => name,
                                _ => format!("{:?}", other),
                            }
                        }
                    };
                    (key, lower_expr_to_json(v))
                })
                .collect();
            ExprIR::LiteralExpr {
                value: serde_json::Value::Object(map),
            }
        }
        Expr::StructLiteral {
            type_name, fields, ..
        } => {
            let ir_fields: Vec<(String, ExprIR)> = fields
                .iter()
                .map(|(name, val)| {
                    (
                        name.clone(),
                        lower_expr(val, variant_map, event_fields.as_deref_mut()),
                    )
                })
                .collect();
            // Check if this is a tagged union variant construction
            if let Some(union_name) = variant_map.get(type_name) {
                ExprIR::VariantLiteral {
                    union_type: union_name.clone(),
                    variant: type_name.clone(),
                    fields: ir_fields,
                }
            } else {
                ExprIR::StructExpr {
                    type_name: type_name.clone(),
                    fields: ir_fields,
                }
            }
        }
        Expr::IfExpr {
            condition,
            then_expr,
            else_expr,
            ..
        } => ExprIR::TernaryIR {
            cond: Box::new(lower_expr(
                condition,
                variant_map,
                event_fields.as_deref_mut(),
            )),
            then_expr: Box::new(lower_expr(
                then_expr,
                variant_map,
                event_fields.as_deref_mut(),
            )),
            else_expr: Box::new(lower_expr(
                else_expr,
                variant_map,
                event_fields.as_deref_mut(),
            )),
        },
        Expr::TernaryExpr {
            condition,
            then_expr,
            else_expr,
            ..
        } => ExprIR::TernaryIR {
            cond: Box::new(lower_expr(
                condition,
                variant_map,
                event_fields.as_deref_mut(),
            )),
            then_expr: Box::new(lower_expr(
                then_expr,
                variant_map,
                event_fields.as_deref_mut(),
            )),
            else_expr: Box::new(lower_expr(
                else_expr,
                variant_map,
                event_fields.as_deref_mut(),
            )),
        },
        Expr::Block { body, .. } => {
            // Block in expression position: preserve ALL statements.
            // The last expression statement provides the value; preceding
            // statements (e.g. print, let, mutations) are kept as-is.
            let block_ir = lower_block(body, variant_map, event_fields.as_deref_mut());
            if let Some(last) = block_ir.statements.last() {
                match last {
                    StatementIR::ExprStmtIR { expr } => {
                        if block_ir.statements.len() == 1 {
                            // Simple block { expr } — just the expression
                            expr.clone()
                        } else {
                            // Multi-statement block: wrap in BlockExpr
                            ExprIR::BlockExpr {
                                statements: block_ir.statements[..block_ir.statements.len() - 1]
                                    .to_vec(),
                                tail: Box::new(expr.clone()),
                            }
                        }
                    }
                    _ => {
                        // Last statement is not an expression (e.g. return, if)
                        // Preserve all statements; value is null
                        ExprIR::BlockExpr {
                            statements: block_ir.statements,
                            tail: Box::new(ExprIR::LiteralExpr {
                                value: serde_json::Value::Null,
                            }),
                        }
                    }
                }
            } else {
                ExprIR::LiteralExpr {
                    value: serde_json::Value::Null,
                }
            }
        }
        Expr::IndexGet { object, index, .. } => ExprIR::IndexExpr {
            object: Box::new(lower_expr(object, variant_map, event_fields.as_deref_mut())),
            index: Box::new(lower_expr(index, variant_map, event_fields.as_deref_mut())),
        },
        Expr::ClosureExpr { params, body, .. } => ExprIR::ClosureExpr {
            params: params.clone(),
            body: Box::new(lower_expr(body, variant_map, event_fields.as_deref_mut())),
        },
        Expr::AwaitExpr { operand, .. } => ExprIR::AwaitExpr {
            operand: Box::new(lower_expr(
                operand,
                variant_map,
                event_fields.as_deref_mut(),
            )),
        },
        Expr::MatchExpr { matched, arms, .. } => {
            let matched_ir = Box::new(lower_expr(
                matched,
                variant_map,
                event_fields.as_deref_mut(),
            ));
            let mut ir_arms = Vec::with_capacity(arms.len());
            for arm in arms {
                let pattern = match &arm.pattern.kind {
                    MatchPatternKind::Variant { name, bindings } => MatchPatternIR {
                        kind: MatchPatternKindIR::Variant {
                            name: name.clone(),
                            bindings: bindings.clone(),
                        },
                    },
                    MatchPatternKind::Wildcard => MatchPatternIR {
                        kind: MatchPatternKindIR::Wildcard,
                    },
                    MatchPatternKind::Literal(lit) => {
                        let value = match lit {
                            Literal::Int(n) => serde_json::Value::Number((*n).into()),
                            Literal::String(s) => serde_json::Value::String(s.clone()),
                            Literal::Bool(b) => serde_json::Value::Bool(*b),
                            _ => serde_json::Value::Null,
                        };
                        MatchPatternIR {
                            kind: MatchPatternKindIR::Literal { value },
                        }
                    }
                };
                ir_arms.push(MatchArmIR {
                    pattern,
                    body: lower_block(&arm.body, variant_map, event_fields.as_deref_mut()),
                });
            }
            ExprIR::MatchExpr {
                matched: matched_ir,
                arms: ir_arms,
            }
        }
        Expr::TemplateString { parts, .. } => {
            // Concatenate template string parts into a single string literal.
            // For expression parts, lower and extract their value representation.
            let concatenated: String = parts
                .iter()
                .map(|p| match p {
                    TemplatePart::Literal(s) => s.clone(),
                    TemplatePart::Expr(e) => {
                        match lower_expr(e, variant_map, event_fields.as_deref_mut()) {
                            ExprIR::LiteralExpr { value } => match &value {
                                serde_json::Value::String(s) => s.clone(),
                                other => other.to_string(),
                            },
                            other => format!("{:?}", other),
                        }
                    }
                })
                .collect();
            ExprIR::LiteralExpr {
                value: serde_json::Value::String(concatenated),
            }
        }
        Expr::NotYetImplemented {
            feature: _feature, ..
        } => {
            // Emit a null literal for unimplemented features rather than
            // silently using a catch-all wildcard. This makes it explicit
            // that the feature gap is known.
            ExprIR::LiteralExpr {
                value: serde_json::Value::Null,
            }
        }
        Expr::Error { .. } => {
            // Error nodes should be filtered upstream; emit null as fallback.
            ExprIR::LiteralExpr {
                value: serde_json::Value::Null,
            }
        }
    }
}

/// Lower an expression to a serde_json::Value (for literal extraction).
pub(super) fn lower_expr_to_json(expr: &Expr) -> serde_json::Value {
    match lower_expr(expr, &HashMap::new(), None) {
        ExprIR::LiteralExpr { value } => value,
        ExprIR::IdentifierExpr { name } => serde_json::Value::String(name),
        other => serde_json::Value::String(format!("{:?}", other)),
    }
}

/// Convert an ExprIR back to a JSON value for inline_style serialization.
pub(super) fn lower_expr_to_json_from_ir(expr: &ExprIR) -> serde_json::Value {
    match expr {
        ExprIR::LiteralExpr { value } => value.clone(),
        ExprIR::IdentifierExpr { name } => serde_json::json!({"$signal": name}),
        ExprIR::FieldAccessExpr { object, field } => {
            // Resolve ThemeStore.currentBg → JS name ThemeStore.currentBg
            if let ExprIR::IdentifierExpr { name: obj_name } = object.as_ref() {
                serde_json::json!({"$signal": format!("{}.{}", obj_name, field)})
            } else {
                let obj = lower_expr_to_json_from_ir(object);
                serde_json::json!({"$signal_concat": [obj, field.as_str()]})
            }
        }
        ExprIR::BinaryOpExpr { op: _, left, right } => {
            serde_json::json!({
                "$concat": [
                    lower_expr_to_json_from_ir(left),
                    lower_expr_to_json_from_ir(right)
                ]
            })
        }
        ExprIR::CallExpr { name, args } => {
            serde_json::json!({
                "$call": {
                    "name": name,
                    "args": args.iter().map(lower_expr_to_json_from_ir).collect::<Vec<_>>()
                }
            })
        }
        ExprIR::StoreFieldAccessExpr {
            store_alias,
            field_name,
            ..
        } => {
            serde_json::json!({"$signal": format!("{}.{}", store_alias, field_name)})
        }
        other => serde_json::Value::String(format!("{:?}", other)),
    }
}
