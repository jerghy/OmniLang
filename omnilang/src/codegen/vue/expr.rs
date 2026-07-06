//! Expression and statement generation for Vue codegen, plus helpers.

use super::CodeWriter;
use crate::codegen::javascript::js_type;
use crate::codegen::shared::expr::FormatExpr;
use crate::codegen::{JsonFormatConfig, JSON_VUE};
use crate::ir::*;

/// Vue expression formatter.
///
/// Uses JS-family syntax with expression-body closures.
pub(super) struct VueFormatExpr;

impl FormatExpr for VueFormatExpr {
    fn json_config(&self) -> &JsonFormatConfig {
        &JSON_VUE
    }

    fn strict_eq(&self) -> bool {
        true
    }

    fn null_fallback(&self) -> &str {
        "undefined"
    }

    fn format_closure(&self, params: &str, body: &str) -> String {
        if params.is_empty() {
            format!("(() => ({body}))")
        } else {
            format!("(({params}) => ({body}))")
        }
    }
}

/// Standalone expression formatter for Vue (tests and template compat).
pub(super) fn format_expr_vue(expr: &ExprIR) -> String {
    VueFormatExpr.format_expr(expr)
}

// ═══════════════════════════════════════════════════════════
// Statement generation (Vue script setup)
// ═══════════════════════════════════════════════════════════

pub(super) fn generate_statement_vue(w: &mut CodeWriter, stmt: &StatementIR) {
    let fmt = VueFormatExpr;
    match stmt {
        StatementIR::LetStmtIR {
            name,
            value,
            r#type,
        } => {
            let rhs = value
                .as_ref()
                .map(|e| fmt.format_expr(e))
                .unwrap_or_else(|| "null".into());
            let type_hint = r#type
                .as_ref()
                .map(|t| {
                    let ann = js_type(&t.kind, &t.type_args, true);
                    if ann.is_empty() {
                        String::new()
                    } else {
                        format!(": {}", ann)
                    }
                })
                .unwrap_or_default();
            w.emit(&format!("const {}{} = {};", name, type_hint, rhs));
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
                generate_statement_vue(w, s);
            }
            w.dedent();
            for (elif_cond, elif_body) in else_ifs {
                let elif_js = fmt.format_expr(elif_cond);
                w.emit(&format!("}} else if ({}) {{", elif_js));
                w.indent();
                for s in &elif_body.statements {
                    generate_statement_vue(w, s);
                }
                w.dedent();
            }
            if let Some(else_blk) = else_body {
                if !else_blk.statements.is_empty() {
                    w.emit("} else {");
                    w.indent();
                    for s in &else_blk.statements {
                        generate_statement_vue(w, s);
                    }
                    w.dedent();
                }
            }
            w.emit("}");
        }
        StatementIR::ReturnStmtIR { value } => {
            let rhs = value
                .as_ref()
                .map(|e| fmt.format_expr(e))
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
                .map(|e| fmt.format_expr(e))
                .unwrap_or_else(|| "0".into());
            match &mut_intent.operation {
                MutationOp::DirectAssign => {
                    w.emit(&format!("{}.value = {};", target, amount));
                }
                MutationOp::Increment => {
                    w.emit(&format!("{}.value += {};", target, amount));
                }
                MutationOp::Decrement => {
                    w.emit(&format!("{}.value -= {};", target, amount));
                }
                MutationOp::Multiply => {
                    w.emit(&format!("{}.value *= {};", target, amount));
                }
                MutationOp::Divide => {
                    w.emit(&format!("{}.value /= {};", target, amount));
                }
                MutationOp::Modulo => {
                    w.emit(&format!("{}.value %= {};", target, amount));
                }
                other => {
                    w.emit(&format!(
                        "// TODO: mutation operation '{}' on {}",
                        other, target
                    ));
                }
            }
        }
        StatementIR::ExprStmtIR { expr } => {
            let line = fmt.format_expr(expr);
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
                generate_statement_vue(w, s);
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
                generate_statement_vue(w, s);
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

// ═══════════════════════════════════════════════════════════
// Helpers
// ═══════════════════════════════════════════════════════════

/// Map OmniView attribute names to Vue HTML attribute names.
/// Vue uses standard HTML attribute names (lowercase, kebab-case).
pub(super) fn to_vue_attr(prop_name: &str) -> String {
    match prop_name {
        "class" | "className" => "class".into(),
        "for" | "htmlFor" => "for".into(),
        "tabindex" => "tabindex".into(),
        "readonly" => "readonly".into(),
        "maxlength" => "maxlength".into(),
        "minlength" => "minlength".into(),
        "autocomplete" => "autocomplete".into(),
        "autofocus" => "autofocus".into(),
        "autoplay" => "autoplay".into(),
        "enctype" => "enctype".into(),
        "formaction" => "formaction".into(),
        "formenctype" => "formenctype".into(),
        "formmethod" => "formmethod".into(),
        "formnovalidate" => "formnovalidate".into(),
        "formtarget" => "formtarget".into(),
        _ => prop_name.to_string(),
    }
}

/// Map OmniView event names to Vue event directive suffixes.
/// Vue uses @click, @change, etc. — same names as OmniView.
pub(super) fn to_vue_event(event_name: &str) -> &str {
    event_name
}
