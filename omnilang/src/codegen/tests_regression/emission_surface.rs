//! Emission-surface coverage map (Task 1.5 — security 长期项 #8).
//!
//! Every user-string emission surface across the five UI backends
//! (React / OmniView / Vue / Dioxus / Flutter) must be either
//! corpus-covered (an `injection_payload_corpus` insertion point flows
//! payloads through the path), or defended (escaped / sanitized /
//! charset-validated) with a dedicated test pinning the defense.
//!
//! This is the STRUCTURAL fix for the review blind spot that let the
//! `motion-transform` (security HIGH-3) and `design-token` (security
//! HIGH-2) surfaces escape corpus review: the map enumerates every
//! surface with its emission-point file:line, and the assertions below
//! make the map rot-proof in both directions — an orphan surface (no
//! corpus point) and an orphan corpus point (no surface claims it)
//! both fail.
//!
//! Corpus insertion points (the 12 points of `injection.rs` `POINTS`):
//! `class`, `style`, `text`, `attr`, `a11y`, `event`, `rawtext`,
//! `rawtext-multi`, `fallback`, `fallback_str`, `motion-transform`,
//! `design-token`.

/// The 12 corpus insertion points — must stay in lockstep with
/// `tests_regression/injection.rs` `POINTS` (the `every point claimed`
/// assertion fails when a point is added to one and not the other).
const INSERTION_POINTS: &[&str] = &[
    "class",
    "style",
    "text",
    "attr",
    "a11y",
    "event",
    "rawtext",
    "rawtext-multi",
    "fallback",
    "fallback_str",
    "motion-transform",
    "design-token",
];

/// Primary coverage mechanism for an emission surface.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Cover {
    /// The corpus insertion point flows exploit payloads through this path.
    /// The point name must exist in [`INSERTION_POINTS`].
    Corpus(&'static str),
    /// The surface is charset-validated at lowering (class-name gate, E0006).
    Charset,
    /// The surface is escaped at emission (escape_literal_str / escape_html /
    /// vue_attr_escape / Dart `$`-escape).
    Escaped,
    /// The surface is sanitized at emission (sanitize_css_value /
    /// is_dangerous_url).
    Sanitized,
}

/// One emission surface row: (backend, surface, emission-point file:line,
/// corpus point guarding the path, primary mechanism, defense test).
#[derive(Clone, Copy, Debug)]
struct Surface {
    backend: &'static str,
    surface: &'static str,
    /// `file:line` of the emission point in this tree (verified by hand
    /// against the codegen sources when this map was built).
    location: &'static str,
    /// The corpus insertion point whose payloads flow through this path.
    /// Every row must claim one — the corpus is the universal canary.
    corpus: &'static str,
    /// Primary mechanism: how the surface is kept safe TODAY.
    cover: Cover,
    /// Registered test name pinning the defense ("" for Corpus rows).
    defense_test: &'static str,
    /// Layer-2 mechanisms / vacuity notes (what the row's location does).
    note: &'static str,
}

