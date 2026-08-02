//! Shared CSS-sanitizer JS helper emission — the single source of truth for
//! the runtime sanitizer pair, generated ONCE and emitted into BOTH the
//! OmniView runtime (`__omni_normalizeCssEscapes` + `__omni_sanitizeCss`)
//! and the React runtime helper (`normalizeCssEscapes` + `sanitizeCss` in
//! useDesignTokens.ts).
//!
//! Before this module existed the two backends emitted their own copies of
//! the sanitizer and drifted: the React copy lost hex-escape normalization,
//! `data:text/html`, `document.`/`window.` and the url() context checks, so
//! `\6a\61\76\61\73\63\72\69\70\74\3a alert(1)` (hex-escaped "javascript:")
//! bypassed it at runtime (three-copy drift, task 3.2).
//!
//! The emitted logic mirrors `sanitize_css_value` in
//! `crates/omnilang-core/src/codegen/sanitize.rs` 1:1: pre-strip CR/LF/TAB,
//! decode CSS hex escapes, post-strip escape-decoded CR/LF/TAB (the `\a`
//! → LF hole), then the keyword block and the url() context check. The
//! runtime copies additionally RETURN the decoded-and-stripped value so the
//! DOM receives exactly what was checked (no residual escape interpretation).

use std::fmt::Write as _;

/// Generate the sanitizer helper pair as a JS string.
///
/// - `sanitize_fn` / `normalize_fn`: emitted function names — the React
///   backend uses `sanitizeCss` / `normalizeCssEscapes` in .ts output; the
///   OmniView runtime uses `__omni_sanitizeCss` / `__omni_normalizeCssEscapes`
///   in plain ES5 JS.
/// - `ts_typed`: annotate parameter/return types (`value: unknown`,
///   `: string`) for the React .ts output. The plain-JS runtime emits
///   untyped ES5 (the regression suite asserts ES5 compatibility — no
///   arrow functions, no const/let).
#[must_use]
pub fn sanitize_css_helpers_js(sanitize_fn: &str, normalize_fn: &str, ts_typed: bool) -> String {
    let normalize_sig = if ts_typed {
        "(value: string): string"
    } else {
        "(value)"
    };
    let sanitize_sig = if ts_typed {
        "(value: unknown): string"
    } else {
        "(value)"
    };
    let mut out = String::with_capacity(4096);
    let _ = write!(
        out,
        r#"function {normalize_fn}{normalize_sig} {{
  // Normalize CSS hex escape sequences like \6a (→j) before pattern matching
  // to prevent attackers from bypassing keyword-based sanitizers.
  var result = '';
  var i = 0;
  var len = value.length;
  while (i < len) {{
    var ch = value.charAt(i);
    if (ch === '\\') {{
      i++;
      var hex = '';
      while (i < len && hex.length < 6 && /^[0-9a-fA-F]$/.test(value.charAt(i))) {{
        hex += value.charAt(i);
        i++;
      }}
      if (hex.length > 0) {{
        // Consume optional trailing space (CSS spec)
        if (i < len && value.charAt(i) === ' ') {{
          i++;
        }}
        var code = parseInt(hex, 16);
        // String.fromCodePoint throws RangeError for code points > 0x10FFFF
        // and surrogates (0xD800-0xDFFF). Skip these — they cannot represent
        // valid characters and preserving the raw escape is safer than crashing.
        // Matches Rust behavior: char::from_u32(code) returns None for these.
        if (!isNaN(code) && code <= 0x10FFFF && !(code >= 0xD800 && code <= 0xDFFF)) {{
          result += String.fromCodePoint(code);
          continue;
        }}
      }}
      // Not a valid hex escape, preserve backslash and next char
      result += '\\';
      if (i < len) {{
        result += value.charAt(i);
        i++;
      }}
    }} else {{
      result += ch;
      i++;
    }}
  }}
  return result;
}}

