//! Cross-backend injection-payload corpus (Task 3.1 regression, extended by
//! the final T7.3 regression).
//!
//! For every UI backend × insertion point × payload, the compiler must
//! either reject the source (compile error) or emit output that contains no
//! executable injection pattern. The assertions run against the GENERATED
//! CODE, not just compilability — a payload that compiles into executable
//! `alert(...)` code fails the corpus even though tsc/esbuild would pass it
//! (review requirement: "tsc 能过 ≠ 安全").
//!
//! T7.3 additions: the `a11y` insertion point (Task 4.5 surface — a11y
//! block string values reach a distinct emission path from the `attr`
//! point) and the `x*/ alert(1) //` comment-terminator payload. Both caught
//! live Flutter gaps: the a11y `label` emitted raw Dart `${...}`
//! interpolation, and the unmappable-`role` degradation comment embedded
//! the role verbatim (`*/` terminated the block comment).
//!
//! v3-fixes additions (task 1.1): the `motion-transform` and `design-token`
//! insertion points (security HIGH-3 / HIGH-2 — keyframe string values and
//! design-token declaration values reach emission paths the existing nine
//! points do not: React inlines keyframe literals raw into
//! `translateX(<value>)`, and the shared CSS emitter writes token values
//! UNQUOTED into `--t1: <value>;`), the unquoted handler-attribute shapes
//! (bare `onmouseover=` forms in raw HTML), and the `.html` inline-`<script>`
//! EXEC_PATTERNS scan. The corpus goes RED here (emission fixes land in
//! tasks 1.2-1.4); the failure shapes below are their acceptance baseline.
//! Task 1.3: the handler-attribute scan is tokenizer-state-aware
//! ([`contains_live_handler_attr`]) — an `onXxx=` inside text content or
//! inside a quoted attribute VALUE is inert per HTML parsing and skipped —
//! and the static attribute-value shapes are REJECTED at compile time by
//! the shared lowering gate (a compile error is the accepted safe outcome;
//! see `omni_html_attribute_context_blocks_unquoted_handlers`).

use rustc_hash::FxHashMap;

use crate::codegen::Target;

/// Charset-validation diagnostic (design doc 域 9, verbatim).
const CLASS_MSG: &str = "类名仅允许字母数字下划线连字符";

/// Single-character payloads from the review corpus, attempted at every
/// insertion point.
const CHAR_PAYLOADS: &[&str] = &["'", "\"", "\\", "`", "${", "</"];

/// Entity-encoded quotes — the Vue template parser decodes ALL character
/// references in attribute values BEFORE parsing them as JS, so an escaping
/// layer that only handles `"` (not `&`) lets `&quot;`/`&#34;`/`&#x22;`
/// survive the replace and re-close the string in code position
/// (review HIGH-1). `&amp;` itself must survive as inert text too.
const ENTITY_PAYLOADS: &[&str] = &["&quot;", "&#34;", "&#x22;", "&amp;"];

/// Exploit-shaped payloads — each attempts a concrete breakout.
const EXPLOIT_PAYLOADS: &[&str] = &[
    "x',alert(1),'",
    "x\\';alert(1);//",
    "x\" onmouseover=\"alert(1)",
    "`${alert(1)}`",
    "</script><script>alert(1)</script>",
    // Entity-encoded breakout: after Vue's attribute decode this becomes
    // `show("x";alert(1)//")` — executable if `&` is not escaped first.
    "x&#34;;alert(1)//",
    "x&quot;;alert(1)//",
    // Bare call in an unquoted position — exercises the parser's
    // interpolation-parse fallback (`{`-leading Text nodes), which
    // backends must escape as plain text (review CRITICAL round 2).
    "alert(1)",
    // Comment-terminator breakout: terminates a Dart/JS block comment in
    // which a string value is embedded verbatim (Flutter a11y role
    // degradation comment), leaving `alert(1)` in code position.
    "x*/ alert(1) //",
    // Motion-transform breakout (security HIGH-3): keyframe string values
    // are inlined raw into `translateX(<value>)` — the raw `"` re-closes
    // the generated string, leaving `[alert(1)]` in code position. The
    // `x',alert(1),'` single-quote twin is the pre-existing payload above.
    "x\", [alert(1)] : \"",
    // Design-token / CSS declaration-value breakout (security HIGH-2):
    // token values emit UNQUOTED into `--t1: <value>;` — `}` terminates the
    // `:root` block, letting attacker CSS escape into the page. The `alert(`
    // marker MUST precede the exfil URL: the stripper treats the `//` in
    // `url(https://…)` as a line comment, so a marker placed in the URL
    // query (`?q=alert(1)`) is stripped away — a bare `}` escape alone is
    // invisible to EXEC_PATTERNS/HTML_PATTERNS/TPL (review Important-1).
    // `alert(` is not a `sanitize_css_value` blocked keyword
    // (javascript:/document./etc. are), so the value survives sanitization
    // and the residual carries `alert(`. The `</style><script` re-open
    // shape is terminated by the .omni lexer before lowering (compile error
    // — safe outcome) but stays in the corpus as a record for the
    // emitted-HTML guard.
    "x; } body { background: alert(1); url(https://evil.example/leak) }",
    "x</style><script>alert(1)</script>",
    // Unquoted inline-handler attribute (security HIGH-2 shape): a value
    // carrying `onmouseover=` without a quote forms a REAL handler
    // attribute in raw HTML (`<div title=x onmouseover=alert(1)>`).
    "x onmouseover=alert(1)",
];

/// Executable-code patterns that must never survive in generated output.
/// Checked after stripping string literals and comments (see
/// `strip_strings_and_comments`), so a payload legitimately rendered as
/// text/string content does not trip it.
const EXEC_PATTERNS: &[&str] = &["alert(", "document.cookie", "javascript:"];

