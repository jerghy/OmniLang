//! LLM error-mode regression corpus (v3 plan task 7.5) — the six error
//! modes an LLM most often writes when generating .omni, each pinned by
//! CI to: diagnostic EXISTS + position EXACT + hint ACTIONABLE.
//!
//!   A. 错误事件名   — unknown @onXxx → E0006 warning + nearest suggestion
//!   B. 错误方法名   — unknown stdlib method on a typed receiver → E0006
//!                     warning + nearest stdlib suggestion
//!   C. 删除语法     — removed syntax (ADR-053 / v2 design) → hard error +
//!                     migration hint (message or structured suggestion)
//!   D. 缺 view{}    — UI component without a view block → E0006 error +
//!                     add-a-view hint (previously compiled SILENT to an
//!                     empty component — zero-diagnostic degradation)
//!   E. 错误闭包形式 — `=>` in a general expression → E_CLOSURE_ARROW;
//!                     `count()` without a predicate → E_ARRAY_METHOD
//!   F. 组件完整性   — missing required prop / unbound ref / unknown ref
//!                     op / removed @hero / store `fn` member
//!
//! Every position below was verified against the real compile pipeline
//! (`compile_multi_internal`) — line numbers are exact, not ranges, so a
//! regression that re-anchors a diagnostic (e.g. back to component start)
//! fails the gate loudly. Each category also carries a positive control
//! (the corrected source) that must compile CLEAN — proving the assertions
//! are not vacuous.
//!
//! NOTE on E_ARRAY_METHOD (case E5): the IR carries no expression-level
//! spans, so the diagnostic anchors at the enclosing declaration (the
//! 4.11 fallback contract). The exact-line assertion pins THAT contract —
//! if expression-level spans are threaded later, this test must be
//! tightened, not deleted.

use rustc_hash::FxHashMap;

use omnilang::codegen::Target;
use omnilang::compile_multi_internal;
use omnilang::error::Diagnostic;

/// One corpus fixture: a source that MUST produce a diagnostic naming
/// `expects_token` at exactly `expected_line`, with `hint` visible in the
/// message or the structured suggestion.
struct ErrorCase {
    name: &'static str,
    category: &'static str,
    src: &'static str,
    /// 1-based line the diagnostic must anchor at (exact match).
    expected_line: u32,
    /// Token the message must name (identifies the right diagnostic when
    /// several fire on one fixture).
    expects_token: &'static str,
    /// Actionable hint that must appear in message OR suggestion.
    hint: &'static str,
}

