//! Dart codegen visitor — generates Dart source from ModuleIR containers.
//!
//! Implements `CodegenVisitor` for the Dart backend. Each per-item method
//! contains the generation logic formerly in free functions in `expr.rs` and
//! `mod.rs`. Helper functions (`dart_type`, `format_type_ir_dart`) remain in
//! `expr.rs`.

use crate::codegen::shared::visitor::CodegenVisitor;
use crate::codegen::CodeWriter;
use crate::ir::*;

pub(crate) struct DartVisitor;

impl CodegenVisitor for DartVisitor {
    fn generate_type_alias(&mut self, w: &mut CodeWriter, alias: &TypeAliasIR) {
        let target = super::expr::format_type_ir_dart(&alias.target);
        w.emit_fmt(format_args!("typedef {} = {};", alias.name, target));
    }

    fn generate_struct(&mut self, w: &mut CodeWriter, s: &StructIR) {
        w.emit_fmt(format_args!("class {} {{", s.name));
        w.indent();

        // Field declarations (final)
        for field in &s.fields {
            let type_str = super::expr::format_type_ir_dart(&field.r#type);
            w.emit_fmt(format_args!("final {} {};", type_str, field.name));
        }

        // Constructor with named params
        if !s.fields.is_empty() {
            w.emit("");
            let mut param_strs = Vec::new();
            for field in &s.fields {
                param_strs.push(format!("required this.{}", field.name));
            }
            let all_params = param_strs.join(", ");
            w.emit_fmt(format_args!("{}({{ {} }});", s.name, all_params));
        }

        if s.fields.is_empty() {
            w.emit_fmt(format_args!("{}();", s.name));
        }

        w.dedent();
        w.emit("}");
    }

    fn generate_enum(&mut self, w: &mut CodeWriter, e: &EnumIR) {
        w.emit_fmt(format_args!("enum {} {{", e.name));
        w.indent();
        if !e.variants.is_empty() {
            let names: Vec<String> = e.variants.iter().map(|v| v.name.clone()).collect();
            w.emit(&names.join(", ").to_string());
        }
        w.dedent();
        w.emit("}");
    }

    fn generate_function(&mut self, w: &mut CodeWriter, f: &FunctionIR) {
        // Build parameter list — Dart uses `Type name` for positional params
        let mut param_strs = Vec::new();
        for p in &f.params {
            let mut s = String::new();
            if let Some(ref ty) = p.r#type {
                let ann = super::expr::dart_type(ty.kind.as_str(), &ty.type_args);
                s.push_str(&format!("{} ", ann));
            }
            s.push_str(&p.name);
            param_strs.push(s);
        }

        // Determine return type (skip void — implicit in Dart)
        let return_prefix = f
            .return_type
            .as_ref()
            .filter(|rt| !matches!(rt.kind, TypeKind::Void))
            .map(|rt| format!("{} ", super::expr::format_type_ir_dart(rt)))
            .unwrap_or_default();

        let all_params = param_strs.join(", ");
        w.emit_fmt(format_args!(
            "{}{}({}) {{",
            return_prefix, f.name, all_params
        ));
        w.indent();

        if !f.body.statements.is_empty() {
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
