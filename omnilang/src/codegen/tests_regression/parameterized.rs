//! Parameterized regression tests — each fixture × each P0 backend = one test.
//!
//! Uses a declarative macro to generate individual `#[test]` functions with
//! compile-time snapshot names, avoiding the need for `rstest` or runtime loops.
//!
//! Two families of tests live here:
//! - IR-level fixtures (below): hand-built `IRProgram` factories compiled by
//!   each backend codegen directly.
//! - Semantic DSL source fixtures (end of file): `.omni` files in
//!   `tests/fixtures/parameterized/` compiled through the FULL pipeline
//!   (lexer → parser → semantic → IR → codegen). These exercise the new
//!   semantic-level DSL syntax (design blocks, slots, refs, motion, stores)
//!   end-to-end on every P0 backend (Task 8.1).

use rustc_hash::FxHashMap;

use crate::codegen::omniview;
use crate::codegen::{CompileOutput, CompiledFile};
use crate::ir::IRProgram;

// ═════════════════════════════════════════════════════════════════════════════
// OmniView adapter — converts OmniViewOutput → CompileOutput
// ═════════════════════════════════════════════════════════════════════════════

fn omni_to_compile_output(ir: &IRProgram) -> CompileOutput {
    let sources = FxHashMap::from_iter([("main.omni".to_string(), String::new())]);
    let output = omniview::generate(ir, false, false, false);
    CompileOutput {
        files: vec![
            CompiledFile {
                path: "index.html".into(),
                content: output.html,
            },
            CompiledFile {
                path: "style.css".into(),
                content: output.css,
            },
            CompiledFile {
                path: "app.js".into(),
                content: output.js,
            },
        ],
        errors: vec![],
        source_map: output.source_map,
        warnings: vec![],
    }
}

// ═════════════════════════════════════════════════════════════════════════════
// Test generation macro — one #[test] fn per (backend, fixture) pair
// ═════════════════════════════════════════════════════════════════════════════

macro_rules! gen_tests_for_fixture {
    ($fixture_fn:ident) => {
        gen_tests_for_fixture!(@omniview, $fixture_fn);
        gen_tests_for_fixture!(@react, $fixture_fn);
        gen_tests_for_fixture!(@flutter, $fixture_fn);
    };
    // Omniview
    (@omniview, $fixture_fn:ident) => {
        #[test]
        fn omniview() {
            let ir = super::super::$fixture_fn();
            let output = super::omni_to_compile_output(&ir);
            assert!(output.errors.is_empty(),
                "omniview {} errors: {:?}", stringify!($fixture_fn), output.errors);
            insta::assert_snapshot!(
                concat!("regression_omniview_", stringify!($fixture_fn)),
                &super::super::format_output(&output)
            );
        }
    };
    // React
    (@react, $fixture_fn:ident) => {
        #[test]
        fn react() {
            use crate::codegen::Codegen;
            let ir = super::super::$fixture_fn();
            let sources = rustc_hash::FxHashMap::from_iter([("main.omni".to_string(), String::new())]);
            let output = crate::codegen::react::ReactCodegen::default().generate(&ir, &sources);
            assert!(output.errors.is_empty(),
                "react {} errors: {:?}", stringify!($fixture_fn), output.errors);
            insta::assert_snapshot!(
                concat!("regression_react_", stringify!($fixture_fn)),
                &super::super::format_output(&output)
            );
        }
    };
    // Flutter
    (@flutter, $fixture_fn:ident) => {
        #[cfg(feature = "codegen-flutter")]
        #[test]
        fn flutter() {
            use crate::codegen::Codegen;
            let ir = super::super::$fixture_fn();
            let sources = rustc_hash::FxHashMap::from_iter([("main.omni".to_string(), String::new())]);
            let output = crate::codegen::flutter::FlutterCodegen.generate(&ir, &sources);
            assert!(output.errors.is_empty(),
                "flutter {} errors: {:?}", stringify!($fixture_fn), output.errors);
            insta::assert_snapshot!(
                concat!("regression_flutter_", stringify!($fixture_fn)),
                &super::super::format_output(&output)
            );
        }
    };
}

// ═════════════════════════════════════════════════════════════════════════════
// Per-fixture modules (each gets its own 3-backend test functions)
// ═════════════════════════════════════════════════════════════════════════════

