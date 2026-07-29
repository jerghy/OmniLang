//! Identifier and value sanitizers for code generation.
//!
//! Provides functions to sanitize CSS values, module names, and
//! JavaScript identifiers to prevent injection attacks and syntax errors.

/// Normalize CSS hex escape sequences to their actual characters.
///
/// CSS allows hex escapes like `\6a` to represent ASCII characters.
/// An attacker could encode `javascript:` as `\6a\61\76\61\73\63\72\69\70\74\3a`
/// to bypass keyword-based sanitizers. This function converts those
/// escapes before pattern matching so the sanitizer catches the decoded form.
///
/// Handles 1-6 hex digits per the CSS spec, consuming optional trailing
/// whitespace after the escape sequence.
#[must_use]
pub fn normalize_css_escapes(value: &str) -> String {
    let mut result = String::with_capacity(value.len());
    let mut chars = value.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' {
            // Collect up to 6 hex digits
            let mut hex = String::with_capacity(6);
            while let Some(&h) = chars.peek() {
                if h.is_ascii_hexdigit() && hex.len() < 6 {
                    hex.push(h);
                    chars.next();
                } else {
                    break;
                }
            }
            if !hex.is_empty() {
                // CSS hex escapes consume optional trailing whitespace (space only)
                if let Some(&' ') = chars.peek() {
                    chars.next();
                }
                if let Ok(code) = u32::from_str_radix(&hex, 16) {
                    if let Some(converted) = char::from_u32(code) {
                        result.push(converted);
                        continue;
                    }
                }
            }
            // Not a valid hex escape, preserve the backslash and following character
            result.push(c);
            if let Some(next) = chars.next() {
                result.push(next);
            }
        } else {
            result.push(c);
        }
    }
    result
}

/// Sanitize a CSS value string to prevent injection attacks.
///
/// First normalizes CSS hex escape sequences to prevent bypasses.
/// Rejects dangerous CSS value patterns that could execute JavaScript:
/// `javascript:`, `expression(`, `-moz-binding`, `data:text/html`.
/// Allows `url()` with safe schemes but blocks javascript:/vbscript: inside url().
/// Returns the original string if safe, or an empty string with a warning.
#[must_use]
pub fn sanitize_css_value(value: &str) -> String {
    // First normalize CSS escape sequences to prevent bypasses
    let normalized = normalize_css_escapes(value);
    let lower = normalized.to_lowercase();

    if lower.contains("javascript:")
        || lower.contains("expression(")
        || lower.contains("-moz-binding")
        || lower.contains("data:text/html")
        || lower.contains("vbscript:")
        || lower.contains("@import")
        || lower.contains("document.")
        || lower.contains("window.")
    {
        // Return a safe fallback — the dangerous value is suppressed
        return String::new();
    }

    // Allow url() but only with safe schemes — block javascript:/vbscript: inside url()
    if let Some(idx) = lower.find("url(") {
        let after_url = &lower[idx..];
        if after_url.contains("javascript:") || after_url.contains("vbscript:") {
            return String::new();
        }
    }

    value.to_string()
}

/// Sanitize a module name for use as a file path component.
///
/// Strips directory traversal sequences (`..`, `/`, `\`) and leading dots
/// to prevent path traversal when constructing output file paths.
/// The actual file I/O is handled by the Python CLI; this ensures
/// generated paths are safe regardless.
#[must_use]
pub fn sanitize_module_name(name: &str) -> String {
    let sanitized = name
        .strip_suffix(".omni")
        .unwrap_or(name)
        .replace("..", "_")
        .replace(['/', '\\', ':'], "_")
        .trim_start_matches('.')
        .to_string();
    // Guard against empty result (e.g., name was just "." or all dots)
    if sanitized.is_empty() {
        "module".to_string()
    } else {
        sanitized
    }
}

/// Sanitize an identifier for use in JavaScript/TypeScript output.
///
/// Appends `_` to names that collide with JavaScript reserved keywords
/// (e.g. `default` → `default_`, `class` → `class_`).
/// This prevents syntax errors when OmniLang identifiers use names
/// that are valid in Omni but reserved in JavaScript.
#[must_use]
pub fn sanitize_js_ident(name: &str) -> String {
    // JavaScript reserved words (ES5+ strict mode) that cannot be used as identifiers
    const JS_RESERVED: &[&str] = &[
        "break",
        "case",
        "catch",
        "continue",
        "debugger",
        "default",
        "delete",
        "do",
        "else",
        "finally",
        "for",
        "function",
        "if",
        "in",
        "instanceof",
        "new",
        "return",
        "switch",
        "this",
        "throw",
        "try",
        "typeof",
        "var",
        "void",
        "while",
        "with",
        // ES6+
        "class",
        "const",
        "enum",
        "export",
        "extends",
        "import",
        "super",
        "implements",
        "interface",
        "let",
        "package",
        "private",
        "protected",
        "public",
        "static",
        "yield",
        // ES2017+
        "async",
        "await",
        // Always reserved as literals / special bindings
        "arguments",
        "null",
        "true",
        "false",
    ];

    if JS_RESERVED.contains(&name) {
        format!("{}_", name)
    } else {
        name.to_string()
    }
}
