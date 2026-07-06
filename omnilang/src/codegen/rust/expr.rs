//! Expression, statement, and type formatting for Rust codegen.
//!
//! Functions for formatting IR expressions, types, and logic constructs
//! (structs, enums, type aliases, standalone functions) as Rust source code.

use super::CodeWriter;
use crate::codegen::{format_json_value, JSON_RUST};
use crate::ir::*;

pub(super) fn format_block_as_rust_expr(block: &BlockIR) -> String {
    super::super::format_block_as_expr(block, &format_expr_ir, "None")
}

pub(super) fn format_expr_ir(expr: &ExprIR) -> String {
    match expr {
        ExprIR::LiteralExpr { value } => format_json_value(value, &JSON_RUST),
        ExprIR::IdentifierExpr { name } => name.clone(),
        ExprIR::FieldAccessExpr { object, field } => {
            format!("{}.{}", format_expr_ir(object), field)
        }
        ExprIR::OptionalChainExpr { object, field, .. } => {
            format!("{}.{}", format_expr_ir(object), field)
        }
        ExprIR::BinaryOpExpr { op, left, right } => {
            let rs_op = binary_op_to_rust(op);
            let lhs = format_expr_ir(left);
            let rhs = format_expr_ir(right);
            format!("{} {} {}", lhs, rs_op, rhs)
        }
        ExprIR::UnaryOpExpr { op, operand } => {
            let rs_op = match op.as_str() {
                "not" => "!",
                "neg" => "-",
                _ => op,
            };
            format!("{}{}", rs_op, format_expr_ir(operand))
        }
        ExprIR::TernaryIR {
            cond,
            then_expr,
            else_expr,
        } => {
            let c = format_expr_ir(cond);
            let t = format_expr_ir(then_expr);
            let e = format_expr_ir(else_expr);
            format!("if {} {{ {} }} else {{ {} }}", c, t, e)
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
            format!("vec![{}]", items.join(", "))
        }
        ExprIR::StructExpr { fields, .. } => {
            let entries: Vec<String> = fields
                .iter()
                .map(|(k, v)| format!("{}: {}", k, format_expr_ir(v)))
                .collect();
            format!("{{ {} }}", entries.join(", "))
        }
        ExprIR::VariantLiteral {
            union_type,
            variant,
            fields,
        } => {
            let entries: Vec<String> = fields
                .iter()
                .map(|(k, v)| format!("{}: {}", k, format_expr_ir(v)))
                .collect();
            format!("{}::{} {{ {} }}", union_type, variant, entries.join(", "))
        }
        ExprIR::MethodCallExpr {
            is_optional: _,
            object,
            method,
            args,
            ..
        } => {
            let obj = format_expr_ir(object);
            let arg_strs: Vec<String> = args.iter().map(format_expr_ir).collect();
            format!("{}.{}({})", obj, method, arg_strs.join(", "))
        }
        ExprIR::BlockExpr {
            statements: _,
            tail,
        } => {
            // Only tail expression emitted; preceding statements preserved in IR for future codegen
            format_expr_ir(tail)
        }
        ExprIR::IndexExpr { object, index } => {
            let obj = format_expr_ir(object);
            let idx = format_expr_ir(index);
            format!("({obj})[{idx} as usize]")
        }
        ExprIR::AwaitExpr { operand } => {
            format!("{}.await", format_expr_ir(operand))
        }
        ExprIR::MatchExpr { matched, arms } => {
            let m = format_expr_ir(matched);
            let cases: Vec<String> = arms
                .iter()
                .map(|arm| {
                    let body = format_block_as_rust_expr(&arm.body);
                    match &arm.pattern.kind {
                        MatchPatternKindIR::Variant { name, .. } => {
                            format!("    UnionType::{} {{ .. }} => {},", name, body)
                        }
                        MatchPatternKindIR::Wildcard => {
                            format!("    _ => {},", body)
                        }
                        _ => String::new(),
                    }
                })
                .collect();
            format!("match &{} {{\n{}\n}}", m, cases.join("\n"))
        }
        ExprIR::ClosureExpr { params, body } => {
            let b = format_expr_ir(body);
            if params.is_empty() {
                format!("(|| {b})")
            } else {
                let p = params.join(", ");
                format!("(|{p}| {b})")
            }
        }
    }
}

/// Map OMNI binary operator name to Rust operator.
pub(super) fn binary_op_to_rust(op: &str) -> &str {
    super::super::binary_op_to_str(op, false, false)
}

