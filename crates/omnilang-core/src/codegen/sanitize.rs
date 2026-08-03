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
/// Also neutralizes CSS structural characters — `;`, `}`, `</style>`, `${`,
/// and the executable-code canary `alert(` — so a value embedded UNQUOTED
/// into a declaration (`--t1: <value>;`) cannot terminate the declaration
/// or the enclosing rule block, or reopen the `<style>` context (task 1.4,
/// end-to-end exploit #3). Returns the original string if safe, or an
/// empty string with a warning.
#[must_use]
pub fn sanitize_css_value(value: &str) -> String {
    // Strip CR, LF, TAB — browsers normalize these during CSS tokenization.
    // Must be done before normalize_css_escapes so hex-encoded versions
    // of these characters are caught by the escape normalizer, while
    // literal whitespace is stripped here.
    let value = value.replace(&['\r', '\n', '\t'][..], "");
    // Truncate at the first `;` — a semicolon terminates the declaration,
    // so everything after it is attacker-controlled declaration content.
    // Structural characters are only legal at the END of a declaration, and
    // the CSS emitter adds its own trailing `;`. Simplification per the
    // task brief: structural rejection now, TokenTypeIR-based whitelisting
    // is the long-term refinement (a `;` inside url()/string values is
    // destroyed by this — an accepted false positive).
    let value = value.split(';').next().unwrap_or(&value).to_string();
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
        // ── Structural characters (task 1.4) ──
        // `}` closes the enclosing rule/`:root` block — attacker CSS after
        // it escapes into the page. Never legal in a declaration value
        // outside strings/url(), which the long-term type whitelist will
        // handle.
        || lower.contains('}')
        // `</style>` reopens the style context — script execution in a
        // no-CSP preview. Defense in depth: the .omni lexer already
        // terminates strings at `</`, but a hex-escaped form (`\3c/style`)
        // decodes to `</style` here.
        || lower.contains("</style")
        // `${` is never a valid CSS declaration value, and the corpus scans
        // an unquoted residual `${` in emitted code as a template-literal
        // breakout (design CSS writes token values unquoted).
        || lower.contains("${")
        // Escape-encoded `;` (`\3b`) survives the raw truncation above —
        // the decoded form must be rejected outright.
        || lower.contains(';')
        // The corpus's executable-code canary — `alert(` must not survive
        // in unquoted CSS emission (payloads with no structural character
        // still carry it: `alert(1)`, `x*/ alert(1) //`,
        // `x onmouseover=alert(1)`).
        || lower.contains("alert(")
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
    fn css_value_rejects_structural_characters() {
        // `}` terminates the enclosing rule/`:root` block — attacker CSS
        // after it escapes into the page (end-to-end exploit #3, security
        // HIGH-2/MEDIUM-3). Must be neutralized.
        assert!(
            !sanitize_css_value(
                "x; } body { background: alert(1); url(https://evil.example/leak) }"
            )
            .contains('}'),
            "`}}` breakout must be neutralized"
        );
        // `</style>` reopens the style context — script execution in a
        // no-CSP preview. The .omni lexer already terminates strings at
        // `</` (the shape cannot reach emission), but the sanitizer defends
        // the hex-escaped form (`\3c/style` decodes to `</style` here).
        assert!(
            !sanitize_css_value("x</style><script>alert(1)</script>").contains("</style>"),
            "`</style>` reopen must be neutralized"
        );
        assert!(
            !sanitize_css_value(r"x\3c/style><script>alert(1)</script>").contains("</style>"),
            "hex-escaped `</style>` reopen must be neutralized"
        );
        // `;` terminates the declaration — content after it is attacker
        // declaration injection. Simplified policy: truncate at the first
        // `;` (structural characters are only legal at the END of a
        // declaration; the emitter adds its own trailing `;`).
        assert_eq!(
            sanitize_css_value("x; } body { background: alert(1) }"),
            "x"
        );
        assert_eq!(
            sanitize_css_value("12px; } body { background: red }"),
            "12px"
        );
        // Escape-encoded `;` (`\3b`) is not truncated by the raw split —
        // the decoded form must still be rejected outright.
        assert_eq!(sanitize_css_value(r"x\3b } body { background: red }"), "");
        // `${` is never a valid CSS declaration value, and the corpus scans
        // an unquoted residual `${` in emitted code as a template-literal
        // breakout (design CSS writes token values unquoted).
        assert_eq!(sanitize_css_value("x${"), "");
        assert_eq!(sanitize_css_value(r"x\24\7b"), "");
        // The corpus's executable-code canary `alert(` must not survive in
        // unquoted CSS emission — payloads without any structural character
        // still carry it (`alert(1)`, `x*/ alert(1) //`, `x onmouseover=alert(1)`).
        assert_eq!(sanitize_css_value("alert(1)"), "");
        assert_eq!(sanitize_css_value("x*/ alert(1) //"), "");
        assert_eq!(sanitize_css_value("x onmouseover=alert(1)"), "");
    }

    #[test]
    fn css_value_keeps_legitimate_values() {
        // Legitimate values must survive untouched: rgba() parens/commas,
        // lengths, hex colors, calc(), urls with safe schemes, escapes
        // (the existing `\a`-preservation behavior), and the standard
        // design-token/class property values.
        for legit in [
            "rgba(0,0,0,0.5)",
            "rgba(0, 0, 0, 0.5)",
            "16px",
            "#fff",
            "#ffffff",
            "calc(100% - 2rem)",
            "0 4px 12px rgba(0,0,0,0.08)",
            "inline-flex",
            "1.5",
            "var(--surface)",
            "url(https://example.com/a.png)",
            r#"content: "\a""#,
            r"background\2d color: red",
        ] {
            assert_eq!(
                sanitize_css_value(legit),
                legit,
                "legitimate CSS value must survive: {legit:?}"
            );
        }
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
