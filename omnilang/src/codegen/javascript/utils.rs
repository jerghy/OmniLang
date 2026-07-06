//! JS/TS type annotation and code generation utilities.
//!
//! Provides `js_type` (pub) for mapping OMNI types to JS/TS type strings,
//! and internal helpers for generating structs, enums, type aliases, and
//! standalone functions.

use super::CodeWriter;
use crate::ir::*;

/// Map OMNI type to JS/TS type annotation string.
pub fn js_type(kind: &str, type_args: &[TypeIR], ts_mode: bool) -> String {
    if !ts_mode {
        return String::new();
    }
    match kind {
        "int" | "i32" | "i64" | "u32" | "u64" | "float" | "f64" | "f32" => "number".into(),
        "bool" => "boolean".into(),
        "string" | "String" | "str" => "string".into(),
        "void" | "()" => "void".into(),
        "Vec" | "Array" | "List" => {
            if let Some(inner) = type_args.first() {
                format!("{}[]", js_type(&inner.kind, &inner.type_args, true))
            } else {
                "any[]".into()
            }
        }
        "HashMap" | "Map" | "Dict" => {
            if type_args.len() >= 2 {
                format!(
                    "Record<{}, {}>",
                    js_type(&type_args[0].kind, &type_args[0].type_args, true),
                    js_type(&type_args[1].kind, &type_args[1].type_args, true)
                )
            } else {
                "Record<string, any>".into()
            }
        }
        "Option" => {
            if let Some(inner) = type_args.first() {
                format!("{} | null", js_type(&inner.kind, &inner.type_args, true))
            } else {
                "any | null".into()
            }
        }
        other => {
            // If it looks like a user-defined type (starts with uppercase), use as-is
            if other.chars().next().is_some_and(|c| c.is_uppercase()) {
                other.into()
            } else {
                "any".into()
            }
        }
    }
}

pub(super) fn format_type_ir_js(ty: &TypeIR, ts_mode: bool) -> String {
    js_type(&ty.kind, &ty.type_args, ts_mode)
}

pub(super) fn generate_struct(w: &mut CodeWriter, struct_ir: &StructIR, ts_mode: bool) {
    let export_prefix = if struct_ir.visibility == Visibility::Public {
        "export "
    } else {
        ""
    };
    w.emit(&format!("{}class {} {{", export_prefix, struct_ir.name));
    w.indent();

    // Build constructor params with optional TS type annotations
    let mut param_strs = Vec::new();
    for field in &struct_ir.fields {
        let mut s = field.name.clone();
        if ts_mode {
            let ann = format_type_ir_js(&field.r#type, true);
            s.push_str(&format!(": {}", ann));
        }
        param_strs.push(s);
    }
    let all_params = param_strs.join(", ");

    if !struct_ir.fields.is_empty() {
        w.emit(&format!("constructor({}) {{", all_params));
        w.indent();
        for field in &struct_ir.fields {
            w.emit(&format!("this.{} = {};", field.name, field.name));
        }
        w.dedent();
        w.emit("}");
    }

    w.dedent();
    w.emit("}");
}

pub(super) fn generate_enum(w: &mut CodeWriter, enum_ir: &EnumIR, _ts_mode: bool) {
    let export_prefix = if enum_ir.visibility == Visibility::Public {
        "export "
    } else {
        ""
    };
    w.emit(&format!(
        "{}const {} = Object.freeze({{",
        export_prefix, enum_ir.name
    ));
    w.indent();
    for (i, variant) in enum_ir.variants.iter().enumerate() {
        if variant.payload.is_some() {
            w.emit(&format!("{}: \"{}\",", variant.name, variant.name));
        } else {
            w.emit(&format!("{}: {},", variant.name, i));
        }
    }
    w.dedent();
    w.emit("});");
}

pub(super) fn generate_type_alias(w: &mut CodeWriter, alias: &TypeAliasIR, ts_mode: bool) {
    if !ts_mode {
        return; // Skip in JS mode
    }
    let target = format_type_ir_js(&alias.target, true);
    let export_prefix = if alias.visibility == Visibility::Public {
        "export "
    } else {
        ""
    };
    w.emit(&format!(
        "{}type {} = {};",
        export_prefix, alias.name, target
    ));
}

pub(super) fn generate_standalone_function(w: &mut CodeWriter, func: &FunctionIR, ts_mode: bool) {
    let mut param_strs = Vec::new();
    for p in &func.params {
        let mut s = p.name.clone();
        if ts_mode {
            if let Some(ref ty) = p.r#type {
                let ann = format_type_ir_js(ty, true);
                if !ann.is_empty() {
                    s.push_str(&format!(": {}", ann));
                }
            }
        }
        param_strs.push(s);
    }

    let ret_ann = if ts_mode {
        func.return_type
            .as_ref()
            .map(|rt| {
                let ann = format_type_ir_js(rt, true);
                if ann.is_empty() {
                    String::new()
                } else {
                    format!(": {}", ann)
                }
            })
            .unwrap_or_default()
    } else {
        String::new()
    };

    let all_params = param_strs.join(", ");

    let async_kw = if func.is_async { "async " } else { "" };
    let prefix = if func.visibility == Visibility::Public {
        format!("export {}function ", async_kw)
    } else {
        format!("{}function ", async_kw)
    };

    w.emit(&format!(
        "{}{}({}){} {{",
        prefix, func.name, all_params, ret_ann
    ));
    w.indent();

    if func.body.statements.is_empty() {
        w.emit(";");
    } else {
        for stmt in &func.body.statements {
            super::generate_statement(w, stmt, ts_mode);
        }
    }

    w.dedent();
    w.emit("}");
}