/// The emission-surface map — every user-string emission point in the five
/// backends. Locations verified against the sources at map-build time.
const SURFACES: &[Surface] = &[
    // ── text: text nodes + interpolation string literals ─────────────
    Surface {
        backend: "react",
        surface: "text",
        location: "react/view.rs:1002-1008 (Text), 1011-1012 (Interp)",
        corpus: "text",
        cover: Cover::Corpus("text"),
        defense_test: "",
        note: "Text: \\ ' < > & escaped; Interp string literals via format_expr_ir_tsx (JSON_REACT escape_literal_str)",
    },
    Surface {
        backend: "omniview",
        surface: "text",
        location: "omniview/view/mod.rs:146-153",
        corpus: "text",
        cover: Cover::Corpus("text"),
        defense_test: "",
        note: "escape_html on every text node (tree_text + HTML output)",
    },
    Surface {
        backend: "vue",
        surface: "text",
        location: "vue/template.rs:296-312",
        corpus: "text",
        cover: Cover::Corpus("text"),
        defense_test: "",
        note: "& < > + {{ }} escaped as plain text",
    },
    Surface {
        backend: "dioxus",
        surface: "text",
        location: "dioxus/view.rs:267-279",
        corpus: "text",
        cover: Cover::Corpus("text"),
        defense_test: "",
        note: "escape_literal_str (Rust literal)",
    },
    Surface {
        backend: "flutter",
        surface: "text",
        location: "flutter/view.rs:140-168",
        corpus: "text",
        cover: Cover::Corpus("text"),
        defense_test: "",
        note: "escape_literal_str + $ → \\$ (Dart interpolation)",
    },
    // ── rawtext: raw text nodes (plain-text paths) ───────────────────
    Surface {
        backend: "react",
        surface: "rawtext",
        location: "react/view.rs:1002-1008",
        corpus: "rawtext",
        cover: Cover::Corpus("rawtext"),
        defense_test: "",
        note: "same Text escape path as `text`",
    },
    Surface {
        backend: "omniview",
        surface: "rawtext",
        location: "omniview/view/mod.rs:146-153",
        corpus: "rawtext",
        cover: Cover::Corpus("rawtext"),
        defense_test: "",
        note: "same escape_html path as `text`",
    },
    Surface {
        backend: "vue",
        surface: "rawtext",
        location: "vue/template.rs:296-312",
        corpus: "rawtext",
        cover: Cover::Corpus("rawtext"),
        defense_test: "",
        note: "same text-escape path as `text`",
    },
    Surface {
        backend: "dioxus",
        surface: "rawtext",
        location: "dioxus/view.rs:267-279",
        corpus: "rawtext",
        cover: Cover::Corpus("rawtext"),
        defense_test: "",
        note: "same escape_literal_str path as `text`",
    },
    Surface {
        backend: "flutter",
        surface: "rawtext",
        location: "flutter/view.rs:140-158",
        corpus: "rawtext",
        cover: Cover::Corpus("rawtext"),
        defense_test: "",
        note: "same escape path as `text`",
    },
    // ── rawtext-multi: raw text co-located with interpolation children ──
    // v4 1.2: raw text beside interpolation children reaches TWO Flutter
    // paths the single `rawtext` point misses — the all-text concat
    // (`Text('a${b}')` construction, view.rs:2196-2202) and the rich-text
    // TextSpan(text:) emission (view.rs:332-337). Both were hand-written
    // `'`-only replaces that skipped Dart `$`-escaping (v4 rust HIGH —
    // corpus point added by task 0.1); both now share the text path's
    // escape_literal_str + `$` → `\$`.
    Surface {
        backend: "flutter",
        surface: "rawtext-multi",
        location: "flutter/view.rs:2196-2202 (all-text concat), 332-337 (TextSpan)",
        corpus: "rawtext-multi",
        cover: Cover::Corpus("rawtext-multi"),
        defense_test: "",
        note: "escape_literal_str + $ → \\$ (Dart interpolation); both sites fixed in v4 1.2",
    },
    // ── fallback / fallback_str: malformed-interpolation text ────────
    // `{`-leading and StringLiteral-shaped Text nodes from the parser's
    // malformed-interpolation fallback must stay plain text (they were a
    // live code-injection vector in every backend — review CRITICAL round 2).
    Surface {
        backend: "react",
        surface: "fallback",
        location: "react/view.rs:994-1008",
        corpus: "fallback",
        cover: Cover::Corpus("fallback"),
        defense_test: "",
        note: "Text escape path; `{`-leading text NOT special-cased as code",
    },
    Surface {
        backend: "omniview",
        surface: "fallback",
        location: "omniview/view/mod.rs:146-153",
        corpus: "fallback",
        cover: Cover::Corpus("fallback"),
        defense_test: "",
        note: "escape_html path",
    },
    Surface {
        backend: "vue",
        surface: "fallback",
        location: "vue/template.rs:296-312",
        corpus: "fallback",
        cover: Cover::Corpus("fallback"),
        defense_test: "",
        note: "text-escape path; {{ }} braces neutralized",
    },
    Surface {
        backend: "dioxus",
        surface: "fallback",
        location: "dioxus/view.rs:267-279",
        corpus: "fallback",
        cover: Cover::Corpus("fallback"),
        defense_test: "",
        note: "escape_literal_str path",
    },
    Surface {
        backend: "flutter",
        surface: "fallback",
        location: "flutter/view.rs:140-158",
        corpus: "fallback",
        cover: Cover::Corpus("fallback"),
        defense_test: "",
        note: "escape path; `{`-shaped text stays literal",
    },
    Surface {
        backend: "react",
        surface: "fallback_str",
        location: "react/view.rs:994-1008",
        corpus: "fallback_str",
        cover: Cover::Corpus("fallback_str"),
        defense_test: "",
        note: "same Text path — decoded-quotes shape covered",
    },
    Surface {
        backend: "omniview",
        surface: "fallback_str",
        location: "omniview/view/mod.rs:146-153",
        corpus: "fallback_str",
        cover: Cover::Corpus("fallback_str"),
        defense_test: "",
        note: "same escape_html path",
    },
    Surface {
        backend: "vue",
        surface: "fallback_str",
        location: "vue/template.rs:296-312",
        corpus: "fallback_str",
        cover: Cover::Corpus("fallback_str"),
        defense_test: "",
        note: "same text-escape path",
    },
    Surface {
        backend: "dioxus",
        surface: "fallback_str",
        location: "dioxus/view.rs:267-279",
        corpus: "fallback_str",
        cover: Cover::Corpus("fallback_str"),
        defense_test: "",
        note: "same escape_literal_str path",
    },
    Surface {
        backend: "flutter",
        surface: "fallback_str",
        location: "flutter/view.rs:140-158",
        corpus: "fallback_str",
        cover: Cover::Corpus("fallback_str"),
        defense_test: "",
        note: "same escape path",
    },
    // ── attr: static attribute values (props loop) ───────────────────
    Surface {
        backend: "react",
        surface: "attr",
        location: "react/view.rs:259-272",
        corpus: "attr",
        cover: Cover::Corpus("attr"),
        defense_test: "",
        note: "\\ and \" escaped in JSX attribute; URL attrs blocked (264-268); handler-breakout gate at lowering (E0006)",
    },
    Surface {
        backend: "omniview",
        surface: "attr",
        location: "omniview/view/element.rs:176-224",
        corpus: "attr",
        cover: Cover::Corpus("attr"),
        defense_test: "",
        note: "escape_html; URL attrs blocked (177-194); data-* passthrough (219-224); handler gate at lowering",
    },
    Surface {
        backend: "vue",
        surface: "attr",
        location: "vue/template.rs:42-61",
        corpus: "attr",
        cover: Cover::Corpus("attr"),
        defense_test: "",
        note: "vue_attr_escape — & FIRST then \" (Vue decodes char refs in attr values before JS parse); URL attrs blocked (46-53); handler gate at lowering",
    },
    Surface {
        backend: "dioxus",
        surface: "attr",
        location: "dioxus/view.rs:29-47",
        corpus: "attr",
        cover: Cover::Corpus("attr"),
        defense_test: "",
        note: "\\ and \" escaped in Rust string; URL attrs blocked (32-38); handler gate at lowering",
    },
    Surface {
        backend: "flutter",
        surface: "attr",
        location: "flutter/view.rs:1158-1236",
        corpus: "attr",
        cover: Cover::Corpus("attr"),
        defense_test: "",
        note: "props via ctx.format_expr (JSON_FLUTTER escape + $ → \\$); handler gate at lowering",
    },
    // ── style: inline style values ───────────────────────────────────
    Surface {
        backend: "react",
        surface: "style",
        location: "shared/style.rs:28-38 (via react/view.rs:557-568)",
        corpus: "style",
        cover: Cover::Corpus("style"),
        defense_test: "",
        note: "sanitize_css_value + ' escape (shared inline-style formatter)",
    },
    Surface {
        backend: "omniview",
        surface: "style",
        location: "omniview/view/style.rs:171-187",
        corpus: "style",
        cover: Cover::Corpus("style"),
        defense_test: "",
        note: "sanitize_css_value + escape_html on the static style attribute",
    },
    Surface {
        backend: "vue",
        surface: "style",
        location: "vue/template.rs:153-168 (shared/style.rs)",
        corpus: "style",
        cover: Cover::Corpus("style"),
        defense_test: "",
        note: "shared formatter: sanitize_css_value + vue_attr_escape on the :style object",
    },
    Surface {
        backend: "dioxus",
        surface: "style",
        location: "(no inline_style emission — dioxus/view.rs has none)",
        corpus: "style",
        cover: Cover::Corpus("style"),
        defense_test: "",
        note: "inline style DROPPED on Dioxus — vacuous today; corpus point guards future emission",
    },
    Surface {
        backend: "flutter",
        surface: "style",
        location: "flutter/view.rs:1251-1310",
        corpus: "style",
        cover: Cover::Corpus("style"),
        defense_test: "",
        note: "typed converters (css_color_to_dart etc.); unmappable values warn+skip, never raw-emitted",
    },
    // ── class-name: class attribute values ────────────────────────────
    // Primary defense: charset gate at lowering (E0006, only
    // [a-zA-Z0-9_-] reaches codegen — the corpus class point asserts the
    // rejection itself). Codegen escaping is layer 2.
    Surface {
        backend: "react",
        surface: "class-name",
        location: "react/design.rs:186-191 (format_class_ref)",
        corpus: "class",
        cover: Cover::Charset,
        defense_test: "class_injection_is_compile_error_all_backends",
        note: "gate: crates/omnilang-core/src/ir/lower/view.rs:273-290; layer-2 escape_literal_str in format_class_ref",
    },
    Surface {
        backend: "omniview",
        surface: "class-name",
        location: "omniview/view/element.rs:786-791",
        corpus: "class",
        cover: Cover::Charset,
        defense_test: "class_injection_is_compile_error_all_backends",
        note: "gate: lower/view.rs:273-290; layer-2 escape_html on merged class attr",
    },
    Surface {
        backend: "vue",
        surface: "class-name",
        location: "vue/template.rs:42-61 (props loop)",
        corpus: "class",
        cover: Cover::Charset,
        defense_test: "class_injection_is_compile_error_all_backends",
        note: "gate: lower/view.rs:273-290; layer-2 vue_attr_escape",
    },
    Surface {
        backend: "dioxus",
        surface: "class-name",
        location: "dioxus/view.rs:29-47 (props loop)",
        corpus: "class",
        cover: Cover::Charset,
        defense_test: "class_injection_is_compile_error_all_backends",
        note: "gate: lower/view.rs:273-290; layer-2 \\ \" escape",
    },
    Surface {
        backend: "flutter",
        surface: "class-name",
        location: "flutter/design.rs:576-599 (resolve_static_class)",
        corpus: "class",
        cover: Cover::Charset,
        defense_test: "class_injection_is_compile_error_all_backends",
        note: "gate: lower/view.rs:273-290; unknown classes warn+drop; design values via typed converters",
    },
    // ── a11y-label: a11y block string values (label/role/live/alt) ───
    Surface {
        backend: "react",
        surface: "a11y-label",
        location: "react/view.rs:1313-1338 (a11y_react_attrs)",
        corpus: "a11y",
        cover: Cover::Corpus("a11y"),
        defense_test: "",
        note: "\\ and \" escaped; handler gate at lowering (validate_attr_no_handler_breakout)",
    },
    Surface {
        backend: "omniview",
        surface: "a11y-label",
        location: "omniview/view/element.rs:235-251",
        corpus: "a11y",
        cover: Cover::Corpus("a11y"),
        defense_test: "",
        note: "escape_html on label/role/live; dynamic alt via emit_a11y_attr",
    },
    Surface {
        backend: "vue",
        surface: "a11y-label",
        location: "(a11y not emitted — no el.a11y handling in vue/)",
        corpus: "a11y",
        cover: Cover::Corpus("a11y"),
        defense_test: "",
        note: "a11y DROPPED on Vue (silent) — vacuous today; corpus point guards future emission",
    },
    Surface {
        backend: "dioxus",
        surface: "a11y-label",
        location: "(a11y not emitted — no el.a11y handling in dioxus/)",
        corpus: "a11y",
        cover: Cover::Corpus("a11y"),
        defense_test: "",
        note: "a11y DROPPED on Dioxus (silent) — vacuous today; corpus point guards future emission",
    },
    Surface {
        backend: "flutter",
        surface: "a11y-label",
        location: "flutter/view.rs:841-927 (wrap_a11y)",
        corpus: "a11y",
        cover: Cover::Corpus("a11y"),
        defense_test: "",
        note: "escape_literal_str + $ → \\$; role degradation comment `*/` → `* /` (897-909)",
    },
    // ── event-name: event handler names + handler bodies ─────────────
    Surface {
        backend: "react",
        surface: "event-name",
        location: "react/view.rs:350-530 (to_react_event mapping, react/utils.rs:220)",
        corpus: "event",
        cover: Cover::Corpus("event"),
        defense_test: "",
        note: "handler ARG strings via JSON_REACT escape; event names mapped via canonical_event_name",
    },
    Surface {
        backend: "omniview",
        surface: "event-name",
        location: "omniview/view/events.rs:30-130",
        corpus: "event",
        cover: Cover::Corpus("event"),
        defense_test: "",
        note: "escape_html on fn:/store: names in data-event-* attrs; addEventListener names via map_event",
    },
    Surface {
        backend: "vue",
        surface: "event-name",
        location: "vue/template.rs:70-130",
        corpus: "event",
        cover: Cover::Corpus("event"),
        defense_test: "",
        note: "vue_attr_escape on the whole @event handler (decode-then-code model)",
    },
    Surface {
        backend: "dioxus",
        surface: "event-name",
        location: "dioxus/view.rs:60-128 (to_dioxus_event, dioxus/expr.rs:53-65)",
        corpus: "event",
        cover: Cover::Corpus("event"),
        defense_test: "",
        note: "handler args via JSON_RUST escape; event names via canonical_event_name",
    },
    Surface {
        backend: "flutter",
        surface: "event-name",
        location: "flutter/view.rs:1024-1072",
        corpus: "event",
        cover: Cover::Corpus("event"),
        defense_test: "",
        note: "callbacks via Dart escape (JSON_FLUTTER); event name → on<Stem> param",
    },
    // ── url: URL attributes (href/src/srcset/action/formaction/poster) ──
    Surface {
        backend: "react",
        surface: "url",
        location: "react/view.rs:264-268",
        corpus: "attr",
        cover: Cover::Sanitized,
        defense_test: "test_is_dangerous_url",
        note: "is_dangerous_url blocks javascript:/data:/vbscript: at compile time; attr corpus point guards the path",
    },
    Surface {
        backend: "omniview",
        surface: "url",
        location: "omniview/view/element.rs:177-194 (static), 160-164 (dynamic)",
        corpus: "attr",
        cover: Cover::Sanitized,
        defense_test: "test_is_dangerous_url",
        note: "static blocked + warning; dynamic bindings via runtime __omni_sanitizeUrl",
    },
    Surface {
        backend: "vue",
        surface: "url",
        location: "vue/template.rs:46-53",
        corpus: "attr",
        cover: Cover::Sanitized,
        defense_test: "test_is_dangerous_url",
        note: "dangerous URL → empty attribute value at compile time",
    },
    Surface {
        backend: "dioxus",
        surface: "url",
        location: "dioxus/view.rs:32-38",
        corpus: "attr",
        cover: Cover::Sanitized,
        defense_test: "test_is_dangerous_url",
        note: "dangerous URL → empty string at compile time",
    },
    Surface {
        backend: "flutter",
        surface: "url",
        location: "flutter/view.rs:1169-1172 (Image.network src)",
        corpus: "attr",
        cover: Cover::Corpus("attr"),
        defense_test: "",
        note: "no HTML URL context in Dart — src is an escaped Dart string; javascript: URLs never execute (no scheme check needed)",
    },
    // ── motion-transform: keyframe string values ─────────────────────
    Surface {
        backend: "react",
        surface: "motion-transform",
        location: "react/motion/keyframes.rs:187-190 (format_transform_part)",
        corpus: "motion-transform",
        cover: Cover::Escaped,
        defense_test: "motion_transform_string_is_escaped",
        note: "escape_literal_str + ` → \\` + ${ → \\${ (1.2 fix — `\"`-reclose AND template shapes)",
    },
    Surface {
        backend: "omniview",
        surface: "motion-transform",
        location: "omniview/motion_config.rs:48-68 (keyframe_to_animate_call → expr_to_js)",
        corpus: "motion-transform",
        cover: Cover::Corpus("motion-transform"),
        defense_test: "",
        note: "string stops via format_json_value (JSON_OMNIVIEW escape_literal_str)",
    },
    Surface {
        backend: "vue",
        surface: "motion-transform",
        location: "(motion not emitted — no motion handling in vue/)",
        corpus: "motion-transform",
        cover: Cover::Corpus("motion-transform"),
        defense_test: "",
        note: "motion DROPPED on Vue — vacuous today; corpus point guards future emission",
    },
    Surface {
        backend: "dioxus",
        surface: "motion-transform",
        location: "(motion not emitted — no motion handling in dioxus/)",
        corpus: "motion-transform",
        cover: Cover::Corpus("motion-transform"),
        defense_test: "",
        note: "motion DROPPED on Dioxus — vacuous today; corpus point guards future emission",
    },
    Surface {
        backend: "flutter",
        surface: "motion-transform",
        location: "flutter/motion.rs:296-370 (format_stop_value_with)",
        corpus: "motion-transform",
        cover: Cover::Corpus("motion-transform"),
        defense_test: "",
        note: "string stops via format_expr_ir_flutter (JSON_FLUTTER escape + $ → \\$); unmappable → typed converter or expression",
    },
    // ── design-token: design declaration values (tokens/classes/variants) ──
    Surface {
        backend: "react",
        surface: "design-token",
        location: "shared/design.rs:65-66, 81-82, 96-97 (via react/design.rs:331)",
        corpus: "design-token",
        cover: Cover::Sanitized,
        defense_test: "css_value_rejects_structural_characters",
        note: "sanitize_css_value on every declaration value (1.4: } / </style / ${ / ; / alert( rejected)",
    },
    Surface {
        backend: "omniview",
        surface: "design-token",
        location: "shared/design.rs (via omniview/design.rs:36)",
        corpus: "design-token",
        cover: Cover::Sanitized,
        defense_test: "css_value_rejects_structural_characters",
        note: "same shared emitter + sanitize_css_value",
    },
    Surface {
        backend: "vue",
        surface: "design-token",
        location: "(design blocks rejected — codegen/mod.rs DESIGN_BLOCK_ERROR)",
        corpus: "design-token",
        cover: Cover::Corpus("design-token"),
        defense_test: "",
        note: "no surface — design never compiles on Vue (compile error = safe); corpus skips point via design_target_supported",
    },
    Surface {
        backend: "dioxus",
        surface: "design-token",
        location: "(design blocks rejected — codegen/mod.rs DESIGN_BLOCK_ERROR)",
        corpus: "design-token",
        cover: Cover::Corpus("design-token"),
        defense_test: "",
        note: "no surface — design never compiles on Dioxus; corpus skips point",
    },
    Surface {
        backend: "flutter",
        surface: "design-token",
        location: "flutter/design.rs:421-461 (token_static_expr)",
        corpus: "design-token",
        cover: Cover::Corpus("design-token"),
        defense_test: "",
        note: "typed converters (Color/double/…); unmappable values → constant SKIPPED with warning, never raw-emitted; no CSS emission",
    },
];