mod empty {
    //! Regression tests for the empty IR program fixture.
    gen_tests_for_fixture!(fixture_empty);
}

mod component {
    //! Regression tests for the component fixture.
    gen_tests_for_fixture!(fixture_component);
}

mod store {
    //! Regression tests for the store fixture.
    gen_tests_for_fixture!(fixture_store);
}

mod component_with_store {
    //! Regression tests for the component-with-store fixture.
    gen_tests_for_fixture!(fixture_component_with_store);
}

mod expressions {
    //! Regression tests for the expressions fixture.
    gen_tests_for_fixture!(fixture_expressions);
}

// ═════════════════════════════════════════════════════════════════════════════
// Semantic DSL source fixtures — full pipeline (source → parse → IR → codegen)
// ═════════════════════════════════════════════════════════════════════════════
//
// Task 8.1: fixtures in `tests/fixtures/parameterized/` exercise the semantic
// DSL syntax end-to-end across all P0 backends (design blocks, slots, refs,
// motion, stores). Each fixture compiles with zero errors and the emitted
// output files are asserted per backend, plus fixture-specific content
// assertions (e.g. slots.omni → React emits `children ??`, refs.omni →
// React emits `useRef`).

/// Compile a fixture source through the full pipeline (lexer → parser →
/// semantic → IR → codegen) to a backend, asserting that compilation
/// succeeds. Returns the generated files keyed by output path.
fn compile_source_fixture(
    source: &str,
    target: crate::codegen::Target,
) -> FxHashMap<String, String> {
    let sources = FxHashMap::from_iter([("main.omni".to_string(), source.to_string())]);
    let path_mapping = crate::path_mapping::derive_path_mapping(&sources);
    let result = crate::compile_multi_internal(sources, target, path_mapping, false, false, None);
    assert!(
        result.success,
        "compilation to {:?} should succeed, errors: {:?}",
        target, result.errors
    );
    result.files
}

/// Assert the standard OmniView output files are present.
fn assert_omniview_files(files: &FxHashMap<String, String>) {
    for expected in ["index.html", "index.css", "index.js", "body.html"] {
        assert!(
            files.contains_key(expected),
            "missing OmniView output file {expected:?}; got: {:?}",
            files.keys().collect::<Vec<_>>()
        );
    }
}

/// Assert at least one React `.tsx` component file is present.
fn assert_react_files(files: &FxHashMap<String, String>) {
    let tsx: Vec<&String> = files.keys().filter(|p| p.ends_with(".tsx")).collect();
    assert!(
        !tsx.is_empty(),
        "expected React .tsx component output; got: {:?}",
        files.keys().collect::<Vec<_>>()
    );
    for path in tsx {
        assert!(!files[path].trim().is_empty(), "empty tsx output: {path}");
    }
}

/// Assert at least one Flutter `.dart` component file is present.
fn assert_flutter_files(files: &FxHashMap<String, String>) {
    let dart: Vec<&String> = files.keys().filter(|p| p.ends_with(".dart")).collect();
    assert!(
        !dart.is_empty(),
        "expected Flutter .dart output; got: {:?}",
        files.keys().collect::<Vec<_>>()
    );
    for path in dart {
        assert!(!files[path].trim().is_empty(), "empty dart output: {path}");
    }
}

/// Helper: assert a substring appears in a specific output file.
fn assert_contains(files: &FxHashMap<String, String>, path: &str, needle: &str) {
    let content = files.get(path).unwrap_or_else(|| {
        panic!(
            "expected output file {path:?}; got: {:?}",
            files.keys().collect::<Vec<_>>()
        )
    });
    assert!(
        content.contains(needle),
        "expected {path:?} to contain {needle:?}; got:\n{}",
        &content[..content.len().min(800)]
    );
}

// ── design_tokens.omni ──

