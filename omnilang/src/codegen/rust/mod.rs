//! Rust codegen: IR -> Rust.
//!
//! Generates Rust source files from Intent IR. Each component becomes
//! a struct with an impl block, each function becomes a method, and
//! stores become standalone structs with impl blocks.

mod expr;
#[cfg(test)]
mod tests;

pub(crate) use expr::*;

use std::collections::HashMap;

use super::{CodeWriter, Codegen, CompileOutput, CompiledFile};
use crate::codegen::{format_json_value, JSON_RUST};
use crate::ir::*;

pub struct RustCodegen;

impl Codegen for RustCodegen {
    fn generate(&self, ir: &IRProgram, _source_files: &HashMap<String, String>) -> CompileOutput {
        let mut files = Vec::new();

        for (module_key, module) in &ir.modules {
            let module_name = super::sanitize_module_name(module_key);
            let source_path = format!("{}.omni", module_name);

            let mut w = CodeWriter::new();
            w.emit(&format!("// Generated from {}", source_path));
            w.emit("");

            // Generate type aliases
            for alias in module.type_aliases.values() {
                generate_type_alias(&mut w, alias);
            }
            if !module.type_aliases.is_empty() {
                w.emit("");
            }

            // Generate structs
            for struct_ir in module.structs.values() {
                generate_struct(&mut w, struct_ir);
                w.emit("");
            }

            // Generate enums
            for enum_ir in module.enums.values() {
                generate_enum(&mut w, enum_ir);
                w.emit("");
            }

            // Generate standalone functions
            for func in module.functions.values() {
                generate_standalone_function(&mut w, func);
                w.emit("");
            }

            // Generate stores from this module
            for (store_name, store) in &module.stores {
                generate_store(&mut w, store_name, store);
                w.emit("");
            }

            // Generate components from this module
            for (comp_name, comp) in &module.components {
                generate_component(&mut w, comp_name, comp);
                w.emit("");
            }

            let out_path = format!("{}.rs", module_name);
            files.push(CompiledFile {
                path: out_path,
                content: w.into_string(),
            });
        }

        CompileOutput::success(files)
    }
}

// ═══════════════════════════════════════════════════════════
// Component generation
// ═══════════════════════════════════════════════════════════

