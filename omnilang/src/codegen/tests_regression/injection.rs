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
//!
//! Task 9 (semantic-event-intent-phase1) additions: the `semantic-key`
//! insertion point — KeyEvent schema field READS (`e.key`/`e.code`，新
//! schema 字段名面) with the payload riding a handler string literal (the
//! D2-validated `e.key == "…"` comparison surface is covered by the
//! validate_semantic_keys tests instead, since non-key payloads would be
//! rejected before reaching codegen) — and the
//! `legacy_event_switch_messages_across_backends` pattern-missing assertion
//! (exact E0006 switch message per backend for @onClick/@click/@onTap/@tap →
//! @onPress and the mouseenter family → @onEnter/@onLeave; spec
//! 2026-08-08 §3.3 切换表事实).
//!
//! Task 7 (semantic-form-load-phase2) additions: the
//! `form_load_semantic_messages_across_backends` pattern-missing assertion —
//! the @form/@load 语义域诊断（spec 2026-08-08 §4.1/§4.2 的 V-F4/V-F7/V-F8：
//! 「未知校验规则 "@x"——合法规则: required/format/min/max/pattern/custom」
//! /「信号名 "@x" 与 @form 生成的保留名冲突」/「信号名 "@x" 与 @load 生成的
//! 保留名冲突」/「@load 仅支持 Array 返回（v0）」）是后端无关的共享语义层产物，
//! 逐字断言其跨全部 5 后端全链路可见（parse → semantic → diagnostics 不因
//! 目标差异而丢失/改写）。
//!
//! Task 7 (semantic-feedback-modal, 批次 1) additions: the
//! `feedback_modal_semantic_messages_across_backends` assertion — the
//! 反馈模态语义域诊断（spec 2026-08-09-semantic-batch1 §3 的 V1-V14：@modal
//! 值非布尔信号/非 dialog 标签/裸 dialog/重复绑定/@for 内、showToast kind
//! 非法/duration 负数/保留名、@skeleton 脱离 loading 分支/形状非法、
//! @errorBoundary 重复绑定/无 fallback 警告/fallback 槽重复）是后端无关的
//! 共享语义层产物，逐字断言其跨全部 5 后端全链路可见，且合法形态（三槽/
//! 无槽/嵌套/@enter 组合/信号开闭、toast 四形态、skeleton 两形状、boundary
//! 有/无 fallback）在全部后端 success 且无 Error 级诊断（V12/V13 为警告级，
//! 不翻转 success——lib.rs success 判定仅看 Error 级）。注：语料仅逐字断言
//! V12 警告通道；V13 消息由 validator 单元测试覆盖
//! （validate_feedback_modal.rs:691-760），此处合法元组全含标题故 V13 路径
//! 永不触发。

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
/// `${…}` interpolation is CODE, not string content — the stripper leaves
/// the expression in the residual so executable patterns inside
/// interpolations are still caught: Dart strings push the expression and
/// RESUME the string (the closing quote still closes it); JS template
/// literals break out at `${` (the break-out IS the closing semantic for
/// `` ` ``). Rust strings have no interpolation — consumed wholesale.
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
        // still caught. In Dart, `'…'`/`"…"` strings interpolate too (unlike
        // Rust, where `${` is inert, and unlike JS single/double quotes,
        // where only backticks interpolate).
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
                    if lang == ScanLang::Dart {
                        // Dart interpolation (`${expr}`): the expression is
                        // CODE position — push it to the residual (scanning
                        // nested string literals as inert content, brace-depth
                        // for the expression), then RESUME the string scan
                        // after the matching `}` — the string's real closing
                        // quote still closes it. The old break-out model
                        // re-scanned the string tail as code, so the closing
                        // quote was mistaken for a NEW string opener: quote
                        // pairing misaligned and later string CONTENT leaked
                        // into the residual as code — a corpus false positive
                        // on the correct `\$`-escaped output (v4 1.2
                        // rawtext-multi point, Flutter `$`-escape fix).
                        out.push('$');
                        out.push('{');
                        i += 2;
                        let mut depth = 1u32;
                        while i < chars.len() {
                            let c3 = chars[i];
                            if c3 == '{' {
                                depth += 1;
                                out.push(c3);
                                i += 1;
                            } else if c3 == '}' {
                                depth -= 1;
                                out.push(c3);
                                i += 1;
                                if depth == 0 {
                                    break;
                                }
                            } else if c3 == '\'' || c3 == '"' {
                                // String literal inside the expression — inert
                                // content, skipped like any string (escape-aware).
                                let inner_quote = c3;
                                i += 1;
                                let mut inner_escaped = false;
                                while i < chars.len() {
                                    let c4 = chars[i];
                                    if inner_escaped {
                                        inner_escaped = false;
                                        i += 1;
                                    } else if c4 == '\\' {
                                        inner_escaped = true;
                                        i += 1;
                                    } else {
                                        i += 1;
                                        if c4 == inner_quote {
                                            break;
                                        }
                                    }
                                }
                            } else {
                                out.push(c3);
                                i += 1;
                            }
                        }
                        continue;
                    }
                    // JS template literals: break out at `${` — the rest of
                    // the backtick string is scanned as code position (the
                    // break-out IS the closing semantic for `` ` ``).
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
        "component App {{\n    fn show(t) {{ return t }}\n    view {{\n        <div @onPress={{show(\"x{payload}\")}}/>\n    }}\n}}\n"
    )
}

