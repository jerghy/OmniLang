//! Statement and function generation for Flutter codegen.

use crate::codegen::CodeWriter;
use crate::ir::*;

use super::expr::format_expr_ir_flutter;

// ═══════════════════════════════════════════════════════════════
// Statement generation (Flutter/Dart)
// ═══════════════════════════════════════════════════════════════

pub(crate) fn generate_statement_flutter(w: &mut CodeWriter, stmt: &StatementIR) {
    match stmt {
        StatementIR::LetStmtIR {
            name,
            value,
            r#type,
        } => {
            let rhs = value
                .as_ref()
                .map(format_expr_ir_flutter)
                .unwrap_or_else(|| "null".into());
            if let Some(ref ty) = r#type {
                let ann = super::super::dart::dart_type(&ty.kind, &ty.type_args);
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
            let cond = format_expr_ir_flutter(condition);
            w.emit(&format!("if ({}) {{", cond));
            w.indent();
            for s in &then_body.statements {
                generate_statement_flutter(w, s);
            }
            w.dedent();
            for (elif_cond, elif_body) in else_ifs {
                let elif_js = format_expr_ir_flutter(elif_cond);
                w.emit(&format!("}} else if ({}) {{", elif_js));
                w.indent();
                for s in &elif_body.statements {
                    generate_statement_flutter(w, s);
                }
                w.dedent();
            }
            if let Some(else_blk) = else_body {
                if !else_blk.statements.is_empty() {
                    w.emit("} else {");
                    w.indent();
                    for s in &else_blk.statements {
                        generate_statement_flutter(w, s);
                    }
                    w.dedent();
                }
            }
            w.emit("}");
        }
        StatementIR::ReturnStmtIR { value } => {
            let rhs = value
                .as_ref()
                .map(format_expr_ir_flutter)
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
                .map(format_expr_ir_flutter)
                .unwrap_or_else(|| "0".into());
            w.emit("setState(() {");
            w.indent();
            match mut_intent.operation.as_str() {
                "direct_assign" => {
                    w.emit(&format!("{} = {};", target, amount));
                }
                "increment" => {
                    w.emit(&format!("{} += {};", target, amount));
                }
                "decrement" => {
                    w.emit(&format!("{} -= {};", target, amount));
                }
                "multiply" => {
                    w.emit(&format!("{} *= {};", target, amount));
                }
                "divide" => {
                    w.emit(&format!("{} /= {};", target, amount));
                }
                "modulo" => {
                    w.emit(&format!("{} %= {};", target, amount));
                }
                other => {
                    w.emit(&format!(
                        "// TODO: mutation operation '{}' on '{}'",
                        other, target
                    ));
                }
            }
            w.dedent();
            w.emit("});");
        }
        StatementIR::ExprStmtIR { expr } => {
            let line = format_expr_ir_flutter(expr);
            if line != "null" {
                w.emit(&format!("{};", line));
            }
        }
        StatementIR::EventEmitStmtIR { event_name, args } => {
            let arg_str = if args.is_empty() {
                String::new()
            } else {
                let formatted: Vec<String> = args.iter().map(format_expr_ir_flutter).collect();
                format!("({})", formatted.join(", "))
            };
            w.emit(&format!("// emit event: {}{}", event_name, arg_str));
        }
        StatementIR::WhileStmtIR { condition, body } => {
            let cond = format_expr_ir_flutter(condition);
            w.emit(&format!("while ({}) {{", cond));
            w.indent();
            for s in &body.statements {
                generate_statement_flutter(w, s);
            }
            w.dedent();
            w.emit("}");
        }
        StatementIR::ForInStmtIR {
            var_name,
            iterable,
            body,
        } => {
            let iter = format_expr_ir_flutter(iterable);
            w.emit(&format!("for (var {} in {}) {{", var_name, iter));
            w.indent();
            for s in &body.statements {
                generate_statement_flutter(w, s);
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
        StatementIR::ThrowStmtIR { expr } => {
            let e = format_expr_ir_flutter(expr);
            w.emit(&format!("throw {};", e));
        }
        StatementIR::TryCatchStmtIR {
            try_body,
            catch_var,
            catch_body,
            finally_body,
        } => {
            w.emit("try {");
            w.indent();
            for s in &try_body.statements {
                generate_statement_flutter(w, s);
            }
            w.dedent();
            if let Some(ref cv) = catch_var {
                w.emit(&format!("}} catch ({}) {{", cv));
            } else if catch_body.is_some() {
                w.emit("} catch (_) {");
            }
            if let Some(ref cb) = catch_body {
                w.indent();
                for s in &cb.statements {
                    generate_statement_flutter(w, s);
                }
                w.dedent();
            }
            if let Some(ref fb) = finally_body {
                w.emit("} finally {");
                w.indent();
                for s in &fb.statements {
                    generate_statement_flutter(w, s);
                }
                w.dedent();
            }
            w.emit("}");
        }
        StatementIR::BlockStmtIR { body } => {
            for s in &body.statements {
                generate_statement_flutter(w, s);
            }
        }
        _ => {}
    }
}

/// Generate a function as a method on the State class.
pub(crate) fn generate_function_flutter(w: &mut CodeWriter, func: &FunctionIR) {
    // Build parameter list — Dart uses `Type name` for positional params
    let mut param_strs = Vec::new();
    for p in &func.params {
        let mut s = String::new();
        if let Some(ref ty) = p.r#type {
            let ann = super::super::dart::dart_type(&ty.kind, &ty.type_args);
            s.push_str(&format!("{} ", ann));
        }
        s.push_str(&p.name);
        param_strs.push(s);
    }

    // Determine return type
    // Wrap in Future<T> for async functions
    let return_prefix = if func.is_async {
        match func.return_type {
            Some(ref rt) if rt.kind != "void" && rt.kind != "()" => {
                format!(
                    "Future<{}> ",
                    super::super::dart::dart_type(&rt.kind, &rt.type_args)
                )
            }
            _ => "Future<void> ".to_string(),
        }
    } else {
        func.return_type
            .as_ref()
            .filter(|rt| rt.kind != "void" && rt.kind != "()")
            .map(|rt| {
                format!(
                    "{} ",
                    super::super::dart::dart_type(&rt.kind, &rt.type_args)
                )
            })
            .unwrap_or_default()
    };

    let async_kw = if func.is_async { "async " } else { "" };
    let all_params = param_strs.join(", ");
    w.emit(&format!(
        "{}{}({}) {}{{",
        return_prefix, func.name, all_params, async_kw
    ));
    w.indent();

    for stmt in &func.body.statements {
        generate_statement_flutter(w, stmt);
    }

    w.dedent();
    w.emit("}");
}