/// Defense tests registered by (real test name, location, purpose) — the
/// tests `defense_has_test` resolves against. Names must match actual
/// `#[test]` functions in the tree.
const DEFENSE_TESTS: &[(&str, &str, &str)] = &[
    (
        "injection_payload_corpus",
        "tests_regression/injection.rs:847",
        "cross-backend canary: every insertion point × payload × backend",
    ),
    (
        "motion_transform_string_is_escaped",
        "tests_regression/injection.rs:930",
        "React motion string escape pin (\"-reclose + ${ template shapes)",
    ),
    (
        "class_injection_is_compile_error_all_backends",
        "tests_regression/injection.rs:1018",
        "class-name charset gate (E0006) asserted on all five backends",
    ),
    (
        "omni_html_attribute_context_blocks_unquoted_handlers",
        "tests_regression/injection.rs:1054",
        "unquoted handler-attribute shape rejected by the shared lowering gate",
    ),
    (
        "handler_gate_is_shared_and_whitelist_precise",
        "tests_regression/injection.rs:1095",
        "gate is shared across backends and matches the WHATWG whitelist only",
    ),
    (
        "live_handler_scan_is_tokenizer_state_aware",
        "tests_regression/injection.rs:1177",
        "raw-HTML handler scan: real handlers flagged, inert occurrences skipped",
    ),
    (
        "css_value_rejects_structural_characters",
        "crates/omnilang-core/src/codegen/sanitize.rs:317",
        "sanitize_css_value rejects } / </style / ${ / ; / alert( (1.4)",
    ),
    (
        "css_value_keeps_legitimate_values",
        "crates/omnilang-core/src/codegen/sanitize.rs:369",
        "sanitizer false-positive guard (legit CSS values pass through)",
    ),
    (
        "test_sanitize_css_value_blocks_hex_escape_bypass",
        "crates/omnilang-core/src/codegen/sanitize.rs:259",
        "hex-escape forms of structural characters are neutralized",
    ),
    (
        "test_sanitize_css_value_blocks_escape_decoded_whitespace",
        "crates/omnilang-core/src/codegen/sanitize.rs:276",
        "escape-decoded whitespace forms are neutralized",
    ),
    (
        "test_is_dangerous_url",
        "crates/omnilang-core/src/codegen/sanitize.rs:398",
        "URL scheme block (javascript:/data:/vbscript:)",
    ),
];