/// KeyEvent 语义 schema 字段面（Task 9，spec §3.1 KeyEvent 行）：handler 内
/// 读取 `e.key`/`e.code`（新 schema 字段名）并携带 payload 于字符串字面量——
/// 字段提取机制按后端改写读取（JS `{raw}.key`/`{raw}.code`；Flutter
/// `logicalKey.keyLabel`/`physicalKey.keyId`，Task 7 映射）。刻意不用
/// `e.key == "…"` 比较：该面被 D2 语义键名枚举验证（validate_semantic_keys.rs）
/// 拦截，非键名 payload 在到达 codegen 前即被拒绝——`e.code` 读取是未验证的
/// schema 面（payload 在字符串字面量中流通），语义键名枚举面由 D2 验证测试
/// 独立覆盖。
fn semantic_key_src(payload: &str) -> String {
    format!(
        "component App {{\n    fn show(t) {{ return t }}\n    view {{\n        <div @onKeyDown={{(e) => {{ show(e.key); show(e.code); show(\"x{payload}\"); }}}}/>\n    }}\n}}\n"
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
        name: "semantic-key",
        make_src: semantic_key_src,
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

/// Task 9 模式缺失断言——旧名切换消息跨后端固定：语义事件分级（原则 8）
/// 的 E0006 硬错误消息是后端无关的共享语义层产物，但必须对每个后端全链路
/// 可见（parse → semantic → diagnostics 不因目标差异而丢失/改写）。spec
/// 2026-08-08 §3.3 切换表事实：@onClick/@click/@onTap/@tap → @onPress；
/// @onMouseEnter/@mouseenter → @onEnter；@onMouseLeave → @onLeave。消息逐字
/// 断言「事件名 "@x" 已弃用——使用 "@y"」（validate_event_names.rs error_diag
/// 格式）。注：@press 是运动触发器指令（TokenKind::AtPress，parser 实证），
/// 非事件名，不进事件校验——不在本表（spec §3.3 注记）。
#[test]
fn legacy_event_switch_messages_across_backends() {
    // (attr 拼写, 消息中的旧名, 迁移目标语义事件)
    let cases: &[(&str, &str, &str)] = &[
        ("@onClick={f}", "onClick", "onPress"),
        ("@click={f}", "click", "onPress"),
        ("@onTap={f}", "onTap", "onPress"),
        ("@tap={f}", "tap", "onPress"),
        ("@onMouseEnter={f}", "onMouseEnter", "onEnter"),
        ("@mouseenter={f}", "mouseenter", "onEnter"),
        ("@onMouseLeave={f}", "onMouseLeave", "onLeave"),
    ];
    for (target, backend) in BACKENDS {
        for (attr, legacy, semantic) in cases {
            let source = format!(
                "component App {{\n    fn f(e) {{}}\n    view {{\n        <div {attr} />\n    }}\n}}\n"
            );
            let result = compile_target(&source, *target);
            let expected = format!("事件名 \"@{legacy}\" 已弃用——使用 \"@{semantic}\"");
            assert!(
                !result.success,
                "[{backend}] `{attr}` 必须硬失败（旧名 {legacy} 已弃用），got: {:?}",
                result.errors
            );
            assert!(
                result.errors.iter().any(|d| {
                    d.severity == crate::error::Severity::Error && d.message == expected
                }),
                "[{backend}] `{attr}` 必须逐字产生 {expected:?}，got: {:?}",
                result.errors
            );
        }
    }
}

/// Task 7 模式缺失断言——@form/@load 语义域诊断消息跨后端固定（spec
/// 2026-08-08 §4.1/§4.2 的 V-F4/V-F7/V-F8）：语义层硬错误是后端无关产物，
/// 必须对每个后端全链路可见（parse → semantic → diagnostics 不因目标差异而
/// 丢失/改写）。消息逐字断言：「未知校验规则 "@x"——合法规则:
/// required/format/min/max/pattern/custom」（validate_form_load.rs V-F4，
/// VALIDATE_RULES join("/")）、「信号名 "@x" 与 @form 生成的保留名冲突」/
/// 「信号名 "@x" 与 @load 生成的保留名冲突」（V-F8）、
/// 「@load 仅支持 Array 返回（v0）」（V-F7）。
#[test]
fn form_load_semantic_messages_across_backends() {
    // (源, 期望逐字错误消息)——源的 @form/@load 语法与后端无关（语义层先于
    // codegen），每个源必须硬失败且逐字命中期望消息。
    let cases: &[(&str, &str)] = &[
        // V-F4 未知校验规则名：validate: 必须与 bind:value 配对（V-F3 通过）
        // 才会走到规则名检查。
        (
            "component App {\n    signal title: string = \"\"\n    fn submit(e) {}\n    view {\n        <form @form @onSubmit={submit}>\n            <input bind:value={title} validate:{bogus: true} />\n        </form>\n    }\n}\n",
            "未知校验规则 \"@bogus\"——合法规则: required/format/min/max/pattern/custom",
        ),
        // V-F8（@form 侧）：用户信号与 @form 生成的保留名冲突。
        (
            "component App {\n    signal formErrors: string = \"\"\n    signal title: string = \"\"\n    fn submit(e) {}\n    view {\n        <form @form @onSubmit={submit}>\n            <input bind:value={title} validate:{required: true} />\n        </form>\n    }\n}\n",
            "信号名 \"@formErrors\" 与 @form 生成的保留名冲突",
        ),
        // V-F8（@load 侧）：用户信号与 @load 生成的保留名冲突。
        (
            "component App {\n    signal loading: bool = false\n    fn fetchTodos() -> Array<String> {}\n    view {\n        @load={fetchTodos}\n        <div>{loading}</div>\n    }\n}\n",
            "信号名 \"@loading\" 与 @load 生成的保留名冲突",
        ),
        // V-F7：@load fn 返回类型非 Array（无返回类型注记）→ v0 硬错误。
        (
            "component App {\n    fn fetchName() {}\n    view {\n        @load={fetchName}\n        <div>x</div>\n    }\n}\n",
            "@load 仅支持 Array 返回（v0）",
        ),
    ];
    for (target, backend) in BACKENDS {
        for (source, expected) in cases {
            let result = compile_target(source, *target);
            assert!(
                !result.success,
                "[{backend}] 语义硬错误必须使编译失败，got: {:?}",
                result.errors
            );
            assert!(
                result.errors.iter().any(|d| {
                    d.severity == crate::error::Severity::Error && d.message == *expected
                }),
                "[{backend}] 必须逐字产生 {expected:?}，got: {:?}",
                result.errors
            );
        }
    }
}

// ════════════════════════════════════════════════════════════════════
// Task 13（批次 4 数据能力，spec 2026-08-09 §2.2/§3）——@load 扩展语义
// 注入语料 28 元组：S1-S24 成功形态 × React/OmniView/Flutter 三端逐字钉住
// 关键发射（命名实例派生信号 / call-form 参数绑定 / 派生 retry·reset /
// paginated 五信号 / paginated scroll 哨兵 / interval 轮询 / std/timing
// debounce·throttle）；E1-E4 语义硬错误 × 全部 5 BACKENDS 逐字断言
// （validate_form_load.rs V2/V7/V8 + ir/lower/component.rs V6）。
// ════════════════════════════════════════════════════════════════════

/// Task 13 成功元组三后端：React/OmniView/Flutter（Task 8/9/11 实机发射
/// 逐字核实；Dioxus/Vue 对 @load 为占位形态，见 stdlib/timing.rs 的
/// ALL_TARGETS 注册注释——故成功形态仅钉实测三端，错误形态仍跨 5 后端）。
const LOAD_BACKENDS: &[(Target, &str)] = &[
    (Target::React, "react"),
    (Target::OmniView, "omniview"),
    (Target::Flutter, "flutter"),
];

// ── Task 13 语料源（S1-S24）──

/// S1-S10 基础形态：call-form 参数绑定（fetchTodos(url)）+ 四态渲染 +
/// 派生 retry/reset（reset 由视图按钮引用——Flutter 非分页 notifier 发射
/// 受 reset_used 门控，flutter/mod.rs:1000-1002）。
const LOAD_BASIC_SRC: &str = "component App {\n    signal url: String = \"abc\"\n    async fn fetchTodos(url: String) -> Array<String> {\n        return [url];\n    }\n    view {\n        <div>\n            @load todos={fetchTodos(url)}\n            @if todosLoading {\n                <p>Loading</p>\n            }\n            @else if todosError != \"\" {\n                <p>Error: {todosError}</p>\n            }\n            @else if todosData == null {\n                <p>Waiting</p>\n            }\n            @else {\n                @for item in todosData {\n                    <div>{item}</div>\n                }\n            }\n            <button @onPress={todosRetry}>Retry</button>\n            <button @onPress={todosReset}>Reset</button>\n        </div>\n    }\n}\n";

/// S11-S18 分页形态：paginated 双实例（history 按钮分页 + feed 无限滚动，
/// scroll 容器为 <div style:{height/overflow}> 演示形态）。
const LOAD_PAGINATED_SRC: &str = "component App {\n    fn fetchHistory(page: i32) -> Page<String> {\n        return Page { items: [\"a\", \"b\"], hasMore: true };\n    }\n    fn fetchFeed(page: i32) -> Page<String> {\n        return Page { items: [\"a\"], hasMore: false };\n    }\n    view {\n        <div>\n            @load history={fetchHistory} paginated\n            @if historyLoading {\n                <p>Loading</p>\n            }\n            @else if historyError != \"\" {\n                <p>Error: {historyError}</p>\n            }\n            @else {\n                @for item in historyData {\n                    <div>{item}</div>\n                }\n                <button @onPress={historyLoadMore} disabled={!historyHasMore}>Load More</button>\n                <button @onPress={historyReset}>Reset</button>\n            }\n            <div style: { height: '300px', overflow: 'auto' }>\n                @load feed={fetchFeed} paginated scroll\n                @if feedLoading {\n                    <p>Loading</p>\n                }\n                @else if feedError != \"\" {\n                    <p>Error: {feedError}</p>\n                }\n                @else if feedData == null {\n                    <p>Scroll to load</p>\n                }\n                @else {\n                    @for item in feedData {\n                        <div>{item}</div>\n                    }\n                }\n            </div>\n        </div>\n    }\n}\n";

/// S19-S20 轮询形态：interval: 5000（冒号形式）定时重取。
const LOAD_INTERVAL_SRC: &str = "component App {\n    fn fetchPrices() -> Array<String> {\n        return [\"1\"];\n    }\n    view {\n        <div>\n            @load prices={fetchPrices} interval: 5000\n            @if pricesLoading {\n                <p>Loading</p>\n            }\n            @else if pricesError != \"\" {\n                <p>Error: {pricesError}</p>\n            }\n            @else if pricesData == null {\n                <p>Waiting</p>\n            }\n            @else {\n                @for item in pricesData {\n                    <div>{item}</div>\n                }\n            }\n        </div>\n    }\n}\n";

/// S21-S22 防抖形态：std/timing debounce 事件处理器（@onPress，button）。
/// 注意：Flutter 端 Timer 拦截（flutter/view.rs `format_inline_handler_flutter`）
/// 只命中 void 回调（onPressed 族，`callback_is_void`）——`@onInput` 映射的
/// `onChanged` 携带 event_data（非 void），走 `(e) {` 事件对象路径落入
/// 占位透传形态（timing.rs Dart/Flutter 臂 = fn 直传，无计时语义）。本形态
/// 与 Task 9 自身测试（flutter/tests.rs `flutter_debounce_timer_wrapper`）一致。
const LOAD_DEBOUNCE_SRC: &str = "import omniview;\nimport { debounce } from \"std/timing\";\n\ncomponent App {\n    async fn doSearch() -> String {\n        return \"\";\n    }\n    view {\n        <div>\n            <button @onPress={debounce(doSearch, 300)}>Search</button>\n        </div>\n    }\n}\n";

/// S23-S24 节流形态：std/timing throttle 事件处理器（@onPress）。
const LOAD_THROTTLE_SRC: &str = "import omniview;\nimport { throttle } from \"std/timing\";\n\ncomponent App {\n    fn doRefresh() {}\n    view {\n        <div>\n            <button @onPress={throttle(doRefresh, 2000)}>Refresh</button>\n        </div>\n    }\n}\n";

// ── Task 13 错误语料（E1-E4，消息逐字断言）──

/// E1: V2——实例名撞保留名 loading/error/data/retry/reset
/// （validate_form_load.rs:279-289）。
const LOAD_ERR_RESERVED_NAME_SRC: &str = "component App {\n    fn fetchTodos() -> Array<String> {\n        return [\"x\"];\n    }\n    view {\n        <div>\n            @load loading={fetchTodos}\n        </div>\n    }\n}\n";

/// E2: V6——匿名 @load 上限 1（ir/lower/component.rs:549 计数）。
const LOAD_ERR_ANON_TWICE_SRC: &str = "component App {\n    fn fetchTodos() -> Array<String> {\n        return [\"x\"];\n    }\n    fn fetchUsers() -> Array<String> {\n        return [\"y\"];\n    }\n    view {\n        <div>\n            @load={fetchTodos}\n            @load={fetchUsers}\n        </div>\n    }\n}\n";

/// E3: V7——分页 @load 必须命名（validate_form_load.rs:294-300）。
const LOAD_ERR_ANON_PAGINATED_SRC: &str = "component App {\n    fn fetchHistory(page: i32) -> Page<String> {\n        return Page { items: [\"x\"], hasMore: true };\n    }\n    view {\n        <div>\n            @load={fetchHistory} paginated\n        </div>\n    }\n}\n";

/// E4: V8——scroll 触发必须与 paginated 组合（validate_form_load.rs:303-309）。
const LOAD_ERR_SCROLL_ALONE_SRC: &str = "component App {\n    fn fetchTodos() -> Array<String> {\n        return [\"x\"];\n    }\n    view {\n        <div>\n            @load s={fetchTodos} scroll\n        </div>\n    }\n}\n";

/// S1-S24 × LOAD_BACKENDS：逐字钉住三端发射形态（substring 包含断言，
/// 输出 = 全部生成文件拼接）。每元组 (名称, 语料, [react, omniview, flutter])。
#[test]
fn load_extended_semantic_products_across_backends() {
    // (元组名, 语料源, 三端断言)——顺序对齐 LOAD_BACKENDS。
    let tuples: &[(&str, &str, [&str; 3])] = &[
        // S1 命名实例派生 loading 信号
        (
            "S1-loading",
            LOAD_BASIC_SRC,
            [
                "const [todosLoading, setTodosLoading] = useState(true);",
                "App_todos_loading.set(true);",
                "todosAsync.isLoading",
            ],
        ),
        // S2 派生 error 信号（catch 通道）
        (
            "S2-error",
            LOAD_BASIC_SRC,
            [
                "const [todosError, setTodosError] = useState(\"\");",
                "App_todos_error.set(String(e));",
                "todosAsync.error",
            ],
        ),
        // S3 派生 data 信号（成功通道 + @for 消费）
        (
            "S3-data",
            LOAD_BASIC_SRC,
            [
                "const [todosData, setTodosData] = useState<string[] | null>(null);",
                "App_todos_data.set(r);",
                "todosAsync.asData?.value ?? const []",
            ],
        ),
        // S4 派生 retry（React useCallback / OmniView 函数 / Flutter 重载）
        (
            "S4-retry",
            LOAD_BASIC_SRC,
            [
                "const todosRetry = useCallback(() => {",
                "function App_todos_retry() {",
                "ref.invalidate(todosProvider)",
            ],
        ),
        // S5 派生 reset
        (
            "S5-reset",
            LOAD_BASIC_SRC,
            [
                "const todosReset = useCallback(() => {",
                "function App_todos_reset() {",
                "ref.read(todosProvider.notifier).reset()",
            ],
        ),
        // S6 参数绑定：call-form loader 调用点
        (
            "S6-call-args",
            LOAD_BASIC_SRC,
            [
                "fetchTodos(url).then((r) => {",
                "Promise.resolve().then(() => App_fetchTodos(App_url.value)).then((r) => {",
                "build() async => fetchTodos(ref.watch(urlProvider));",
            ],
        ),
        // S7 参数绑定：依赖通道（React deps / Flutter watch）
        (
            "S7-deps",
            LOAD_BASIC_SRC,
            [
                "}, [todosRetry, url]);",
                "App_todos_retry();",
                "ref.watch(urlProvider)",
            ],
        ),
        // S8 参数绑定：loader 函数声明
        (
            "S8-fn-decl",
            LOAD_BASIC_SRC,
            [
                "const fetchTodos = async (url: string) => {",
                "async function App_fetchTodos(url) {",
                "Future<List<String>> fetchTodos(String url) async {",
            ],
        ),
        // S9 参数绑定：挂载即取
        (
            "S9-mount",
            LOAD_BASIC_SRC,
            [
                "useEffect(() => { todosRetry(); }, [todosRetry, url]);",
                "__omni_effect(() => { App_todos_retry(); });",
                "ref.watch(todosProvider);",
            ],
        ),
        // S10 Reset 非分页：data 归 null（Flutter 非分页 notifier 门控于
        // reset_used——视图按钮引用使 notifier 必发射）
        (
            "S10-reset-nonpag",
            LOAD_BASIC_SRC,
            [
                "setTodosData(null);",
                "App_todos_data.set(null);",
                "state = AsyncData(null);",
            ],
        ),
        // S11 Reset 分页：page/hasMore/loadingMore 全归位
        (
            "S11-reset-pag",
            LOAD_PAGINATED_SRC,
            [
                "setHistoryPage(1);",
                "App_history_has_more.set(true);",
                "state = AsyncData<List<String>>(const []);",
            ],
        ),
        // S12 paginated page 信号
        (
            "S12-page",
            LOAD_PAGINATED_SRC,
            [
                "setHistoryPage((p) => p + 1);",
                "App_history_page.set(App_history_page.value + 1);",
                "int page = 1;",
            ],
        ),
        // S13 paginated hasMore 信封校验（V12 运行时守卫）
        (
            "S13-hasmore-guard",
            LOAD_PAGINATED_SRC,
            [
                "typeof r.hasMore !== 'boolean'",
                "App_history_has_more.set(r.hasMore);",
                "if (r.hasMore == null) {",
            ],
        ),
        // S14 paginated loadingMore 信号
        (
            "S14-loadingmore",
            LOAD_PAGINATED_SRC,
            [
                "setHistoryLoadingMore(true);",
                "App_history_loading_more.set(true);",
                "bool loadingMore = false;",
            ],
        ),
        // S15 paginated loadMore 派生函数
        (
            "S15-loadmore",
            LOAD_PAGINATED_SRC,
            [
                "historyLoadMore({",
                "function App_history_load_more() {",
                "ref.read(historyProvider.notifier).loadMore()",
            ],
        ),
        // S16 paginated 数据追加（concat 而非覆盖）
        (
            "S16-append",
            LOAD_PAGINATED_SRC,
            [
                "setHistoryData((prev) => [...(prev ?? []), ...r.items]);",
                "App_history_data.set((App_history_data.value ?? []).concat(r.items));",
                "state = AsyncData<List<String>>([...(state.value ?? const []), ...(r.items ?? const [])]);",
            ],
        ),
        // S17 scroll 哨兵（IntersectionObserver / 滚动监听）
        (
            "S17-sentinel",
            LOAD_PAGINATED_SRC,
            [
                "const feedScrollSentinelRef = useRef<HTMLDivElement>(null);",
                "ctr.previousElementSibling",
                "final ScrollController _scrollControllerFeed = ScrollController();",
            ],
        ),
        // S18 scroll 触发阈值
        (
            "S18-threshold",
            LOAD_PAGINATED_SRC,
            [
                "}, { threshold: 0 });",
                "}, { threshold: 0.1 });",
                "maxScrollExtent - 200",
            ],
        ),
        // S19 interval 定时器（interval: 5000）
        (
            "S19-interval",
            LOAD_INTERVAL_SRC,
            [
                "const timer = setInterval(() => { pricesRetry(); }, 5000);",
                "setInterval(function() { App_prices_retry(); }, 5000)",
                "Timer.periodic(const Duration(milliseconds: 5000)",
            ],
        ),
        // S20 interval 卸载清理
        (
            "S20-interval-cleanup",
            LOAD_INTERVAL_SRC,
            [
                "return () => clearInterval(timer);",
                "clearInterval(App_prices_timer)",
                "_pollTimerPrices?.cancel();",
            ],
        ),
        // S21 debounce 事件包装（std/timing 首用点）
        (
            "S21-debounce",
            LOAD_DEBOUNCE_SRC,
            [
                "useMemo(() => ((_fn, _ms) => { let _t; return (..._a) => { clearTimeout(_t);",
                "const __timingonPress_el_button_00",
                "onPressed: () {\n    _omniTimers['debounce_0']?.cancel();",
            ],
        ),
        // S22 debounce 延迟窗口（300ms）
        (
            "S22-debounce-ms",
            LOAD_DEBOUNCE_SRC,
            [
                "})(doSearch_ref, 300)",
                "})(App_doSearch, 300);",
                "Timer(const Duration(milliseconds: 300), () => doSearch());",
            ],
        ),
        // S23 throttle 事件包装
        (
            "S23-throttle",
            LOAD_THROTTLE_SRC,
            [
                "useMemo(() => ((_fn, _ms) => { let _t; let _args;",
                "const __timingonPress_el_button_00",
                "if (_omniTimers['throttle_0']?.isActive ?? false) return;",
            ],
        ),
        // S24 throttle 窗口（2000ms）
        (
            "S24-throttle-ms",
            LOAD_THROTTLE_SRC,
            [
                "})(doRefresh_ref, 2000)",
                "})(App_doRefresh, 2000);",
                "Timer(const Duration(milliseconds: 2000), () => doRefresh());",
            ],
        ),
    ];
    for (name, source, pins) in tuples {
        for (i, (target, backend)) in LOAD_BACKENDS.iter().enumerate() {
            let result = compile_target(source, *target);
            assert!(
                result.success,
                "[{backend}/{name}] 语料必须编译成功，got: {:?}",
                result.errors
            );
            let output = result.files.values().cloned().collect::<Vec<_>>().join("\n");
            assert!(
                output.contains(pins[i]),
                "[{backend}/{name}] 缺失发射形态 {pins:?}，实际输出:\n{output}"
            );
        }
    }
}

/// E1-E4 × 全部 5 BACKENDS：@load 扩展语义硬错误为后端无关共享层产物
/// （语义/IR 先于 codegen），逐字断言不因目标差异丢失/改写。消息来源：
/// V2 保留名（validate_form_load.rs:279-289）、V6 匿名上限
/// （ir/lower/component.rs:549）、V7 分页必命名（validate_form_load.rs:294-300）、
/// V8 scroll 组合（validate_form_load.rs:303-309）。
#[test]
fn load_extended_error_messages_across_backends() {
    // (源, 期望逐字错误消息)——V7/V8 在 validate_form_load 内先于
    // V-F7/V11 触发（294-317 行序），匿名分页/裸 scroll 不会被信封检查抢占。
    let cases: &[(&str, &str)] = &[
        (
            LOAD_ERR_RESERVED_NAME_SRC,
            "@load 实例名称 `loading` 与保留名/信号 `loading` 冲突——保留名 loading/error/data/retry/reset 专供匿名实例，请更换实例名",
        ),
        (
            LOAD_ERR_ANON_TWICE_SRC,
            "组件内至多一个匿名 @load——多个数据源请使用命名实例（@load name={fn}），当前匿名 2 个",
        ),
        (
            LOAD_ERR_ANON_PAGINATED_SRC,
            "分页 @load 必须命名（@load name={fn} paginated）——分页信号 page/hasMore/loadingMore 需要实例归属",
        ),
        (
            LOAD_ERR_SCROLL_ALONE_SRC,
            "scroll 触发须与 paginated 组合使用（@load name={fn} paginated scroll）",
        ),
    ];
    for (target, backend) in BACKENDS {
        for (source, expected) in cases {
            let result = compile_target(source, *target);
            assert!(
                !result.success,
                "[{backend}] 语义硬错误必须使编译失败，got: {:?}",
                result.errors
            );
            assert!(
                result.errors.iter().any(|d| {
                    d.severity == crate::error::Severity::Error && d.message == *expected
                }),
                "[{backend}] 必须逐字产生 {expected:?}，got: {:?}",
                result.errors
            );
        }
    }
}

/// Task 7 (semantic-feedback-modal, 批次 1) regression — the 反馈模态语义域
/// 诊断（spec 2026-08-09-semantic-batch1 §3 的 V1-V14）是后端无关的共享语义
/// 层产物：逐字断言其跨全部 5 后端全链路可见（parse → semantic →
/// diagnostics 不因目标差异而丢失/改写），且合法形态（三槽/无槽/嵌套/@enter
/// 组合/信号开闭、toast 四形态、skeleton 两形状、boundary 有/无 fallback）
/// 在全部后端 success 且无 Error 级诊断（V12/V13 为警告级，不翻转
/// success——lib.rs success 判定仅看 Error 级）。注：语料仅逐字断言 V12
/// 警告通道；V13 消息由 validator 单元测试覆盖
/// （validate_feedback_modal.rs:691-760），此处合法元组全含标题故 V13 路径
/// 永不触发。语料源逐字镜像
/// validate_feedback_modal.rs 的 V1-V14 测试（组件名 Text/Button 替换为内建
/// p/button——语料上下文无声明组件）。
#[test]
fn feedback_modal_semantic_messages_across_backends() {
    // V1-V14 非法形态：13 元组，每元组 = (源, 逐字 V# 消息)。
    let illegal: &[(&str, &str)] = &[
        // V1 — "@modal" 的值必须是布尔信号
        (
            r#"component App { signal open: String = "x"; view { <dialog @modal={open} /> } }"#,
            r#""@modal" 的值必须是布尔信号"#,
        ),
        // V2 — "@modal" 只能用于 dialog 元素
        (
            r#"component App { signal open: bool = false; view { <div @modal={open} /> } }"#,
            r#""@modal" 只能用于 dialog 元素"#,
        ),
        // V3 — 裸 dialog（两种形态都禁）
        (
            r#"component App { view { <dialog>x</dialog> } }"#,
            r#"dialog 元素必须绑定 "@modal""#,
        ),
        (
            r#"component App { view { <dialog /> } }"#,
            r#"dialog 元素必须绑定 "@modal""#,
        ),
        // V4 — 重复绑定 "@modal"
        (
            r#"component App { signal open: bool = false; view { <dialog @modal={open} @modal={open} /> } }"#,
            r#"元素不能重复绑定 "@modal""#,
        ),
        // V5 — "@modal" 不能用在 @for 循环内
        (
            r#"component App { signal open: bool = false; signal items: Array<Int> = []; view { <div> @for item in items { <dialog @modal={open} /> } </div> } }"#,
            r#""@modal" 不能用在 @for 循环内"#,
        ),
        // V6 — showToast 未知 kind
        (
            r#"component App { fn doToast() { showToast("x", { kind: "bogus" }) } view { <div/> } }"#,
            r#"未知 toast 类型 "bogus"——合法类型: info/success/error"#,
        ),
        // V7 — showToast duration 负数
        (
            r#"component App { fn doToast() { showToast("x", { duration: -5 }) } view { <div/> } }"#,
            r#""showToast" 的 duration 不能为负数"#,
        ),
        // V8 — showToast 保留名（组件声明位）
        (
            r#"component showToast { view { <div/> } }"#,
            r#""showToast" 是内建语义函数，不能用作组件声明名"#,
        ),
        // V9 — "@skeleton" 脱离 @load 的 loading 分支
        (
            r#"component App { fn f() -> Array<Int> {} view { @load={f} <div @skeleton /> } }"#,
            r#""@skeleton" 只能用在 @load 的 loading 分支内（@if loading）"#,
        ),
        // V10 — @skeleton 未知形状
        (
            r#"component App { fn f() -> Array<Int> {} view { @load={f} @if loading { <div @skeleton="bogus" /> } } }"#,
            r#"未知骨架形状 "bogus"——合法形状: text/line/circle/card"#,
        ),
        // V11 — 重复绑定 "@errorBoundary"
        (
            r#"component App { view { <div @errorBoundary @errorBoundary /> } }"#,
            r#"元素不能重复绑定 "@errorBoundary""#,
        ),
        // V14 — fallback 槽重复声明（Text→p 内建替换）
        (
            r#"component App { view { <div @errorBoundary> @slot fallback { <p>A</p> } @slot fallback { <p>B</p> } </div> } }"#,
            r#""fallback" 槽重复声明——降级内容只能有一个"#,
        ),
    ];
    for (target, backend) in BACKENDS {
        for (source, expected) in illegal {
            let result = compile_target(source, *target);
            assert!(
                !result.success,
                "[{backend}] 语义硬错误必须使编译失败，got: {:?}",
                result.errors
            );
            assert!(
                result
                    .errors
                    .iter()
                    .any(|d| d.severity == crate::error::Severity::Error && d.message == *expected),
                "[{backend}] 必须逐字产生 {expected:?}，got: {:?}",
                result.errors
            );
        }
    }

    // 合法形态：13 元组，全部后端 success 且无 Error 级诊断。
    let legal: &[&str] = &[
        // 三槽（header/body/footer 命名槽）
        r#"component App { signal isOpen: bool = false; fn close() { isOpen = false } view { <dialog @modal={isOpen}> @slot header { <h1>确认删除</h1> } @slot body { <p>此操作不可撤销。</p> } @slot footer { <button @onPress={close}>取消</button> } </dialog> } }"#,
        // 无槽（匿名 children 落入 body 槽）
        r#"component App { signal open: bool = false; view { <dialog @modal={open}><h1>标题</h1></dialog> } }"#,
        // 嵌套容器内
        r#"component App { signal open: bool = false; fn close() { open = false } view { <div><dialog @modal={open}><h1>标题</h1><div><p>内容</p></div></dialog></div> } }"#,
        // @enter 组合（spec §2.1 规范形）
        r#"component App { @motion fadeIn { from { opacity: 0, translateY: 20 } to { opacity: 1, translateY: 0 } over: 400ms easing: ease-out } signal isOpen: bool = false; view { <dialog @modal={isOpen} @enter={fadeIn}><h1>标题</h1></dialog> } }"#,
        // 信号开闭（fn 写回信号）
        r#"component App { signal isOpen: bool = false; fn open() { isOpen = true } fn close() { isOpen = false } view { <button @onPress={open}>打开</button><dialog @modal={isOpen}><h1>标题</h1></dialog> } }"#,
        // toast 默认 kind（省略 kind，仅 duration）
        r#"component App { fn doToast() { showToast("保存成功", { duration: 100 }) } view { <div/> } }"#,
        // toast 三 kind（spec §2.2 规范形）
        r#"component App { fn save() { showToast("已保存", { kind: "info" }) showToast("保存成功", { kind: "success" }) showToast("保存失败", { kind: "error" }) } view { <div/> } }"#,
        // toast duration 0 = 不自动关闭
        r#"component App { fn doToast() { showToast("x", { duration: 0 }) } view { <div/> } }"#,
        // toast 省略 options（单参）
        r#"component App { fn doToast() { showToast("保存成功") } view { <div/> } }"#,
        // skeleton 布尔形（@load loading 分支内）
        r#"component App { fn f() -> Array<Int> {} view { @load={f} @if loading { <div @skeleton /> } } }"#,
        // skeleton 形状形
        r#"component App { fn f() -> Array<Int> {} view { @load={f} @if loading { <div @skeleton="card" /> } } }"#,
        // errorBoundary 有 fallback 槽
        r#"component App { view { <div @errorBoundary> @slot fallback { <p>加载失败</p> } </div> } }"#,
        // errorBoundary 无 fallback 槽（V12 警告级，不翻转 success）
        r#"component App { view { <div @errorBoundary /> } }"#,
    ];
    for (target, backend) in BACKENDS {
        for source in legal {
            let result = compile_target(source, *target);
            assert!(
                result.success,
                "[{backend}] 合法形态必须编译成功，got: {:?}",
                result.errors
            );
            assert!(
                !result
                    .errors
                    .iter()
                    .any(|d| d.severity == crate::error::Severity::Error),
                "[{backend}] 合法形态不得有 Error 级诊断，got: {:?}",
                result.errors
            );
        }
    }

    // V12 警告形态：success 保持 true，但必须产生逐字 Warning 级诊断。
    let v12_source = r#"component App { view { <div @errorBoundary /> } }"#;
    let v12_message = r#""@errorBoundary" 没有提供 fallback 槽——渲染失败时将显示空内容，建议添加 @slot fallback 降级内容"#;
    for (target, backend) in BACKENDS {
        let result = compile_target(v12_source, *target);
        assert!(
            result.success,
            "[{backend}] V12 警告不得翻转 success，got: {:?}",
            result.errors
        );
        assert!(
            result
                .errors
                .iter()
                .any(|d| d.severity == crate::error::Severity::Warning && d.message == v12_message),
            "[{backend}] 必须逐字产生 V12 警告，got: {:?}",
            result.errors
        );
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