/// Template-literal interpolation is executable only in JS-family code —
/// in Rust (`"x${"` is a plain string) and Dart output it is inert. The
/// stripper already breaks out of backtick strings at `${`, so for JS files
/// a residual `${` means a real template-literal breakout (or legit code the
/// corpus sources never produce).
const JS_TEMPLATE_PATTERN: &str = "${";

/// Tag-reopen breakout forms — dangerous wherever they appear (a re-opened
/// `<script>`/`<style>` context is live even from text position: the parser
/// switches to RAWTEXT and the injected element executes), so they are
/// checked raw, any position. A lone `</script>` IS legitimate in the
/// OmniView HTML wrapper, so the script check targets a re-opened tag; the
/// `</style><script` form catches a CSS-escape payload re-opening a
/// `<style>` context to inject `<script>` (design-token values emit into
/// CSS).
///
/// Handler-attribute patterns are NOT in this list: they are checked
/// tokenizer-state-aware by [`contains_live_handler_attr`] — an `onXxx=`
/// inside TEXT content or inside a quoted attribute VALUE is inert per HTML
/// parsing (the parser never turns it into an attribute boundary), while
/// one in attribute-name position in a tag (or raw inside a `<script>`/
/// `<style>` body, where an HTML string inside an innerHTML assignment
/// stays visible) is a real live handler. The quote state matters: an
/// ESCAPED attribute (`onmouseover=&quot;`) is inert data, while
/// `onmouseover="` in attribute position means a real inline handler
/// attribute was formed. No backend emits inline event-handler attributes
/// legitimately (verified across the codegen).
const HTML_REOPEN_PATTERNS: &[&str] = &["</script><script", "</style><script"];

/// HTML tokenizer state-awareness for the handler-attribute scan.
#[derive(Clone, Copy, PartialEq)]
enum HtmlScanState {
    /// Element text content — handler-shaped text is inert (never parsed
    /// as attributes).
    Data,
    /// After `<` — consuming the tag name.
    TagOpen,
    /// Inside a tag, between attributes.
    Tag,
    /// Inside a double-quoted attribute value.
    QuotedD,
    /// Inside a single-quoted attribute value.
    QuotedS,
    /// Inside an unquoted attribute value.
    Unquoted,
    /// Inside a `<script>`/`<style>` element body — raw-conservative scan.
    RawElement,
}

/// Handler-attribute names the raw-HTML scan flags (subset of the compiler
/// gate's full WHATWG whitelist — the corpus is the regression canary, the
/// shared lowering gate is the boundary).
const HANDLER_ATTRS: &[&[u8]] = &[b"onmouseover", b"onerror", b"onload", b"onclick"];

/// Whether `bytes[i..]` starts with an identifier boundary followed by the
/// handler name — i.e. the name is a standalone token, not part of a longer
/// identifier (`nonmouseover=`, `onmouseoveralert=`).
fn handler_boundary_ok(bytes: &[u8], i: usize) -> bool {
    i == 0
        || !(bytes[i - 1].is_ascii_alphanumeric() || bytes[i - 1] == b'_' || bytes[i - 1] == b'-')
}

/// Whether the handler name at `bytes[i..]` forms a REAL handler attribute:
/// name (boundary-checked) followed by `=` — optional whitespace between,
/// as HTML allows — whose value is NOT entity-escaped-quote-led (an
/// `onmouseover=&quot;` is escaped inert data, not an attribute boundary).
fn handler_breakout_at(bytes: &[u8], i: usize, name: &[u8]) -> bool {
    if !handler_boundary_ok(bytes, i)
        || i + name.len() > bytes.len()
        || &bytes[i..i + name.len()] != name
    {
        return false;
    }
    let mut j = i + name.len();
    while j < bytes.len() && bytes[j].is_ascii_whitespace() {
        j += 1;
    }
    if j >= bytes.len() || bytes[j] != b'=' {
        return false;
    }
    let after = &bytes[j + 1..];
    !(after.starts_with(b"&quot;")
        || after.starts_with(b"&#34;")
        || after.starts_with(b"&#x22;")
        || after.starts_with(b"&apos;")
        || after.starts_with(b"&#39;")
        || after.starts_with(b"&#x27;"))
}

