//! Shared JSON value formatting for all codegen backends.
//!
//! Provides target-language-specific JSON literal formatting via
//! [`JsonFormatConfig`] and pre-defined constants for each backend.

/// Escape a string value for use as a string literal in generated code.
/// Handles backslash, the target quote character, and common whitespace escapes.
#[must_use]
pub fn escape_literal_str(s: &str, quote: char) -> String {
    s.replace('\\', "\\\\")
        .replace(quote, &format!("\\{}", quote))
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
        .replace('\0', "\\0")
}

/// Configuration for target-language-specific JSON value formatting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JsonFormatConfig {
    pub null_literal: &'static str,
    pub bool_true: &'static str,
    pub bool_false: &'static str,
    pub quote: char,
    pub string_suffix: &'static str,
    pub array_fmt: JsonArrayFmt,
    pub object_fmt: JsonObjectFmt,
    pub strip_trailing_dot_zero: bool,
    /// Suffix to append to large integer literals that exceed MAX_SAFE_INTEGER (2^53-1).
    /// JS-family targets use `"n"` for BigInt; other targets leave this empty.
    pub large_integer_suffix: &'static str,
}

/// How to format JSON arrays for the target language.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JsonArrayFmt {
    /// `[a, b, c]`
    Bracket,
    /// `vec![a, b, c]`
    VecMacro,
    /// Use serde_json Display
    Passthrough,
}

/// How to format JSON objects for the target language.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JsonObjectFmt {
    /// `{ k: v }`
    Bracket,
    /// Use serde_json Display
    Passthrough,
}

/// Format a `serde_json::Value` as a target-language literal.
///
/// Handles null, bool, number, string, array, and object values
/// according to the provided [`JsonFormatConfig`].
#[must_use]
pub fn format_json_value(value: &serde_json::Value, cfg: &JsonFormatConfig) -> String {
    match value {
        serde_json::Value::Null => cfg.null_literal.into(),
        serde_json::Value::Bool(b) => {
            if *b {
                cfg.bool_true.into()
            } else {
                cfg.bool_false.into()
            }
        }
        serde_json::Value::Number(n) => {
            let mut s = n.to_string();
            if cfg.strip_trailing_dot_zero {
                s = s.strip_suffix(".0").unwrap_or(&s).to_string();
            }
            // If the integer exceeds JS MAX_SAFE_INTEGER (2^53 - 1), append BigInt suffix.
            if !cfg.large_integer_suffix.is_empty() {
                const MAX_SAFE: u64 = 9007199254740991; // 2^53 - 1
                let is_large = if let Some(i) = n.as_i64() {
                    i > MAX_SAFE as i64 || i < -(MAX_SAFE as i64)
                } else if let Some(u) = n.as_u64() {
                    u > MAX_SAFE
                } else {
                    false
                };
                if is_large {
                    s.push_str(cfg.large_integer_suffix);
                }
            }
            s
        }
        serde_json::Value::String(s) => {
            let escaped = escape_literal_str(s, cfg.quote);
            if cfg.string_suffix.is_empty() {
                format!("{c}{escaped}{c}", c = cfg.quote)
            } else {
                format!(
                    "{c}{escaped}{c}{suffix}",
                    c = cfg.quote,
                    suffix = cfg.string_suffix
                )
            }
        }
        serde_json::Value::Array(arr) => match cfg.array_fmt {
            JsonArrayFmt::Bracket => {
                let items: Vec<_> = arr.iter().map(|v| format_json_value(v, cfg)).collect();
                format!("[{}]", items.join(", "))
            }
            JsonArrayFmt::VecMacro => {
                let items: Vec<_> = arr.iter().map(|v| format_json_value(v, cfg)).collect();
                format!("vec![{}]", items.join(", "))
            }
            JsonArrayFmt::Passthrough => value.to_string(),
        },
        serde_json::Value::Object(obj) => match cfg.object_fmt {
            JsonObjectFmt::Bracket => {
                let items: Vec<_> = obj
                    .iter()
                    .map(|(k, v)| format!("{}: {}", k, format_json_value(v, cfg)))
                    .collect();
                format!("{{ {} }}", items.join(", "))
            }
            JsonObjectFmt::Passthrough => value.to_string(),
        },
    }
}

// ── Config constants per backend family ─────────────────

/// Config: JavaScript / TypeScript
pub const JSON_JS: JsonFormatConfig = JsonFormatConfig {
    null_literal: "null",
    bool_true: "true",
    bool_false: "false",
    quote: '"',
    string_suffix: "",
    array_fmt: JsonArrayFmt::Bracket,
    object_fmt: JsonObjectFmt::Bracket,
    strip_trailing_dot_zero: false,
    large_integer_suffix: "n",
};

/// Config: React (same as JS)
pub const JSON_REACT: JsonFormatConfig = JSON_JS;

/// Config: Vue (same as JS)
pub const JSON_VUE: JsonFormatConfig = JSON_JS;

/// Config: OmniView (same as JS)
pub const JSON_OMNIVIEW: JsonFormatConfig = JSON_JS;

/// Config: Dart
pub const JSON_DART: JsonFormatConfig = JsonFormatConfig {
    null_literal: "null",
    bool_true: "true",
    bool_false: "false",
    quote: '\'',
    string_suffix: "",
    array_fmt: JsonArrayFmt::Bracket,
    object_fmt: JsonObjectFmt::Bracket,
    strip_trailing_dot_zero: true,
    large_integer_suffix: "",
};

/// Config: Flutter (same as Dart)
pub const JSON_FLUTTER: JsonFormatConfig = JSON_DART;

/// Config: Rust
pub const JSON_RUST: JsonFormatConfig = JsonFormatConfig {
    null_literal: "None",
    bool_true: "true",
    bool_false: "false",
    quote: '"',
    string_suffix: ".to_string()",
    array_fmt: JsonArrayFmt::VecMacro,
    object_fmt: JsonObjectFmt::Passthrough,
    strip_trailing_dot_zero: true,
    large_integer_suffix: "",
};

/// Config: Dioxus (same as Rust)
pub const JSON_DIOXUS: JsonFormatConfig = JSON_RUST;

/// Config: Python
pub const JSON_PYTHON: JsonFormatConfig = JsonFormatConfig {
    null_literal: "None",
    bool_true: "True",
    bool_false: "False",
    quote: '\'',
    string_suffix: "",
    array_fmt: JsonArrayFmt::Passthrough,
    object_fmt: JsonObjectFmt::Passthrough,
    strip_trailing_dot_zero: true,
    large_integer_suffix: "",
};
