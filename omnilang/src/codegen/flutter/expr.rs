//! Expression formatting for Flutter codegen.

use crate::ir::*;

pub(crate) fn format_block_as_flutter_expr(block: &BlockIR) -> String {
    if let Some(last) = block.statements.last() {
        match last {
            StatementIR::ExprStmtIR { expr } => format_expr_ir_flutter(expr),
            StatementIR::ReturnStmtIR { value } => value
                .as_ref()
                .map(format_expr_ir_flutter)
                .unwrap_or_else(|| "null".into()),
            // Non-value-producing statements — block yields null
            StatementIR::LetStmtIR { .. }
            | StatementIR::IfStmtIR { .. }
            | StatementIR::MutationStmt { .. }
            | StatementIR::EventEmitStmtIR { .. }
            | StatementIR::BlockStmtIR { .. }
            | StatementIR::WhileStmtIR { .. }
            | StatementIR::ForInStmtIR { .. }
            | StatementIR::BreakStmtIR
            | StatementIR::ContinueStmtIR
            | StatementIR::ThrowStmtIR { .. }
            | StatementIR::TryCatchStmtIR { .. } => "null".into(),
        }
    } else {
        "null".into()
    }
}

// ═══════════════════════════════════════════════════════════════
// Expression formatting
// ═══════════════════════════════════════════════════════════════

pub(crate) fn format_expr_ir_flutter(expr: &ExprIR) -> String {
    match expr {
        ExprIR::LiteralExpr { value } => format_json_value_flutter(value),
        ExprIR::IdentifierExpr { name } => name.clone(),
        ExprIR::FieldAccessExpr { object, field } => {
            format!("{}.{}", format_expr_ir_flutter(object), field)
        }
        ExprIR::OptionalChainExpr { object, field, .. } => {
            format!("{}.{}", format_expr_ir_flutter(object), field)
        }
        ExprIR::BinaryOpExpr { op, left, right } => {
            let dart_op = binary_op_to_flutter(op);
            let lhs = format_expr_ir_flutter(left);
            let rhs = format_expr_ir_flutter(right);
            format!("{} {} {}", lhs, dart_op, rhs)
        }
        ExprIR::UnaryOpExpr { op, operand } => {
            let dart_op = match op.as_str() {
                "not" => "!",
                "neg" => "-",
                _ => op,
            };
            format!("{}{}", dart_op, format_expr_ir_flutter(operand))
        }
        ExprIR::TernaryIR {
            cond,
            then_expr,
            else_expr,
        } => {
            let c = format_expr_ir_flutter(cond);
            let t = format_expr_ir_flutter(then_expr);
            let e = format_expr_ir_flutter(else_expr);
            format!("{} ? {} : {}", c, t, e)
        }
        ExprIR::CallExpr { name, args } => {
            let arg_strs: Vec<String> = args.iter().map(format_expr_ir_flutter).collect();
            format!("{}({})", name, arg_strs.join(", "))
        }
        ExprIR::StoreFieldAccessExpr {
            store_alias,
            field_name,
            call_args,
        } => {
            let args: Vec<String> = call_args.iter().map(format_expr_ir_flutter).collect();
            let arg_str = if args.is_empty() {
                String::new()
            } else {
                format!("({})", args.join(", "))
            };
            format!("{}.{}{}", store_alias, field_name, arg_str)
        }
        ExprIR::ArrayExpr { elements } => {
            let items: Vec<String> = elements.iter().map(format_expr_ir_flutter).collect();
            format!("[{}]", items.join(", "))
        }
        ExprIR::StructExpr { fields, .. } => {
            let entries: Vec<String> = fields
                .iter()
                .map(|(k, v)| format!("\"{}\": {}", k, format_expr_ir_flutter(v)))
                .collect();
            format!("{{{}}}", entries.join(", "))
        }
        ExprIR::VariantLiteral {
            union_type,
            variant,
            fields,
        } => {
            let entries: Vec<String> = fields
                .iter()
                .map(|(k, v)| format!("{}: {}", k, format_expr_ir_flutter(v)))
                .collect();
            format!("{}.{}({})", union_type, variant, entries.join(", "))
        }
        ExprIR::MethodCallExpr {
            is_optional: _,
            object,
            method,
            args,
            ..
        } => {
            let obj = format_expr_ir_flutter(object);
            let arg_strs: Vec<String> = args.iter().map(format_expr_ir_flutter).collect();
            format!("{}.{}({})", obj, method, arg_strs.join(", "))
        }
        ExprIR::BlockExpr {
            statements: _,
            tail,
        } => {
            // Only tail expression emitted; preceding statements preserved in IR for future codegen
            format_expr_ir_flutter(tail)
        }
        ExprIR::IndexExpr { object, index } => {
            let obj = format_expr_ir_flutter(object);
            let idx = format_expr_ir_flutter(index);
            format!("({obj})[{idx}]")
        }
        ExprIR::AwaitExpr { operand } => {
            format!("await {}", format_expr_ir_flutter(operand))
        }
        ExprIR::MatchExpr { matched, arms } => {
            let m = format_expr_ir_flutter(matched);
            let cases: Vec<String> = arms
                .iter()
                .map(|arm| {
                    let body = format_block_as_flutter_expr(&arm.body);
                    match &arm.pattern.kind {
                        MatchPatternKindIR::Variant { name, .. } => {
                            format!("  case \"{}\": return {};", name, body)
                        }
                        MatchPatternKindIR::Wildcard => {
                            format!("  default: return {};", body)
                        }
                        _ => String::new(),
                    }
                })
                .collect();
            format!(
                "(() {{\nswitch ({m}._variant) {{\n{}\n}}\n}})()",
                cases.join("\n")
            )
        }
        ExprIR::ClosureExpr { params, body } => {
            let b = format_expr_ir_flutter(body);
            if params.is_empty() {
                format!("(() => {b})")
            } else {
                let p = params.join(", ");
                format!("(({p}) => {b})")
            }
        }
    }
}

/// Format a JSON value as a Dart literal (shared with Flutter).
pub(crate) fn format_json_value_flutter(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Null => "null".into(),
        serde_json::Value::Bool(b) => {
            if *b {
                "true".into()
            } else {
                "false".into()
            }
        }
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::String(s) => {
            let escaped = s
                .replace('\\', "\\\\")
                .replace('\'', "\\'")
                .replace('\n', "\\n")
                .replace('\r', "\\r")
                .replace('\t', "\\t");
            format!("'{}'", escaped)
        }
        other => other.to_string(),
    }
}

/// Map OMNI binary operator name to Dart/Flutter operator.
pub(crate) fn binary_op_to_flutter(op: &str) -> &str {
    match op {
        "add" => "+",
        "sub" => "-",
        "mul" => "*",
        "div" => "/",
        "mod" => "%",
        "eq" => "==",
        "neq" => "!=",
        "lt" => "<",
        "gt" => ">",
        "lteq" => "<=",
        "gteq" => ">=",
        "and" => "&&",
        "or" => "||",
        "assign" => "=",
        "addassign" => "+=",
        "subassign" => "-=",
        "mulassign" => "*=",
        "divassign" => "/=",
        "modassign" => "%=",
        _ => op,
    }
}