const LLM_ERROR_CASES: &[ErrorCase] = &[
    // ══════════════════════════════════════════════════════════════════
    // A. 错误事件名 (wrong event names) — element line, nearest suggestion
    // ══════════════════════════════════════════════════════════════════
    ErrorCase {
        name: "event_totally_bogus",
        category: "A 错误事件名",
        src: "import omniview;\n\
              component V {\n\
              \x20   fn f(e) {}\n\
              \x20   view {\n\
              \x20       <div @onTotallyBogus={f}/>\n\
              \x20   }\n\
              }",
        expected_line: 5,
        expects_token: "onTotallyBogus",
        hint: "最近合法建议",
    },
    ErrorCase {
        name: "event_typo_fcous",
        category: "A 错误事件名",
        src: "import omniview;\n\
              component V {\n\
              \x20   fn f(e) {}\n\
              \x20   view {\n\
              \x20       <div @onFcous={f}/>\n\
              \x20   }\n\
              }",
        expected_line: 5,
        expects_token: "onFcous",
        hint: "最近合法建议",
    },
    ErrorCase {
        name: "event_typo_clickk",
        category: "A 错误事件名",
        src: "import omniview;\n\
              component V {\n\
              \x20   fn f(e) {}\n\
              \x20   view {\n\
              \x20       <div @onClickk={f}/>\n\
              \x20   }\n\
              }",
        expected_line: 5,
        expects_token: "onClickk",
        hint: "@onClick",
    },
    ErrorCase {
        name: "event_typo_keydownn",
        category: "A 错误事件名",
        src: "import omniview;\n\
              component V {\n\
              \x20   fn f(e) {}\n\
              \x20   view {\n\
              \x20       <div @onKeyDownn={f}/>\n\
              \x20   }\n\
              }",
        expected_line: 5,
        expects_token: "onKeyDownn",
        hint: "@onKeyDown",
    },
    ErrorCase {
        name: "event_typo_wheell",
        category: "A 错误事件名",
        src: "import omniview;\n\
              component V {\n\
              \x20   fn f(e) {}\n\
              \x20   view {\n\
              \x20       <div @onWheell={f}/>\n\
              \x20   }\n\
              }",
        expected_line: 5,
        expects_token: "onWheell",
        hint: "@onWheel",
    },
    // ══════════════════════════════════════════════════════════════════
    // B. 错误方法名 (wrong method names) — call line, stdlib suggestion
    // ══════════════════════════════════════════════════════════════════
    ErrorCase {
        name: "method_to_upper",
        category: "B 错误方法名",
        src: "import omniview;\n\
              component V {\n\
              \x20   fn f(s: String) {\n\
              \x20       s.toUpper()\n\
              \x20   }\n\
              \x20   view { <div/> }\n\
              }",
        expected_line: 4,
        expects_token: "toUpper",
        hint: "to_uppercase",
    },
    ErrorCase {
        name: "method_lengh",
        category: "B 错误方法名",
        src: "import omniview;\n\
              component V {\n\
              \x20   fn f(s: String) {\n\
              \x20       s.lengh()\n\
              \x20   }\n\
              \x20   view { <div/> }\n\
              }",
        expected_line: 4,
        expects_token: "lengh",
        hint: "\"len\"",
    },
    ErrorCase {
        name: "method_puch",
        category: "B 错误方法名",
        src: "import omniview;\n\
              component V {\n\
              \x20   fn f(items: Array<i32>) {\n\
              \x20       items.puch(1)\n\
              \x20   }\n\
              \x20   view { <div/> }\n\
              }",
        expected_line: 4,
        expects_token: "puch",
        hint: "push",
    },
    ErrorCase {
        name: "method_to_stri_n",
        category: "B 错误方法名",
        src: "import omniview;\n\
              component V {\n\
              \x20   fn f(n: i32) {\n\
              \x20       n.toStrin()\n\
              \x20   }\n\
              \x20   view { <div/> }\n\
              }",
        expected_line: 4,
        expects_token: "toStrin",
        hint: "to_string",
    },
    ErrorCase {
        name: "method_containsk",
        category: "B 错误方法名",
        src: "import omniview;\n\
              component V {\n\
              \x20   fn f(cfg: Map<String, i32>) {\n\
              \x20       cfg.containsk(\"k\")\n\
              \x20   }\n\
              \x20   view { <div/> }\n\
              }",
        expected_line: 4,
        expects_token: "containsk",
        hint: "contains",
    },
    // ══════════════════════════════════════════════════════════════════
    // C. 删除语法 (deleted syntax) — hard error + migration hint
    // ══════════════════════════════════════════════════════════════════
    ErrorCase {
        name: "deleted_value_binding",
        category: "C 删除语法",
        // v4 5.4: only the v1 双向形状 (value={sig} WITH @ref) errors —
        // without @ref it is a legal plain one-way prop.
        src: "import omniview;\n\
              component V {\n\
              \x20   signal x: String = \"\";\n\
              \x20   view {\n\
              \x20       <div @ref={box} value={x}/>\n\
              \x20   }\n\
              }",
        expected_line: 5,
        expects_token: "value={sig}",
        hint: "bind:value",
    },
    ErrorCase {
        name: "deleted_ref_write",
        category: "C 删除语法",
        src: "import omniview;\n\
              component V {\n\
              \x20   ref box: Element;\n\
              \x20   fn go() { box.value = \"x\"; }\n\
              \x20   view { <div @ref={box}/> }\n\
              }",
        expected_line: 4,
        expects_token: "命令式写已删除",
        hint: "bind:value",
    },
    ErrorCase {
        name: "deleted_on_block",
        category: "C 删除语法",
        src: "import omniview;\n\
              component V {\n\
              \x20   fn f(e) {}\n\
              \x20   view {\n\
              \x20       <div on: { click: f } />\n\
              \x20   }\n\
              }",
        expected_line: 5,
        expects_token: "on: 已删除",
        hint: "@onXxx",
    },
    ErrorCase {
        name: "deleted_paused",
        category: "C 删除语法",
        src: "import omniview;\n\
              @motion m {\n\
              \x20   from { opacity: 0 }\n\
              \x20   to { opacity: 1 }\n\
              \x20   over: 300ms\n\
              \x20   paused\n\
              }\n\
              component V { view { <div/> } }",
        expected_line: 6,
        expects_token: "paused 已删除",
        hint: "@if",
    },
    ErrorCase {
        name: "deleted_slot_type_constraint",
        category: "C 删除语法",
        src: "import omniview;\n\
              component V {\n\
              \x20   @slot header: String { <span>d</span> }\n\
              \x20   view { <div>{header}</div> }\n\
              }",
        expected_line: 3,
        expects_token: "类型约束与数据参数已删除",
        hint: "父传内容",
    },
    // ══════════════════════════════════════════════════════════════════
    // D. 缺 view{} (missing view block) — component line, add-a-view hint
    // ══════════════════════════════════════════════════════════════════
    ErrorCase {
        name: "no_view_empty",
        category: "D 缺 view{}",
        src: "import omniview;\n\
              component V {\n\
              }",
        expected_line: 2,
        expects_token: "缺少 view 块",
        hint: "view { <div/> }",
    },
    ErrorCase {
        name: "no_view_signal_only",
        category: "D 缺 view{}",
        src: "import omniview;\n\
              component V {\n\
              \x20   signal x: i32 = 0;\n\
              }",
        expected_line: 2,
        expects_token: "缺少 view 块",
        hint: "view { <div/> }",
    },
    ErrorCase {
        name: "no_view_fn_only",
        category: "D 缺 view{}",
        src: "import omniview;\n\
              component V {\n\
              \x20   fn f() {}\n\
              }",
        expected_line: 2,
        expects_token: "缺少 view 块",
        hint: "view { <div/> }",
    },
    ErrorCase {
        name: "no_view_ref_only",
        category: "D 缺 view{}",
        src: "import omniview;\n\
              component V {\n\
              \x20   ref r: Element;\n\
              }",
        expected_line: 2,
        expects_token: "缺少 view 块",
        hint: "view { <div/> }",
    },
    ErrorCase {
        name: "no_view_store_ref_only",
        category: "D 缺 view{}",
        src: "import omniview;\n\
              store S {\n\
              \x20   signal x: i32 = 0;\n\
              }\n\
              component V {\n\
              \x20   @store S;\n\
              }",
        expected_line: 5,
        expects_token: "缺少 view 块",
        hint: "view { <div/> }",
    },
    // ══════════════════════════════════════════════════════════════════
    // E. 错误闭包形式 (wrong closure form) — arrow position / call position
    // ══════════════════════════════════════════════════════════════════
    ErrorCase {
        name: "closure_arrow_in_computed_filter",
        category: "E 错误闭包形式",
        src: "import omniview;\n\
              component V {\n\
              \x20   signal items: Array<i32> = [1, 2, 3];\n\
              \x20   computed evens: Array<i32> = items.filter(x => x % 2 == 0);\n\
              \x20   view { <div/> }\n\
              }",
        expected_line: 4,
        expects_token: "通用闭包用管道",
        hint: "|x|",
    },
    ErrorCase {
        name: "closure_arrow_in_computed_literal",
        category: "E 错误闭包形式",
        src: "import omniview;\n\
              component V {\n\
              \x20   signal n: i32 = 0;\n\
              \x20   computed double: i32 = (x) => x;\n\
              \x20   view { <div/> }\n\
              }",
        expected_line: 4,
        expects_token: "通用闭包用管道",
        hint: "|x|",
    },
    ErrorCase {
        name: "closure_arrow_in_fn_body",
        category: "E 错误闭包形式",
        src: "import omniview;\n\
              component V {\n\
              \x20   fn f(xs: Array<i32>) {\n\
              \x20       xs.map(x => x * 2)\n\
              \x20   }\n\
              \x20   view { <div/> }\n\
              }",
        expected_line: 4,
        expects_token: "通用闭包用管道",
        hint: "|x|",
    },
    ErrorCase {
        name: "closure_arrow_in_store_action",
        category: "E 错误闭包形式",
        src: "import omniview;\n\
              store S {\n\
              \x20   signal items: Array<i32> = [1, 2];\n\
              \x20   action go() {\n\
              \x20       items.map(x => x)\n\
              \x20   }\n\
              }\n\
              component V {\n\
              \x20   @store S;\n\
              \x20   view {\n\
              \x20       <div/>\n\
              \x20   }\n\
              }",
        expected_line: 5,
        expects_token: "通用闭包用管道",
        hint: "|x|",
    },
    ErrorCase {
        name: "array_method_count_without_predicate",
        category: "E 错误闭包形式",
        src: "import omniview;\n\
              component V {\n\
              \x20   signal items: Array<i32> = [1, 2, 3];\n\
              \x20   computed c: i32 = items.count();\n\
              \x20   view { <div/> }\n\
              }",
        // E_ARRAY_METHOD anchors at the ENCLOSING DECLARATION (IR carries
        // no expression spans — the 4.11 declaration-anchoring fallback
        // contract). The computed sits on line 4; the diagnostic on line 2.
        // Pin the current contract; tighten when IR gains expr spans.
        expected_line: 2,
        expects_token: "count 需要一个谓词闭包",
        hint: "count",
    },
    // ══════════════════════════════════════════════════════════════════
    // F. 组件完整性 (component integrity) — instantiation / declaration
    // ══════════════════════════════════════════════════════════════════
    ErrorCase {
        name: "missing_required_prop",
        category: "F 组件完整性",
        src: "import omniview;\n\
              component Child { prop name: String; view { <span>{name}</span> } }\n\
              component App { view { <Child /> } }",
        expected_line: 3,
        expects_token: "required prop 'name'",
        hint: "add 'name'",
    },
    ErrorCase {
        name: "ref_declared_never_bound",
        category: "F 组件完整性",
        src: "import omniview;\n\
              component V {\n\
              \x20   ref r: Element;\n\
              \x20   view {\n\
              \x20       <div/>\n\
              \x20   }\n\
              }",
        // Anchors at the component declaration (ref-level spans are not
        // threaded into this pass) — verified pipeline position.
        expected_line: 2,
        expects_token: "ref 'r' is declared but never bound",
        hint: "@ref={r}",
    },
    ErrorCase {
        name: "unknown_ref_operation",
        category: "F 组件完整性",
        src: "import omniview;\n\
              component V {\n\
              \x20   ref el: Element;\n\
              \x20   fn go() { el.bogusOp(); }\n\
              \x20   view { <div @ref={el}/> }\n\
              }",
        expected_line: 4,
        expects_token: "bogusOp",
        hint: "unknown ref operation",
    },
    ErrorCase {
        name: "hero_removed",
        category: "F 组件完整性",
        src: "import omniview;\n\
              component V {\n\
              \x20   view { <div @hero=\"fade\" /> }\n\
              }",
        expected_line: 3,
        expects_token: "@hero",
        hint: "@enter",
    },
    ErrorCase {
        name: "store_fn_member",
        category: "F 组件完整性",
        src: "import omniview;\n\
              store S {\n\
              \x20   signal x: i32 = 0;\n\
              \x20   fn helper() {}\n\
              }\n\
              component V {\n\
              \x20   @store S;\n\
              \x20   view {\n\
              \x20       <div/>\n\
              \x20   }\n\
              }",
        expected_line: 4,
        expects_token: "store 内 fn 成员不允许",
        hint: "getter 或 action",
    },
];

