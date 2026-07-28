//! Rust codegen visitor — generates Rust source from ModuleIR containers.
//!
//! Implements `CodegenVisitor` for the Rust backend. Each per-item method
//! contains the generation logic formerly in free functions in `expr.rs`.
//! Helper functions (`rs_type`, `format_type_ir_rust`) remain in `expr.rs`.

use crate::codegen::shared::visitor::CodegenVisitor;
use crate::codegen::CodeWriter;
use crate::ir::*;

pub(crate) struct RustVisitor;

impl CodegenVisitor for RustVisitor {
    fn generate_type_alias(&mut self, w: &mut CodeWriter, alias: &TypeAliasIR) {
        let vis = if alias.visibility == Visibility::Public {
            "pub "
        } else {
            ""
        };
        let target = super::expr::format_type_ir_rust(&alias.target);
        w.emit_fmt(format_args!("{}type {} = {};", vis, alias.name, target));
    }

    fn generate_struct(&mut self, w: &mut CodeWriter, s: &StructIR) {
        let vis = if s.visibility == Visibility::Public {
            "pub "
        } else {
            ""
        };
        w.emit_fmt(format_args!("{}struct {} {{", vis, s.name));
        w.indent();
        for field in &s.fields {
            let field_vis = if field.visibility == Visibility::Public {
                "pub "
            } else {
                ""
            };
            let type_str = super::expr::format_type_ir_rust(&field.r#type);
            w.emit_fmt(format_args!("{}{}: {},", field_vis, field.name, type_str));
        }
        w.dedent();
        w.emit("}");
    }

    fn generate_enum(&mut self, w: &mut CodeWriter, e: &EnumIR) {
        let vis = if e.visibility == Visibility::Public {
            "pub "
        } else {
            ""
        };
        w.emit_fmt(format_args!("{}enum {} {{", vis, e.name));
        w.indent();
        for variant in &e.variants {
            if variant.fields.is_empty() {
                w.emit_fmt(format_args!("{},", variant.name));
            } else if variant.fields.len() == 1 {
                let payload_str = super::expr::format_type_ir_rust(&variant.fields[0].r#type);
                w.emit_fmt(format_args!("{}({}),", variant.name, payload_str));
            } else {
                w.emit_fmt(format_args!("{} {{", variant.name));
                w.indent();
                for f in &variant.fields {
                    let type_str = super::expr::format_type_ir_rust(&f.r#type);
                    w.emit_fmt(format_args!("{}: {},", f.name, type_str));
                }
                w.dedent();
                w.emit("},");
            }
        }
        w.dedent();
        w.emit("}");
    }

    fn generate_function(&mut self, w: &mut CodeWriter, f: &FunctionIR) {
        let vis = if f.visibility == Visibility::Public {
            "pub "
        } else {
            ""
        };

        // Build parameter list — NO &self for standalone
        let mut param_strs = Vec::new();
        for p in &f.params {
            let mut s = p.name.clone();
            if let Some(ref ty) = p.r#type {
                let ann = super::expr::format_type_ir_rust(ty);
                s.push_str(&format!(": {}", ann));
            }
            param_strs.push(s);
        }

        // Determine return type annotation
        let ret_ann = f
            .return_type
            .as_ref()
            .map(|rt| format!(" -> {}", super::expr::format_type_ir_rust(rt)))
            .unwrap_or_default();

        let all_params = param_strs.join(", ");

        w.emit_fmt(format_args!(
            "{}fn {}({}){} {{",
            vis, f.name, all_params, ret_ann
        ));
        w.indent();

        if f.body.statements.is_empty() {
            // Empty body — need at least a semicolon or ()
            w.emit(";");
        } else {
            for stmt in &f.body.statements {
                super::generate_statement(w, stmt);
            }
        }

        w.dedent();
        w.emit("}");
    }

    fn generate_store(&mut self, w: &mut CodeWriter, name: &str, store: &StoreIR) {
        super::generate_store(w, name, store);
    }

    fn generate_component(&mut self, w: &mut CodeWriter, name: &str, comp: &ComponentIR) {
        super::generate_component(w, name, comp);
    }
}