/// Whether raw HTML contains a LIVE inline-handler attribute (`onXxx=` in
/// attribute-name position inside a tag, or raw inside a `<script>`/
/// `<style>` element body). Tokenizer-state aware: occurrences inside TEXT
/// content or inside a quoted/unquoted attribute VALUE are inert — the HTML
/// parser never turns them into attribute boundaries — so they are skipped
/// (task 1.3 corpus relaxation: the compiler gate rejects the
/// attribute-value shapes outright at compile time; text-position
/// occurrence of the shape is inert by construction and must not trip the
/// canary).
fn contains_live_handler_attr(content: &str) -> bool {
    let bytes = content.as_bytes();
    let n = bytes.len();
    let mut state = HtmlScanState::Data;
    let mut raw_element = false;
    let mut i = 0;
    while i < n {
        let c = bytes[i];
        match state {
            HtmlScanState::Data => {
                if c == b'<'
                    && i + 1 < n
                    && (bytes[i + 1].is_ascii_alphabetic()
                        || bytes[i + 1] == b'/'
                        || bytes[i + 1] == b'!')
                {
                    state = HtmlScanState::TagOpen;
                    i += 1;
                } else {
                    i += 1;
                }
            }
            HtmlScanState::TagOpen => {
                // Consume the tag name (until whitespace / '/' / '>').
                let name_start = i;
                while i < n
                    && !bytes[i].is_ascii_whitespace()
                    && bytes[i] != b'/'
                    && bytes[i] != b'>'
                {
                    i += 1;
                }
                raw_element =
                    &bytes[name_start..i] == b"script" || &bytes[name_start..i] == b"style";
                state = HtmlScanState::Tag;
            }
            HtmlScanState::Tag => {
                if c == b'>' {
                    state = if raw_element {
                        HtmlScanState::RawElement
                    } else {
                        HtmlScanState::Data
                    };
                    i += 1;
                } else if c == b'"' {
                    state = HtmlScanState::QuotedD;
                    i += 1;
                } else if c == b'\'' {
                    state = HtmlScanState::QuotedS;
                    i += 1;
                } else if c.is_ascii_whitespace() || c == b'/' {
                    i += 1;
                } else {
                    // Attribute-name position: a handler name followed by
                    // `=` is a live inline handler.
                    if HANDLER_ATTRS
                        .iter()
                        .any(|name| handler_breakout_at(bytes, i, name))
                    {
                        return true;
                    }
                    // Consume the attribute name (then `=` and its value).
                    while i < n
                        && !bytes[i].is_ascii_whitespace()
                        && bytes[i] != b'='
                        && bytes[i] != b'/'
                        && bytes[i] != b'>'
                    {
                        i += 1;
                    }
                    while i < n && bytes[i].is_ascii_whitespace() {
                        i += 1;
                    }
                    if i < n && bytes[i] == b'=' {
                        i += 1;
                        if i < n && bytes[i] == b'"' {
                            state = HtmlScanState::QuotedD;
                            i += 1;
                        } else if i < n && bytes[i] == b'\'' {
                            state = HtmlScanState::QuotedS;
                            i += 1;
                        } else {
                            state = HtmlScanState::Unquoted;
                        }
                    }
                }
            }
            HtmlScanState::QuotedD => {
                if c == b'"' {
                    state = HtmlScanState::Tag;
                }
                i += 1;
            }
            HtmlScanState::QuotedS => {
                if c == b'\'' {
                    state = HtmlScanState::Tag;
                }
                i += 1;
            }
            HtmlScanState::Unquoted => {
                if c.is_ascii_whitespace() {
                    state = HtmlScanState::Tag;
                } else if c == b'>' {
                    state = HtmlScanState::Data;
                }
                i += 1;
            }
            HtmlScanState::RawElement => {
                // Raw-conservative scan of `<script>`/`<style>` bodies: an
                // HTML string inside an innerHTML assignment stays visible.
                if HANDLER_ATTRS
                    .iter()
                    .any(|name| handler_breakout_at(bytes, i, name))
                {
                    return true;
                }
                if c == b'<' && i + 1 < n && bytes[i + 1] == b'/' {
                    state = HtmlScanState::TagOpen;
                    i += 1;
                } else {
                    i += 1;
                }
            }
        }
    }
    false
}

/// Split raw HTML into its `<script …>…</script>` regions — the only CODE
/// positions in a `.html` file. Markup text and attribute values are inert
/// and must not trip the EXEC_PATTERNS scan (payload text like `alert(1)`
/// legitimately renders into element text).
fn extract_script_regions(content: &str) -> Vec<&str> {
    let mut regions = Vec::new();
    let mut rest = content;
    while let Some(open) = rest.find("<script") {
        let after_open = &rest[open + "<script".len()..];
        let tag_end = after_open.find('>').unwrap_or(after_open.len());
        let body_start = open + "<script".len() + tag_end + 1;
        let Some(close_rel) = rest[body_start..].find("</script>") else {
            regions.push(&rest[body_start..]);
            break;
        };
        regions.push(&rest[body_start..body_start + close_rel]);
        rest = &rest[body_start + close_rel + "</script>".len()..];
    }
    regions
}

/// Escape a payload for embedding inside a double-quoted .omni string
/// literal, so it stays string CONTENT (the attack model: attacker-controlled
/// string flows through the compiler into generated code) instead of
/// re-parsing as source structure (`"` would terminate the literal).
fn embed(payload: &str) -> String {
    payload.replace('\\', "\\\\").replace('"', "\\\"")
}

/// Which string-literal semantics a generated file uses.
#[derive(Clone, Copy, PartialEq)]
enum ScanLang {
    /// JS family (.tsx/.js/.vue): `'…'` `"…"` `` `…` ``; `${…}` interpolates
    /// inside backtick strings only.
    Js,
    /// Rust (.rs): `'…'` char/string literals and `"…"` strings; backticks
    /// are ordinary content, `${` is inert.
    Rust,
    /// Dart (.dart): `'…'` `"…"`; `${…}` interpolates inside BOTH quote
    /// kinds (the Flutter codegen wraps interpolations as `'${expr}'`).
    Dart,
}

fn scan_lang_for(path: &str) -> ScanLang {
    if path.ends_with(".rs") {
        ScanLang::Rust
    } else if path.ends_with(".dart") {
        ScanLang::Dart
    } else {
        ScanLang::Js
    }
}

/// Strip string literals (backslash escapes honored) plus `//`, `/* */` and
/// `<!-- -->` comments from generated code, in a single pass (so `//` inside
/// a string is not mistaken for a comment). What remains is executable code
/// position. Generated runtimes legitimately discuss attack payloads in
/// comments/strings (e.g. the OmniView sanitizer notes
/// `"java\nscript:alert(1)"`), which is why comments are stripped too.
///
/// `${…}` interpolation is CODE, not string content — the stripper breaks
/// out of the string at it (per [`ScanLang`] rules) so executable patterns
/// inside interpolations are still caught.
fn strip_strings_and_comments(input: &str, lang: ScanLang) -> String {
    let chars: Vec<char> = input.chars().collect();
    let mut out = String::with_capacity(input.len());
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        // Line comment (// …)
        if c == '/' && i + 1 < chars.len() && chars[i + 1] == '/' {
            i += 2;
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
            if i < chars.len() {
                out.push('\n');
            }
            continue;
        }
        // Block comment (/* … */)
        if c == '/' && i + 1 < chars.len() && chars[i + 1] == '*' {
            i += 2;
            while i + 1 < chars.len() && !(chars[i] == '*' && chars[i + 1] == '/') {
                i += 1;
            }
            i = (i + 2).min(chars.len());
            continue;
        }
        // HTML comment (<!-- … -->)
        if c == '<'
            && i + 3 < chars.len()
            && chars[i + 1] == '!'
            && chars[i + 2] == '-'
            && chars[i + 3] == '-'
        {
            i += 4;
            while i + 2 < chars.len()
                && !(chars[i] == '-' && chars[i + 1] == '-' && chars[i + 2] == '>')
            {
                i += 1;
            }
            i = (i + 3).min(chars.len());
            continue;
        }
        // String literal. Interpolation (`${…}`) is CODE, not string content
        // — leave it in the residual so executable patterns inside it are
        // still caught.
        let is_string_delim = match lang {
            ScanLang::Js => c == '\'' || c == '"' || c == '`',
            ScanLang::Rust | ScanLang::Dart => c == '\'' || c == '"',
        };
        if is_string_delim {
            let quote = c;
            let interp_breaks = match lang {
                ScanLang::Js => quote == '`',
                ScanLang::Dart => true,
                ScanLang::Rust => false,
            };
            i += 1;
            let mut escaped = false;
            while i < chars.len() {
                let c2 = chars[i];
                if escaped {
                    escaped = false;
                    i += 1;
                    continue;
                }
                if c2 == '\\' {
                    escaped = true;
                    i += 1;
                    continue;
                }
                if interp_breaks && c2 == '$' && i + 1 < chars.len() && chars[i + 1] == '{' {
                    break;
                }
                i += 1;
                if c2 == quote {
                    break;
                }
            }
            continue;
        }
        out.push(c);
        i += 1;
    }
    out
}