/// Compile one source through the full pipeline and return its diagnostics
/// (semantic errors + warnings in `errors`; lint warnings in `warnings` —
/// merged so both channels are covered, mirroring `diagnostic_position.rs`).
fn compile_with_diags(src: &str, target: Target) -> ((), Vec<Diagnostic>) {
    let sources = FxHashMap::from_iter([("input.omni".to_string(), src.to_string())]);
    let result = compile_multi_internal(sources, target, FxHashMap::default(), false, false, None);
    let mut all = result.errors;
    all.extend(result.warnings);
    ((), all)
}

/// Corpus gate — every fixture must produce a diagnostic that names the
/// offending token, anchors at the EXACT expected line, and carries an
/// actionable hint (in the message or the structured suggestion).
#[test]
fn llm_error_corpus_diagnoses() {
    assert_eq!(LLM_ERROR_CASES.len(), 30, "corpus must be 6 categories × 5");
    for case in LLM_ERROR_CASES {
        let (_, diags) = compile_with_diags(case.src, Target::React);
        let d = diags
            .iter()
            .find(|d| d.message.contains(case.expects_token))
            .unwrap_or_else(|| {
                panic!(
                    "[{}] case {}: no diagnostic names {:?}; got: {:?}",
                    case.category,
                    case.name,
                    case.expects_token,
                    diags.iter().map(|d| &d.message).collect::<Vec<_>>()
                )
            });
        assert_eq!(
            d.line, case.expected_line,
            "[{}] case {}: wrong position (expected line {}, got {} with col {}): {}",
            case.category, case.name, case.expected_line, d.line, d.col, d.message
        );
        let hint_source = format!("{} {:?}", d.message, d.suggestion);
        assert!(
            hint_source.contains(case.hint),
            "[{}] case {}: hint {:?} not actionable in message/suggestion; got: {} / {:?}",
            case.category,
            case.name,
            case.hint,
            d.message,
            d.suggestion
        );
    }
}

