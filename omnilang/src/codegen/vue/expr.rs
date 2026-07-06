//! Expression and statement generation for Vue codegen, plus helpers.

use super::CodeWriter;
use crate::codegen::javascript::js_type;
use crate::codegen::sanitize_css_value;
use crate::ir::*;

// ═══════════════════════════════════════════════════════════
// Statement generation (Vue script setup)
// ═══════════════════════════════════════════════════════════

pub(super) fn generate_statement_vue(w: &mut CodeWriter, stmt: &StatementIR) {
    match stmt {
        StatementIR::LetStmtIR {
            name,
            value,
            r#type,
        } => {
            let rhs = value
                .as_ref()
                .map(format_expr_vue)
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
            let cond = format_expr_vue(condition);
            w.emit(&format!("if ({}) {{", cond));
            w.indent();
            for s in &then_body.statements {
                generate_statement_vue(w, s);
            }
            w.dedent();
            for (elif_cond, elif_body) in else_ifs {
                let elif_js = format_expr_vue(elif_cond);
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
                .map(format_expr_vue)
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
                .map(format_expr_vue)
                .unwrap_or_else(|| "0".into());
            match mut_intent.operation.as_str() {
                "direct_assign" => {
                    w.emit(&format!("{}.value = {};", target, amount));
                }
                "increment" => {
                    w.emit(&format!("{}.value += {};", target, amount));
                }
                "decrement" => {
                    w.emit(&format!("{}.value -= {};", target, amount));
                }
                "multiply" => {
                    w.emit(&format!("{}.value *= {};", target, amount));
                }
                "divide" => {
                    w.emit(&format!("{}.value /= {};", target, amount));
                }
                "modulo" => {
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
            let line = format_expr_vue(expr);
            if line != "null" {
                w.emit(&format!("{};", line));
            }
        }
        StatementIR::EventEmitStmtIR { event_name, args } => {
            let arg_str = if args.is_empty() {
                String::new()
            } else {
                let formatted: Vec<String> = args.iter().map(format_expr_vue).collect();
                format!("({})", formatted.join(", "))
            };
            w.emit(&format!("// emit event: {}{}", event_name, arg_str));
        }
        StatementIR::WhileStmtIR { condition, body } => {
            let cond = format_expr_vue(condition);
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
            let iter = format_expr_vue(iterable);
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

pub(super) fn format_block_as_vue_expr(block: &BlockIR) -> String {
    if let Some(last) = block.statements.last() {
        match last {
            StatementIR::ExprStmtIR { expr } => format_expr_vue(expr),
            StatementIR::ReturnStmtIR { value } => value
                .as_ref()
                .map(|e| format_expr_vue(e))
                .unwrap_or_else(|| "undefined".into()),
            _ => "undefined".into(),
        }
    } else {
        "undefined".into()
    }
}

// ═══════════════════════════════════════════════════════════
// Expression formatting (Vue)
// ═══════════════════════════════════════════════════════════

pub(super) fn format_expr_vue(expr: &ExprIR) -> String {
    match expr {
        ExprIR::LiteralExpr { value } => format_json_value_vue(value),
        ExprIR::IdentifierExpr { name } => name.clone(),
        ExprIR::FieldAccessExpr { object, field } => {
            format!("{}.{}", format_expr_vue(object), field)
        }
        ExprIR::OptionalChainExpr { object, field, .. } => {
            format!("{}.{}", format_expr_vue(object), field)
        }
        ExprIR::BinaryOpExpr { op, left, right } => {
            let js_op = binary_op_to_vue(op);
            let lhs = format_expr_vue(left);
            let rhs = format_expr_vue(right);
            format!("{} {} {}", lhs, js_op, rhs)
        }
        ExprIR::UnaryOpExpr { op, operand } => {
            let js_op = match op.as_str() {
                "not" => "!",
                "neg" => "-",
                _ => op,
            };
            format!("{}{}", js_op, format_expr_vue(operand))
        }
        ExprIR::TernaryIR {
            cond,
            then_expr,
            else_expr,
        } => {
            let c = format_expr_vue(cond);
            let t = format_expr_vue(then_expr);
            let e = format_expr_vue(else_expr);
            format!("{} ? {} : {}", c, t, e)
        }
        ExprIR::CallExpr { name, args } => {
            let arg_strs: Vec<String> = args.iter().map(format_expr_vue).collect();
            format!("{}({})", name, arg_strs.join(", "))
        }
        ExprIR::StoreFieldAccessExpr {
            store_alias,
            field_name,
            call_args,
        } => {
            let args: Vec<String> = call_args.iter().map(format_expr_vue).collect();
            let arg_str = if args.is_empty() {
                String::new()
            } else {
                format!("({})", args.join(", "))
            };
            format!("{}.{}{}", store_alias, field_name, arg_str)
        }
        ExprIR::ArrayExpr { elements } => {
            let items: Vec<String> = elements.iter().map(format_expr_vue).collect();
            format!("[{}]", items.join(", "))
        }
        ExprIR::StructExpr { fields, .. } => {
            let entries: Vec<String> = fields
                .iter()
                .map(|(k, v)| format!("\"{}\": {}", k, format_expr_vue(v)))
                .collect();
            format!("{{{}}}", entries.join(", "))
        }
        ExprIR::VariantLiteral {
            variant, fields, ..
        } => {
            let mut entries: Vec<String> = fields
                .iter()
                .map(|(k, v)| format!("\"{}\": {}", k, format_expr_vue(v)))
                .collect();
            entries.insert(0, format!("\"_variant\": \"{}\"", variant));
            format!("{{{}}}", entries.join(", "))
        }
        ExprIR::MethodCallExpr {
            is_optional: _,
            object,
            method,
            args,
            ..
        } => {
            let obj = format_expr_vue(object);
            let arg_strs: Vec<String> = args.iter().map(|a| format_expr_vue(a)).collect();
            format!("{}.{}({})", obj, method, arg_strs.join(", "))
        }
        ExprIR::BlockExpr {
            statements: _,
            tail,
        } => {
            // Only tail expression emitted; preceding statements preserved in IR for future codegen
            format_expr_vue(tail)
        }
        ExprIR::IndexExpr { object, index } => {
            let obj = format_expr_vue(object);
            let idx = format_expr_vue(index);
            format!("({obj})[{idx}]")
        }
        ExprIR::AwaitExpr { operand } => {
            format!("await {}", format_expr_vue(operand))
        }
        ExprIR::MatchExpr { matched, arms } => {
            let m = format_expr_vue(matched);
            let cases: Vec<String> = arms
                .iter()
                .map(|arm| {
                    let body = format_block_as_vue_expr(&arm.body);
                    match &arm.pattern.kind {
                        MatchPatternKindIR::Variant { name, .. } => {
                            format!("if (_v._variant === \"{}\") {{ return {}; }}", name, body)
                        }
                        MatchPatternKindIR::Wildcard => {
                            format!("return {};", body)
                        }
                        MatchPatternKindIR::Literal { value } => {
                            let v = format_json_value_vue(value);
                            format!("if (_v._variant === {}) {{ return {}; }}", v, body)
                        }
                    }
                })
                .collect();
            format!("(() => {{ const _v = {}; {} }})()", m, cases.join(" "))
        }
        ExprIR::ClosureExpr { params, body } => {
            let p = params.join(", ");
            let b = format_expr_vue(body);
            format!("(({p}) => ({b}))")
        }
    }
}

// ═══════════════════════════════════════════════════════════
// Helpers
// ═══════════════════════════════════════════════════════════

#[allow(dead_code)]
pub(super) fn capitalize_first(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}

pub(super) fn format_json_value_vue(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Null => "null".into(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::String(s) => {
            let escaped = s
                .replace('\\', "\\\\")
                .replace('"', "\\\"")
                .replace('\n', "\\n")
                .replace('\r', "\\r")
                .replace('\t', "\\t");
            format!("\"{}\"", escaped)
        }
        serde_json::Value::Array(arr) => {
            let items: Vec<_> = arr.iter().map(format_json_value_vue).collect();
            format!("[{}]", items.join(", "))
        }
        serde_json::Value::Object(obj) => {
            let items: Vec<_> = obj
                .iter()
                .map(|(k, v)| format!("{}: {}", k, format_json_value_vue(v)))
                .collect();
            format!("{{ {} }}", items.join(", "))
        }
    }
}

pub(super) fn binary_op_to_vue(op: &str) -> &str {
    match op {
        "add" => "+",
        "sub" => "-",
        "mul" => "*",
        "div" => "/",
        "mod" => "%",
        "eq" => "===",
        "neq" => "!==",
        "lt" => "<",
        "gt" => ">",
        "lteq" => "<=",
        "gteq" => ">=",
        "and" => "&&",
        "or" => "||",
        "assign" => "=",
        "addassign" => "+=",
        "subassign" => "-=",
        "mulassign" => "*=",
        "divassign" => "/=",
        "modassign" => "%=",
        _ => op,
    }
}

pub(super) fn format_css_value(value: &ExprIR) -> String {
    match value {
        ExprIR::LiteralExpr { value } => match value {
            serde_json::Value::String(s) => sanitize_css_value(s),
            serde_json::Value::Number(n) => format!("{}px", n),
            _ => format!("{}", value),
        },
        _ => String::new(),
    }
}

pub(super) fn to_css_prop(omni_prop: &str) -> String {
    let mut result = String::new();
    for c in omni_prop.chars() {
        if c.is_uppercase() {
            result.push('-');
            result.push(c.to_ascii_lowercase());
        } else {
            result.push(c);
        }
    }
    result
}

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