fn assert_design_tokens_omniview(files: &FxHashMap<String, String>) {
    // Tokens → :root CSS custom properties (default theme) + dark overrides.
    assert_contains(files, "index.css", "--primary: #6366F1;");
    assert_contains(files, "index.css", "--bg: #FFFFFF;");
    assert_contains(files, "index.css", "[data-theme=\"dark\"]");
    assert_contains(files, "index.css", "--primary: #A5B4FC;");
    // Classes → scoped rules with token() → var(--name) references.
    assert_contains(files, "index.css", "[data-design~=\"AppDesign\"] .Card {");
    assert_contains(files, "index.css", "background: var(--bg);");
    // Variants → base + dimension branch rules.
    assert_contains(files, "index.css", "[data-design~=\"AppDesign\"] .Button {");
    assert_contains(files, "index.css", "background: var(--primary);");
    assert_contains(
        files,
        "index.css",
        "[data-design~=\"AppDesign\"] .Button.secondary {",
    );
    assert_contains(files, "index.css", "background: #8B5CF6;");
}

fn assert_design_tokens_react(files: &FxHashMap<String, String>) {
    // React emits unscaled selectors (module-scoped via CSS import) but keeps
    // the same token/theme variable declarations as OmniView.
    assert_contains(files, "design.css", "--primary: #6366F1;");
    assert_contains(files, "design.css", ".Card {");
    assert_contains(files, "design.css", "background: var(--bg);");
    assert_contains(files, "useThemeStore.ts", "useThemeStore");
    // Consuming component emits design class references.
    assert_contains(files, "DesignTokensDemo.tsx", "Button primary");
    assert_contains(files, "DesignTokensDemo.tsx", "Button secondary");
}

fn assert_design_tokens_flutter(files: &FxHashMap<String, String>) {
    // design.dart carries the typed token constants + decoration factories
    // namespaced by the design block name.
    assert_contains(files, "design.dart", "AppDesignTokens");
    // The consuming component still emits (widgets, no direct design refs —
    // design classes are style-only on Flutter).
    assert_contains(files, "DesignTokensDemo.dart", "class DesignTokensDemo");
}

// ── slots.omni ──

fn assert_slots_omniview(files: &FxHashMap<String, String>) {
    // Named slot with fallback renders <slot name="...">fallback</slot>.
    assert_contains(files, "body.html", "<slot name=\"header\">");
    assert_contains(files, "body.html", "Default header");
    // Empty named slot stays an empty <slot> element.
    assert_contains(files, "body.html", "<slot name=\"footer\"></slot>");
    // Unnamed slot falls back to an attribute-less <slot>.
    assert_contains(files, "body.html", "<slot>");
    assert_contains(files, "body.html", "Default body");
}

fn assert_slots_react(files: &FxHashMap<String, String>) {
    // Named slot → {header ?? (fallback)}; unnamed slot → {children ?? (...)}.
    assert_contains(files, "Card.tsx", "header ??");
    assert_contains(files, "Card.tsx", "children ??");
    assert_contains(files, "Card.tsx", "Default header");
    // Empty slot (no fallback) → bare {footer} projection.
    assert_contains(files, "Card.tsx", "{footer}");
}

fn assert_slots_flutter(files: &FxHashMap<String, String>) {
    // widget.header ?? fallback; widget.children ?? fallback; bare footer.
    assert_contains(files, "Card.dart", "widget.header ??");
    assert_contains(files, "Card.dart", "widget.children ??");
    assert_contains(files, "Card.dart", "Default header");
    assert_contains(files, "Card.dart", "widget.footer");
}

// ── slots_body.omni (ADR-053 D5 body-level declarations) ──

fn assert_slots_body_omniview(files: &FxHashMap<String, String>) {
    // Body-declared named slot with fallback renders as a <slot name="...">
    // element at the {header} interpolation position when the consumer
    // projects nothing (second <Modal>); the unnamed slot becomes <slot>.
    assert_contains(files, "body.html", "<slot name=\"header\">");
    assert_contains(files, "body.html", "Default header");
    assert_contains(files, "body.html", "Default body");
    // Typed data slot renders as an empty <slot name="footer">.
    assert_contains(files, "body.html", "<slot name=\"footer\"></slot>");
    // Consumer named-prop projection: <Modal header={<div>Custom title</div>}>
    // injects the projected div into the child's slot position.
    assert_contains(files, "body.html", "Custom title");
    assert_contains(files, "body.html", "Custom body");
    // A bare <Modal /> keeps both declared fallbacks (header + unnamed).
}