/// Map OMNI type to Rust type annotation string.
pub fn rs_type(kind: &str, type_args: &[TypeIR]) -> String {
    match kind {
        "int" | "i32" => "i32".into(),
        "i64" => "i64".into(),
        "u32" => "u32".into(),
        "u64" => "u64".into(),
        "float" | "f64" => "f64".into(),
        "f32" => "f32".into(),
        "bool" => "bool".into(),
        "string" | "String" | "str" => "String".into(),
        "void" | "()" => "()".into(),
        "Vec" | "Array" | "List" => {
            if let Some(inner) = type_args.first() {
                format!("Vec<{}>", rs_type(&inner.kind, &inner.type_args))
            } else {
                "Vec<()>".into()
            }
        }
        "HashMap" | "Map" | "Dict" => {
            if type_args.len() >= 2 {
                format!(
                    "HashMap<{}, {}>",
                    rs_type(&type_args[0].kind, &type_args[0].type_args),
                    rs_type(&type_args[1].kind, &type_args[1].type_args)
                )
            } else {
                "HashMap<String, ()>".into()
            }
        }
        "Option" => {
            if let Some(inner) = type_args.first() {
                format!("Option<{}>", rs_type(&inner.kind, &inner.type_args))
            } else {
                "Option<()>".into()
            }
        }
        other => {
            // If it looks like a user-defined type, use as-is
            if other.chars().next().is_some_and(|c| c.is_uppercase()) {
                other.into()
            } else {
                "()".into()
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════
// Logic construct generation (structs, enums, type aliases, standalone functions)
// ═══════════════════════════════════════════════════════════

pub(super) fn format_type_ir_rust(ty: &TypeIR) -> String {
    rs_type(&ty.kind, &ty.type_args)
}

pub(super) fn generate_struct(w: &mut CodeWriter, struct_ir: &StructIR) {
    let vis = if struct_ir.visibility == "public" {
        "pub "
    } else {
        ""
    };
    w.emit(&format!("{}struct {} {{", vis, struct_ir.name));
    w.indent();
    for field in &struct_ir.fields {
        let field_vis = if field.visibility == "public" {
            "pub "
        } else {
            ""
        };
        let type_str = format_type_ir_rust(&field.r#type);
        w.emit(&format!("{}{}: {},", field_vis, field.name, type_str));
    }
    w.dedent();
    w.emit("}");
}

pub(super) fn generate_enum(w: &mut CodeWriter, enum_ir: &EnumIR) {
    let vis = if enum_ir.visibility == "public" {
        "pub "
    } else {
        ""
    };
    w.emit(&format!("{}enum {} {{", vis, enum_ir.name));
    w.indent();
    for variant in &enum_ir.variants {
        if let Some(payload) = &variant.payload {
            let payload_str = format_type_ir_rust(payload);
            w.emit(&format!("{}({}),", variant.name, payload_str));
        } else {
            w.emit(&format!("{},", variant.name));
        }
    }
    w.dedent();
    w.emit("}");
}

pub(super) fn generate_type_alias(w: &mut CodeWriter, alias: &TypeAliasIR) {
    let vis = if alias.visibility == "public" {
        "pub "
    } else {
        ""
    };
    let target = format_type_ir_rust(&alias.target);
    w.emit(&format!("{}type {} = {};", vis, alias.name, target));
}

pub(super) fn generate_standalone_function(w: &mut CodeWriter, func: &FunctionIR) {
    let vis = if func.visibility == "public" {
        "pub "
    } else {
        ""
    };

    // Build parameter list — NO &self for standalone
    let mut param_strs = Vec::new();
    for p in &func.params {
        let mut s = p.name.clone();
        if let Some(ref ty) = p.r#type {
            let ann = format_type_ir_rust(ty);
            s.push_str(&format!(": {}", ann));
        }
        param_strs.push(s);
    }

    // Determine return type annotation
    let ret_ann = func
        .return_type
        .as_ref()
        .map(|rt| format!(" -> {}", format_type_ir_rust(rt)))
        .unwrap_or_default();

    let all_params = param_strs.join(", ");

    w.emit(&format!(
        "{}fn {}({}){} {{",
        vis, func.name, all_params, ret_ann
    ));
    w.indent();

    if func.body.statements.is_empty() {
        // Empty body — need at least a semicolon or ()
        w.emit(";");
    } else {
        for stmt in &func.body.statements {
            super::generate_statement(w, stmt);
        }
    }

    w.dedent();
    w.emit("}");
}