/// Compile one .omni source to the given target, returning the full result.
fn compile_target(source: &str, target: Target) -> crate::InternalCompileResult {
    let mut sources = FxHashMap::default();
    sources.insert("input.omni".to_string(), source.to_string());
    let pm = crate::path_mapping::derive_path_mapping(&sources);
    crate::compile_multi_internal(sources, target, pm, false, false, None)
}

/// Decode the HTML character references the Vue template compiler decodes in
/// attribute values (single pass — `&amp;quot;` becomes literal `&quot;`
/// text, never a quote; the result is not re-scanned, matching HTML entity
/// semantics). Vue attribute values are CODE positions after this decode.
fn decode_html_entities(input: &str) -> String {
    input
        .replace("&quot;", "\"")
        .replace("&#34;", "\"")
        .replace("&#x22;", "\"")
        .replace("&#39;", "'")
        .replace("&#x27;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
}

/// Scan a .vue file to its executable residual (review MEDIUM-2:
/// `.vue` attribute code positions are decode-then-code).
///
/// A .vue SFC mixes three kinds of content; only two are code:
/// - `<script …>…</script>` sections — raw JS, stripped as JS.
/// - In the template: code-VALUED attribute spans (`@event`, `:binding`,
///   `v-*`) are decode-then-code — the Vue compiler decodes all character
///   references there BEFORE parsing the JS, so the span is entity-decoded
///   and then string-stripped (the decoded `"`s are the JS-level quotes);
///   `{{ … }}` interpolation spans are JS expressions, raw (never
///   entity-decoded), stripped as JS.
/// - Everything else — template markup and text (Vue renders it as escaped
///   text, inert), plain DATA attributes (`title="…"`, `data-*`), comments,
///   `<style>` — is dropped (cannot execute).
fn scan_vue(content: &str) -> String {
    let chars: Vec<char> = content.chars().collect();
    let n = chars.len();
    let mut out = String::with_capacity(content.len());
    let mut i = 0;
    while i < n {
        // `<script …>` … `</script>` region — raw JS.
        if i + 7 < n && chars[i..i + 7].iter().collect::<String>() == "<script" {
            let mut j = i + 7;
            while j < n && chars[j] != '>' {
                j += 1;
            }
            let open_end = (j + 1).min(n);
            let mut k = open_end;
            while k + 8 < n && chars[k..k + 9].iter().collect::<String>() != "</script>" {
                k += 1;
            }
            let region: String = chars[open_end..k].iter().collect();
            out.push_str(&strip_strings_and_comments(&region, ScanLang::Js));
            i = (k + 9).min(n);
            continue;
        }
        // Template attribute value span: `="` … `"`. Only CODE-VALUED
        // attributes are decode-then-code; plain attributes are DATA (the
        // decoded content never executes — dropped).
        if chars[i] == '=' && i + 1 < n && chars[i + 1] == '"' {
            let mut name_start = i;
            while name_start > 0
                && (chars[name_start - 1].is_ascii_alphanumeric()
                    || chars[name_start - 1] == '_'
                    || chars[name_start - 1] == '-'
                    || chars[name_start - 1] == '@'
                    || chars[name_start - 1] == ':')
            {
                name_start -= 1;
            }
            let name: String = chars[name_start..i].iter().collect();
            let code_valued =
                name.starts_with('@') || name.starts_with(':') || name.starts_with("v-");
            if code_valued {
                i += 2;
                let start = i;
                while i < n && chars[i] != '"' {
                    i += 1;
                }
                let value: String = chars[start..i].iter().collect();
                let decoded = decode_html_entities(&value);
                out.push_str(&strip_strings_and_comments(&decoded, ScanLang::Js));
                if i < n {
                    i += 1;
                }
                continue;
            }
        }
        // `{{ … }}` interpolation span — JS expression, not entity-decoded.
        if chars[i] == '{' && i + 1 < n && chars[i + 1] == '{' {
            let mut j = i + 2;
            while j + 1 < n && !(chars[j] == '}' && chars[j + 1] == '}') {
                j += 1;
            }
            let inner: String = chars[i + 2..j].iter().collect();
            out.push_str(&strip_strings_and_comments(&inner, ScanLang::Js));
            i = (j + 2).min(n);
            continue;
        }
        // Template text / markup / data attributes / comments — inert.
        i += 1;
    }
    out
}

/// Assert a successfully-compiled output is free of executable patterns.
///
/// - `.html` files are raw HTML: the event-handler attr / script-reopen
///   patterns are checked on the raw content (attribute values are not
///   string literals in the code sense — a `"` there IS a parse boundary;
///   the HTML parser tokenizes attributes on RAW text, so entity-decoding
///   does not create attribute boundaries).
/// - `.vue` files are decode-then-code: the Vue template compiler decodes
///   character references in attribute values BEFORE parsing them as JS,
///   so the entity-decoded form is scanned (review HIGH-1 blind spot).
/// - All other files are code: strings and comments are stripped first so a
///   payload legitimately rendered as text/string content does not trip the
///   check, while executable position (including template-literal
///   interpolation) is fully scanned.
fn assert_no_executable_patterns(
    result: &crate::InternalCompileResult,
    backend: &str,
    point: &str,
    payload: &str,
) {
    for (path, content) in &result.files {
        // Vendored pre-built runtime bundle (fixed boilerplate, not derived
        // from the payload) — contains legit template-literal code.
        if path == "omni-animation.js" {
            continue;
        }
        if path.ends_with(".html") {
            for pat in HTML_REOPEN_PATTERNS {
                assert!(
                    !content.contains(pat),
                    "[{backend}/{point}] raw HTML breakout {pat:?} leaked into {path} for payload {payload:?}:\n{content}"
                );
            }
            assert!(
                !contains_live_handler_attr(content),
                "[{backend}/{point}] live inline-handler attribute leaked into {path} for payload {payload:?}:\n{content}"
            );
            // Inline `<script>` regions are the only code positions in a
            // `.html` file: string/comment-strip each and run EXEC_PATTERNS,
            // so a payload breaking out of a string inside an inline script
            // is caught while inert markup text/attribute content is not.
            for (region_idx, region) in extract_script_regions(content).iter().enumerate() {
                let code = strip_strings_and_comments(region, ScanLang::Js);
                for pat in EXEC_PATTERNS {
                    assert!(
                        !code.contains(pat),
                        "[{backend}/{point}] executable pattern {pat:?} leaked into {path} <script> region {region_idx} for payload {payload:?}:\n{content}"
                    );
                }
            }
            continue;
        }
        let lang = scan_lang_for(path);
        let code = if path.ends_with(".vue") {
            scan_vue(content)
        } else {
            strip_strings_and_comments(content, lang)
        };
        for pat in EXEC_PATTERNS {
            assert!(
                !code.contains(pat),
                "[{backend}/{point}] executable pattern {pat:?} leaked into {path} for payload {payload:?}:\n{content}"
            );
        }
        if matches!(lang, ScanLang::Js) && !path.ends_with(".vue") {
            // In JS, `${` in residual code position means a real
            // template-literal breakout (Dart interpolations are
            // legitimate and their content is scanned via the break-out
            // rules above). Excluded for `.vue`: template attribute values
            // are single expressions where a bare `${` is invalid JS
            // (inert), and the Vue codegen never emits backtick literals
            // there (strings format as `"…"`).
            assert!(
                !code.contains(JS_TEMPLATE_PATTERN),
                "[{backend}/{point}] template-literal interpolation leaked into {path} for payload {payload:?}:\n{content}"
            );
        }
    }
}

/// Insertion-point source templates: payload embedded into a `class:` value,
/// a style value, a text node, an attribute value, an event handler, motion
/// keyframe strings, and design-token declaration values.
struct Point {
    name: &'static str,
    make_src: fn(&str) -> String,
}

fn class_src(payload: &str) -> String {
    format!("component App {{\n    view {{\n        <div class: \"x{payload}\"/>\n    }}\n}}\n")
}

fn style_src(payload: &str) -> String {
    format!(
        "component App {{\n    view {{\n        <div style: {{ color: \"x{payload}\" }}/>\n    }}\n}}\n"
    )
}

fn text_src(payload: &str) -> String {
    format!("component App {{\n    view {{\n        <div>{{\"x{payload}\"}}</div>\n    }}\n}}\n")
}

/// RAW text node (no interpolation) — exercises the plain-text escaping
/// paths (Dioxus/Flutter text emission), which the interpolation form does
/// not reach (review LOW-3). Payloads containing `{` or `<` break the .omni
/// parse — a compile error is an acceptable corpus outcome.
fn rawtext_src(payload: &str) -> String {
    format!("component App {{\n    view {{\n        <div>x{payload}</div>\n    }}\n}}\n")
}

/// RAW text followed by an interpolation child — the mixed-content
/// paragraph shape (`<p>价格 $total 元</p>`). Two sites so the payload text
/// actually flows through BOTH Flutter text paths the v4 review flagged
/// (rust HIGH — `$`-escaping regression): the all-text-concat
/// (flutter/view.rs:2193) via `<span>x{payload}{1}</span>` and the TextSpan
/// rich path (flutter/view.rs:332) via `<p>{payload}<b>{2}</b></p>`. The
/// payload must sit in a Text-MAPPED element: placed in the wrapper `<div>`
/// (→ Container) it routes through the main escaped Text emission
/// (view.rs:225, `$` → `\$`) and the regression is not observable — the
/// single-child rawtext point does not reach these sites either.
fn rawtext_multi_src(payload: &str) -> String {
    format!(
        "component App {{\n    view {{\n        <div><span>x{payload}{{1}}</span><p>{payload}<b>{{2}}</b></p></div>\n    }}\n}}\n"
    )
}

/// Malformed-interpolation fallback, LBrace shape (review CRITICAL round 2):
/// the parser emits `{`-leading TEXT nodes when interpolation parsing fails
/// (parser/view/mod.rs:111-194) — the payload lands UNQUOTED in the token
/// stream (empty-string tokens contribute nothing), so a backend that treats
/// the `{`-shaped text as code emits it executable (React JSX comma
/// operator: `{,,alert(1),,}`).
fn fallback_src(payload: &str) -> String {
    format!(
        "component App {{\n    view {{\n        <div>{{\"\" , {payload} , \"\"}}</div>\n    }}\n}}\n"
    )
}

/// Malformed-interpolation fallback, StringLiteral shape
/// (parser/view/mod.rs:87-88): the lexer decodes `\"` → `"` into the string
/// token and the fallback pushes the WHOLE string — braces AND decoded
/// quotes — as a Text node. A backend that treats `{`-shaped text as code
/// emits the decoded quotes intact, which is what makes the payload
/// executable (Dioxus `"{x", std::process::exit(0), "}"`, Vue
/// `{{ "", alert(document.cookie), "" }}`, React `{"", alert(1), ""}`).
fn fallback_str_src(payload: &str) -> String {
    format!(
        "component App {{\n    view {{\n        <div>\"{{\"\", {payload}, \"\"}}\"</div>\n    }}\n}}\n"
    )
}

fn attr_src(payload: &str) -> String {
    format!("component App {{\n    view {{\n        <div title=\"x{payload}\"/>\n    }}\n}}\n")
}

/// a11y block string values (Task 4.5 surface — added to the corpus by the
/// final T7.3 regression): `label`/`role` are string literals flowing into
/// React JSX attributes, Flutter Dart strings and degradation comments, and
/// OmniView HTML attributes — a distinct emission path from the `attr`
/// point's static-props loop (`a11y_react_attrs` / `wrap_a11y` /
/// `emit_a11y_attr`). The Flutter label path originally lacked the Dart
/// `$`-interpolation escaping (the text path's `$` → `\$`), and the Flutter
/// role-degradation comment embeds the role verbatim (a `*/` payload can
/// terminate the block comment).
fn a11y_src(payload: &str) -> String {
    format!(
        "component App {{\n    view {{\n        <div a11y: {{ label: \"x{payload}\", role: \"x{payload}\" }}/>\n    }}\n}}\n"
    )
}

fn event_src(payload: &str) -> String {
    format!(
        "component App {{\n    fn show(t) {{ return t }}\n    view {{\n        <div @onClick={{show(\"x{payload}\")}}/>\n    }}\n}}\n"
    )
}

/// Motion keyframe string values (security HIGH-3): `translateX`-style
/// values flow from MotionBodyIR stops into the backend keyframe emissions.
/// The React emitter inlines literal string values raw into
/// `translateX(<value>)` — an embedded `"` re-closes the generated string
/// and leaves the rest of the payload in code position (`[alert(1)]`). The
/// motion must be REFERENCED by a trigger (`@enter={slide}`) — unreferenced
/// motions are dropped by the codegen, which would make the point vacuously
/// green.
fn motion_src(payload: &str) -> String {
    // Both stops are STRING literals: the semantic layer's keyframe
    // type-mismatch check compares stop types per property, so mixing a
    // string `from` with a number `to` would be rejected before codegen.
    format!(
        "component App {{\n    @motion slide {{\n        from {{ translateX: \"x{payload}\" }}\n        to   {{ translateX: \"100px\" }}\n        over: 300ms\n    }}\n    view {{\n        <div @enter={{slide}}/>\n    }}\n}}\n"
    )
}

/// Design token declaration values (security HIGH-2): `token` values flow
/// from DesignIR into the shared CSS emitter as `--t1: <value>;` — written
/// UNQUOTED, so a `}` breakout terminates the `:root` block and payload
/// text lands in CSS declaration position.
fn design_src(payload: &str) -> String {
    format!(
        "design D {{\n    token t1: color = \"x{payload}\";\n}}\ncomponent App {{\n    view {{\n        <div class=\"Card\"/>\n    }}\n}}\n"
    )
}

/// Design blocks compile only on the OmniView, React, and Flutter targets —
/// Dioxus and Vue reject them outright (ECodegen diagnostic: "Design blocks
/// are only supported on the OmniView, React, and Flutter targets"), so the
/// `design-token` point is skipped there (every payload is a compile error
/// — safe, but the control source would not compile either).
fn design_target_supported(target: &Target) -> bool {
    matches!(target, Target::React | Target::OmniView | Target::Flutter)
}

const POINTS: &[Point] = &[
    Point {
        name: "class",
        make_src: class_src,
    },
    Point {
        name: "style",
        make_src: style_src,
    },
    Point {
        name: "text",
        make_src: text_src,
    },
    Point {
        name: "attr",
        make_src: attr_src,
    },
    Point {
        name: "a11y",
        make_src: a11y_src,
    },
    Point {
        name: "event",
        make_src: event_src,
    },
    Point {
        name: "rawtext",
        make_src: rawtext_src,
    },
    Point {
        name: "rawtext-multi",
        make_src: rawtext_multi_src,
    },
    Point {
        name: "fallback",
        make_src: fallback_src,
    },
    Point {
        name: "fallback_str",
        make_src: fallback_str_src,
    },
    Point {
        name: "motion-transform",
        make_src: motion_src,
    },
    Point {
        name: "design-token",
        make_src: design_src,
    },
];

/// The five UI backends. The corpus runs only when all are enabled
/// (all-backends feature).
#[cfg(all(
    feature = "codegen-react",
    feature = "codegen-omniview",
    feature = "codegen-dioxus",
    feature = "codegen-vue",
    feature = "codegen-flutter"
))]
const BACKENDS: &[(Target, &str)] = &[
    (Target::React, "react"),
    (Target::OmniView, "omniview"),
    (Target::Dioxus, "dioxus"),
    (Target::Flutter, "flutter"),
    (Target::Vue, "vue"),
];