function {sanitize_fn}{sanitize_sig} {{
  var str = String(value);
  // Strip CR, LF, TAB before any pattern matching — browsers strip these
  // characters during CSS tokenization (CSS Syntax §4.3.7), so
  // "jav\r\nascript:" would be normalized by the browser to "javascript:".
  // This mirrors the same normalization in __omni_sanitizeUrl.
  str = str.replace(/[\r\n\t]/g, '');
  // Normalize CSS hex escapes before checking for dangerous patterns
  str = {normalize_fn}(str);
  // Strip again: escapes decode AFTER the pre-strip, so \a produces a
  // literal LF (\d a CR, \9 a TAB) here. Without this second strip,
  // "java\ascript:" — decoded to "java\nscript:" — would bypass the
  // keyword checks below. Mirrors sanitize_css_value in sanitize.rs.
  str = str.replace(/[\r\n\t]/g, '');
  var lower = str.toLowerCase();
  // Block protocol-level injection patterns
  if (lower.indexOf('javascript:') >= 0 || lower.indexOf('expression(') >= 0 ||
      lower.indexOf('-moz-binding') >= 0 ||
      lower.indexOf('data:text/html') >= 0 || lower.indexOf('@import') >= 0 ||
      lower.indexOf('vbscript:') >= 0 || lower.indexOf('document.') >= 0 ||
      lower.indexOf('window.') >= 0) {{
    return '';
  }}
  // Allow url() only with safe schemes. url() is needed for
  // background-image and other standard CSS resource loading.
  // Block url() containing javascript: or vbscript: schemes.
  // The outer block already catches standalone javascript:/vbscript:,
  // but they could appear inside a url() value after stripping.
  var urlIdx = lower.indexOf('url(');
  if (urlIdx >= 0) {{
    var afterUrl = lower.substring(urlIdx);
    if (afterUrl.indexOf('javascript:') >= 0 || afterUrl.indexOf('vbscript:') >= 0) {{
      return '';
    }}
  }}
  return str;
}}
"#
    );
    out
}

#[cfg(test)]
mod tests {
    use super::sanitize_css_helpers_js;

    #[test]
    fn emits_both_functions_with_requested_names() {
        let js = sanitize_css_helpers_js("__omni_sanitizeCss", "__omni_normalizeCssEscapes", false);
        assert!(js.contains("function __omni_normalizeCssEscapes(value) {"));
        assert!(js.contains("function __omni_sanitizeCss(value) {"));
        assert!(js.contains("str = __omni_normalizeCssEscapes(str);"));
    }

    #[test]
    fn ts_typed_variant_annotates_react_signatures() {
        let js = sanitize_css_helpers_js("sanitizeCss", "normalizeCssEscapes", true);
        assert!(js.contains("function sanitizeCss(value: unknown): string {"));
        assert!(js.contains("function normalizeCssEscapes(value: string): string {"));
    }

    #[test]
    fn emission_contains_full_keyword_and_url_checks() {
        let js = sanitize_css_helpers_js("s", "n", false);
        // Full keyword block (the React copy lost these — drift fix)
        for needle in [
            "javascript:",
            "expression(",
            "-moz-binding",
            "data:text/html",
            "@import",
            "vbscript:",
            "document.",
            "window.",
        ] {
            assert!(
                js.contains(needle),
                "emitted helper must contain keyword {needle:?}"
            );
        }
        // Hex-escape normalization + the post-normalize second strip
        assert!(js.contains("String.fromCodePoint"));
        assert!(js.contains("/^[0-9a-fA-F]$/"));
        assert!(js.contains("str = str.replace(/[\\r\\n\\t]/g, '');"));
        // url() context check
        assert!(js.contains("var urlIdx = lower.indexOf('url(');"));
        assert!(js.contains("afterUrl.indexOf('javascript:')"));
    }

    #[test]
    fn emission_is_es5_compatible() {
        let js = sanitize_css_helpers_js("__omni_sanitizeCss", "__omni_normalizeCssEscapes", false);
        assert!(!js.contains("const "));
        assert!(!js.contains("let "));
        assert!(!js.contains("=>"));
        assert!(!js.contains(".includes("));
    }
}
