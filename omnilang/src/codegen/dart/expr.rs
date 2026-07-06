//! Expression, statement, and type formatting for Dart codegen.
//!
//! Functions for formatting IR expressions, types, and logic constructs
//! (structs, enums, type aliases, standalone functions) as Dart source code.

use super::CodeWriter;
use crate::codegen::shared::expr::{DartFormatExpr, FormatExpr};
use crate::ir::*;

// ═══════════════════════════════════════════════════════════
// Expression formatting
// ═══════════════════════════════════════════════════════════

pub(super) fn format_expr_ir(expr: &ExprIR) -> String {
    DartFormatExpr.format_expr(expr)
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
