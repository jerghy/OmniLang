//! Shared inline-style expression formatting for JS-family backends.
//!
//! Converts `ElementIR.inline_style` JSON values into JS expression
//! strings used by the React (`:style` JSX objects) and Vue (`:style`
//! binding objects) backends.

use crate::codegen::sanitize_css_value;
use rustc_hash::FxHashSet;

/// Format a CSS property key for use in a JSX style object.
/// Quotes the key if it starts with `--` (custom property) or contains hyphens.
pub fn format_style_prop_key(key: &str) -> String {
    if key.starts_with("--") || key.contains('-') {
        format!("'{}'", key)
    } else {
        key.to_string()
    }
}

/// Translate an inline style value from the IR JSON representation into a TSX expression.
/// Handles $signal, $call, and $concat meta-objects.
pub fn format_style_value_expr(
    val: &serde_json::Value,
    ref_names: &FxHashSet<String>,
) -> String {
    match val {
        // Static string: 'value' (with CSS sanitization)
        serde_json::Value::String(s) => {
            let sanitized = sanitize_css_value(s);
            if sanitized.is_empty() {
                // Value was blocked — emit empty string as safe fallback
                "''".to_string()
            } else {
                // Escape single quotes in the sanitized value
                let escaped = sanitized.replace('\\', "\\\\").replace('\'', "\\'");
                format!("'{}'", escaped)
            }
        }
        // Static number: 42 (with px suffix for numbers)
        serde_json::Value::Number(n) => {
            if let Some(f) = n.as_f64() {
                format!("{}", f)
            } else {
                format!("{}", n)
            }
        }
        // $signal: { "$signal": "name" } -> accentColor (bare identifier, no braces)
        // $call:   { "$call": { "name": "fnName", "args": [...] } } -> fnName(arg1, arg2)
        // $concat: { "$concat": [part1, part2, ...] } -> `part1${part2}`
        serde_json::Value::Object(obj) => {
            if let Some(sig_name) = obj.get("$signal").and_then(|v| v.as_str()) {
                if ref_names.contains(sig_name) {
                    return format!("{}Ref.current", sig_name);
                }
                return sig_name.to_string();
            }
            // $call
            if let Some(call_obj) = obj.get("$call").and_then(|v| v.as_object()) {
                let fn_name = call_obj.get("name").and_then(|v| v.as_str()).unwrap_or("");
                let args_str = if let Some(args) = call_obj.get("args").and_then(|v| v.as_array()) {
                    let translated: Vec<String> = args
                        .iter()
                        .map(|v| format_style_value_expr(v, ref_names))
                        .collect();
                    translated.join(", ")
                } else {
                    String::new()
                };
                return format!("{}({})", fn_name, args_str);
            }
            // $concat
            if let Some(parts) = obj.get("$concat").and_then(|v| v.as_array()) {
                let mut template_parts: Vec<String> = Vec::new();
                for part in parts {
                    match part {
                        serde_json::Value::String(s) => {
                            let escaped = s
                                .replace('\\', "\\\\")
                                .replace('`', "\\`")
                                .replace("${", "\\${");
                            template_parts.push(escaped);
                        }
                        _ => {
                            // Dynamic part — resolve to expression and wrap in ${...}
                            let inner = format_style_value_expr(part, ref_names);
                            template_parts.push(format!("${{{}}}", inner));
                        }
                    }
                }
                return format!("`{}`", template_parts.join(""));
            }
            // Unknown object — emit as-is by serializing
            serde_json::to_string(val).unwrap_or_else(|e| {
                eprintln!("react/codegen: style value serialization failed: {}", e);
                "\"__STYLE_ERROR__\"".into()
            })
        }
        // Boolean, null
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Null => "null".into(),
        // Array (unlikely in style context)
        serde_json::Value::Array(arr) => {
            let items: Vec<_> = arr
                .iter()
                .map(|v| format_style_value_expr(v, ref_names))
                .collect();
            format!("[{}]", items.join(", "))
        }
    }
}
