//! JavaScript/TypeScript codegen visitor — generates JS/TS source from ModuleIR containers.
//!
//! Implements `CodegenVisitor` for the JavaScript/TypeScript backend.
//! Stores `ts_mode` flag for type-annotated (TS) vs plain (JS) output.
//! Helper functions (`js_type`, `format_type_ir_js`) remain in `utils.rs`.

use crate::codegen::shared::visitor::CodegenVisitor;
use crate::codegen::CodeWriter;
use crate::ir::*;

pub(crate) struct JavaScriptVisitor {
    pub ts_mode: bool,
}

impl CodegenVisitor for JavaScriptVisitor {
    /// JS-specific: emit blank line after type aliases if any other container exists.
    fn generate_type_aliases(&mut self, w: &mut CodeWriter, module: &ModuleIR) {
        for alias in module.type_aliases.values() {
            self.generate_type_alias(w, alias);
        }
        if !module.structs.is_empty() || !module.enums.is_empty() || !module.functions.is_empty() {
            w.emit("");
        }
    }
    fn generate_type_alias(&mut self, w: &mut CodeWriter, alias: &TypeAliasIR) {
        if !self.ts_mode {
            return;
        }
        let target = super::utils::format_type_ir_js(&alias.target, true);
        let export_prefix = if alias.visibility == Visibility::Public {
            "export "
        } else {
            ""
        };
        w.emit_fmt(format_args!(
            "{}type {} = {};",
            export_prefix, alias.name, target
        ));
    }

    fn generate_struct(&mut self, w: &mut CodeWriter, s: &StructIR) {
        let export_prefix = if s.visibility == Visibility::Public {
            "export "
        } else {
            ""
        };
        w.emit_fmt(format_args!("{}class {} {{", export_prefix, s.name));
        w.indent();

        // Build constructor params with optional TS type annotations
        let mut param_strs = Vec::new();
        for field in &s.fields {
            let mut param = field.name.clone();
            if self.ts_mode {
                let ann = super::utils::format_type_ir_js(&field.r#type, true);
                param.push_str(&format!(": {}", ann));
            }
            param_strs.push(param);
        }
        let all_params = param_strs.join(", ");

        if !s.fields.is_empty() {
            w.emit_fmt(format_args!("constructor({}) {{", all_params));
            w.indent();
            for field in &s.fields {
                w.emit_fmt(format_args!("this.{} = {};", field.name, field.name));
            }
            w.dedent();
            w.emit("}");
        }

        w.dedent();
        w.emit("}");
    }

    fn generate_enum(&mut self, w: &mut CodeWriter, e: &EnumIR) {
        let export_prefix = if e.visibility == Visibility::Public {
            "export "
        } else {
            ""
        };
        w.emit_fmt(format_args!(
            "{}const {} = Object.freeze({{",
            export_prefix, e.name
        ));
        w.indent();
        for (i, variant) in e.variants.iter().enumerate() {
            if variant.payload.is_some() {
                w.emit_fmt(format_args!("{}: \"{}\",", variant.name, variant.name));
            } else {
                w.emit_fmt(format_args!("{}: {},", variant.name, i));
            }
        }
        w.dedent();
        w.emit("});");
    }

    fn generate_function(&mut self, w: &mut CodeWriter, f: &FunctionIR) {
        let mut param_strs = Vec::new();
        for p in &f.params {
            let mut s = p.name.clone();
            if self.ts_mode {
                if let Some(ref ty) = p.r#type {
                    let ann = super::utils::format_type_ir_js(ty, true);
                    if !ann.is_empty() {
                        s.push_str(&format!(": {}", ann));
                    }
                }
            }
            param_strs.push(s);
        }

        let ret_ann = if self.ts_mode {
            f.return_type
                .as_ref()
                .map(|rt| {
                    let ann = super::utils::format_type_ir_js(rt, true);
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

        let async_kw = if f.is_async { "async " } else { "" };
        let prefix = if f.visibility == Visibility::Public {
            format!("export {}function ", async_kw)
        } else {
            format!("{}function ", async_kw)
        };

        w.emit_fmt(format_args!(
            "{}{}({}){} {{",
            prefix, f.name, all_params, ret_ann
        ));
        w.indent();

        if f.body.statements.is_empty() {
            w.emit(";");
        } else {
            for stmt in &f.body.statements {
                super::generate_statement(w, stmt, self.ts_mode);
            }
        }

        w.dedent();
        w.emit("}");
    }

    fn generate_store(&mut self, w: &mut CodeWriter, name: &str, store: &StoreIR) {
        super::generate_store(w, name, store, self.ts_mode);
    }

    fn generate_component(&mut self, w: &mut CodeWriter, name: &str, comp: &ComponentIR) {
        super::generate_component(w, name, comp, self.ts_mode);
    }
}
