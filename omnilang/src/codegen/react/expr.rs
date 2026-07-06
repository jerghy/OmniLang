// ═══════════════════════════════════════════════════════════
// Statement generation (TSX)
// ═══════════════════════════════════════════════════════════

use std::collections::HashSet;

use crate::codegen::CodeWriter;
use crate::ir::*;

use super::utils::{binary_op_to_tsx, capitalize_first, format_json_value_tsx};

pub(super) fn generate_statement_tsx(
    w: &mut CodeWriter,
    stmt: &StatementIR,
    signal_names: &HashSet<String>,
) {
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
            w.emit(&format!("let {}{} = {};", name, type_hint, rhs));
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
                generate_statement_tsx(w, s, signal_names);
            }
            w.dedent();
            for (elif_cond, elif_body) in else_ifs {
                let elif_js = format_expr_ir_tsx(elif_cond);
                w.emit(&format!("}} else if ({}) {{", elif_js));
                w.indent();
                for s in &elif_body.statements {
                    generate_statement_tsx(w, s, signal_names);
                }
                w.dedent();
            }
            if let Some(else_blk) = else_body {
                if !else_blk.statements.is_empty() {
                    w.emit("} else {");
                    w.indent();
                    for s in &else_blk.statements {
                        generate_statement_tsx(w, s, signal_names);
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
            let is_store_signal = mut_intent.target.owner.is_some();
            let is_component_state = !is_store_signal && signal_names.contains(target.as_str());
            let use_setter = is_store_signal || is_component_state;
            let setter = format!("set{}", capitalize_first(target));
            match mut_intent.operation.as_str() {
                "direct_assign" => {
                    if use_setter {
                        w.emit(&format!("{}({});", setter, amount));
                    } else {
                        w.emit(&format!("{} = {};", target, amount));
                    }
                }
                "increment" => {
                    if use_setter {
                        w.emit(&format!("{}(prev => prev + {});", setter, amount));
                    } else {
                        w.emit(&format!("{} = {} + {};", target, target, amount));
                    }
                }
                "decrement" => {
                    if use_setter {
                        w.emit(&format!("{}(prev => prev - {});", setter, amount));
                    } else {
                        w.emit(&format!("{} = {} - {};", target, target, amount));
                    }
                }
                "multiply" => {
                    if use_setter {
                        w.emit(&format!("{}(prev => prev * {});", setter, amount));
                    } else {
                        w.emit(&format!("{} = {} * {};", target, target, amount));
                    }
                }
                "divide" => {
                    if use_setter {
                        w.emit(&format!("{}(prev => prev / {});", setter, amount));
                    } else {
                        w.emit(&format!("{} = {} / {};", target, target, amount));
                    }
                }
                "modulo" => {
                    if use_setter {
                        w.emit(&format!("{}(prev => prev % {});", setter, amount));
                    } else {
                        w.emit(&format!("{} = {} % {};", target, target, amount));
                    }
                }
                "push" => {
                    // Array push
                    if use_setter {
                        w.emit(&format!("{}(prev => [...prev, {}]);", setter, amount));
                    } else {
                        w.emit(&format!("{}.push({});", target, amount));
                    }
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
            w.emit(&format!("{}{};", event_name, arg_str));
        }
        StatementIR::WhileStmtIR { condition, body } => {
            let cond = format_expr_ir_tsx(condition);
            w.emit(&format!("while ({}) {{", cond));
            w.indent();
            for s in &body.statements {
                generate_statement_tsx(w, s, signal_names);
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
                generate_statement_tsx(w, s, signal_names);
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
                generate_statement_tsx(w, s, signal_names);
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
                    generate_statement_tsx(w, s, signal_names);
                }
                w.dedent();
            }
            if let Some(ref fb) = finally_body {
                w.emit("} finally {");
                w.indent();
                for s in &fb.statements {
                    generate_statement_tsx(w, s, signal_names);
                }
                w.dedent();
            }
            w.emit("}");
        }
        StatementIR::BlockStmtIR { body } => {
            for s in &body.statements {
                generate_statement_tsx(w, s, signal_names);
            }
        }
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
        ExprIR::OptionalChainExpr { object, field, .. } => {
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
            translate_builtin_call(name, &arg_strs)
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
            translate_method_call(&obj, method, &arg_strs)
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
                        MatchPatternKindIR::Variant { name, bindings } => {
                            let destructure = if bindings.is_empty() {
                                String::new()
                            } else {
                                format!("const {{ {} }} = _v; ", bindings.join(", "))
                            };
                            format!(
                                "if (_v._variant === \"{}\") {{ {}{}; }}",
                                name, destructure, body
                            )
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

// ═══════════════════════════════════════════════════════════
// OmniLang → JavaScript built-in translation
// ═══════════════════════════════════════════════════════════

fn translate_builtin_call(name: &str, args: &[String]) -> String {
    match name {
        // Console
        "console_log" | "println" | "print" => format!("console.log({})", args.join(", ")),
        "console_error" => format!("console.error({})", args.join(", ")),
        "console_warn" => format!("console.warn({})", args.join(", ")),
        // JSON
        "json_parse" => format!("JSON.parse({})", args.join(", ")),
        "json_stringify" => format!("JSON.stringify({})", args.join(", ")),
        // Storage
        "storage_get" => format!("localStorage.getItem({})", args.join(", ")),
        "storage_set" => format!("localStorage.setItem({})", args.join(", ")),
        "storage_remove" => format!("localStorage.removeItem({})", args.join(", ")),
        // Array
        "array_is_empty" => {
            let a = args.first().map(|s| s.as_str()).unwrap_or("");
            format!("(!({}) || {}.length === 0)", a, a)
        }
        "array_length" => {
            let a = args.first().map(|s| s.as_str()).unwrap_or("");
            format!("{}.length", a)
        }
        "array_push" => {
            let arr = args.first().map(|s| s.as_str()).unwrap_or("");
            let rest: Vec<&str> = args.iter().skip(1).map(|s| s.as_str()).collect();
            format!("[{}, {}]", arr, rest.join(", "))
        }
        "array_pop" => {
            let a = args.first().map(|s| s.as_str()).unwrap_or("");
            format!("{}.slice(0, -1)", a)
        }
        "array_filter" => {
            let a = args.first().map(|s| s.as_str()).unwrap_or("");
            let pred = args.get(1).map(|s| s.as_str()).unwrap_or("");
            format!("{}.filter({})", a, pred)
        }
        "array_map" => {
            let a = args.first().map(|s| s.as_str()).unwrap_or("");
            let mapper = args.get(1).map(|s| s.as_str()).unwrap_or("");
            format!("{}.map({})", a, mapper)
        }
        "array_find" => {
            let a = args.first().map(|s| s.as_str()).unwrap_or("");
            let pred = args.get(1).map(|s| s.as_str()).unwrap_or("");
            format!("{}.find({})", a, pred)
        }
        "array_contains" => {
            let a = args.first().map(|s| s.as_str()).unwrap_or("");
            let item = args.get(1).map(|s| s.as_str()).unwrap_or("");
            format!("{}.includes({})", a, item)
        }
        "array_join" => {
            let a = args.first().map(|s| s.as_str()).unwrap_or("");
            let sep = args.get(1).map(|s| s.as_str()).unwrap_or(",");
            format!("{}.join({})", a, sep)
        }
        "array_sort" => {
            let a = args.first().map(|s| s.as_str()).unwrap_or("");
            format!("{}.slice().sort()", a)
        }
        "array_remove" => {
            let a = args.first().map(|s| s.as_str()).unwrap_or("");
            let idx = args.get(1).map(|s| s.as_str()).unwrap_or("0");
            format!("{}.filter((_, __i) => __i !== {})", a, idx)
        }
        // Math
        "math_abs" => format!("Math.abs({})", args.join(", ")),
        "math_min" => format!("Math.min({})", args.join(", ")),
        "math_max" => format!("Math.max({})", args.join(", ")),
        "math_floor" => format!("Math.floor({})", args.join(", ")),
        "math_ceil" => format!("Math.ceil({})", args.join(", ")),
        "math_round" => format!("Math.round({})", args.join(", ")),
        "math_sqrt" => format!("Math.sqrt({})", args.join(", ")),
        "math_random" => "Math.random()".to_string(),
        "math_random_int" => {
            let min = args.first().map(|s| s.as_str()).unwrap_or("0");
            let max = args.get(1).map(|s| s.as_str()).unwrap_or("100");
            format!(
                "Math.floor(Math.random() * ({} - {} + 1)) + {}",
                max, min, min
            )
        }
        "math_pow" => format!("Math.pow({})", args.join(", ")),
        "math_clamp" => {
            let val = args.first().map(|s| s.as_str()).unwrap_or("0");
            let lo = args.get(1).map(|s| s.as_str()).unwrap_or("0");
            let hi = args.get(2).map(|s| s.as_str()).unwrap_or("1");
            format!("Math.min(Math.max({}, {}), {})", val, lo, hi)
        }
        // String builtins (function style: string_substring(obj, start, end))
        "string_length" => {
            let a = args.first().map(|s| s.as_str()).unwrap_or("");
            format!("({}).length", a)
        }
        "string_substring" => {
            let a = args.first().map(|s| s.as_str()).unwrap_or("");
            let rest: Vec<&str> = args.iter().skip(1).map(|s| s.as_str()).collect();
            format!("({}).substring({})", a, rest.join(", "))
        }
        "string_split" => {
            let a = args.first().map(|s| s.as_str()).unwrap_or("");
            let rest: Vec<&str> = args.iter().skip(1).map(|s| s.as_str()).collect();
            format!("({}).split({})", a, rest.join(", "))
        }
        "string_replace" => {
            let a = args.first().map(|s| s.as_str()).unwrap_or("");
            let rest: Vec<&str> = args.iter().skip(1).map(|s| s.as_str()).collect();
            format!("({}).replace({})", a, rest.join(", "))
        }
        "string_join" => {
            let a = args.first().map(|s| s.as_str()).unwrap_or("");
            let rest: Vec<&str> = args.iter().skip(1).map(|s| s.as_str()).collect();
            format!("({}).join({})", a, rest.join(", "))
        }
        "string_ends_with" => {
            let a = args.first().map(|s| s.as_str()).unwrap_or("");
            let rest: Vec<&str> = args.iter().skip(1).map(|s| s.as_str()).collect();
            format!("({}).endsWith({})", a, rest.join(", "))
        }
        "string_starts_with" => {
            let a = args.first().map(|s| s.as_str()).unwrap_or("");
            let rest: Vec<&str> = args.iter().skip(1).map(|s| s.as_str()).collect();
            format!("({}).startsWith({})", a, rest.join(", "))
        }
        "string_concat" => {
            let a = args.first().map(|s| s.as_str()).unwrap_or("");
            let rest: Vec<&str> = args.iter().skip(1).map(|s| s.as_str()).collect();
            format!("({}).concat({})", a, rest.join(", "))
        }
        "string_contains" => {
            let a = args.first().map(|s| s.as_str()).unwrap_or("");
            let rest: Vec<&str> = args.iter().skip(1).map(|s| s.as_str()).collect();
            format!("({}).includes({})", a, rest.join(", "))
        }
        "string_trim" => {
            let a = args.first().map(|s| s.as_str()).unwrap_or("");
            format!("({}).trim()", a)
        }
        // Date/time
        "datetime_now" => "new Date().toISOString()".to_string(),
        "datetime_from_timestamp" => {
            let ts = args.first().map(|s| s.as_str()).unwrap_or("0");
            format!("new Date({})", ts)
        }
        "datetime_format" => {
            let dt = args.first().map(|s| s.as_str()).unwrap_or("new Date()");
            let _fmt = args.get(1).map(|s| s.as_str()).unwrap_or("");
            // Simple format using toLocaleDateString or manual formatting
            format!("new Date({}).toLocaleDateString()", dt)
        }
        // Regex
        "regex_replace" => {
            let text = args.first().map(|s| s.as_str()).unwrap_or("");
            let pattern = args.get(1).map(|s| s.as_str()).unwrap_or("");
            let replacement = args.get(2).map(|s| s.as_str()).unwrap_or("");
            format!(
                "({}).replace(new RegExp({}, 'g'), {})",
                text, pattern, replacement
            )
        }
        "regex_search" => {
            let text = args.first().map(|s| s.as_str()).unwrap_or("");
            let pattern = args.get(1).map(|s| s.as_str()).unwrap_or("");
            format!("({}).search(new RegExp({}))", text, pattern)
        }
        "regex_match" => {
            let text = args.first().map(|s| s.as_str()).unwrap_or("");
            let pattern = args.get(1).map(|s| s.as_str()).unwrap_or("");
            format!("({}).match(new RegExp({}))", text, pattern)
        }
        // Async
        "async_spawn" => {
            let body = args.first().map(|s| s.as_str()).unwrap_or("");
            format!("setTimeout(() => {{ {}; }}, 0)", body)
        }
        "async_sleep" => {
            let ms = args.first().map(|s| s.as_str()).unwrap_or("0");
            format!("new Promise(_r => setTimeout(_r, {}))", ms)
        }
        "async_all" => {
            let a = args.first().map(|s| s.as_str()).unwrap_or("[]");
            format!("Promise.all({})", a)
        }
        "async_race" => {
            let a = args.first().map(|s| s.as_str()).unwrap_or("[]");
            format!("Promise.race({})", a)
        }
        "async_timeout" => {
            let promise = args.first().map(|s| s.as_str()).unwrap_or("");
            let ms = args.get(1).map(|s| s.as_str()).unwrap_or("5000");
            format!("Promise.race([{}, new Promise((_, _rej) => setTimeout(() => _rej(new Error('timeout')), {}))])", promise, ms)
        }
        // HTTP
        "http_get" => {
            let url = args.first().map(|s| s.as_str()).unwrap_or("");
            format!("fetch({})", url)
        }
        "http_post" => {
            let url = args.first().map(|s| s.as_str()).unwrap_or("");
            let body = args.get(1).map(|s| s.as_str()).unwrap_or("{}");
            format!("fetch({}, {{ method: 'POST', body: {} }})", url, body)
        }
        // Encoding
        "encode_url_encode" => format!("encodeURIComponent({})", args.join(", ")),
        "encode_url_decode" => format!("decodeURIComponent({})", args.join(", ")),
        "encode_base64_encode" => format!("btoa({})", args.join(", ")),
        "encode_base64_decode" => format!("atob({})", args.join(", ")),
        "encode_hex_encode" => {
            let s = args.first().map(|s| s.as_str()).unwrap_or("");
            format!(
                "Array.from({}, c => c.charCodeAt(0).toString(16).padStart(2, '0')).join('')",
                s
            )
        }
        "encode_hex_decode" => {
            let s = args.first().map(|s| s.as_str()).unwrap_or("");
            format!(
                "({}).match(/.{{1,2}}/g).map(b => String.fromCharCode(parseInt(b, 16))).join('')",
                s
            )
        }
        _ => format!("{}({})", name, args.join(", ")),
    }
}

fn translate_method_call(obj: &str, method: &str, args: &[String]) -> String {
    match method {
        // Array element access: arr.get(i) → arr[i]
        "get" => {
            let idx = args.first().map(|s| s.as_str()).unwrap_or("0");
            format!("({})[{}]", obj, idx)
        }
        // Rust-style conversions → JS
        "to_string" => format!("{}.toString({})", obj, args.join(", ")),
        "to_float" => format!("parseFloat({})", obj),
        "len" => format!("{}.length", obj),
        // String methods
        "starts_with" => format!("{}.startsWith({})", obj, args.join(", ")),
        "ends_with" => format!("{}.endsWith({})", obj, args.join(", ")),
        "contains" => format!("{}.includes({})", obj, args.join(", ")),
        "substring" => format!("{}.substring({})", obj, args.join(", ")),
        "split" => format!("{}.split({})", obj, args.join(", ")),
        "replace" => format!("{}.replace({})", obj, args.join(", ")),
        "join" => format!("{}.join({})", obj, args.join(", ")),
        "trim" => format!("{}.trim()", obj),
        "concat" => format!("{}.concat({})", obj, args.join(", ")),
        // Default: call as-is
        _ => format!("{}.{}({})", obj, method, args.join(", ")),
    }
}