fn assert_slots_body_react(files: &FxHashMap<String, String>) {
    // Body-declared slots become props: same-named for named slots,
    // `children` for the unnamed slot, and the typed slot carries its
    // type constraint.
    assert_contains(files, "Modal.tsx", "header?: ReactNode;");
    assert_contains(files, "Modal.tsx", "children?: ReactNode;");
    assert_contains(files, "Modal.tsx", "footer?: string;");
    // Interpolations render the declared slot with its fallback
    // ({header ?? (fallback)} / {children ?? (...)} / bare {footer}).
    assert_contains(files, "Modal.tsx", "{header ?? (");
    assert_contains(files, "Modal.tsx", "{children ?? (");
    assert_contains(files, "Modal.tsx", "Default header");
    assert_contains(files, "Modal.tsx", "Default body");
    assert_contains(files, "Modal.tsx", "{footer}");
    // Consumer named-prop projection; a bare <Modal /> keeps the declared
    // fallbacks.
    assert_contains(files, "App.tsx", "header={<div>");
    assert_contains(files, "App.tsx", "Custom title");
    assert_contains(files, "App.tsx", "Custom body");
    assert_contains(files, "App.tsx", "<Modal />");
}

fn assert_slots_body_flutter(files: &FxHashMap<String, String>) {
    // Body-declared slots become Widget? constructor params.
    assert_contains(files, "Modal.dart", "this.header,");
    assert_contains(files, "Modal.dart", "this.children,");
    assert_contains(files, "Modal.dart", "this.footer,");
    // Interpolations render widget.<name> ?? fallback.
    assert_contains(files, "Modal.dart", "widget.header ??");
    assert_contains(files, "Modal.dart", "widget.children ??");
    assert_contains(files, "Modal.dart", "Default header");
    assert_contains(files, "Modal.dart", "Default body");
    // Consumer named-prop projection; a bare <Modal /> keeps the declared
    // fallbacks.
    assert_contains(files, "App.dart", "header: ");
    assert_contains(files, "App.dart", "Custom title");
    assert_contains(files, "App.dart", "Custom body");
    assert_contains(files, "App.dart", "Modal()");
}

// ── refs.omni ──

fn assert_refs_omniview(files: &FxHashMap<String, String>) {
    // Ref ops lower to guarded DOM calls on the ref wrapper.
    assert_contains(files, "index.js", "input.focus()");
    // The clear action writes the bound signal (the imperative ref write
    // `input.value = ""` was removed in favor of the value binding).
    assert_contains(
        files,
        "index.js",
        "__omni_subs.get(\"SearchBox_query\").set(\"\");",
    );
    // bind:value → the two-way runtime: DOM input listener writing the
    // signal plus the guarded reactive effect reading it back (方向一致性).
    assert_contains(
        files,
        "index.js",
        "addEventListener(\"input\", (e) => { if (!_guard_value_el_input_001) { __omni_subs.get(\"SearchBox_query\").set(e.target.value); } });",
    );
    assert_contains(
        files,
        "index.js",
        "__omni_effect(() => { const el = __omni_el(\"el-input-001\"); if (el) { _guard_value_el_input_001 = true; el.value = __omni_subs.get(\"SearchBox_query\").value; _guard_value_el_input_001 = false; } });",
    );
    // Semantic binding effects drive focus()/blur() from the boolean signal.
    assert_contains(files, "index.js", "el.focus(); } else { el.blur(); }");
}

fn assert_refs_react(files: &FxHashMap<String, String>) {
    assert_contains(files, "SearchBox.tsx", "useRef");
    // bind:value → value prop + onChange write-back (two-way, 方向一致性).
    assert_contains(files, "SearchBox.tsx", "value={query}");
    assert_contains(files, "SearchBox.tsx", "setQuery(e.target.value)");
    assert!(
        !files["SearchBox.tsx"].contains("inputRef.current.value = query"),
        "the one-way DOM-syncing effect for value bindings must be gone:\n{}",
        files["SearchBox.tsx"]
    );
    // The focus={isFocused} semantic binding drives focus()/blur().
    assert_contains(files, "SearchBox.tsx", "inputRef.current?.focus()");
    // A signal read ONLY inside style: { ... } must still get a state hook —
    // inline-style reads are collected for the useState decision (regression:
    // boxColor used to be emitted without a declaration, throwing a
    // ReferenceError on first render). The generic is typed from the IR
    // signal type (T1.6 TS batch).
    assert_contains(
        files,
        "SearchBox.tsx",
        "const [boxColor, setBoxColor] = useState<string>(\"red\");",
    );
    assert_contains(files, "SearchBox.tsx", "color: boxColor");
}