#[test]
fn injection_payload_corpus() {
    for (target, backend) in BACKENDS {
        for point in POINTS {
            if point.name == "design-token" && !design_target_supported(target) {
                continue;
            }
            // Control: the template itself must compile with a safe payload —
            // otherwise the corpus would pass vacuously.
            let control = compile_target(&(point.make_src)("ok"), *target);
            assert!(
                control.success,
                "[{backend}/{}] control source must compile: {:?}",
                point.name, control.errors
            );

            for payload in CHAR_PAYLOADS
                .iter()
                .chain(ENTITY_PAYLOADS)
                .chain(EXPLOIT_PAYLOADS)
            {
                let embedded = embed(payload);
                let result = compile_target(&(point.make_src)(&embedded), *target);
                if point.name == "class" {
                    // Layer 1: every class-name payload must be rejected —
                    // either by the charset validation, or (payloads with
                    // `</`) by the .omni lexer terminating the string.
                    assert!(
                        !result.success,
                        "[{backend}/class] payload {payload:?} must fail compilation:\n{}",
                        result
                            .files
                            .values()
                            .cloned()
                            .collect::<Vec<_>>()
                            .join("\n")
                    );
                    // Even a failed compile's output must not carry the payload
                    // in executable position (runtimes legitimately mention
                    // `alert(` in comments/strings — check the stripped code).
                    let all_output: String = result
                        .files
                        .values()
                        .cloned()
                        .collect::<Vec<_>>()
                        .join("\n");
                    let stripped = strip_strings_and_comments(&all_output, ScanLang::Js);
                    assert!(
                        !stripped.contains("alert("),
                        "[{backend}/class] payload {payload:?} leaked into failed-compile output:\n{}",
                        all_output
                    );
                    // Payloads that reach lowering (no `</` to terminate the
                    // string at the lexer) must report the charset error.
                    if !payload.contains("</") {
                        assert!(
                            result.errors.iter().any(|d| {
                                d.severity == crate::error::Severity::Error
                                    && d.message.contains(CLASS_MSG)
                            }),
                            "[{backend}/class] payload {payload:?} must report {CLASS_MSG:?}, got: {:?}",
                            result.errors
                        );
                    }
                }
                if result.success {
                    assert_no_executable_patterns(&result, backend, point.name, payload);
                }
                // else: a compile error is an acceptable safe outcome.
            }
        }
    }
}

