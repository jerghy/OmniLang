//! Statement and function generation for Python codegen.

use super::expr::{self, PythonFormatExpr};
use crate::codegen::shared::expr::FormatExpr;
use crate::codegen::shared::stmt::FormatStmt;
use crate::codegen::CodeWriter;
use crate::ir::*;

/// Python statement formatter — no semicolons, `elif` keyword, colon blocks.
struct PythonStmtFormatter {
    expr: PythonFormatExpr,
}

impl FormatStmt for PythonStmtFormatter {
    type ExprFormatter = PythonFormatExpr;

    fn expr_fmt(&self) -> &Self::ExprFormatter {
        &self.expr
    }

    fn needs_semicolon(&self) -> bool {
        false
    }

    fn null_value(&self) -> &str {
        "None"
    }

    fn for_in_keyword(&self) -> &str {
        "" // Python uses `for x in y:`
    }

    fn for_in_uses_of(&self) -> bool {
        false // Python uses `in`
    }

    fn format_let_stmt(
        &self,
        w: &mut CodeWriter,
        name: &str,
        value: &Option<ExprIR>,
        type_ann: &Option<TypeIR>,
    ) {
        let rhs = value
            .as_ref()
            .map(|v| self.expr.format_expr(v))
            .unwrap_or_else(|| "None".to_string());
        let type_hint = type_ann
            .as_ref()
            .map(|t| format!(": {}", expr::format_type_ir(t)))
            .unwrap_or_default();
        w.emit_fmt(format_args!("{name}{type_hint} = {rhs}"));
    }

    fn format_if_stmt(
        &self,
        w: &mut CodeWriter,
        condition: &ExprIR,
        then_body: &BlockIR,
        else_ifs: &[(ExprIR, BlockIR)],
        else_body: &Option<BlockIR>,
    ) {
        let cond = self.expr.format_expr(condition);
        w.emit_fmt(format_args!("if {cond}:"));
        w.indent();
        self.format_block(w, then_body);
        w.dedent();
        for (elif_cond, elif_body) in else_ifs {
            let elif = self.expr.format_expr(elif_cond);
            w.emit_fmt(format_args!("elif {elif}:"));
            w.indent();
            self.format_block(w, elif_body);
            w.dedent();
        }
        if let Some(ref eb) = else_body {
            if !eb.statements.is_empty() {
                w.emit("else:");
                w.indent();
                self.format_block(w, eb);
                w.dedent();
            }
        }
    }

    fn format_while_stmt(&self, w: &mut CodeWriter, condition: &ExprIR, body: &BlockIR) {
        let cond = self.expr.format_expr(condition);
        w.emit_fmt(format_args!("while {cond}:"));
        w.indent();
        self.format_block(w, body);
        w.dedent();
    }

    fn format_for_in_stmt(
        &self,
        w: &mut CodeWriter,
        var_name: &str,
        iterable: &ExprIR,
        body: &BlockIR,
    ) {
        let iter = self.expr.format_expr(iterable);
        w.emit_fmt(format_args!("for {var_name} in {iter}:"));
        w.indent();
        self.format_block(w, body);
        w.dedent();
    }

    fn mutation_target_expr(&self, target: &str) -> String {
        target.to_string()
    }

    fn mutation_push_method(&self) -> &'static str {
        "append"
    }
}

/// Generate a function (standalone or method).
pub(super) fn generate_function(w: &mut CodeWriter, func: &FunctionIR, is_standalone: bool) {
    // Build parameter list
    let mut param_strs = Vec::new();
    if !is_standalone {
        param_strs.push("self".to_string());
    }
    for p in &func.params {
        let mut s = p.name.clone();
        // Optionally include type annotation
        if let Some(ref ty) = p.r#type {
            s.push_str(&format!(": {}", expr::format_type_ir(ty)));
        }
        param_strs.push(s);
    }

    // Determine return type annotation
    let ret_ann = func
        .return_type
        .as_ref()
        .map(|rt| format!(" -> {}", expr::format_type_ir(rt)))
        .unwrap_or_default();

    let all_params = param_strs.join(", ");

    w.emit_fmt(format_args!(
        "def {}({}){}:",
        func.name, all_params, ret_ann
    ));
    w.indent();

    if func.body.statements.is_empty() {
        w.emit("pass");
    } else {
        let fmt = PythonStmtFormatter {
            expr: PythonFormatExpr,
        };
        for stmt in &func.body.statements {
            fmt.format_stmt(w, stmt);
        }
    }

    w.dedent();
}

/// Generate a component class.
pub(super) fn generate_component(w: &mut CodeWriter, name: &str, comp: &ComponentIR) {
    w.emit_fmt(format_args!("class {}:", name));
    w.indent();

    // Constructor with signals as instance variables
    if !comp.states.is_empty() || !comp.props.is_empty() {
        w.emit("def __init__(self):");
        w.indent();
        let py_fmt = PythonFormatExpr;
        for prop in comp.props.keys() {
            w.emit_fmt(format_args!("self.{} = None  # prop", prop));
        }
        for state in &comp.states {
            let py_init = py_fmt.format_expr(&state.initial_value);
            w.emit_fmt(format_args!("self.{} = {}", state.name, py_init));
        }
        w.dedent();
        w.emit("");
    }

    // Computed properties as @property methods
    for computed in &comp.computeds {
        let py_fmt = PythonFormatExpr;
        let body = py_fmt.format_expr(&computed.body);
        // Use @cached_property or simple @property for now
        w.emit("@property");
        w.emit_fmt(format_args!("def {}(self):", computed.name));
        w.indent();
        w.emit_fmt(format_args!("return {}", body));
        w.dedent();
        w.emit("");
    }

    // Functions (methods)
    for func in &comp.fns {
        generate_function(w, func, false);
        w.emit("");
    }

    w.dedent();
}

/// Generate a store class.
pub(super) fn generate_store(w: &mut CodeWriter, name: &str, store: &StoreIR) {
    w.emit_fmt(format_args!("class {}:", name));
    w.indent();

    // Constructor with signal defaults
    w.emit("def __init__(self):");
    w.indent();
    for signal_name in store.signals.keys() {
        let default = store
            .signal_defaults
            .get(signal_name)
            .map(|v| crate::codegen::format_json_value(v, &crate::codegen::JSON_PYTHON))
            .unwrap_or_else(|| "None".into());
        w.emit_fmt(format_args!("self.{} = {}", signal_name, default));
    }
    w.dedent();

    // Getters as properties
    let py_fmt = PythonFormatExpr;
    for (getter_name, getter_expr) in &store.getters {
        w.emit("");
        w.emit("@property");
        w.emit_fmt(format_args!("def {}(self):", getter_name));
        w.indent();
        w.emit_fmt(format_args!("return {}", py_fmt.format_expr(getter_expr)));
        w.dedent();
    }

    w.dedent();
}

/// Generate a statement.
#[allow(dead_code)]
pub(super) fn generate_statement(w: &mut CodeWriter, stmt: &StatementIR) {
    let fmt = PythonStmtFormatter {
        expr: PythonFormatExpr,
    };
    fmt.format_stmt(w, stmt);
}
