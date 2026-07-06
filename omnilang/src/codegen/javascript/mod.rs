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
mod visitor;

use std::collections::HashMap;

use super::{CodeWriter, Codegen, CompileOutput, CompiledFile};
use crate::codegen::shared::expr::FormatExpr;
use crate::codegen::shared::stmt::FormatStmt;
use crate::codegen::shared::types::{FormatType, JsTypeFormatter};
use crate::codegen::shared::visitor::CodegenVisitor;
use crate::codegen::{format_json_value, JSON_JS};
use crate::ir::*;

use expr::*;
pub use utils::*;

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
            w.emit_fmt(format_args!("// Generated from {}", source_path));
            w.emit("");

            let mut visitor = visitor::JavaScriptVisitor {
                ts_mode: self.ts_mode,
            };
            visitor.generate_module(&mut w, module);

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
    w.emit_fmt(format_args!("class {} {{", name));
    w.indent();

    // Constructor with state initialization
    if !comp.states.is_empty() || !comp.props.is_empty() {
        // TS: emit field declarations before constructor
        if fmt.ts_mode() {
            for (prop_name, prop_type) in &comp.props {
                let ann = JsTypeFormatter.format_type(&prop_type.kind, &prop_type.type_args);
                w.emit_fmt(format_args!("{}: {};", prop_name, ann));
            }
            for state in &comp.states {
                let ann = JsTypeFormatter.format_type(&state.r#type.kind, &state.r#type.type_args);
                w.emit_fmt(format_args!("{}: {};", state.name, ann));
            }
        }

        w.emit("constructor() {");
        w.indent();
        for prop in comp.props.keys() {
            w.emit_fmt(format_args!("this.{} = null;  // prop", prop));
        }
        for state in &comp.states {
            let js_init = fmt.format_expr(&state.initial_value);
            w.emit_fmt(format_args!("this.{} = {};", state.name, js_init));
        }
        w.dedent();
        w.emit("}");
    }

    // Computed properties as getters
    for computed in &comp.computeds {
        let body = fmt.format_expr(&computed.body);
        let ret_ann = if fmt.ts_mode() {
            let ann =
                JsTypeFormatter.format_type(&computed.r#type.kind, &computed.r#type.type_args);
            if ann.is_empty() {
                String::new()
            } else {
                format!(": {}", ann)
            }
        } else {
            String::new()
        };
        w.emit_fmt(format_args!("get {}(){}{{", computed.name, ret_ann));
        w.indent();
        w.emit_fmt(format_args!("return {};", body));
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
    w.emit_fmt(format_args!("class {} {{", name));
    w.indent();

    // TS: field declarations
    if fmt.ts_mode() {
        for (signal_name, signal_type) in &store.signals {
            let ann = JsTypeFormatter.format_type(&signal_type.kind, &signal_type.type_args);
            w.emit_fmt(format_args!("{}: {};", signal_name, ann));
        }
        for getter_name in store.getters.keys() {
            w.emit_fmt(format_args!("get {}(): any;", getter_name));
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
        w.emit_fmt(format_args!("this.{} = {};", signal_name, default));
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
        w.emit_fmt(format_args!("get {}(){}{{", getter_name, ret_ann));
        w.indent();
        w.emit_fmt(format_args!("return {};", fmt.format_expr(getter_expr)));
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
                let ann = JsTypeFormatter.format_type(&ty.kind, &ty.type_args);
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
                let ann = JsTypeFormatter.format_type(&rt.kind, &rt.type_args);
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
    w.emit_fmt(format_args!(
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

/// JavaScript/TypeScript statement formatter.
pub(super) struct JsStmtFormatter {
    pub expr: JavaScriptFormatExpr,
}

impl FormatStmt for JsStmtFormatter {
    type ExprFormatter = JavaScriptFormatExpr;

    fn expr_fmt(&self) -> &Self::ExprFormatter {
        &self.expr
    }

    fn null_value(&self) -> &str {
        "null"
    }

    fn for_in_keyword(&self) -> &str {
        "const"
    }

    fn format_let_type_ann(&self, ty: &Option<TypeIR>) -> String {
        if self.expr.ts_mode() {
            ty.as_ref()
                .map(|t| {
                    let ann = JsTypeFormatter.format_type(&t.kind, &t.type_args);
                    if ann.is_empty() {
                        String::new()
                    } else {
                        format!(": {}", ann)
                    }
                })
                .unwrap_or_default()
        } else {
            String::new()
        }
    }

    fn format_let_stmt(
        &self,
        w: &mut CodeWriter,
        name: &str,
        value: &Option<ExprIR>,
        type_ann: &Option<TypeIR>,
    ) {
        let safe_name = self.expr.format_identifier(name);
        let kw = if self.use_const_for_immutable() {
            "const"
        } else {
            self.let_keyword()
        };
        let ty = self.format_let_type_ann(type_ann);
        let rhs = value
            .as_ref()
            .map(|v| self.expr.format_expr(v))
            .unwrap_or_else(|| self.null_value().to_string());
        w.emit_fmt(format_args!("{kw} {safe_name}{ty} = {rhs};"));
    }

    fn format_mutation_stmt(&self, w: &mut CodeWriter, intent: &MutationIntent) {
        let target = &intent.target.name;
        let amount = intent
            .amount
            .as_ref()
            .map(|v| self.expr.format_expr(v))
            .unwrap_or_else(|| "0".into());
        match &intent.operation {
            MutationOp::DirectAssign => {
                w.emit_fmt(format_args!("this.{} = {};", target, amount));
            }
            MutationOp::Increment => {
                w.emit_fmt(format_args!("this.{} += {};", target, amount));
            }
            MutationOp::Decrement => {
                w.emit_fmt(format_args!("this.{} -= {};", target, amount));
            }
            MutationOp::Multiply => {
                w.emit_fmt(format_args!("this.{} *= {};", target, amount));
            }
            MutationOp::Divide => {
                w.emit_fmt(format_args!("this.{} /= {};", target, amount));
            }
            MutationOp::Modulo => {
                w.emit_fmt(format_args!("this.{} %= {};", target, amount));
            }
            MutationOp::Push => {
                let item = intent
                    .item
                    .as_ref()
                    .map(|v| self.expr.format_expr(v))
                    .unwrap_or_else(|| "undefined".into());
                w.emit_fmt(format_args!("this.{}.push({});", target, item));
            }
            MutationOp::Pop => {
                w.emit_fmt(format_args!("this.{}.pop();", target));
            }
        }
    }
}

pub(super) fn generate_statement(w: &mut CodeWriter, stmt: &StatementIR, ts_mode: bool) {
    let fmt = JsStmtFormatter {
        expr: JavaScriptFormatExpr { ts_mode },
    };
    fmt.format_stmt(w, stmt);
}
