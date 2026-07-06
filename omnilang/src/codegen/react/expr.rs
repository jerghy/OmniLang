// ═══════════════════════════════════════════════════════════
// Statement generation (TSX)
// ═══════════════════════════════════════════════════════════

use std::collections::HashSet;

use crate::codegen::shared::expr::FormatExpr;
use crate::codegen::shared::stmt::FormatStmt;
use crate::codegen::CodeWriter;
use crate::codegen::{capitalize_first, JsonFormatConfig, JSON_REACT};
use crate::ir::*;

/// React TSX expression formatter.
///
/// Uses JS-family syntax with `translate_builtin_call` for stdlib calls
/// and `translate_method_call` for method dispatch.
pub(crate) struct ReactFormatExpr;

impl FormatExpr for ReactFormatExpr {
    fn json_config(&self) -> &JsonFormatConfig {
        &JSON_REACT
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

    fn format_call(&self, name: &str, args: &[String]) -> String {
        translate_builtin_call(name, args)
    }

    fn format_method_call(&self, obj: &str, method: &str, args: &[String]) -> String {
        translate_method_call(obj, method, args)
    }

    fn format_match(&self, matched: &str, arms: &[MatchArmIR]) -> String {
        let cases: Vec<String> = arms
            .iter()
            .map(|arm| {
                let body = self.format_block_as_expr(&arm.body);
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
                        let v = crate::codegen::format_json_value(value, &JSON_REACT);
                        format!("if (_v._variant === {}) {{ return {}; }}", v, body)
                    }
                }
            })
            .collect();
        format!("(() => {{ const _v = {matched}; {} }})()", cases.join(" "))
    }
}

/// React TSX statement formatter.
///
/// Uses `signal_names` to determine mutation strategy (setter calls vs direct assignment).
pub(super) struct ReactStmtFormatter {
    pub expr: ReactFormatExpr,
    pub signal_names: HashSet<String>,
}

impl FormatStmt for ReactStmtFormatter {
    type ExprFormatter = ReactFormatExpr;

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
        ty.as_ref()
            .map(|t| {
                let ann = super::super::javascript::js_type(&t.kind, &t.type_args, true);
                if ann.is_empty() {
                    String::new()
                } else {
                    format!(": {}", ann)
                }
            })
            .unwrap_or_default()
    }

    fn format_event_emit_stmt(&self, w: &mut CodeWriter, event_name: &str, args: &[ExprIR]) {
        let arg_str = if args.is_empty() {
            String::new()
        } else {
            let formatted: Vec<String> = args.iter().map(|a| self.expr.format_expr(a)).collect();
            format!("({})", formatted.join(", "))
        };
        w.emit(&format!("{}{};", event_name, arg_str));
    }

    fn format_mutation_stmt(&self, w: &mut CodeWriter, intent: &MutationIntent) {
        let target = &intent.target.name;
        let amount = intent
            .amount
            .as_ref()
            .map(|v| self.expr.format_expr(v))
            .unwrap_or_else(|| "0".into());
        let is_store_signal = intent.target.owner.is_some();
        let is_component_state = !is_store_signal && self.signal_names.contains(target.as_str());
        let use_setter = is_store_signal || is_component_state;
        let setter = format!("set{}", capitalize_first(target));
        match &intent.operation {
            MutationOp::DirectAssign => {
                if use_setter {
                    w.emit(&format!("{}({});", setter, amount));
                } else {
                    w.emit(&format!("{} = {};", target, amount));
                }
            }
            MutationOp::Increment => {
                if use_setter {
                    w.emit(&format!("{}(prev => prev + {});", setter, amount));
                } else {
                    w.emit(&format!("{} = {} + {};", target, target, amount));
                }
            }
            MutationOp::Decrement => {
                if use_setter {
                    w.emit(&format!("{}(prev => prev - {});", setter, amount));
                } else {
                    w.emit(&format!("{} = {} - {};", target, target, amount));
                }
            }
            MutationOp::Multiply => {
                if use_setter {
                    w.emit(&format!("{}(prev => prev * {});", setter, amount));
                } else {
                    w.emit(&format!("{} = {} * {};", target, target, amount));
                }
            }
            MutationOp::Divide => {
                if use_setter {
                    w.emit(&format!("{}(prev => prev / {});", setter, amount));
                } else {
                    w.emit(&format!("{} = {} / {};", target, target, amount));
                }
            }
            MutationOp::Modulo => {
                if use_setter {
                    w.emit(&format!("{}(prev => prev % {});", setter, amount));
                } else {
                    w.emit(&format!("{} = {} % {};", target, target, amount));
                }
            }
            MutationOp::Push => {
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
}

pub(super) fn generate_statement_tsx(
    w: &mut CodeWriter,
    stmt: &StatementIR,
    signal_names: &HashSet<String>,
) {
    let fmt = ReactStmtFormatter {
        expr: ReactFormatExpr,
        signal_names: signal_names.clone(),
    };
    fmt.format_stmt(w, stmt);
}

// ═══════════════════════════════════════════════════════════
// Expression formatting (TSX)
// ═══════════════════════════════════════════════════════════

pub(super) fn format_expr_ir_tsx(expr: &ExprIR) -> String {
    ReactFormatExpr.format_expr(expr)
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