/// Task 1.2 dedicated regression — React motion transform string escaping
/// (security HIGH-1, the verified `[alert(1)]` computed-key vector).
/// Keyframe string values inline into `translateX(<value>)`; a raw `"` in
/// the value re-closes the generated JS string literal, leaving the rest of
/// the payload in code position, and a raw `` ` `` / `${` breaks out of the
/// template-literal wrapper used when a sibling stop part is an expression.
/// The escape must neutralize BOTH shapes (the corpus's motion-transform
/// point is the acceptance gate — this test pins the escaped shapes).
#[test]
fn motion_transform_string_is_escaped() {
    // `"`-reclose shape: the decoded `"` must be emitted as `\"` so it
    // cannot terminate the generated string.
    let reclose = r#"component App {
    @motion slide {
        from { translateX: "10px\", [alert(1)] : \"" }
        to   { translateX: "100px" }
        over: 300ms
    }
    view {
        <div @enter={slide}/>
    }
}
"#;
    let result = compile_target(reclose, Target::React);
    assert!(
        result.success,
        "control source must compile: {:?}",
        result.errors
    );
    let out: String = result
        .files
        .values()
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");
    // The escaped literal is emitted — `\"` for the payload's decoded `"`.
    assert!(
        out.contains(r#"translateX(10px\", [alert(1)] : \")"#),
        "expected escaped literal, got: {}",
        out
    );
    // `alert(` must not survive in EXECUTABLE position — string content is
    // inert (corpus semantics: scan the string-stripped code).
    let stripped = strip_strings_and_comments(&out, ScanLang::Js);
    assert!(
        !stripped.contains("alert("),
        "motion string leaked into JS code position: {}",
        out
    );

    // `${`-template shape: a literal `${`/backtick in the value must not
    // become template interpolation or terminate the template literal.
    let tpl = r#"component App {
    @motion slide {
        from { translateX: "10px${alert(1)}`" }
        to   { translateX: "100px" }
        over: 300ms
    }
    view {
        <div @enter={slide}/>
    }
}
"#;
    let tpl_result = compile_target(tpl, Target::React);
    assert!(
        tpl_result.success,
        "control source must compile: {:?}",
        tpl_result.errors
    );
    let tpl_out: String = tpl_result
        .files
        .values()
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");
    let tpl_stripped = strip_strings_and_comments(&tpl_out, ScanLang::Js);
    assert!(
        !tpl_stripped.contains("alert("),
        "motion template breakout leaked into JS code position: {}",
        tpl_out
    );
    assert!(
        !tpl_stripped.contains("${"),
        "motion template interpolation leaked into JS code position: {}",
        tpl_out
    );
    assert!(
        tpl_out.contains(r#"translateX(10px\${alert(1)}\`)"#),
        "expected template-escaped literal, got: {}",
        tpl_out
    );
}

