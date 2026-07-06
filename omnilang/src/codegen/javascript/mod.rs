//! JavaScript/TypeScript codegen: IR -> JS/TS.
//!
//! Generates JS/TS source files from Intent IR. Each component becomes
//! a class, each function becomes a method, and stores become standalone
//! classes. In JS mode (ts_mode: false), no type annotations are emitted.
//! In TS mode (ts_mode: true), type annotations are added.

mod expr;
#[cfg(test)]
mod tests;
mod utils;

use expr::*;
pub use utils::*;

use std::collections::HashMap;

use super::{CodeWriter, Codegen, CompileOutput, CompiledFile};
use crate::codegen::shared::expr::FormatExpr;
use crate::codegen::{format_json_value, JSON_JS};
use crate::ir::*;

pub struct JavaScriptCodegen {
    pub ts_mode: bool,
}

impl JavaScriptCodegen {
    pub fn new(ts_mode: bool) -> Self {
        Self { ts_mode }
    }
}

impl Codegen for JavaScriptCodegen {
    fn generate(&self, ir: &IRProgram, _source_files: &HashMap<String, String>) -> CompileOutput {
        let mut files = Vec::new();
        let ext = if self.ts_mode { ".ts" } else { ".js" };
        for (module_key, module) in &ir.modules {
            let module_name = super::sanitize_module_name(module_key);
            let source_path = format!("{}.omni", module_name);

            let mut w = CodeWriter::new();
            w.emit(&format!("// Generated from {}", source_path));
            w.emit("");

            // Generate type aliases
            for alias in module.type_aliases.values() {
                generate_type_alias(&mut w, alias, self.ts_mode);
            }
            if !module.structs.is_empty()
                || !module.enums.is_empty()
                || !module.functions.is_empty()
            {
                w.emit("");
            }

            // Generate structs as classes
            for struct_ir in module.structs.values() {
                generate_struct(&mut w, struct_ir, self.ts_mode);
                w.emit("");
            }

            // Generate enums as frozen objects
            for enum_ir in module.enums.values() {
                generate_enum(&mut w, enum_ir, self.ts_mode);
                w.emit("");
            }

            // Generate standalone functions
            let mut has_main = false;
            for (fname, func) in &module.functions {
                generate_standalone_function(&mut w, func, self.ts_mode);
                w.emit("");
                if fname == "main" {
                    has_main = true;
                }
            }

            // Entry point: if there's a main(), call it
            if has_main {
                w.emit("main();");
                w.emit("");
            }

            // Generate stores from this module
            for (store_name, store) in &module.stores {
                generate_store(&mut w, store_name, store, self.ts_mode);
                w.emit("");
            }

            // Generate components from this module
            for (comp_name, comp) in &module.components {
                generate_component(&mut w, comp_name, comp, self.ts_mode);
                w.emit("");
            }

            let out_path = format!("{}{}", module_name, ext);
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

fn generate_component(w: &mut CodeWriter, name: &str, comp: &ComponentIR, ts_mode: bool) {
    let fmt = JavaScriptFormatExpr { ts_mode };
    w.emit(&format!("class {} {{", name));
    w.indent();

    // Constructor with state initialization
    if !comp.states.is_empty() || !comp.props.is_empty() {
        // TS: emit field declarations before constructor
        if fmt.ts_mode() {
            for (prop_name, prop_type) in &comp.props {
                let ann = js_type(&prop_type.kind, &prop_type.type_args, true);
                w.emit(&format!("{}: {};", prop_name, ann));
            }
            for state in &comp.states {
                let ann = js_type(&state.r#type.kind, &state.r#type.type_args, true);
                w.emit(&format!("{}: {};", state.name, ann));
            }
        }

        w.emit("constructor() {");
        w.indent();
        for prop in comp.props.keys() {
            w.emit(&format!("this.{} = null;  // prop", prop));
        }
        for state in &comp.states {
            let js_init = fmt.format_expr(&state.initial_value);
            w.emit(&format!("this.{} = {};", state.name, js_init));
        }
        w.dedent();
        w.emit("}");
    }

    // Computed properties as getters
    for computed in &comp.computeds {
        let body = fmt.format_expr(&computed.body);
        let ret_ann = if fmt.ts_mode() {
            let ann = js_type(&computed.r#type.kind, &computed.r#type.type_args, true);
            if ann.is_empty() {
                String::new()
            } else {
                format!(": {}", ann)
            }
        } else {
            String::new()
        };
        w.emit(&format!("get {}(){}{{", computed.name, ret_ann));
        w.indent();
        w.emit(&format!("return {};", body));
        w.dedent();
        w.emit("}");
    }

    // Functions (methods)
    for func in &comp.fns {
        generate_function(w, func, ts_mode);
    }

    w.dedent();
    w.emit("}");
}

// ═══════════════════════════════════════════════════════════
// Store generation
// ═══════════════════════════════════════════════════════════

fn generate_store(w: &mut CodeWriter, name: &str, store: &StoreIR, ts_mode: bool) {
    let fmt = JavaScriptFormatExpr { ts_mode };
    w.emit(&format!("class {} {{", name));
    w.indent();

    // TS: field declarations
    if fmt.ts_mode() {
        for (signal_name, signal_type) in &store.signals {
            let ann = js_type(&signal_type.kind, &signal_type.type_args, true);
            w.emit(&format!("{}: {};", signal_name, ann));
        }
        for getter_name in store.getters.keys() {
            w.emit(&format!("get {}(): any;", getter_name));
        }
    }

    // Constructor with signal defaults
    w.emit("constructor() {");
    w.indent();
    for signal_name in store.signals.keys() {
        let default = store
            .signal_defaults
            .get(signal_name)
            .map(|v| format_json_value(v, &JSON_JS))
            .unwrap_or_else(|| "null".into());
        w.emit(&format!("this.{} = {};", signal_name, default));
    }
    w.dedent();
    w.emit("}");

    // Getters as get accessors
    for (getter_name, getter_expr) in &store.getters {
        w.emit("");
        let ret_ann = if fmt.ts_mode() {
            ": any".into()
        } else {
            String::new()
        };
        w.emit(&format!("get {}(){}{{", getter_name, ret_ann));
        w.indent();
        w.emit(&format!("return {};", fmt.format_expr(getter_expr)));
        w.dedent();
        w.emit("}");
    }

    w.dedent();
    w.emit("}");
}

// ═══════════════════════════════════════════════════════════
// Function generation
// ═══════════════════════════════════════════════════════════

fn generate_function(w: &mut CodeWriter, func: &FunctionIR, ts_mode: bool) {
    // Build parameter list — no `self` equivalent in JS classes
    let mut param_strs = Vec::new();
    for p in &func.params {
        let safe_name = super::sanitize_js_ident(&p.name);
        let mut s = safe_name;
        if ts_mode {
            if let Some(ref ty) = p.r#type {
                let ann = js_type(&ty.kind, &ty.type_args, true);
                if !ann.is_empty() {
                    s.push_str(&format!(": {}", ann));
                }
            }
        }
        param_strs.push(s);
    }

    // Determine return type annotation (TS only)
    let ret_ann = if ts_mode {
        func.return_type
            .as_ref()
            .map(|rt| {
                let ann = js_type(&rt.kind, &rt.type_args, true);
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
    let async_prefix = if func.is_async { "async " } else { "" };
    w.emit(&format!(
        "{}{}({}){} {{",
        async_prefix, func.name, all_params, ret_ann
    ));
    w.indent();

    if func.body.statements.is_empty() {
        w.emit(";");
    } else {
        for stmt in &func.body.statements {
            generate_statement(w, stmt, ts_mode);
        }
    }

    w.dedent();
    w.emit("}");
}

// ═══════════════════════════════════════════════════════════
// Statement generation
// ═══════════════════════════════════════════════════════════

pub(super) fn generate_statement(w: &mut CodeWriter, stmt: &StatementIR, ts_mode: bool) {
    let fmt = JavaScriptFormatExpr { ts_mode };
    match stmt {
        StatementIR::LetStmtIR {
            name,
            value,
            r#type,
        } => {
            let safe_name = fmt.format_identifier(name);
            let rhs = value
                .as_ref()
                .map(|v| fmt.format_expr(v))
                .unwrap_or_else(|| "null".into());
            let kw = "let";
            let ts_mode = fmt.ts_mode();
            let type_hint = if ts_mode {
                r#type
                    .as_ref()
                    .map(|t| {
                        let ann = js_type(&t.kind, &t.type_args, true);
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
            w.emit(&format!("{} {}{} = {};", kw, safe_name, type_hint, rhs));
        }
        StatementIR::IfStmtIR {
            condition,
            then_body,
            else_ifs,
            else_body,
        } => {
            let cond = fmt.format_expr(condition);
            w.emit(&format!("if ({}) {{", cond));
            w.indent();
            for s in &then_body.statements {
                generate_statement(w, s, ts_mode);
            }
            w.dedent();
            for (elif_cond, elif_body) in else_ifs {
                let elif_js = fmt.format_expr(elif_cond);
                w.emit(&format!("}} else if ({}) {{", elif_js));
                w.indent();
                for s in &elif_body.statements {
                    generate_statement(w, s, ts_mode);
                }
                w.dedent();
            }
            if let Some(else_blk) = else_body {
                if !else_blk.statements.is_empty() {
                    w.emit("} else {");
                    w.indent();
                    for s in &else_blk.statements {
                        generate_statement(w, s, ts_mode);
                    }
                    w.dedent();
                }
            }
            w.emit("}");
        }
        StatementIR::ReturnStmtIR { value } => {
            let rhs = value
                .as_ref()
                .map(|v| fmt.format_expr(v))
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
                .map(|v| fmt.format_expr(v))
                .unwrap_or_else(|| "0".into());
            match &mut_intent.operation {
                MutationOp::DirectAssign => {
                    w.emit(&format!("this.{} = {};", target, amount));
                }
                MutationOp::Increment => {
                    w.emit(&format!("this.{} += {};", target, amount));
                }
                MutationOp::Decrement => {
                    w.emit(&format!("this.{} -= {};", target, amount));
                }
                MutationOp::Multiply => {
                    w.emit(&format!("this.{} *= {};", target, amount));
                }
                MutationOp::Divide => {
                    w.emit(&format!("this.{} /= {};", target, amount));
                }
                MutationOp::Modulo => {
                    w.emit(&format!("this.{} %= {};", target, amount));
                }
                MutationOp::Push => {
                    let item = mut_intent
                        .item
                        .as_ref()
                        .map(|v| fmt.format_expr(v))
                        .unwrap_or_else(|| "undefined".into());
                    w.emit(&format!("this.{}.push({});", target, item));
                }
                MutationOp::Pop => {
                    w.emit(&format!("this.{}.pop();", target));
                }
            }
        }
        StatementIR::ExprStmtIR { expr } => {
            let line = fmt.format_expr(expr);
            // Skip null expressions
            if line != "null" {
                w.emit(&format!("{};", line));
            }
        }
        StatementIR::EventEmitStmtIR { event_name, args } => {
            let arg_str = if args.is_empty() {
                String::new()
            } else {
                let formatted: Vec<String> = args.iter().map(|a| fmt.format_expr(a)).collect();
                format!("({})", formatted.join(", "))
            };
            w.emit(&format!("// emit event: {}{}", event_name, arg_str));
        }
        StatementIR::WhileStmtIR { condition, body } => {
            let cond = fmt.format_expr(condition);
            w.emit(&format!("while ({}) {{", cond));
            w.indent();
            for s in &body.statements {
                generate_statement(w, s, ts_mode);
            }
            w.dedent();
            w.emit("}");
        }
        StatementIR::ForInStmtIR {
            var_name,
            iterable,
            body,
        } => {
            let iter = fmt.format_expr(iterable);
            w.emit(&format!("for (const {} of {}) {{", var_name, iter));
            w.indent();
            for s in &body.statements {
                generate_statement(w, s, ts_mode);
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
