//! Expression, statement, and type formatting for Dart codegen.
//!
//! Functions for formatting IR expressions, types, and logic constructs
//! (structs, enums, type aliases, standalone functions) as Dart source code.

use super::CodeWriter;
use crate::codegen::{format_json_value, JSON_DART};
use crate::ir::*;

pub(super) fn format_block_as_dart_expr(block: &BlockIR) -> String {
    super::super::format_block_as_expr(block, &format_expr_ir, "null")
}

// ═══════════════════════════════════════════════════════════
// Expression formatting
// ═══════════════════════════════════════════════════════════

pub(super) fn format_expr_ir(expr: &ExprIR) -> String {
    match expr {
        ExprIR::LiteralExpr { value } => format_json_value(value, &JSON_DART),
        ExprIR::IdentifierExpr { name } => name.clone(),
        ExprIR::FieldAccessExpr { object, field } => {
            format!("{}.{}", format_expr_ir(object), field)
        }
        ExprIR::OptionalChainExpr { object, field, .. } => {
            format!("{}.{}", format_expr_ir(object), field)
        }
        ExprIR::BinaryOpExpr { op, left, right } => {
            let dart_op = super::super::binary_op_to_str(op, false, false);
            let lhs = format_expr_ir(left);
            let rhs = format_expr_ir(right);
            format!("{} {} {}", lhs, dart_op, rhs)
        }
        ExprIR::UnaryOpExpr { op, operand } => {
            let dart_op = match op.as_str() {
                "not" => "!",
                "neg" => "-",
                _ => op,
            };
            format!("{}{}", dart_op, format_expr_ir(operand))
        }
        ExprIR::TernaryIR {
            cond,
            then_expr,
            else_expr,
        } => {
            let c = format_expr_ir(cond);
            let t = format_expr_ir(then_expr);
            let e = format_expr_ir(else_expr);
            format!("{} ? {} : {}", c, t, e)
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
        ExprIR::StructExpr { fields, .. } => {
            let entries: Vec<String> = fields
                .iter()
                .map(|(k, v)| format!("\"{}\": {}", k, format_expr_ir(v)))
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
                .map(|(k, v)| format!("{}: {}", k, format_expr_ir(v)))
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
            format!("({obj})[{idx}]")
        }
        ExprIR::AwaitExpr { operand } => {
            format!("await {}", format_expr_ir(operand))
        }
        ExprIR::MatchExpr { matched, arms } => {
            let m = format_expr_ir(matched);
            let cases: Vec<String> = arms
                .iter()
                .map(|arm| {
                    let body = format_block_as_dart_expr(&arm.body);
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
            let b = format_expr_ir(body);
            if params.is_empty() {
                format!("(() => {b})")
            } else {
                let p = params.join(", ");
                format!("(({p}) => {b})")
            }
        }
    }
}

/// Map OMNI type to Dart type annotation string.
pub fn dart_type(kind: &str, type_args: &[TypeIR]) -> String {
    match kind {
        "int" | "i32" | "i64" | "u32" | "u64" => "int".into(),
        "float" | "f64" | "f32" => "double".into(),
        "bool" => "bool".into(),
        "string" | "String" | "str" => "String".into(),
        "void" | "()" => "void".into(),
        "Vec" | "Array" | "List" => {
            if let Some(inner) = type_args.first() {
                format!("List<{}>", dart_type(&inner.kind, &inner.type_args))
            } else {
                "List".into()
            }
        }
        "HashMap" | "Map" | "Dict" => {
            if type_args.len() >= 2 {
                format!(
                    "Map<{}, {}>",
                    dart_type(&type_args[0].kind, &type_args[0].type_args),
                    dart_type(&type_args[1].kind, &type_args[1].type_args)
                )
            } else {
                "Map".into()
            }
        }
        "Option" => {
            if let Some(inner) = type_args.first() {
                format!("{}?", dart_type(&inner.kind, &inner.type_args))
            } else {
                "Object?".into()
            }
        }
        other => {
            // If it looks like a user-defined type (uppercase), use as-is
            if other.chars().next().is_some_and(|c| c.is_uppercase()) {
                other.into()
            } else {
                "dynamic".into()
            }
        }
    }
}

/// Format a TypeIR into a Dart type string via the existing `dart_type()`.
pub(super) fn format_type_ir_dart(ty: &TypeIR) -> String {
    dart_type(&ty.kind, &ty.type_args)
}

// ═══════════════════════════════════════════════════════════
// Logic construct generation (structs, enums, type aliases, standalone functions)
// ═══════════════════════════════════════════════════════════

pub(super) fn generate_struct(w: &mut CodeWriter, struct_ir: &StructIR) {
    w.emit(&format!("class {} {{", struct_ir.name));
    w.indent();

    // Field declarations (final)
    for field in &struct_ir.fields {
        let type_str = format_type_ir_dart(&field.r#type);
        w.emit(&format!("final {} {};", type_str, field.name));
    }

    // Constructor with named params
    if !struct_ir.fields.is_empty() {
        w.emit("");
        let mut param_strs = Vec::new();
        for field in &struct_ir.fields {
            param_strs.push(format!("required this.{}", field.name));
        }
        let all_params = param_strs.join(", ");
        w.emit(&format!("{}({{ {} }});", struct_ir.name, all_params));
    }

    if struct_ir.fields.is_empty() {
        w.emit(&format!("{}();", struct_ir.name));
    }

    w.dedent();
    w.emit("}");
}

pub(super) fn generate_enum(w: &mut CodeWriter, enum_ir: &EnumIR) {
    w.emit(&format!("enum {} {{", enum_ir.name));
    w.indent();
    if !enum_ir.variants.is_empty() {
        let names: Vec<String> = enum_ir.variants.iter().map(|v| v.name.clone()).collect();
        w.emit(&names.join(", ").to_string());
    }
    w.dedent();
    w.emit("}");
}

pub(super) fn generate_type_alias(w: &mut CodeWriter, alias: &TypeAliasIR) {
    let target = format_type_ir_dart(&alias.target);
    w.emit(&format!("typedef {} = {};", alias.name, target));
}

pub(super) fn generate_standalone_function(w: &mut CodeWriter, func: &FunctionIR) {
    // Build parameter list — Dart uses `Type name` for positional params
    let mut param_strs = Vec::new();
    for p in &func.params {
        let mut s = String::new();
        if let Some(ref ty) = p.r#type {
            let ann = format_type_ir_dart(ty);
            s.push_str(&format!("{} ", ann));
        }
        s.push_str(&p.name);
        param_strs.push(s);
    }

    // Determine return type (skip void — implicit in Dart)
    let return_prefix = func
        .return_type
        .as_ref()
        .filter(|rt| rt.kind != "void" && rt.kind != "()")
        .map(|rt| format!("{} ", format_type_ir_dart(rt)))
        .unwrap_or_default();

    let all_params = param_strs.join(", ");
    w.emit(&format!(
        "{}{}({}) {{",
        return_prefix, func.name, all_params
    ));
    w.indent();

    if !func.body.statements.is_empty() {
        for stmt in &func.body.statements {
            super::generate_statement(w, stmt);
        }
    }

    w.dedent();
    w.emit("}");
}
