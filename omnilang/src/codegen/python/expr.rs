//! Expression formatting for Python codegen.

use crate::codegen::{format_json_value, JSON_PYTHON};
use crate::ir::*;

/// Format an IR expression as a Python expression string.
pub(super) fn format_expr_ir(expr: &ExprIR) -> String {
    match expr {
        ExprIR::LiteralExpr { value } => format_json_value(value, &JSON_PYTHON),
        ExprIR::IdentifierExpr { name } => name.clone(),
        ExprIR::FieldAccessExpr { object, field } => {
            format!("{}.{}", format_expr_ir(object), field)
        }
        ExprIR::BinaryOpExpr { op, left, right } => {
            let py_op = binary_op_to_python(op);
            let lhs = format_expr_ir(left);
            let rhs = format_expr_ir(right);
            format!("{} {} {}", lhs, py_op, rhs)
        }
        ExprIR::UnaryOpExpr { op, operand } => {
            let py_op = match op.as_str() {
                "not" => "not ",
                "neg" => "-",
                _ => op,
            };
            format!("{}{}", py_op, format_expr_ir(operand))
        }
        ExprIR::TernaryIR {
            cond,
            then_expr,
            else_expr,
        } => {
            let c = format_expr_ir(cond);
            let t = format_expr_ir(then_expr);
            let e = format_expr_ir(else_expr);
            format!("{} if {} else {}", t, c, e)
        }
        ExprIR::CallExpr { name, args } => {
            let arg_strs: Vec<String> = args.iter().map(format_expr_ir).collect();
            format!("{}({})", name, arg_strs.join(", "))
        }
        ExprIR::StoreFieldAccessExpr {
            store_alias,
            field_name,
            call_args,
        } => {
            let args: Vec<String> = call_args.iter().map(format_expr_ir).collect();
            let arg_str = if args.is_empty() {
                String::new()
            } else {
                format!("({})", args.join(", "))
            };
            format!("{}.{}{}", store_alias, field_name, arg_str)
        }
        ExprIR::ArrayExpr { elements } => {
            let items: Vec<String> = elements.iter().map(format_expr_ir).collect();
            format!("[{}]", items.join(", "))
        }
        ExprIR::StructExpr { type_name, fields } => {
            let field_strs: Vec<String> = fields
                .iter()
                .map(|(name, val)| format!("{}={}", name, format_expr_ir(val)))
                .collect();
            format!("{}({})", type_name, field_strs.join(", "))
        }
        ExprIR::MethodCallExpr {
            is_optional: _,
            object,
            method,
            args,
        } => {
            let obj = format_expr_ir(object);
            let arg_strs: Vec<String> = args.iter().map(format_expr_ir).collect();
            match method.as_str() {
                "len" | "length" => format!("len({})", obj),
                "is_empty" => format!("len({}) == 0", obj),
                "push" => format!("{}.append({})", obj, arg_strs.join(", ")),
                "pop" => format!("{}.pop()", obj),
                "contains" => format!("({} in {})", arg_strs.join(", "), obj),
                _ => format!("{}.{}({})", obj, method, arg_strs.join(", ")),
            }
        }
        ExprIR::BlockExpr {
            statements: _,
            tail,
        } => {
            // Only tail expression emitted; preceding statements preserved in IR for future codegen
            format_expr_ir(tail)
        }
        ExprIR::IndexExpr { object, index } => {
            format!("{}[{}]", format_expr_ir(object), format_expr_ir(index))
        }
        ExprIR::ClosureExpr { params, body } => {
            let body_str = format_expr_ir(body);
            format!("lambda {}: {}", params.join(", "), body_str)
        }
        ExprIR::AwaitExpr { operand } => {
            format!("await {}", format_expr_ir(operand))
        }
        ExprIR::VariantLiteral {
            variant, fields, ..
        } => {
            let mut entries: Vec<String> = fields
                .iter()
                .map(|(k, v)| format!("\"{}\": {}", k, format_expr_ir(v)))
                .collect();
            entries.insert(0, format!("\"_variant\": \"{}\"", variant));
            format!("{{{}}}", entries.join(", "))
        }
        ExprIR::MatchExpr { matched, arms } => {
            let m = format_expr_ir(matched);
            let mut parts = Vec::new();
            for (i, arm) in arms.iter().enumerate() {
                let body = super::stmt::format_block_as_python_expr(&arm.body);
                match &arm.pattern.kind {
                    MatchPatternKindIR::Variant { name, .. } => {
                        let kw = if i == 0 { "if" } else { "elif" };
                        parts.push(format!("({body}) {kw} {m}.get(\"_variant\") == \"{name}\""));
                    }
                    MatchPatternKindIR::Wildcard => {
                        parts.push(format!("({body})"));
                    }
                    _ => {}
                }
            }
            if parts.is_empty() {
                "None".into()
            } else {
                parts.join(" else ")
            }
        }
        _ => "None".into(),
    }
}

/// Map OMNI binary operator name to Python operator.
fn binary_op_to_python(op: &str) -> &str {
    crate::codegen::binary_op_to_str(op, false, true)
}

/// Format a TypeIR as a Python type annotation string.
pub(super) fn format_type_ir(ty: &TypeIR) -> String {
    match ty.kind.as_str() {
        "i32" | "i64" | "int" | "u32" | "u64" => "int".into(),
        "f64" | "f32" | "float" => "float".into(),
        "bool" => "bool".into(),
        "string" | "String" | "str" => "str".into(),
        "void" | "()" => "None".into(),
        "Vec" | "Array" | "List" => {
            if let Some(inner) = ty.type_args.first() {
                format!("list[{}]", format_type_ir(inner))
            } else {
                "list".into()
            }
        }
        "HashMap" | "Map" | "Dict" => {
            if ty.type_args.len() >= 2 {
                let args: Vec<String> = ty.type_args.iter().map(format_type_ir).collect();
                format!("dict[{}]", args.join(", "))
            } else {
                "dict".into()
            }
        }
        "Option" => {
            if let Some(inner) = ty.type_args.first() {
                format!("Optional[{}]", format_type_ir(inner))
            } else {
                "Optional[Any]".into()
            }
        }
        other => {
            // If it looks like a user-defined type, use as-is
            if other.chars().next().is_some_and(|c| c.is_uppercase()) {
                format!("'{}'", other) // forward reference
            } else {
                "Any".into()
            }
        }
    }
}