fn assert_refs_flutter(files: &FxHashMap<String, String>) {
    assert_contains(files, "SearchBox.dart", "FocusNode");
    // bind:value on a ref'd TextField → controller + guarded listener
    // write-back via setState (the two-way mechanism, 方向一致性).
    assert_contains(
        files,
        "SearchBox.dart",
        "final TextEditingController _inputText = TextEditingController();",
    );
    assert_contains(files, "SearchBox.dart", "_inputText.addListener(() {");
    assert_contains(files, "SearchBox.dart", "query = _inputText.text;");
    assert_contains(files, "SearchBox.dart", "_syncingInputText");
    assert_contains(files, "SearchBox.dart", "controller: _inputText");
    assert_contains(files, "SearchBox.dart", "_inputText.text = query");
}

// ── animation.omni ──

fn assert_animation_omniview(files: &FxHashMap<String, String>) {
    // Named motion is emitted as a motion function; the inline tween is
    // hoisted into a synthesized __anon_m_N motion.
    assert_contains(files, "index.js", "__omni_motion_pulse");
    assert_contains(files, "index.js", "__omni_motion___anon_m_0");
    // Both triggers survive lowering: the inline enter tween and the named
    // hover reference (regression: multi-root views previously dropped the
    // second top-level element).
    assert_contains(files, "body.html", "data-enter=\"__anon_m_0\"");
    assert_contains(files, "body.html", "data-hover=\"pulse\"");
    assert_contains(files, "body.html", "Hover me");
}

fn assert_animation_react(files: &FxHashMap<String, String>) {
    assert_contains(files, "MotionCard.tsx", "_M[\"pulse\"]");
    assert_contains(files, "MotionCard.tsx", "__anon_m_0");
}

fn assert_animation_flutter(files: &FxHashMap<String, String>) {
    assert_contains(files, "MotionCard.dart", "AnimationController");
    assert_contains(files, "MotionCard.dart", "pulse");
}

// ── store.omni ──

fn assert_store_omniview(files: &FxHashMap<String, String>) {
    // Store wiring: store signal, action fn, and the interpolation effect
    // that binds the qualified read into the DOM.
    assert_contains(
        files,
        "index.js",
        "const CounterStore_count = __omni_signal",
    );
    assert_contains(files, "index.js", "function CounterStore_increment()");
    assert_contains(files, "index.js", "String(CounterStore_count.value)");
    // The action handler calls the store action, not a mutation of a
    // non-existent "CounterStore" signal (regression).
    assert_contains(files, "index.js", "CounterStore_increment()");
}

fn assert_store_react(files: &FxHashMap<String, String>) {
    // Zustand v5 store hook + consumption inside the component.
    assert_contains(files, "useCounterStore.ts", "create");
    assert_contains(files, "Counter.tsx", "useCounterStore");
}

fn assert_store_flutter(files: &FxHashMap<String, String>) {
    // Riverpod 3.0 store file + component import.
    assert_contains(files, "CounterStore_store.dart", "Notifier");
    assert_contains(files, "Counter.dart", "CounterStore");
}

// ── Test generation macro: one #[test] fn per (fixture, backend) pair ──

macro_rules! gen_semantic_dsl_tests {
    ($mod_name:ident, $fixture_file:literal, $omniview_assert:ident, $react_assert:ident, $flutter_assert:ident) => {
        mod $mod_name {
            #[test]
            fn omniview() {
                let source = include_str!(concat!(
                    "../../../tests/fixtures/parameterized/",
                    $fixture_file
                ));
                let files = super::compile_source_fixture(source, crate::codegen::Target::OmniView);
                super::assert_omniview_files(&files);
                super::$omniview_assert(&files);
            }

            #[test]
            fn react() {
                let source = include_str!(concat!(
                    "../../../tests/fixtures/parameterized/",
                    $fixture_file
                ));
                let files = super::compile_source_fixture(source, crate::codegen::Target::React);
                super::assert_react_files(&files);
                super::$react_assert(&files);
            }

            #[cfg(feature = "codegen-flutter")]
            #[test]
            fn flutter() {
                let source = include_str!(concat!(
                    "../../../tests/fixtures/parameterized/",
                    $fixture_file
                ));
                let files = super::compile_source_fixture(source, crate::codegen::Target::Flutter);
                super::assert_flutter_files(&files);
                super::$flutter_assert(&files);
            }
        }
    };
}

