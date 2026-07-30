//! Rust codegen: IR -> Rust.
//!
//! Generates Rust source files from Intent IR. Each component becomes
//! a struct with an impl block, each function becomes a method, and
//! stores become standalone structs with impl blocks.

mod expr;
#[cfg(test)]
mod tests;
mod visitor;

use super::{CodeWriter, Codegen, CompileOutput, CompiledFile};
use crate::codegen::shared::expr::{FormatExpr, RustFormatExpr};
use crate::codegen::shared::stmt::FormatStmt;
use crate::codegen::shared::visitor::CodegenVisitor;
use crate::codegen::{format_json_value, JSON_RUST};
use crate::ir::*;
use rustc_hash::FxHashMap;
use std::fmt::Write;

pub(crate) use expr::format_expr_ir;
pub(crate) use expr::rs_type;

pub struct RustCodegen;

impl Codegen for RustCodegen {
    fn generate(&self, ir: &IRProgram, _source_files: &FxHashMap<String, String>) -> CompileOutput {
        let mut files = Vec::new();

        for (module_key, module) in &ir.modules {
            let module_name = super::sanitize_module_name(module_key);
            let source_path = format!("{}.omni", module_name);

            let mut w = CodeWriter::new();
            w.emit_fmt(format_args!("// Generated from {}", source_path));
            w.emit("");

            let mut visitor = visitor::RustVisitor;
            visitor.generate_module(&mut w, module);

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
    let has_fields = !comp.signals.is_empty() || !comp.props.is_empty();
    let has_methods = has_fields || !comp.fns.is_empty() || !comp.computeds.is_empty();

    // Emit struct definition
    if has_fields {
        w.emit_fmt(format_args!("struct {} {{", name));
        w.indent();
        for (prop_name, prop_type) in &comp.props {
            w.emit_fmt(format_args!(
                "{}: Option<{}>,",
                prop_name,
                rs_type(prop_type.kind.as_str(), &prop_type.type_args)
            ));
        }
        for state in &comp.signals {
            w.emit_fmt(format_args!(
                "{}: {},",
                state.name,
                rs_type(state.r#type.kind.as_str(), &state.r#type.type_args)
            ));
        }
        w.dedent();
        w.emit("}");
    } else {
        w.emit_fmt(format_args!("struct {};", name));
    }

    if !has_methods {
        return;
    }

    // Emit impl block
    w.emit_fmt(format_args!("impl {} {{", name));
    w.indent();

    // Constructor (new) if there are fields
    if has_fields {
        w.emit("fn new() -> Self {");
        w.indent();
        if comp.signals.is_empty() && comp.props.is_empty() {
            w.emit(name);
        } else {
            w.emit("Self {");
            w.indent();
            for prop_name in comp.props.keys() {
                w.emit_fmt(format_args!("{}: None,  // prop", prop_name));
            }
            for state in &comp.signals {
                let rs_init = format_expr_ir(&state.initial_value);
                w.emit_fmt(format_args!("{}: {},", state.name, rs_init));
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
        let ret_type = rs_type(computed.r#type.kind.as_str(), &computed.r#type.type_args);
        w.emit_fmt(format_args!(
            "fn {}(&self) -> {} {{",
            computed.name, ret_type
        ));
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
        w.emit_fmt(format_args!("struct {};", name));
        w.emit("");
        w.emit_fmt(format_args!("impl {} {{", name));
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
    w.emit_fmt(format_args!("struct {} {{", name));
    w.indent();
    for (signal_name, signal_type) in &store.signals {
        w.emit_fmt(format_args!(
            "{}: {},",
            signal_name,
            rs_type(signal_type.kind.as_str(), &signal_type.type_args)
        ));
    }
    w.dedent();
    w.emit("}");

    // Impl block
    w.emit_fmt(format_args!("impl {} {{", name));
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
        w.emit_fmt(format_args!("{}: {},", signal_name, default));
    }
    w.dedent();
    w.emit("}");
    w.dedent();
    w.emit("}");

    // Getters as methods (no return type — TODO: infer from expression)
    for (getter_name, getter_expr) in &store.getters {
        w.emit("");
        let body = format_expr_ir(getter_expr);
        w.emit_fmt(format_args!("fn {}(&self) {{", getter_name));
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
            let ann = rs_type(ty.kind.as_str(), &ty.type_args);
            let _ = write!(s, ": {}", ann);
        }
        param_strs.push(s);
    }

    // Determine return type annotation
    let ret_ann = func
        .return_type
        .as_ref()
        .map(|rt| format!(" -> {}", rs_type(rt.kind.as_str(), &rt.type_args)))
        .unwrap_or_default();

    let all_params = param_strs.join(", ");

    let async_kw = if func.is_async { "async " } else { "" };
    w.emit_fmt(format_args!(
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

/// Rust statement formatter.
struct RustStmtFormatter {
    expr: RustFormatExpr,
}

impl FormatStmt for RustStmtFormatter {
    type ExprFormatter = RustFormatExpr;

    fn expr_fmt(&self) -> &Self::ExprFormatter {
        &self.expr
    }

    fn null_value(&self) -> &str {
        "None"
    }

    fn return_null_value(&self) -> &str {
        "()"
    }

    fn for_in_keyword(&self) -> &str {
        "" // Rust uses `for x in y` (no keyword before variable)
    }

    fn for_in_uses_of(&self) -> bool {
        false // Rust uses `in`
    }

    fn format_let_type_ann(&self, ty: &Option<TypeIR>) -> String {
        ty.as_ref()
            .map(|t| format!(": {}", rs_type(t.kind.as_str(), &t.type_args)))
            .unwrap_or_default()
    }

    // Rust uses `if cond {` without parentheses
    fn format_if_stmt(
        &self,
        w: &mut CodeWriter,
        condition: &ExprIR,
        then_body: &BlockIR,
        else_ifs: &[(ExprIR, BlockIR)],
        else_body: &Option<BlockIR>,
    ) {
        let cond = self.expr.format_expr(condition);
        w.emit_fmt(format_args!("if {cond} {{"));
        w.indent();
        self.format_block(w, then_body);
        w.dedent();
        for (elif_cond, elif_body) in else_ifs {
            let elif = self.expr.format_expr(elif_cond);
            w.emit_fmt(format_args!("}} else if {elif} {{"));
            w.indent();
            self.format_block(w, elif_body);
            w.dedent();
        }
        if let Some(ref eb) = else_body {
            if !eb.statements.is_empty() {
                w.emit("} else {");
                w.indent();
                self.format_block(w, eb);
                w.dedent();
            }
        }
        w.emit("}");
    }

    // Rust uses `while cond {` without parentheses
    fn format_while_stmt(&self, w: &mut CodeWriter, condition: &ExprIR, body: &BlockIR) {
        let cond = self.expr.format_expr(condition);
        w.emit_fmt(format_args!("while {cond} {{"));
        w.indent();
        self.format_block(w, body);
        w.dedent();
        w.emit("}");
    }

    fn mutation_target_expr(&self, target: &str) -> String {
        format!("self.{}", target)
    }
}

fn generate_statement(w: &mut CodeWriter, stmt: &StatementIR) {
    let fmt = RustStmtFormatter {
        expr: RustFormatExpr {
            move_closures: false,
            event_param_name: None,
        },
    };
    fmt.format_stmt(w, stmt);
}
