//! JavaScript/TypeScript expression formatting utilities.
//!
//! Formats IR expressions, statements (as expressions), JSON values,
//! and binary operators into JavaScript/TypeScript source strings.

use crate::codegen::{format_block_as_expr, format_json_value, try_format_stdlib_call, JSON_JS, Target};
use crate::ir::*;

/// Format an IR expression node as a JS/TS expression string.
pub(super) fn format_expr_ir(expr: &ExprIR, _ts_mode: bool) -> String {
    match expr {
        ExprIR::LiteralExpr { value } => format_json_value(value, &JSON_JS),
        ExprIR::IdentifierExpr { name } => super::super::sanitize_js_ident(name),
        ExprIR::FieldAccessExpr { object, field } => {
            format!("{}.{}", format_expr_ir(object, _ts_mode), field)
        }
        ExprIR::OptionalChainExpr { object, field, .. } => {
            format!("{}?.{}", format_expr_ir(object, _ts_mode), field)
        }
        ExprIR::BinaryOpExpr { op, left, right } => {
            let js_op = crate::codegen::binary_op_to_str(op, true, false);
            let lhs = format_expr_ir(left, _ts_mode);
            let rhs = format_expr_ir(right, _ts_mode);
            format!("{} {} {}", lhs, js_op, rhs)
        }
        ExprIR::UnaryOpExpr { op, operand } => {
            let js_op = match op.as_str() {
                "not" => "!",
                "neg" => "-",
                "await" => "await ",
                _ => op,
            };
            format!("{}{}", js_op, format_expr_ir(operand, _ts_mode))
        }
        ExprIR::TernaryIR {
            cond,
            then_expr,
            else_expr,
        } => {
            let c = format_expr_ir(cond, _ts_mode);
            let t = format_expr_ir(then_expr, _ts_mode);
            let e = format_expr_ir(else_expr, _ts_mode);
            format!("{} ? {} : {}", c, t, e)
        }
        ExprIR::CallExpr { name, args } => {
            let arg_strs: Vec<String> = args.iter().map(|a| format_expr_ir(a, _ts_mode)).collect();
            // Check stdlib intrinsics
            let target = if _ts_mode {
                Target::TypeScript
            } else {
                Target::JavaScript
            };
            if let Some(native) = try_format_stdlib_call(name, &arg_strs, target) {
                return native;
            }
            match name.as_str() {
                "println" => format!("console.log({})", arg_strs.join(", ")),
                "print" => format!("process.stdout.write(String({}))", arg_strs.join(", ")),
                _ => format!("{}({})", name, arg_strs.join(", ")),
            }
        }
        ExprIR::StoreFieldAccessExpr {
            store_alias,
            field_name,
            call_args,
        } => {
            let args: Vec<String> = call_args
                .iter()
                .map(|a| format_expr_ir(a, _ts_mode))
                .collect();
            let arg_str = if args.is_empty() {
                String::new()
            } else {
                format!("({})", args.join(", "))
            };
            format!("{}.{}{}", store_alias, field_name, arg_str)
        }
        ExprIR::ArrayExpr { elements } => {
            let items: Vec<String> = elements
                .iter()
                .map(|e| format_expr_ir(e, _ts_mode))
                .collect();
            format!("[{}]", items.join(", "))
        }
        ExprIR::StructExpr { fields, .. } => {
            let entries: Vec<String> = fields
                .iter()
                .map(|(k, v)| format!("\"{}\": {}", k, format_expr_ir(v, _ts_mode)))
                .collect();
            format!("{{{}}}", entries.join(", "))
        }
        ExprIR::VariantLiteral {
            variant, fields, ..
        } => {
            let mut entries: Vec<String> = fields
                .iter()
                .map(|(k, v)| format!("\"{}\": {}", k, format_expr_ir(v, _ts_mode)))
                .collect();
            entries.insert(0, format!("\"_variant\": \"{}\"", variant));
            format!("{{{}}}", entries.join(", "))
        }
        ExprIR::MethodCallExpr {
            is_optional: _,
            object,
            method,
            args,
            ..
        } => {
            let obj = format_expr_ir(object, _ts_mode);
            let arg_strs: Vec<String> = args.iter().map(|a| format_expr_ir(a, _ts_mode)).collect();
            format!("{}.{}({})", obj, method, arg_strs.join(", "))
        }
        ExprIR::BlockExpr { statements, tail } => {
            // Emit as IIFE with preceding statements
            let stmts: Vec<String> = statements
                .iter()
                .map(|s| format!("  {};", format_stmt_ir(s, _ts_mode)))
                .collect();
            let tail_str = format_expr_ir(tail, _ts_mode);
            format!(
                "(() => {{\n{}\n  return {};\n}})()",
                stmts.join("\n"),
                tail_str
            )
        }
        ExprIR::IndexExpr { object, index } => {
            let obj = format_expr_ir(object, false);
            let idx = format_expr_ir(index, false);
            format!("({obj})[{idx}]")
        }
        ExprIR::AwaitExpr { operand } => {
            format!("await {}", format_expr_ir(operand, _ts_mode))
        }
        ExprIR::MatchExpr { matched, arms } => {
            let matched_str = format_expr_ir(matched, _ts_mode);
            let cases: Vec<String> = arms
                .iter()
                .map(|arm| {
                    let body_expr = format_block_as_js_expr(&arm.body, _ts_mode);
                    match &arm.pattern.kind {
                        MatchPatternKindIR::Variant { name, .. } => {
                            format!(
                                "  if (_v._variant === \"{}\") {{ return {}; }}",
                                name, body_expr
                            )
                        }
                        MatchPatternKindIR::Wildcard => {
                            format!("  return {};", body_expr)
                        }
                        MatchPatternKindIR::Literal { value } => {
                            let v = format_json_value(value, &JSON_JS);
                            format!("  if (_v._variant === {}) {{ return {}; }}", v, body_expr)
                        }
                    }
                })
                .collect();
            format!(
                "(() => {{\n  const _v = {};\n{}\n}})()",
                matched_str,
                cases.join("\n  ")
            )
        }
        ExprIR::ClosureExpr { params, body } => {
            let p = params.join(", ");
            let b = format_expr_ir(body, false);
            format!("(({p}) => {{ return {b}; }})")
        }
    }
}

/// Format a block's trailing expression as a JS value (for match arm bodies).
pub(super) fn format_block_as_js_expr(block: &BlockIR, ts_mode: bool) -> String {
    let fmt = |e: &ExprIR| format_expr_ir(e, ts_mode);
    format_block_as_expr(block, &fmt, "undefined")
}

/// Format a single statement IR node as a JS expression string (for block exprs).
pub(super) fn format_stmt_ir(stmt: &StatementIR, ts_mode: bool) -> String {
    match stmt {
        StatementIR::LetStmtIR { name, value, .. } => {
            let safe_name = super::super::sanitize_js_ident(name);
            let val = value
                .as_ref()
                .map(|e| format_expr_ir(e, ts_mode))
                .unwrap_or_else(|| "undefined".into());
            format!("let {} = {}", safe_name, val)
        }
        StatementIR::ExprStmtIR { expr } => format_expr_ir(expr, ts_mode),
        _ => "/* stmt */".into(),
    }
}