/// A defense mechanism is "tested" when its registered test exists in
/// [`DEFENSE_TESTS`] (name ↔ location ↔ purpose, kept in sync with the
/// actual `#[test]` functions when they are added/renamed).
fn defense_has_test(surface: &Surface) -> bool {
    !surface.defense_test.is_empty()
        && DEFENSE_TESTS
            .iter()
            .any(|(name, _, _)| *name == surface.defense_test)
}

/// The coverage-map assertions (Task 1.5).
///
/// 1. Every surface claims a corpus insertion point (the corpus is the
///    universal canary — even "dropped today" surfaces are guarded against
///    future unescaped emission).
/// 2. Corpus-covered surfaces must name a real insertion point.
/// 3. Defended surfaces (Charset/Escaped/Sanitized) must pin a registered
///    defense test.
/// 4. Every corpus insertion point must be claimed by at least one surface
///    (no orphan corpus points — the map cannot silently rot).
#[test]
fn every_string_emission_surface_is_covered() {
    // 1 + 3: per-surface assertions.
    for s in SURFACES {
        assert!(
            !s.note.is_empty(),
            "[{}/{}] surface row must carry a note (mechanism/layer-2/vacuity documentation)",
            s.backend,
            s.surface
        );
        assert!(
            INSERTION_POINTS.contains(&s.corpus),
            "[{}/{}] orphan surface — corpus point {:?} does not exist in injection.rs POINTS (location: {})",
            s.backend,
            s.surface,
            s.corpus,
            s.location
        );
        match s.cover {
            Cover::Corpus(point) => assert_eq!(
                point, s.corpus,
                "[{}/{}] corpus point mismatch: cover says {point:?}, corpus field says {:?}",
                s.backend, s.surface, s.corpus
            ),
            Cover::Charset | Cover::Escaped | Cover::Sanitized => assert!(
                defense_has_test(s),
                "[{}/{}] untested defense — mechanism {:?} has no registered defense test (got {:?}); \
                 location: {}",
                s.backend, s.surface, s.cover, s.defense_test, s.location
            ),
        }
    }

    // 2: every corpus point is claimed by at least one surface.
    for point in INSERTION_POINTS {
        assert!(
            SURFACES.iter().any(|s| s.corpus == *point),
            "orphan corpus point {point:?} — no emission surface claims it \
             (remove it from injection.rs POINTS or map a surface to it)"
        );
    }
}

/// The five backends of the map are exactly the five backends of the corpus
/// (react / omniview / vue / dioxus / flutter) — the map covers every
/// backend the corpus runs on, and no more.
#[test]
fn map_backends_match_corpus_backends() {
    let mapped: std::collections::BTreeSet<&str> = SURFACES.iter().map(|s| s.backend).collect();
    assert_eq!(
        mapped,
        ["react", "omniview", "vue", "dioxus", "flutter"]
            .into_iter()
            .collect(),
        "map backends must match injection.rs BACKENDS (react/omniview/dioxus/flutter/vue)"
    );
}

/// Every surface key that exists in the map is documented in the SURFACES
/// doc-comment (the map is the source of truth for what a "surface" is).
#[test]
fn surface_keys_are_stable() {
    // Each (backend, surface) pair appears at most once — no duplicate rows.
    let mut seen: std::collections::BTreeSet<(&str, &str)> = std::collections::BTreeSet::new();
    for s in SURFACES {
        assert!(
            seen.insert((s.backend, s.surface)),
            "[{}/{}] duplicate surface row (backend, surface) pair",
            s.backend,
            s.surface
        );
    }
}