fn generate_component(w: &mut CodeWriter, name: &str, comp: &ComponentIR) {
    let has_fields = !comp.states.is_empty() || !comp.props.is_empty();
    let has_methods = has_fields || !comp.fns.is_empty() || !comp.computeds.is_empty();

    // Emit struct definition
    if has_fields {
        w.emit(&format!("struct {} {{", name));
        w.indent();
        for (prop_name, prop_type) in &comp.props {
            w.emit(&format!(
                "{}: Option<{}>,",
                prop_name,
                rs_type(&prop_type.kind, &prop_type.type_args)
            ));
        }
        for state in &comp.states {
            w.emit(&format!(
                "{}: {},",
                state.name,
                rs_type(&state.r#type.kind, &state.r#type.type_args)
            ));
        }
        w.dedent();
        w.emit("}");
    } else {
        w.emit(&format!("struct {};", name));
    }

    if !has_methods {
        return;
    }

    // Emit impl block
    w.emit(&format!("impl {} {{", name));
    w.indent();

    // Constructor (new) if there are fields
    if has_fields {
        w.emit("fn new() -> Self {");
        w.indent();
        if comp.states.is_empty() && comp.props.is_empty() {
            w.emit(name);
        } else {
            w.emit("Self {");
            w.indent();
            for prop_name in comp.props.keys() {
                w.emit(&format!("{}: None,  // prop", prop_name));
            }
            for state in &comp.states {
                let rs_init = format_expr_ir(&state.initial_value);
                w.emit(&format!("{}: {},", state.name, rs_init));
            }
            w.dedent();
            w.emit("}");
        }
        w.dedent();
        w.emit("}");
        w.emit("");
    }

    // Computed properties as methods
    for computed in &comp.computeds {
        let body = format_expr_ir(&computed.body);
        let ret_type = rs_type(&computed.r#type.kind, &computed.r#type.type_args);
        w.emit(&format!("fn {}(&self) -> {} {{", computed.name, ret_type));
        w.indent();
        w.emit(&body.to_string());
        w.dedent();
        w.emit("}");
        w.emit("");
    }

    // Functions (methods)
    for func in &comp.fns {
        generate_function(w, func);
        w.emit("");
    }

    w.dedent();
    w.emit("}");
}

// ═══════════════════════════════════════════════════════════
// Store generation
// ═══════════════════════════════════════════════════════════

fn generate_store(w: &mut CodeWriter, name: &str, store: &StoreIR) {
    if store.signals.is_empty() {
        w.emit(&format!("struct {};", name));
        w.emit("");
        w.emit(&format!("impl {} {{", name));
        w.indent();
        w.emit("fn new() -> Self {");
        w.indent();
        w.emit(name);
        w.dedent();
        w.emit("}");
        w.dedent();
        w.emit("}");
        return;
    }

    // Struct with fields
    w.emit(&format!("struct {} {{", name));
    w.indent();
    for (signal_name, signal_type) in &store.signals {
        w.emit(&format!(
            "{}: {},",
            signal_name,
            rs_type(&signal_type.kind, &signal_type.type_args)
        ));
    }
    w.dedent();
    w.emit("}");

    // Impl block
    w.emit(&format!("impl {} {{", name));
    w.indent();

    // Constructor
    w.emit("fn new() -> Self {");
    w.indent();
    w.emit("Self {");
    w.indent();
    for signal_name in store.signals.keys() {
        let default = store
            .signal_defaults
            .get(signal_name)
            .map(|v| format_json_value(v, &JSON_RUST))
            .unwrap_or_else(|| "None".into());
        w.emit(&format!("{}: {},", signal_name, default));
    }
    w.dedent();
    w.emit("}");
    w.dedent();
    w.emit("}");

    // Getters as methods (no return type — TODO: infer from expression)
    for (getter_name, getter_expr) in &store.getters {
        w.emit("");
        let body = format_expr_ir(getter_expr);
        w.emit(&format!("fn {}(&self) {{", getter_name));
        w.indent();
        w.emit(&body.to_string());
        w.dedent();
        w.emit("}");
    }

    w.dedent();
    w.emit("}");
}

// ═══════════════════════════════════════════════════════════
// Function generation
// ═══════════════════════════════════════════════════════════

fn generate_function(w: &mut CodeWriter, func: &FunctionIR) {
    // Build parameter list — prepend &self for method
    let mut param_strs = vec!["&self".to_string()];
    for p in &func.params {
        let mut s = p.name.clone();
        if let Some(ref ty) = p.r#type {
            let ann = rs_type(&ty.kind, &ty.type_args);
            s.push_str(&format!(": {}", ann));
        }
        param_strs.push(s);
    }

    // Determine return type annotation
    let ret_ann = func
        .return_type
        .as_ref()
        .map(|rt| format!(" -> {}", rs_type(&rt.kind, &rt.type_args)))
        .unwrap_or_default();

    let all_params = param_strs.join(", ");

    let async_kw = if func.is_async { "async " } else { "" };
    w.emit(&format!(
        "{}fn {}({}){} {{",
        async_kw, func.name, all_params, ret_ann
    ));
    w.indent();

    if func.body.statements.is_empty() {
        // Empty body — valid Rust syntax
    } else {
        for stmt in &func.body.statements {
            generate_statement(w, stmt);
        }
    }

    w.dedent();
    w.emit("}");
}

// ═══════════════════════════════════════════════════════════
// Statement generation
// ═══════════════════════════════════════════════════════════

fn generate_statement(w: &mut CodeWriter, stmt: &StatementIR) {
    match stmt {
        StatementIR::LetStmtIR {
            name,
            value,
            r#type,
        } => {
            let rhs = value
                .as_ref()
                .map(format_expr_ir)
                .unwrap_or_else(|| "None".into());
            let type_ann = r#type
                .as_ref()
                .map(|t| format!(": {}", rs_type(&t.kind, &t.type_args)))
                .unwrap_or_default();
            w.emit(&format!("let {}{} = {};", name, type_ann, rhs));
        }
        StatementIR::IfStmtIR {
            condition,
            then_body,
            else_ifs,
            else_body,
        } => {
            let cond = format_expr_ir(condition);
            w.emit(&format!("if {} {{", cond));
            w.indent();
            for s in &then_body.statements {
                generate_statement(w, s);
            }
            w.dedent();
            for (elif_cond, elif_body) in else_ifs {
                let elif_js = format_expr_ir(elif_cond);
                w.emit(&format!("}} else if ({}) {{", elif_js));
                w.indent();
                for s in &elif_body.statements {
                    generate_statement(w, s);
                }
                w.dedent();
            }
            if let Some(else_blk) = else_body {
                if !else_blk.statements.is_empty() {
                    w.emit("} else {");
                    w.indent();
                    for s in &else_blk.statements {
                        generate_statement(w, s);
                    }
                    w.dedent();
                }
            }
            w.emit("}");
        }
        StatementIR::ReturnStmtIR { value } => {
            let rhs = value
                .as_ref()
                .map(format_expr_ir)
                .unwrap_or_else(|| "()".into());
            w.emit(&format!("return {};", rhs));
        }
        StatementIR::MutationStmt { intent: None } => {}
        StatementIR::MutationStmt {
            intent: Some(mut_intent),
        } => {
            let target = &mut_intent.target.name;
            let amount = mut_intent
                .amount
                .as_ref()
                .map(format_expr_ir)
                .unwrap_or_else(|| "0".into());
            match &mut_intent.operation {
                MutationOp::DirectAssign => {
                    w.emit(&format!("self.{} = {};", target, amount));
                }
                MutationOp::Increment => {
                    w.emit(&format!("self.{} += {};", target, amount));
                }
                MutationOp::Decrement => {
                    w.emit(&format!("self.{} -= {};", target, amount));
                }
                MutationOp::Multiply => {
                    w.emit(&format!("self.{} *= {};", target, amount));
                }
                MutationOp::Divide => {
                    w.emit(&format!("self.{} /= {};", target, amount));
                }
                MutationOp::Modulo => {
                    w.emit(&format!("self.{} %= {};", target, amount));
                }
                other => {
                    w.emit(&format!(
                        "// TODO: mutation operation '{}' on '{}'",
                        other, target
                    ));
                }
            }
        }
        StatementIR::ExprStmtIR { expr } => {
            let line = format_expr_ir(expr);
            // Skip null expressions
            if line != "None" {
                w.emit(&format!("{};", line));
            }
        }
        StatementIR::EventEmitStmtIR { event_name, args } => {
            let arg_str = if args.is_empty() {
                String::new()
            } else {
                let formatted: Vec<String> = args.iter().map(format_expr_ir).collect();
                format!("({})", formatted.join(", "))
            };
            w.emit(&format!("// emit event: {}{}", event_name, arg_str));
        }
        StatementIR::WhileStmtIR { condition, body } => {
            let cond = format_expr_ir(condition);
            w.emit(&format!("while {} {{", cond));
            w.indent();
            for s in &body.statements {
                generate_statement(w, s);
            }
            w.dedent();
            w.emit("}");
        }
        StatementIR::ForInStmtIR {
            var_name,
            iterable,
            body,
        } => {
            let iter = format_expr_ir(iterable);
            w.emit(&format!("for {} in {} {{", var_name, iter));
            w.indent();
            for s in &body.statements {
                generate_statement(w, s);
            }
            w.dedent();
            w.emit("}");
        }
        StatementIR::BreakStmtIR => {
            w.emit("break;");
        }
        StatementIR::ContinueStmtIR => {
            w.emit("continue;");
        }
        _ => {}
    }
}
