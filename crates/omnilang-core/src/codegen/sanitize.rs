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
    // Strip CR, LF, TAB — browsers normalize these during CSS tokenization.
    // Must be done before normalize_css_escapes so hex-encoded versions
    // of these characters are caught by the escape normalizer, while
    // literal whitespace is stripped here.
    let value = value.replace(&['\r', '\n', '\t'][..], "");
    // Normalize CSS escape sequences to prevent bypasses
    let normalized = normalize_css_escapes(&value);
    // Strip again: escapes decode AFTER the pre-strip, so `\a` produces a
    // literal LF (`\d` a CR, `\9` a TAB) here. Without this second strip,
    // "java\ascript:" — decoded to "java\nscript:" — would slip past the
    // keyword checks below. The returned value keeps the raw escapes so
    // legitimate CSS like `content: "\a"` survives; only the CHECK sees the
    // decoded-and-stripped form (which is exactly what a browser tokenizes).
    let normalized = normalized.replace(&['\r', '\n', '\t'][..], "");
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

/// Check whether a static URL attribute value contains a dangerous protocol
/// scheme. Returns `true` if the value should be blocked.
///
/// This mirrors `__omni_sanitizeUrl` in the generated JS runtime, catching
/// dangerous URLs in literal attribute values at compile time as defense-in-depth
/// against supply-chain compromises where a third-party .omni component
/// contains literal `javascript:` or `data:` URLs.
#[must_use]
pub fn is_dangerous_url(value: &str) -> bool {
    let trimmed = value.trim();
    let lower = trimmed.to_ascii_lowercase();
    // Strip CR, LF, TAB — browsers strip these before URL navigation
    let stripped: String = lower
        .chars()
        .filter(|c| !matches!(c, '\n' | '\r' | '\t'))
        .collect();
    stripped.starts_with("javascript:")
        || stripped.starts_with("data:")
        || stripped.starts_with("vbscript:")
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

#[cfg(test)]
mod tests {
    use super::{is_dangerous_url, sanitize_css_value};

    #[test]
    fn test_sanitize_css_value_blocks_hex_escape_bypass() {
        // Hex-escaped "javascript:" — the payload that pierced the React
        // runtime copy before the single-source helper (task 3.2).
        assert_eq!(
            sanitize_css_value(r"\6a\61\76\61\73\63\72\69\70\74\3a alert(1)"),
            "",
            "hex-escaped javascript: must be blocked"
        );
        // Hex-escaped javascript: inside url()
        assert_eq!(
            sanitize_css_value(r"url(\6a\61\76\61\73\63\72\69\70\74\3a alert(1))"),
            "",
            "hex-escaped javascript: inside url() must be blocked"
        );
    }

    #[test]
    fn test_sanitize_css_value_blocks_escape_decoded_whitespace() {
        // Review LOW: `\a` decodes to LF (`\d` to CR, `\9` to TAB) AFTER the
        // pre-strip, so the post-normalize strip must catch it or
        // "java\ascript:" (decoded "java\nscript:") bypasses the keywords.
        assert_eq!(
            sanitize_css_value(r"url(java\ascript:alert(1))"),
            "",
            "\\a-decoded LF in url() javascript: must be blocked"
        );
        assert_eq!(
            sanitize_css_value(r"url(java\dscript:alert(1))"),
            "",
            "\\d-decoded CR in url() javascript: must be blocked"
        );
        assert_eq!(
            sanitize_css_value(r"url(java\9script:alert(1))"),
            "",
            "\\9-decoded TAB in url() javascript: must be blocked"
        );
        // Plain (non-url) escape-decoded newline must also be blocked
        assert_eq!(
            sanitize_css_value(r"java\ascript:alert(1)"),
            "",
            "\\a-decoded LF in plain javascript: must be blocked"
        );
        // Legitimate escapes must survive in the RETURNED value (compile-time
        // output keeps the raw escape so browsers decode it — e.g. the common
        // `content: "\a"` newline escape must not be destroyed).
        assert_eq!(
            sanitize_css_value(r#"content: "\a""#),
            r#"content: "\a""#,
            "legitimate \\a content escape must be preserved in the return"
        );
        assert_eq!(
            sanitize_css_value(r"background\2d color: red"),
            r"background\2d color: red",
            "legitimate safe hex escape must be preserved in the return"
        );
    }

    #[test]
    fn test_is_dangerous_url() {
        assert!(is_dangerous_url("javascript:alert(1)"));
        assert!(is_dangerous_url("  javascript:void(0)"));
        assert!(is_dangerous_url("data:text/html,<script>alert(1)</script>"));
        assert!(is_dangerous_url("data:image/svg+xml,..."));
        assert!(is_dangerous_url("vbscript:msgbox(1)"));
        // Newline/tab/CR bypass: browsers strip these before URL navigation
        assert!(is_dangerous_url("java\nscript:alert(1)"));
        assert!(is_dangerous_url("java\tscript:alert(1)"));
        assert!(is_dangerous_url("java\rscript:alert(1)"));
        assert!(!is_dangerous_url("https://example.com"));
        assert!(!is_dangerous_url("/relative/path"));
        assert!(!is_dangerous_url("#anchor"));
        assert!(!is_dangerous_url("mailto:user@example.com"));
    }
}