/// The review's exact exploit payload, per backend: must be a compile error
/// carrying the charset-validation message (the full-pipeline shape of the
/// react-only `class_name_injection_rejected` test).
#[test]
fn class_injection_is_compile_error_all_backends() {
    let source = r##"
component App {
    view {
        <div class: "x',alert(document.cookie),'"/>
    }
}
"##;
    for (target, backend) in BACKENDS {
        let result = compile_target(source, *target);
        assert!(
            !result.success,
            "[{backend}] injected class name must fail compilation:\n{:?}",
            result.files
        );
        assert!(
            result.errors.iter().any(|d| d.message.contains(CLASS_MSG)),
            "[{backend}] expected {CLASS_MSG:?}, got: {:?}",
            result.errors
        );
    }
}

/// Handler-breakout rejection message (shared lowering gate, E0006).
const HANDLER_MSG: &str = "静态属性值禁止包含 HTML 事件处理器形状";

/// Task 1.3 dedicated regression — unquoted inline-handler attribute
/// breakout (security HIGH-2): a STATIC attribute value carrying `onXxx=`
/// (a WHATWG event-handler attribute name followed by `=`) would parse as a
/// real inline handler if the emission were ever unquoted, and keeps the
/// exploitable shape in the HTML artifact even quoted (`<div title="x
/// onmouseover=alert(1)"/>` — exactly the corpus's RED shape). Escaping
/// cannot eliminate it (the payload has no chars beyond the `&"'< >` escape
/// set), so the fix is compile-time rejection by the shared lowering gate —
/// a compile error is the corpus-accepted safe outcome.
#[test]
fn omni_html_attribute_context_blocks_unquoted_handlers() {
    let src = r#"component App {
    view {
        <div title="x onmouseover=alert(1)"/>
    }
}
"#;
    let result = compile_target(src, Target::OmniView);
    assert!(
        !result.success,
        "unquoted handler shape must fail compilation"
    );
    assert!(
        result
            .errors
            .iter()
            .any(|d| d.message.contains(HANDLER_MSG)),
        "expected {HANDLER_MSG:?}, got: {:?}",
        result.errors
    );
    // Even a failed compile's output must not carry the shape (the gate
    // sanitizes the value like validate_class_names does).
    let out: String = result
        .files
        .values()
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        !out.contains("onmouseover="),
        "unquoted handler leaked into failed-compile output: {}",
        out
    );
}