/// Category coverage — each of the six categories must have exactly five
/// registered fixtures (guards the corpus shape contract).
#[test]
fn llm_error_corpus_six_categories_five_each() {
    let mut counts: std::collections::BTreeMap<&'static str, usize> = Default::default();
    for case in LLM_ERROR_CASES {
        *counts.entry(case.category).or_default() += 1;
    }
    assert_eq!(
        counts.len(),
        6,
        "corpus must span exactly 6 categories, got: {counts:?}"
    );
    for (category, n) in &counts {
        assert_eq!(
            *n, 5,
            "category {category:?} must register exactly 5 fixtures, got {n}"
        );
    }
}

/// Positive controls — the corrected form of each error mode must compile
/// CLEAN (zero diagnostics). This is what makes the negative assertions
/// non-vacuous: the same harness that flags the wrong form must clear the
/// right form, so a compiler that never diagnoses anything fails HERE.
#[test]
fn llm_error_corpus_positive_controls_compile_clean() {
    let clean: &[(&str, &str)] = &[
        // A — canonical event name, no warning.
        ("event_correct",
         "import omniview;\ncomponent V {\n  fn f(e) {}\n  view { <div @onPress={f}/> }\n}"),
        // B — canonical stdlib method, no warning (f bound to an event so
        // the dead-code lint does not fire on the control itself).
        ("method_correct",
         "import omniview;\ncomponent V {\n  fn f(e) {\n    let s = \"hi\";\n    s.to_uppercase();\n  }\n  view { <button @onPress={f}>b</button> }\n}"),
        // C — bind:value two-way binding instead of value={sig}.
        ("deleted_syntax_correct",
         "import omniview;\ncomponent V {\n  signal x: String = \"\";\n  view { <input bind:value={x} /> }\n}"),
        // D — view block present.
        ("view_correct",
         "import omniview;\ncomponent V {\n  signal x: i32 = 0;\n  view { <div>{x}</div> }\n}"),
        // E — pipe closure in a general expression (evens rendered so the
        // dead-code lint does not fire on the control itself).
        ("closure_correct",
         "import omniview;\ncomponent V {\n  signal items: Array<i32> = [1, 2, 3];\n  computed evens: Array<i32> = items.filter(|x| x % 2 == 0);\n  view { <div>{evens.len()}</div> }\n}"),
        // F — prop provided on the child instantiation.
        ("integrity_correct",
         "import omniview;\ncomponent Child { prop name: String; view { <span>{name}</span> } }\ncomponent App { view { <Child name=\"x\" /> } }"),
    ];
    for (name, src) in clean {
        let (_, diags) = compile_with_diags(src, Target::React);
        assert!(
            diags.is_empty(),
            "positive control {name} must compile clean, got: {:?}",
            diags.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }
}

/// Generated-code traceability (task 7.5): every component file emitted
/// from a REAL compile must carry the `// [omni] <file>:<line>` marker
/// mapping back to the .omni declaration. The marker is what makes the
/// regeneration contract ("edit .omni, never the generated output")
/// enforceable — a generated file without it is untraceable.
#[test]
fn generated_component_files_carry_source_marker() {
    let src = "import omniview;\n\
               component TraceMe {\n\
               \x20   signal x: i32 = 0;\n\
               \x20   view { <div>{x}</div> }\n\
               }";
    let sources = FxHashMap::from_iter([("input.omni".to_string(), src.to_string())]);
    let result = compile_multi_internal(
        sources,
        Target::React,
        FxHashMap::default(),
        false,
        false,
        None,
    );
    assert!(result.success, "must compile, errors: {:?}", result.errors);
    let tsx = result
        .files
        .get("TraceMe.tsx")
        .expect("TraceMe.tsx must be emitted");
    assert!(
        tsx.starts_with("// Generated by OmniLang — React target\n// [omni] input.omni:2\n"),
        "tsx must open with the Generated + [omni] source marker, got:\n{}",
        &tsx[..tsx.len().min(160)]
    );
    // Flutter mirrors the marker on component .dart files (P0 parity).
    #[cfg(feature = "codegen-flutter")]
    {
        let sources = FxHashMap::from_iter([("input.omni".to_string(), src.to_string())]);
        let result = compile_multi_internal(
            sources,
            Target::Flutter,
            FxHashMap::default(),
            false,
            false,
            None,
        );
        assert!(result.success, "flutter must compile: {:?}", result.errors);
        let dart = result.files.get("TraceMe.dart").expect("TraceMe.dart");
        assert!(
            dart.contains("// [omni] input.omni:2"),
            "dart component file must carry the [omni] marker, got:\n{}",
            &dart[..dart.len().min(160)]
        );
    }
}
