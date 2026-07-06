//! Statement and function generation for Python codegen.

use super::expr;
use crate::codegen::{format_block_as_expr, CodeWriter};
use crate::ir::*;

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

    w.emit(&format!("def {}({}){}:", func.name, all_params, ret_ann));
    w.indent();

    if func.body.statements.is_empty() {
        w.emit("pass");
    } else {
        for stmt in &func.body.statements {
            generate_statement(w, stmt);
        }
    }

    w.dedent();
}

/// Generate a component class.
pub(super) fn generate_component(w: &mut CodeWriter, name: &str, comp: &ComponentIR) {
    w.emit(&format!("class {}:", name));
    w.indent();

    // Constructor with signals as instance variables
    if !comp.states.is_empty() || !comp.props.is_empty() {
        w.emit("def __init__(self):");
        w.indent();
        for prop in comp.props.keys() {
            w.emit(&format!("self.{} = None  # prop", prop));
        }
        for state in &comp.states {
            let py_init = expr::format_expr_ir(&state.initial_value);
            w.emit(&format!("self.{} = {}", state.name, py_init));
        }
        w.dedent();
        w.emit("");
    }

    // Computed properties as @property methods
    for computed in &comp.computeds {
        let body = expr::format_expr_ir(&computed.body);
        // Use @cached_property or simple @property for now
        w.emit("@property");
        w.emit(&format!("def {}(self):", computed.name));
        w.indent();
        w.emit(&format!("return {}", body));
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
    w.emit(&format!("class {}:", name));
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
        w.emit(&format!("self.{} = {}", signal_name, default));
    }
    w.dedent();

    // Getters as properties
    for (getter_name, getter_expr) in &store.getters {
        w.emit("");
        w.emit("@property");
        w.emit(&format!("def {}(self):", getter_name));
        w.indent();
        w.emit(&format!("return {}", expr::format_expr_ir(getter_expr)));
        w.dedent();
    }

    w.dedent();
}

/// Generate a statement.
pub(super) fn generate_statement(w: &mut CodeWriter, stmt: &StatementIR) {
    match stmt {
        StatementIR::LetStmtIR {
            name,
            value,
            r#type,
        } => {
            let rhs = value
                .as_ref()
                .map(expr::format_expr_ir)
                .unwrap_or_else(|| "None".into());
            let type_hint = r#type
                .as_ref()
                .map(|t| format!(": {}", expr::format_type_ir(t)))
                .unwrap_or_default();
            w.emit(&format!("{}{} = {}", name, type_hint, rhs));
        }
        StatementIR::IfStmtIR {
            condition,
            then_body,
            else_ifs,
            else_body,
        } => {
            let cond = expr::format_expr_ir(condition);
            w.emit(&format!("if {}:", cond));
            w.indent();
            for s in &then_body.statements {
                generate_statement(w, s);
            }
            w.dedent();
            for (elif_cond, elif_body) in else_ifs {
                let elif_cond_str = expr::format_expr_ir(elif_cond);
                w.emit(&format!("elif {}:", elif_cond_str));
                w.indent();
                for s in &elif_body.statements {
                    generate_statement(w, s);
                }
                w.dedent();
            }
            if let Some(else_blk) = else_body {
                if !else_blk.statements.is_empty() {
                    w.emit("else:");
                    w.indent();
                    for s in &else_blk.statements {
                        generate_statement(w, s);
                    }
                    w.dedent();
                }
            }
        }
        StatementIR::ReturnStmtIR { value } => {
            let rhs = value
                .as_ref()
                .map(expr::format_expr_ir)
                .unwrap_or_else(|| "None".into());
            w.emit(&format!("return {}", rhs));
        }
        StatementIR::MutationStmt {
            intent: Some(mut_intent),
        } => {
            let target = &mut_intent.target.name;
            let amount = mut_intent
                .amount
                .as_ref()
                .map(expr::format_expr_ir)
                .unwrap_or_else(|| "0".into());
            match &mut_intent.operation {
                MutationOp::DirectAssign => {
                    w.emit(&format!("{} = {}", target, amount));
                }
                MutationOp::Increment => {
                    w.emit(&format!("{} += {}", target, amount));
                }
                MutationOp::Decrement => {
                    w.emit(&format!("{} -= {}", target, amount));
                }
                other => {
                    w.emit(&format!(
                        "# TODO: mutation operation '{}' on {}",
                        other, target
                    ));
                }
            }
        }
        StatementIR::ExprStmtIR { expr: stmt_expr } => {
            let line = expr::format_expr_ir(stmt_expr);
            // Skip null expressions
            if line != "None" {
                w.emit(&line);
            }
        }
        StatementIR::EventEmitStmtIR { event_name, args } => {
            let arg_str = if args.is_empty() {
                String::new()
            } else {
                let formatted: Vec<String> = args.iter().map(expr::format_expr_ir).collect();
                format!("({})", formatted.join(", "))
            };
            w.emit(&format!("# emit event: {}{}", event_name, arg_str));
        }
        StatementIR::BlockStmtIR { body } => {
            for s in &body.statements {
                generate_statement(w, s);
            }
        }
        StatementIR::WhileStmtIR { condition, body } => {
            let cond = expr::format_expr_ir(condition);
            w.emit(&format!("while {}:", cond));
            w.indent();
            for s in &body.statements {
                generate_statement(w, s);
            }
            w.dedent();
        }
        StatementIR::ForInStmtIR {
            var_name,
            iterable,
            body,
        } => {
            let iter = expr::format_expr_ir(iterable);
            w.emit(&format!("for {} in {}:", var_name, iter));
            w.indent();
            for s in &body.statements {
                generate_statement(w, s);
            }
            w.dedent();
        }
        StatementIR::BreakStmtIR => {
            w.emit("break");
        }
        StatementIR::ContinueStmtIR => {
            w.emit("continue");
        }
        _ => {}
    }
}

/// Format a block IR as a Python expression (tail-only).
pub(super) fn format_block_as_python_expr(block: &BlockIR) -> String {
    format_block_as_expr(block, &expr::format_expr_ir, "None")
}
