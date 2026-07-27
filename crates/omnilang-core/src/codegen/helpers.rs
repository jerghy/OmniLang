//! Shared codegen utility functions.
//!
//! Provides operator mapping, string case conversions, CSS formatting,
//! and expression block helpers used by multiple backends.

use crate::codegen::sanitize_css_value;
#[cfg(any(feature = "codegen-react", feature = "codegen-vue"))]
use crate::codegen::CodeWriter;

/// Map an OMNI binary operator IR to the target language's operator symbol.
/// - `strict_eq`: if true, uses `===`/`!==` (JS-style); otherwise `==`/`!=`
/// - `textual_bool`: if true, uses `and`/`or` (Python-style); otherwise `&&`/`||`
#[must_use]
pub fn binary_op_to_str(op: &crate::ir::BinaryOpIR, strict_eq: bool, textual_bool: bool) -> &str {
    use crate::ir::BinaryOpIR::*;
    match op {
        Add => "+",
        Sub => "-",
        Mul => "*",
        Div => "/",
        Mod => "%",
        Eq => {
            if strict_eq {
                "==="
            } else {
                "=="
            }
        }
        Neq => {
            if strict_eq {
                "!=="
            } else {
                "!="
            }
        }
        Lt => "<",
        Gt => ">",
        LtEq => "<=",
        GtEq => ">=",
        And => {
            if textual_bool {
                "and"
            } else {
                "&&"
            }
        }
        Or => {
            if textual_bool {
                "or"
            } else {
                "||"
            }
        }
        NullishCoalesce => "??",
        Assign => "=",
        AddAssign => "+=",
        SubAssign => "-=",
        MulAssign => "*=",
        DivAssign => "/=",
        ModAssign => "%=",
    }
}

/// Format a block IR node as a single expression by taking the last statement's value.
/// Calls `fmt` for expression/return statements, returns `fallback` for empty/non-value blocks.
#[must_use]
pub fn format_block_as_expr<F>(block: &crate::ir::BlockIR, fmt: &F, fallback: &str) -> String
where
    F: Fn(&crate::ir::ExprIR) -> String,
{
    if let Some(last) = block.statements.last() {
        match last {
            crate::ir::StatementIR::ExprStmtIR { expr } => fmt(expr),
            crate::ir::StatementIR::ReturnStmtIR { value } => {
                value.as_ref().map(fmt).unwrap_or_else(|| fallback.into())
            }
            _ => fallback.into(),
        }
    } else {
        fallback.into()
    }
}

/// Capitalize the first character of a string.
#[must_use]
pub fn capitalize_first(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}

/// Convert CamelCase or camelCase to separator-delimited lowercase.
/// "borderRadius" becomes "border-radius" (sep='-') or "border_radius" (sep='_').
/// Handles initial uppercase (PascalCase) without a leading separator.
#[must_use]
pub fn camel_to_separated(s: &str, sep: char) -> String {
    let mut result = String::new();
    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() {
            if i > 0 {
                result.push(sep);
            }
            result.push(c.to_ascii_lowercase());
        } else {
            result.push(c);
        }
    }
    result
}

/// Convert a camelCase CSS property name to kebab-case.
/// "backgroundColor" → "background-color", "--customProp" → "--custom-prop"
#[must_use]
pub fn camel_to_kebab(s: &str) -> String {
    camel_to_separated(s, '-')
}

/// Extract a CSS value string from an IR expression node.
/// Numbers get "px" suffix; strings are sanitized via sanitize_css_value.
#[must_use]
pub fn format_css_value(value: &crate::ir::ExprIR) -> String {
    match value {
        crate::ir::ExprIR::LiteralExpr { value } => match value {
            serde_json::Value::String(s) => sanitize_css_value(s),
            serde_json::Value::Number(n) => format!("{}px", n),
            _ => format!("{}", value),
        },
        _ => String::new(),
    }
}

/// Generate a CSS style block for component styles.
/// Handles both empty (generates default `.container {}`) and populated style lists.
#[cfg(any(feature = "codegen-react", feature = "codegen-vue"))]
pub fn generate_css_style_block(w: &mut CodeWriter, styles: &[crate::ir::StyleRuleIR]) {
    if styles.is_empty() {
        w.emit(".container {");
        w.emit("}");
        return;
    }
    for style in styles {
        let prefix = if style.is_tag_selector { "" } else { "." };
        w.emit_fmt(format_args!("{}{} {{", prefix, style.class_name));
        for (prop, value) in &style.properties {
            let css_val = format_css_value(value);
            w.emit_fmt(format_args!("  {}: {};", camel_to_kebab(prop), css_val));
        }
        w.emit("}");
    }
}
