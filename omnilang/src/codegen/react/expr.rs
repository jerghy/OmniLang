// ═══════════════════════════════════════════════════════════
// Statement generation (TSX)
// ═══════════════════════════════════════════════════════════

use rustc_hash::FxHashSet;
use crate::codegen::shared::expr::FormatExpr;
use crate::codegen::shared::stmt::FormatStmt;
use crate::codegen::shared::types::{FormatType, JsTypeFormatter};
use crate::codegen::CodeWriter;
use crate::codegen::{
    capitalize_first, try_format_stdlib_call, JsonFormatConfig, Target, JSON_REACT,
};
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

    fn codegen_target(&self) -> Target {
        Target::React
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
        if let Some(native) = try_format_stdlib_call(name, args, Target::React) {
            return native;
        }
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
    pub signal_names: FxHashSet<String>,
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
                let ann = JsTypeFormatter.format_type(t.kind.as_str(), &t.type_args);
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
        w.emit_fmt(format_args!("{}{};", event_name, arg_str));
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
                    w.emit_fmt(format_args!("{}({});", setter, amount));
                } else {
                    w.emit_fmt(format_args!("{} = {};", target, amount));
                }
            }
            MutationOp::Increment => {
                if use_setter {
                    w.emit_fmt(format_args!("{}(prev => prev + {});", setter, amount));
                } else {
                    w.emit_fmt(format_args!("{} = {} + {};", target, target, amount));
                }
            }
            MutationOp::Decrement => {
                if use_setter {
                    w.emit_fmt(format_args!("{}(prev => prev - {});", setter, amount));
                } else {
                    w.emit_fmt(format_args!("{} = {} - {};", target, target, amount));
                }
            }
            MutationOp::Multiply => {
                if use_setter {
                    w.emit_fmt(format_args!("{}(prev => prev * {});", setter, amount));
                } else {
                    w.emit_fmt(format_args!("{} = {} * {};", target, target, amount));
                }
            }
            MutationOp::Divide => {
                if use_setter {
                    w.emit_fmt(format_args!("{}(prev => prev / {});", setter, amount));
                } else {
                    w.emit_fmt(format_args!("{} = {} / {};", target, target, amount));
                }
            }
            MutationOp::Modulo => {
                if use_setter {
                    w.emit_fmt(format_args!("{}(prev => prev % {});", setter, amount));
                } else {
                    w.emit_fmt(format_args!("{} = {} % {};", target, target, amount));
                }
            }
            MutationOp::Push => {
                if use_setter {
                    w.emit_fmt(format_args!("{}(prev => [...prev, {}]);", setter, amount));
                } else {
                    w.emit_fmt(format_args!("{}.push({});", target, amount));
                }
            }
            MutationOp::Pop => {
                if use_setter {
                    w.emit_fmt(format_args!("{}(prev => prev.slice(0, -1));", setter));
                } else {
                    w.emit_fmt(format_args!("{}.pop();", target));
                }
            }
        }
    }
}

pub(super) fn generate_statement_tsx(
    w: &mut CodeWriter,
    stmt: &StatementIR,
    signal_names: &FxHashSet<String>,
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
        // Console aliases (console_log itself is in stdlib, but println/print are not)
        "println" | "print" => format!("console.log({})", args.join(", ")),
        // Storage (storage_get/set are in stdlib; storage_remove is not)
        "storage_remove" => format!("localStorage.removeItem({})", args.join(", ")),
        // Array (most array_* are in stdlib; array_join is not)
        "array_join" => {
            let a = args.first().map(|s| s.as_str()).unwrap_or("");
            let sep = args.get(1).map(|s| s.as_str()).unwrap_or(",");
            format!("{}.join({})", a, sep)
        }
        // Math (most math_* are in stdlib; math_random_int and math_clamp are not)
        "math_random_int" => {
            let min = args.first().map(|s| s.as_str()).unwrap_or("0");
            let max = args.get(1).map(|s| s.as_str()).unwrap_or("100");
            format!(
                "Math.floor(Math.random() * ({} - {} + 1)) + {}",
                max, min, min
            )
        }
        "math_clamp" => {
            let val = args.first().map(|s| s.as_str()).unwrap_or("0");
            let lo = args.get(1).map(|s| s.as_str()).unwrap_or("0");
            let hi = args.get(2).map(|s| s.as_str()).unwrap_or("1");
            format!("Math.min(Math.max({}, {}), {})", val, lo, hi)
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