gen_semantic_dsl_tests!(
    design_tokens,
    "design_tokens.omni",
    assert_design_tokens_omniview,
    assert_design_tokens_react,
    assert_design_tokens_flutter
);
gen_semantic_dsl_tests!(
    slots,
    "slots.omni",
    assert_slots_omniview,
    assert_slots_react,
    assert_slots_flutter
);
gen_semantic_dsl_tests!(
    slots_body,
    "slots_body.omni",
    assert_slots_body_omniview,
    assert_slots_body_react,
    assert_slots_body_flutter
);
gen_semantic_dsl_tests!(
    refs,
    "refs.omni",
    assert_refs_omniview,
    assert_refs_react,
    assert_refs_flutter
);
gen_semantic_dsl_tests!(
    animation,
    "animation.omni",
    assert_animation_omniview,
    assert_animation_react,
    assert_animation_flutter
);
gen_semantic_dsl_tests!(
    store_consumer,
    "store.omni",
    assert_store_omniview,
    assert_store_react,
    assert_store_flutter
);

// ── array_methods.omni ──

fn assert_array_methods_omniview(files: &FxHashMap<String, String>) {
    // Array language methods (design doc 域 8): JS-family mapping with the
    // defensive `|| []` guard; component signals resolve to the nid-qualified
    // `.value` reads. filter → .filter(pred); map → .map(pred);
    // count → .filter(pred).length.
    assert_contains(
        files,
        "index.js",
        "(((ArrayMethodsDemo_items.value) || []).filter(((i) => i % 2 === 0)))",
    );
    assert_contains(
        files,
        "index.js",
        "(((ArrayMethodsDemo_items.value) || []).map(((i) => i * 2)))",
    );
    assert_contains(
        files,
        "index.js",
        "(((ArrayMethodsDemo_items.value) || []).filter(((i) => i % 2 === 0)).length)",
    );
}

fn assert_array_methods_react(files: &FxHashMap<String, String>) {
    // JS-family mapping: filter/map verbatim, count → filter(pred).length.
    assert_contains(
        files,
        "ArrayMethodsDemo.tsx",
        "items.filter(((i) => (i % 2 === 0)))",
    );
    assert_contains(files, "ArrayMethodsDemo.tsx", "items.map(((i) => (i * 2)))");
    assert_contains(
        files,
        "ArrayMethodsDemo.tsx",
        "items.filter(((i) => (i % 2 === 0))).length",
    );
    // The useMemo deps array must track the subject signal — regression
    // guard: the computed-deps collector used to miss ExprIR::ArrayMethod
    // (`_ => false`), emitting `useMemo(..., [])` and stale computeds.
    assert_contains(files, "ArrayMethodsDemo.tsx", "}, [items]);");
}

fn assert_array_methods_flutter(files: &FxHashMap<String, String>) {
    // Dart-family mapping (the fix for the Flutter `.filter()` silent
    // failure): filter → where(...).toList(); map → map(...).toList();
    // count → where(...).length (no toList — length is on the lazy
    // Iterable).
    assert_contains(
        files,
        "ArrayMethodsDemo.dart",
        "items.where(((i) => i % 2 == 0)).toList()",
    );
    assert_contains(
        files,
        "ArrayMethodsDemo.dart",
        "items.map(((i) => i * 2)).toList()",
    );
    assert_contains(
        files,
        "ArrayMethodsDemo.dart",
        "items.where(((i) => i % 2 == 0)).length",
    );
}

// ── control_flow.omni ──

