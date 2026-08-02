//! Cross-backend injection-payload corpus (Task 3.1 regression).
//!
//! For every UI backend × insertion point × payload, the compiler must
//! either reject the source (compile error) or emit output that contains no
//! executable injection pattern. The assertions run against the GENERATED
//! CODE, not just compilability — a payload that compiles into executable
//! `alert(...)` code fails the corpus even though tsc/esbuild would pass it
//! (review requirement: "tsc 能过 ≠ 安全").

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

/// Raw HTML breakout forms — dangerous even inside an emitted HTML string
/// (e.g. `class="x" onmouseover="alert(1)"` inside an innerHTML assignment).
/// The quote is part of the pattern: an ESCAPED attribute (`onmouseover=&quot;`)
/// is inert data, while `onmouseover="` means a real inline handler attribute
/// was formed. No backend emits inline event-handler attributes legitimately
/// (verified across the codegen). A lone `</script>` IS legitimate in the
/// OmniView HTML wrapper, so the script check targets a re-opened tag.
const HTML_PATTERNS: &[&str] = &[
    "onmouseover=\"",
    "onerror=\"",
    "onload=\"",
    "onclick=\"",
    "onmouseover='",
    "onerror='",
    "onload='",
    "onclick='",
    "</script><script",
];

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
            while k + 8 < n
                && chars[k..k + 9].iter().collect::<String>() != "</script>"
            {
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
            for pat in HTML_PATTERNS {
                assert!(
                    !content.contains(pat),
                    "[{backend}/{point}] raw HTML breakout {pat:?} leaked into {path} for payload {payload:?}:\n{content}"
                );
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
/// a style value, a text node, an attribute value, and an event handler.
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

fn attr_src(payload: &str) -> String {
    format!("component App {{\n    view {{\n        <div title=\"x{payload}\"/>\n    }}\n}}\n")
}

fn event_src(payload: &str) -> String {
    format!(
        "component App {{\n    fn show(t) {{ return t }}\n    view {{\n        <div @onClick={{show(\"x{payload}\")}}/>\n    }}\n}}\n"
    )
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
        name: "event",
        make_src: event_src,
    },
    Point {
        name: "rawtext",
        make_src: rawtext_src,
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
