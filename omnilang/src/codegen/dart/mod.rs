//! Dart codegen: IR -> Dart.
//!
//! Generates Dart source files from Intent IR. Each component becomes
//! a class, each function becomes a method, and stores become standalone
//! classes.

mod expr;
#[cfg(test)]
mod tests;

pub(crate) use expr::*;

use std::collections::HashMap;

use super::{CodeWriter, Codegen, CompileOutput, CompiledFile};
use crate::codegen::{format_json_value, JSON_DART};
use crate::ir::*;

pub struct DartCodegen;

impl Codegen for DartCodegen {
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

            // Generate structs as classes
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

            let out_path = format!("{}.dart", module_name);
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
    w.emit(&format!("class {} {{", name));
    w.indent();

    if comp.states.is_empty()
        && comp.props.is_empty()
        && comp.fns.is_empty()
        && comp.computeds.is_empty()
    {
        // Empty class — just emit default constructor
        w.emit(&format!("{}();", name));
        w.dedent();
        w.emit("}");
        return;
    }

    // Field declarations for props (initialized to null)
    for (prop_name, prop_type) in &comp.props {
        let ann = dart_type(&prop_type.kind, &prop_type.type_args);
        w.emit(&format!("{} {};", ann, prop_name));
    }

    // Field declarations + initialization for states
    for state in &comp.states {
        let ann = dart_type(&state.r#type.kind, &state.r#type.type_args);
        let init = format_expr_ir(&state.initial_value);
        // Use `late` for fields initialized in constructor
        w.emit(&format!("{} {} = {};", ann, state.name, init));
    }

    // Constructor with state/prop initialization
    if !comp.states.is_empty() || !comp.props.is_empty() {
        w.emit("");
        w.emit(&format!("{}() {{", name));
        w.indent();
        for prop in comp.props.keys() {
            w.emit(&format!("this.{} = null;  // prop", prop));
        }
        // States are already initialized inline, but include as comments for clarity
        for state in &comp.states {
            let init = format_expr_ir(&state.initial_value);
            w.emit(&format!("this.{} = {};", state.name, init));
        }
        w.dedent();
        w.emit("}");
    }

    // Computed properties as methods
    for computed in &comp.computeds {
        w.emit("");
        let body = format_expr_ir(&computed.body);
        let ret_type = dart_type(&computed.r#type.kind, &computed.r#type.type_args);
        w.emit(&format!("{} {}() {{", ret_type, computed.name));
        w.indent();
        w.emit(&format!("return {};", body));
        w.dedent();
        w.emit("}");
    }

    // Functions (methods)
    for func in &comp.fns {
        w.emit("");
        generate_function(w, func);
    }

    w.dedent();
    w.emit("}");
}

// ═══════════════════════════════════════════════════════════
// Store generation
// ═══════════════════════════════════════════════════════════

fn generate_store(w: &mut CodeWriter, name: &str, store: &StoreIR) {
    w.emit(&format!("class {} {{", name));
    w.indent();

    if store.signals.is_empty() && store.getters.is_empty() {
        w.emit(&format!("{}();", name));
        w.dedent();
        w.emit("}");
        return;
    }

    // Field declarations with defaults for signals
    for (signal_name, signal_type) in &store.signals {
        let ann = dart_type(&signal_type.kind, &signal_type.type_args);
        let default = store
            .signal_defaults
            .get(signal_name)
            .map(|v| format_json_value(v, &JSON_DART))
            .unwrap_or_else(|| "null".into());
        w.emit(&format!("{} {} = {};", ann, signal_name, default));
    }

    // Default constructor
    if !store.signals.is_empty() {
        w.emit("");
        w.emit(&format!("{}() {{", name));
        w.indent();
        for signal_name in store.signals.keys() {
            let default = store
                .signal_defaults
                .get(signal_name)
                .map(|v| format_json_value(v, &JSON_DART))
                .unwrap_or_else(|| "null".into());
            w.emit(&format!("this.{} = {};", signal_name, default));
        }
        w.dedent();
        w.emit("}");
    }

    // Getters
    for (getter_name, getter_expr) in &store.getters {
        w.emit("");
        w.emit(&format!("get {} {{", getter_name));
        w.indent();
        w.emit(&format!("return {};", format_expr_ir(getter_expr)));
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
    // Build parameter list — Dart uses `Type name` for positional params
    let mut param_strs = Vec::new();
    for p in &func.params {
        let mut s = String::new();
        if let Some(ref ty) = p.r#type {
            let ann = dart_type(&ty.kind, &ty.type_args);
            s.push_str(&format!("{} ", ann));
        }
        s.push_str(&p.name);
        param_strs.push(s);
    }

    // Determine return type (skip void — implicit in Dart)
    // Wrap in Future<T> for async functions
    let return_prefix = if func.is_async {
        match func.return_type {
            Some(ref rt) if rt.kind != "void" && rt.kind != "()" => {
                format!("Future<{}> ", dart_type(&rt.kind, &rt.type_args))
            }
            _ => "Future<void> ".to_string(),
        }
    } else {
        func.return_type
            .as_ref()
            .filter(|rt| rt.kind != "void" && rt.kind != "()")
            .map(|rt| format!("{} ", dart_type(&rt.kind, &rt.type_args)))
            .unwrap_or_default()
    };

    let async_kw = if func.is_async { "async " } else { "" };
    let all_params = param_strs.join(", ");
    w.emit(&format!(
        "{}{}({}) {}{{",
        return_prefix, func.name, all_params, async_kw
    ));
    w.indent();

    if func.body.statements.is_empty() {
        // Empty body — valid Dart syntax
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
                .unwrap_or_else(|| "null".into());
            if let Some(ref ty) = r#type {
                let ann = dart_type(&ty.kind, &ty.type_args);
                w.emit(&format!("{} {} = {};", ann, name, rhs));
            } else {
                w.emit(&format!("var {} = {};", name, rhs));
            }
        }
        StatementIR::IfStmtIR {
            condition,
            then_body,
            else_ifs,
            else_body,
        } => {
            let cond = format_expr_ir(condition);
            w.emit(&format!("if ({}) {{", cond));
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
                .unwrap_or_else(|| "null".into());
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
            match mut_intent.operation.as_str() {
                "direct_assign" => {
                    w.emit(&format!("this.{} = {};", target, amount));
                }
                "increment" => {
                    w.emit(&format!("this.{} += {};", target, amount));
                }
                "decrement" => {
                    w.emit(&format!("this.{} -= {};", target, amount));
                }
                "multiply" => {
                    w.emit(&format!("this.{} *= {};", target, amount));
                }
                "divide" => {
                    w.emit(&format!("this.{} /= {};", target, amount));
                }
                "modulo" => {
                    w.emit(&format!("this.{} %= {};", target, amount));
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
            if line != "null" {
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
            w.emit(&format!("while ({}) {{", cond));
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
            w.emit(&format!("for (var {} in {}) {{", var_name, iter));
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