/// The rejection gate is a SHARED lowering layer — every backend rejects
/// the same shapes (no platform asymmetry), whitelist matching only (the
/// `one=` substring in `done=`/`gone=` must not false-positive), static
/// values only (dynamic/interpolated values are runtime-escaped, different
/// mechanism), and the same shapes on the style/a11y attribute surfaces.
#[test]
fn handler_gate_is_shared_and_whitelist_precise() {
    let src = r#"component App {
    view {
        <div title="x onmouseover=alert(1)"/>
    }
}
"#;
    for (target, backend) in BACKENDS {
        let result = compile_target(src, *target);
        assert!(
            !result.success,
            "[{backend}] unquoted handler shape must fail compilation"
        );
        assert!(
            result
                .errors
                .iter()
                .any(|d| d.message.contains(HANDLER_MSG)),
            "[{backend}] expected {HANDLER_MSG:?}, got: {:?}",
            result.errors
        );
    }

    // `done=` / `gone=` contain the `one=` substring — whitelist matching
    // must not reject legitimate values.
    let legit = r#"component App {
    view {
        <div title="x done=1 gone=2"/>
    }
}
"#;
    let result = compile_target(legit, Target::OmniView);
    assert!(
        result.success,
        "legitimate `done=`/`gone=` values must compile: {:?}",
        result.errors
    );

    // style and a11y literal values land in attributes too — same gate.
    for gated in [
        r#"component App {
    view {
        <div style: { color: "x onmouseover=alert(1)" }/>
    }
}
"#,
        r#"component App {
    view {
        <div a11y: { label: "x onmouseover=alert(1)" }/>
    }
}
"#,
    ] {
        let result = compile_target(gated, Target::OmniView);
        assert!(
            !result.success,
            "style/a11y literal with handler shape must fail compilation"
        );
    }

    // Dynamic (bound) values are NOT gated — runtime escaping is the
    // mechanism for them.
    let dynamic = r#"component App {
    signal t: String = "ok";
    view {
        <div title={t}/>
    }
}
"#;
    let result = compile_target(dynamic, Target::OmniView);
    assert!(
        result.success,
        "dynamic bound values must not be gated: {:?}",
        result.errors
    );
}

/// Pins the tokenizer-state-aware handler scan's sensitivity: real
/// handler attributes (quoted or bare, whitespace around `=`) are flagged,
/// while inert occurrences (text content, inside quoted/unquoted VALUES,
/// entity-escaped values, non-handler names) are skipped.
#[test]
fn live_handler_scan_is_tokenizer_state_aware() {
    // Real unquoted leaks — MUST flag.
    assert!(
        contains_live_handler_attr(r#"<div title=x onmouseover=alert(1)>"#),
        "unquoted leak"
    );
    assert!(
        contains_live_handler_attr(r#"<div onmouseover="alert(1)">"#),
        "quoted handler attr"
    );
    assert!(
        contains_live_handler_attr(r#"<div onmouseover = alert(1)>"#),
        "ws around ="
    );
    assert!(
        contains_live_handler_attr(r#"<div title=x onerror=alert(1)>"#),
        "other handler"
    );
    assert!(
        contains_live_handler_attr(
            r#"<script> x.innerHTML = '<div onmouseover=alert(1)>' </script>"#
        ),
        "script body"
    );
    assert!(
        contains_live_handler_attr(r#"<div onmouseover='alert(1)'>"#),
        "single-quoted handler"
    );
    assert!(
        contains_live_handler_attr(r#"<div onmouseover=alert(1) title="y">"#),
        "bare handler value"
    );
    assert!(
        contains_live_handler_attr(r#"<div nonmouseover=1 onclick=2>"#),
        "onclick after non-handler"
    );
    // Inert shapes — must NOT flag.
    assert!(
        !contains_live_handler_attr(r#"<div title="x onmouseover=alert(1)">"#),
        "quoted value inert"
    );
    assert!(
        !contains_live_handler_attr(r#"<div style="color:xx onmouseover=alert(1)">"#),
        "style value inert"
    );
    assert!(
        !contains_live_handler_attr("  xx onmouseover=alert(1)\n"),
        "text inert"
    );
    assert!(
        !contains_live_handler_attr(r#"<div title=onmouseover=alert(1)>"#),
        "unquoted value contains shape"
    );
    assert!(
        !contains_live_handler_attr(r#"<div onmouseover=&quot;alert(1)&quot;>"#),
        "entity-exempt"
    );
    assert!(
        !contains_live_handler_attr(r#"<div nonmouseover=1>"#),
        "nonmouseover not a handler"
    );
    assert!(
        !contains_live_handler_attr(r#"<div onclicker=1>"#),
        "onclicker not a handler"
    );
    assert!(
        !contains_live_handler_attr(r#"<onmouseover-el x=1>"#),
        "tag name not scanned"
    );
    assert!(
        !contains_live_handler_attr(r#"a < b onerror=c"#),
        "lt-in-text not tag"
    );
    assert!(
        !contains_live_handler_attr(r#"<div title='x onerror=alert(1)'>"#),
        "single-quoted value inert"
    );
}
