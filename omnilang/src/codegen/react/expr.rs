// ═══════════════════════════════════════════════════════════
// Statement generation (TSX)
// ═══════════════════════════════════════════════════════════

use crate::codegen::CodeWriter;
use crate::ir::*;

use super::utils::{binary_op_to_tsx, capitalize_first, format_json_value_tsx};

pub(super) fn generate_statement_tsx(w: &mut CodeWriter, stmt: &StatementIR) {
    match stmt {
        StatementIR::LetStmtIR {
            name,
            value,
            r#type,
        } => {
            let rhs = value
                .as_ref()
                .map(format_expr_ir_tsx)
                .unwrap_or_else(|| "null".into());
            let type_hint = r#type
                .as_ref()
                .map(|t| {
                    let ann = super::super::javascript::js_type(&t.kind, &t.type_args, true);
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
            let cond = format_expr_ir_tsx(condition);
            w.emit(&format!("if ({}) {{", cond));
            w.indent();
            for s in &then_body.statements {
                generate_statement_tsx(w, s);
            }
            w.dedent();
            for (elif_cond, elif_body) in else_ifs {
                let elif_js = format_expr_ir_tsx(elif_cond);
                w.emit(&format!("}} else if ({}) {{", elif_js));
                w.indent();
                for s in &elif_body.statements {
                    generate_statement_tsx(w, s);
                }
                w.dedent();
            }
            if let Some(else_blk) = else_body {
                if !else_blk.statements.is_empty() {
                    w.emit("} else {");
                    w.indent();
                    for s in &else_blk.statements {
                        generate_statement_tsx(w, s);
                    }
                    w.dedent();
                }
            }
            w.emit("}");
        }
        StatementIR::ReturnStmtIR { value } => {
            let rhs = value
                .as_ref()
                .map(format_expr_ir_tsx)
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
                .map(format_expr_ir_tsx)
                .unwrap_or_else(|| "0".into());
            let setter = format!("set{}", capitalize_first(target));
            match mut_intent.operation.as_str() {
                "direct_assign" => {
                    w.emit(&format!("{}({});", setter, amount));
                }
                "increment" => {
                    w.emit(&format!("{}(prev => prev + {});", setter, amount));
                }
                "decrement" => {
                    w.emit(&format!("{}(prev => prev - {});", setter, amount));
                }
                "multiply" => {
                    w.emit(&format!("{}(prev => prev * {});", setter, amount));
                }
                "divide" => {
                    w.emit(&format!("{}(prev => prev / {});", setter, amount));
                }
                "modulo" => {
                    w.emit(&format!("{}(prev => prev % {});", setter, amount));
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
            let line = format_expr_ir_tsx(expr);
            if line != "null" {
                w.emit(&format!("{};", line));
            }
        }
        StatementIR::EventEmitStmtIR { event_name, args } => {
            let arg_str = if args.is_empty() {
                String::new()
            } else {
                let formatted: Vec<String> = args.iter().map(format_expr_ir_tsx).collect();
                format!("({})", formatted.join(", "))
            };
            w.emit(&format!("// emit event: {}{}", event_name, arg_str));
        }
        StatementIR::WhileStmtIR { condition, body } => {
            let cond = format_expr_ir_tsx(condition);
            w.emit(&format!("while ({}) {{", cond));
            w.indent();
            for s in &body.statements {
                generate_statement_tsx(w, s);
            }
            w.dedent();
            w.emit("}");
        }
        StatementIR::ForInStmtIR {
            var_name,
            iterable,
            body,
        } => {
            let iter = format_expr_ir_tsx(iterable);
            w.emit(&format!("for (const {} of {}) {{", var_name, iter));
            w.indent();
            for s in &body.statements {
                generate_statement_tsx(w, s);
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
            let e = format_expr_ir_tsx(expr);
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
                generate_statement_tsx(w, s);
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
                    generate_statement_tsx(w, s);
                }
                w.dedent();
            }
            if let Some(ref fb) = finally_body {
                w.emit("} finally {");
                w.indent();
                for s in &fb.statements {
                    generate_statement_tsx(w, s);
                }
                w.dedent();
            }
            w.emit("}");
        }
        StatementIR::BlockStmtIR { body } => {
            for s in &body.statements {
                generate_statement_tsx(w, s);
            }
        }
        _ => {}
    }
}

fn format_block_as_tsx_expr(block: &BlockIR) -> String {
    if let Some(last) = block.statements.last() {
        match last {
            StatementIR::ExprStmtIR { expr } => format_expr_ir_tsx(expr),
            StatementIR::ReturnStmtIR { value } => value
                .as_ref()
                .map(|e| format_expr_ir_tsx(e))
                .unwrap_or_else(|| "undefined".into()),
            _ => "undefined".into(),
        }
    } else {
        "undefined".into()
    }
}

// ═══════════════════════════════════════════════════════════
// Expression formatting (TSX)
// ═══════════════════════════════════════════════════════════

pub(super) fn format_expr_ir_tsx(expr: &ExprIR) -> String {
    match expr {
        ExprIR::LiteralExpr { value } => format_json_value_tsx(value),
        ExprIR::IdentifierExpr { name } => name.clone(),
        ExprIR::FieldAccessExpr { object, field } => {
            format!("{}.{}", format_expr_ir_tsx(object), field)
        }
        ExprIR::OptionalChainExpr { object, field } => {
            format!("{}.{}", format_expr_ir_tsx(object), field)
        }
        ExprIR::BinaryOpExpr { op, left, right } => {
            let js_op = binary_op_to_tsx(op);
            let lhs = format_expr_ir_tsx(left);
            let rhs = format_expr_ir_tsx(right);
            format!("{} {} {}", lhs, js_op, rhs)
        }
        ExprIR::UnaryOpExpr { op, operand } => {
            let js_op = match op.as_str() {
                "not" => "!",
                "neg" => "-",
                _ => op,
            };
            format!("{}{}", js_op, format_expr_ir_tsx(operand))
        }
        ExprIR::TernaryIR {
            cond,
            then_expr,
            else_expr,
        } => {
            let c = format_expr_ir_tsx(cond);
            let t = format_expr_ir_tsx(then_expr);
            let e = format_expr_ir_tsx(else_expr);
            format!("{} ? {} : {}", c, t, e)
        }
        ExprIR::CallExpr { name, args } => {
            let arg_strs: Vec<String> = args.iter().map(format_expr_ir_tsx).collect();
            format!("{}({})", name, arg_strs.join(", "))
        }
        ExprIR::StoreFieldAccessExpr {
            store_alias,
            field_name,
            call_args,
        } => {
            let args: Vec<String> = call_args.iter().map(format_expr_ir_tsx).collect();
            let arg_str = if args.is_empty() {
                String::new()
            } else {
                format!("({})", args.join(", "))
            };
            format!("{}.{}{}", store_alias, field_name, arg_str)
        }
        ExprIR::ArrayExpr { elements } => {
            let items: Vec<String> = elements.iter().map(format_expr_ir_tsx).collect();
            format!("[{}]", items.join(", "))
        }
        ExprIR::StructExpr { fields, .. } => {
            let entries: Vec<String> = fields
                .iter()
                .map(|(k, v)| format!("\"{}\": {}", k, format_expr_ir_tsx(v)))
                .collect();
            format!("{{{}}}", entries.join(", "))
        }
        ExprIR::VariantLiteral {
            variant, fields, ..
        } => {
            let mut entries: Vec<String> = fields
                .iter()
                .map(|(k, v)| format!("\"{}\": {}", k, format_expr_ir_tsx(v)))
                .collect();
            entries.insert(0, format!("\"_variant\": \"{}\"", variant));
            format!("{{{}}}", entries.join(", "))
        }
        ExprIR::MethodCallExpr {
            object,
            method,
            args,
            ..
        } => {
            let obj = format_expr_ir_tsx(object);
            let arg_strs: Vec<String> = args.iter().map(|a| format_expr_ir_tsx(a)).collect();
            format!("{}.{}({})", obj, method, arg_strs.join(", "))
        }
        ExprIR::BlockExpr {
            statements: _,
            tail,
        } => {
            // Only tail expression emitted; preceding statements preserved in IR for future codegen
            format_expr_ir_tsx(tail)
        }
        ExprIR::IndexExpr { object, index } => {
            let obj = format_expr_ir_tsx(object);
            let idx = format_expr_ir_tsx(index);
            format!("({obj})[{idx}]")
        }
        ExprIR::AwaitExpr { operand } => {
            format!("await {}", format_expr_ir_tsx(operand))
        }
        ExprIR::MatchExpr { matched, arms } => {
            let m = format_expr_ir_tsx(matched);
            let cases: Vec<String> = arms
                .iter()
                .map(|arm| {
                    let body = format_block_as_tsx_expr(&arm.body);
                    match &arm.pattern.kind {
                        MatchPatternKindIR::Variant { name, .. } => {
                            format!("if (_v._variant === \"{}\") {{ return {}; }}", name, body)
                        }
                        MatchPatternKindIR::Wildcard => {
                            format!("return {};", body)
                        }
                        MatchPatternKindIR::Literal { value } => {
                            let v = format_json_value_tsx(value);
                            format!("if (_v._variant === {}) {{ return {}; }}", v, body)
                        }
                    }
                })
                .collect();
            format!("(() => {{ const _v = {}; {} }})()", m, cases.join(" "))
        }
        ExprIR::ClosureExpr { params, body } => {
            let p = params.join(", ");
            let b = format_expr_ir_tsx(body);
            format!("(({p}) => ({b}))")
        }
    }
}