fn assert_control_flow_omniview(files: &FxHashMap<String, String>) {
    // Keyed @for: FLIP keyed reconciliation + data-for-key clone tracking.
    assert_contains(files, "index.js", "Omni.flipBefore(parent)");
    assert_contains(files, "index.js", "data-for-key");
    // Empty body: empty-state template clone + length check.
    assert_contains(files, "index.js", "data-for-empty");
    assert_contains(files, "index.js", "length === 0");
    // @if mode:"fade": branch display toggling + both branch texts.
    assert_contains(files, "index.js", "_show(");
    assert_contains(files, "body.html", "shown");
    assert_contains(files, "body.html", "hidden");
    assert_contains(files, "body.html", "No items");
}

fn assert_control_flow_react(files: &FxHashMap<String, String>) {
    // Keyed @for → key={item}; empty body → length === 0 ternary.
    assert_contains(files, "ListPage.tsx", "key={item}");
    assert_contains(files, "ListPage.tsx", ".length === 0");
    // @if mode:"fade" → CSS-module fade wrapper around each branch.
    assert_contains(files, "ListPage.tsx", "omni-if-fade");
    assert_contains(files, "ListPage.tsx", "shown");
    assert_contains(files, "ListPage.tsx", "hidden");
    assert_contains(files, "ListPage.tsx", "No items");
}

fn assert_control_flow_flutter(files: &FxHashMap<String, String>) {
    // Today-passing subset. The keyed @for contract (key: ValueKey(expr))
    // and @if mode:"fade" (AnimatedSwitcher) are the Task 5.5 TARGET
    // contract — asserted in the #[ignore]d `flutter_control_flow_contract`
    // test below (current codegen ignores key_expr and fade mode silently).
    assert_contains(files, "ListPage.dart", "ListView.builder(");
    // Empty body → Builder with isEmpty switch.
    assert_contains(files, "ListPage.dart", "isEmpty ?");
    assert_contains(files, "ListPage.dart", "No items");
    assert_contains(files, "ListPage.dart", "shown");
    assert_contains(files, "ListPage.dart", "hidden");
}

/// T5.4/T5.5 TARGET contract for Flutter control flow: @if fade + keyed @for.
///
/// Today the Flutter codegen ignores `key_expr` (unconditional
/// ListView.builder with no ValueKey) and `@if mode:"fade"` is a silent
/// no-op (no AnimatedSwitcher) — known #30-era gaps owned by Task 5.4
/// (`@if mode:"fade"` → AnimatedSwitcher) and Task 5.5 (key_expr →
/// `key: ValueKey(expr)`; [scroll] → ListView.builder dispatch) of the
/// 2026-08-02-semantic-syntax-v2 plan (§5.4/§5.5; design doc mapping
/// table). The assertions below are the TARGET shape and FAIL on the
/// current codegen, so the test stays #[ignore]d until 5.4/5.5 land.
// TODO(T5.4/T5.5): Flutter @if fade AnimatedSwitcher + keyed @for
// ValueKey/scroll dispatch — un-ignore when Tasks 5.4/5.5 land
// (key_expr → key: ValueKey(expr); [scroll] → ListView.builder
// dispatch; @if mode:"fade" → AnimatedSwitcher).
#[cfg(feature = "codegen-flutter")]
#[test]
#[ignore = "T5.4/T5.5: Flutter @if fade AnimatedSwitcher + keyed @for ValueKey — known gaps, owned by Tasks 5.4/5.5"]
fn flutter_control_flow_contract() {
    let source = include_str!("../../../tests/fixtures/parameterized/control_flow.omni");
    let files = compile_source_fixture(source, crate::codegen::Target::Flutter);
    assert_flutter_files(&files);
    // key_expr → key: ValueKey(expr) in the item builder (T5.5).
    assert_contains(&files, "ListPage.dart", "ValueKey");
    assert_contains(&files, "ListPage.dart", "ValueKey(item)");
    // @if mode:"fade" → AnimatedSwitcher (currently a silent no-op) (T5.4).
    assert_contains(&files, "ListPage.dart", "AnimatedSwitcher");
}

gen_semantic_dsl_tests!(
    array_methods,
    "array_methods.omni",
    assert_array_methods_omniview,
    assert_array_methods_react,
    assert_array_methods_flutter
);
gen_semantic_dsl_tests!(
    control_flow,
    "control_flow.omni",
    assert_control_flow_omniview,
    assert_control_flow_react,
    assert_control_flow_flutter
);
