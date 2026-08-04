//! Shared JSON value formatting for all codegen backends.
//!
//! Provides target-language-specific JSON literal formatting via
//! [`JsonFormatConfig`] and pre-defined constants for each backend.

/// Escape a string value for use as a string literal in generated code.
/// Handles backslash, the target quote character, and common whitespace escapes.
///
/// Single pass over the characters (one allocation instead of the previous
/// six chained `str::replace` calls, each of which allocated). Each input
/// char maps to at most two output bytes, so `s.len()` is a safe lower
/// bound for the capacity — reallocation only happens on the rare path
/// where escapes actually occur.
///
/// Byte-identical to the chained-replace version for every `quote` value
/// used by callers (`'"'` and `'\''`): each char is escaped independently,
/// and the escape arms are disjoint from the quote arm for those quotes.
#[must_use]
pub fn escape_literal_str(s: &str, quote: char) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\0' => out.push_str("\\0"),
            c if c == quote => {
                out.push('\\');
                out.push(c);
            }
            c => out.push(c),
        }
    }
    out
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
    /// Escape `$` as `\$` in string literals. Required for Dart targets:
    /// `$var` / `${expr}` inside a Dart string is interpolation, so an
    /// unescaped `$` from a payload executes arbitrary code (injection
    /// corpus finding, Task 3.1).
    pub dollar_escape: bool,
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
            let mut escaped = escape_literal_str(s, cfg.quote);
            if cfg.dollar_escape {
                escaped = escaped.replace('$', "\\$");
            }
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
                // Keys are string literals in the generated code: escape and
                // quote them (injection hardening — raw keys could terminate
                // the literal early on the target side).
                let items: Vec<_> = obj
                    .iter()
                    .map(|(k, v)| {
                        let mut escaped_key = escape_literal_str(k, cfg.quote);
                        if cfg.dollar_escape {
                            escaped_key = escaped_key.replace('$', "\\$");
                        }
                        format!(
                            "{c}{escaped_key}{c}: {}",
                            format_json_value(v, cfg),
                            c = cfg.quote
                        )
                    })
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
    dollar_escape: false,
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
    dollar_escape: true,
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
    dollar_escape: false,
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
    dollar_escape: false,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_object_keys_are_escaped_and_quoted() {
        let value = serde_json::json!({ "a\"b": 1 });
        let s = format_json_value(&value, &JSON_JS);
        assert!(!s.contains("a\"b"), "raw key leaked: {}", s);
        assert!(
            s.contains("\"a\\\"b\": 1"),
            "expected escaped quoted key: {}",
            s
        );
    }

    #[test]
    fn escape_literal_str_matches_chained_replace_semantics() {
        // Reference implementation: the original six-chained-`replace` form.
        // The single-pass rewrite must be byte-identical for the quote
        // characters used by all callers (`'"'` and `'\''`).
        fn chained(s: &str, quote: char) -> String {
            s.replace('\\', "\\\\")
                .replace(quote, &format!("\\{}", quote))
                .replace('\n', "\\n")
                .replace('\r', "\\r")
                .replace('\t', "\\t")
                .replace('\0', "\\0")
        }
        let cases: [(&str, char); 12] = [
            ("", '"'),
            ("plain text", '"'),
            ("back\\slash", '"'),
            ("quote \" inside", '"'),
            ("quote ' inside", '\''),
            ("tab\there", '\''),
            ("newline\nhere", '"'),
            ("cr\rhere", '"'),
            ("nul\0here", '"'),
            ("\\\\mixed\\\"\n\t\r\0", '"'),
            ("\\'both'\\\"quotes\"", '\''),
            ("a\"b'c\\d\ne\tf\0", '"'),
        ];
        for (input, quote) in cases {
            assert_eq!(
                escape_literal_str(input, quote),
                chained(input, quote),
                "mismatch for input {input:?} with quote {quote:?}"
            );
        }
    }

    #[test]
    fn escape_literal_str_exact_outputs() {
        assert_eq!(escape_literal_str("a\"b", '"'), "a\\\"b");
        assert_eq!(escape_literal_str("a'b", '\''), "a\\'b");
        assert_eq!(escape_literal_str("a\\b", '"'), "a\\\\b");
        assert_eq!(escape_literal_str("x\ny", '"'), "x\\ny");
        assert_eq!(escape_literal_str("x\0y", '"'), "x\\0y");
        // A quote of the OTHER kind passes through untouched.
        assert_eq!(escape_literal_str("it's", '"'), "it's");
    }

    #[test]
    fn json_object_keys_escape_backslash_and_dollar() {
        // A backslash in a key must not terminate the quote; a Dart `$` in a
        // key must not interpolate (mirrors string-value hardening).
        let value = serde_json::json!({ "a\\b": 1, "price$": 2 });
        let js = format_json_value(&value, &JSON_JS);
        assert!(js.contains("\"a\\\\b\""), "backslash key not escaped: {js}");
        let dart = format_json_value(&value, &JSON_DART);
        assert!(
            dart.contains(r"price\$"),
            "dollar key not escaped for Dart: {dart}"
        );
    }
}
