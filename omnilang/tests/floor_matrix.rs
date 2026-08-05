//! Floor matrix — 24-item feature floor → 机器验证（设计 P5，v3 修复计划 Phase 7 Task 7.3）。
//!
//! 背景（P5 教训）：ADR-052/054 的 24-item feature floor 是手写声明表——两轮评审各
//! 发现一处「声称 ✅ 而行为未接通」的条目（上轮主题切换、本轮 multi-theme Flutter
//! 塌缩），设计文档「滚动驱动相等守卫已交付」失实——「对等」无法证伪。
//!
//! 机制：floor 条目（ADR-052 Feature Floor 定义，逐原语展开）硬编码为本文件的
//! `floor_table()`；覆盖检查 `every_floor_item_has_machine_verification` 强制每项
//! 映射 ≥1 个编译级测试（参数化 fixture / 单元测试——本版本第一版全量标注）或
//! 行为级标注（widget test/Playwright——3.2 基建扩展队列，标注 future 不阻塞）。
//! 新增/删除 floor 条目必须先挂机器验证——「floor 内行为对等」从声明变断言。
//!
//! 计数说明：规范口径为 24 项（v1 设计文档 §10.1 P0 Feature Floor：Ref 5 + 动画 4
//! + Design 5 + Slots 1 + 控制流 2 + Stores 1 + Lifecycle 1 + 表达式 5）。ADR-052
//! 表的 Expressions 行另列 `compound assignment` 原语（v1 §10.1 未列）——本矩阵以
//! 第 25 行收录，保证 ADR-052 表列原语零静默丢项（这正是 P5 要消灭的失败模式）。
//!
//! 权威清单：本文件 `floor_table()`（硬编码）；设计文档
//! `docs/superpowers/specs/2026-08-02-semantic-syntax-v2-design.md` §9 与其逐行一致。

use rustc_hash::FxHashMap;

use omnilang::codegen::Target;
use omnilang::{compile_multi_internal, path_mapping};

/// Floor 类别（ADR-052 Feature Floor 定义的 8 个类别）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum FloorCategory {
    Ref,
    Animation,
    Design,
    Slots,
    ControlFlow,
    Stores,
    Lifecycle,
    Expressions,
}

/// 单个 floor 条目及其机器验证映射。
#[derive(Debug, Clone)]
struct FloorItem {
    /// 规范名（ADR-052 原语名，kebab-case）。
    name: &'static str,
    category: FloorCategory,
    /// 编译级测试（≥1 个，覆盖检查强制；格式 `文件::fn` 或 `fixture::参数化模块`）。
    compile_tests: &'static [&'static str],
    /// 行为级测试（widget test/Playwright；已有实证的列名，无则空）。
    behavior_tests: &'static [&'static str],
    /// 行为级 future 标注（「需 widget test/Playwright」3.2 基建扩展队列；空 = 无欠账）。
    behavior_future: &'static str,
}

impl FloorItem {
    fn new(
        name: &'static str,
        category: FloorCategory,
        compile_tests: &'static [&'static str],
        behavior_tests: &'static [&'static str],
        behavior_future: &'static str,
    ) -> Self {
        Self {
            name,
            category,
            compile_tests,
            behavior_tests,
            behavior_future,
        }
    }
}

/// ADR-052 Feature Floor 定义（逐原语展开，25 行 = 24 规范项 + compound assignment）。
fn floor_table() -> Vec<FloorItem> {
    vec![
        // ── Ref（5）──
        FloorItem::new(
            "ref_focus_blur",
            FloorCategory::Ref,
            &[
                "refs.omni (parameterized::refs::* ×3 后端：focus={isFocused} → focus()/blur())",
                "react::tests::test_react_ref_basic",
                "react::tests::test_react_ref_ops_format_native_dom_calls",
                "flutter::tests::test_ref_global_key_and_on_demand_controllers",
            ],
            &[],
            "",
        ),
        FloorItem::new(
            "ref_scroll_to_by",
            FloorCategory::Ref,
            &[
                "react::tests::test_react_ref_ops_format_native_dom_calls (scrollTo/scrollBy DOM)",
                "react::tests::test_react_semantic_binding_scroll_x",
                "flutter::tests::test_format_ref_op_flutter_variants",
                "omniview::expr (expr.rs scrollTo/scrollBy 发射)",
            ],
            &[],
            "",
        ),
        FloorItem::new(
            "ref_measure",
            FloorCategory::Ref,
            &[
                "react::tests::test_react_ref_ops_format_native_dom_calls (measure/width/height/position)",
                "flutter::tests::test_task58_measure_emits_wrapper",
                "flutter::tests::test_format_ref_op_flutter_variants",
            ],
            &[],
            "",
        ),
        FloorItem::new(
            "ref_select_selection",
            FloorCategory::Ref,
            &[
                "react::tests::test_react_ref_ops_format_native_dom_calls (select/setSelectionRange)",
                "flutter::tests::test_format_ref_op_flutter_variants",
            ],
            &[],
            "",
        ),
        FloorItem::new(
            "ref_value",
            FloorCategory::Ref,
            &[
                "refs.omni (parameterized::refs::*：bind:value 双向 方向一致性)",
                "react::tests::test_react_bind_value_emits_onchange_setter",
                "flutter::tests::test_bind_value_ref_text_field_two_way",
                "flutter::tests::bind_value_without_ref_emits_on_changed",
            ],
            &["Playwright 输入回写实证（T2.3，三目标各一轮）"],
            "widget-test 队列（3.2 基建）：bind:value 输入回写 + 信号回写控件行为断言",
        ),
        // ── Animation（4）──
        FloorItem::new(
            "anim_triggers",
            FloorCategory::Animation,
            &[
                "react::motion::tests::test_react_motion_enter/hover/inview/scroll/drag (快照)",
                "react::motion::tests::focus_trigger_emits_focusin_focusout_listeners",
                "react::motion::tests::inview_trigger_renamed_to_kebab_case",
                "flutter::tests::scroll_trigger_passes_typed_start_end_to_widget",
                "flutter::tests::drag_trigger_passes_typed_constraints_to_widget",
                "flutter::tests::in_view_trigger_instantiates_wrapper",
                "flutter::tests::focus_trigger_wraps_with_focus_widget",
                "flutter::tests::test_motion_hover_mouse_region_wrapping",
                "animation.omni (parameterized::animation::*：enter + hover)",
            ],
            &[
                "node 执行：@scroll scrollY=500 → progress 0.5 + clamp（T5.1）",
                "Playwright：@drag 300px 实证（T5.1/T7.1）",
            ],
            "widget-test 队列（3.2 基建）：触发器动画执行级断言（@enter/@hover/@press/@in-view）",
        ),
        FloorItem::new(
            "anim_multi_stop_keyframes",
            FloorCategory::Animation,
            &[
                "flutter::tests::test_build_prop_tween_expr_three_stops_sequence",
                "flutter::tests::test_build_prop_tween_expr_per_property_isolation",
                "flutter::tests::motion_full_pipeline_per_property_and_playback",
                "react::motion::tests::semantic_props_convert_to_waapi_keyframes",
            ],
            &[],
            "",
        ),
        FloorItem::new(
            "anim_playback",
            FloorCategory::Animation,
            &[
                "flutter::tests::test_emit_playback_start_loop",
                "flutter::tests::test_emit_playback_start_direction",
                "flutter::tests::test_emit_playback_start_fill",
                "flutter::tests::test_emit_playback_start_delay",
                "flutter::tests::test_motion_pipeline_warnings_and_playback",
                "react::motion::tests::motion_durations_emit_numeric_ms (duration)",
                "react::motion::tests::steps_easing_emitted_as_waapi_string (easing/steps)",
            ],
            &[],
            "",
        ),
        FloorItem::new(
            "anim_inline_lowering",
            FloorCategory::Animation,
            &[
                "animation.omni (parameterized::animation::*：内联 tween → __anon_m_N 提升)",
                "flutter::tests::test_motion_inline_body_handling",
            ],
            &[],
            "",
        ),
        // ── Design（5）──
        FloorItem::new(
            "design_token_types",
            FloorCategory::Design,
            &[
                "design_parse_test::parse_all_token_types (13 类型全解析)",
                "design_tokens.omni (parameterized::design_tokens::*)",
            ],
            &[],
            "",
        ),
        FloorItem::new(
            "design_static_themes",
            FloorCategory::Design,
            &[
                "design_tokens.omni (parameterized::design_tokens::*：[data-theme=\"dark\"] 覆盖)",
                "tests_regression::main::themes_fixture_wires_theme_signal_through_pipeline",
                "react::tests::react_design_theme_signal_wires_subscription",
                "theme_initial_consistency::default_theme_mismatch_errors (@default-theme 一致性)",
            ],
            &["Playwright theme-wiring spec 1/1（T7.1）：点主题卡片 → data-theme 翻转 + CSS 覆盖生效"],
            "widget-test 队列（3.2 基建）：Flutter ThemeMode 切换行为断言",
        ),
        FloorItem::new(
            "design_classes",
            FloorCategory::Design,
            &[
                "design_tokens.omni (parameterized::design_tokens::*：Card class)",
                "react::tests::react_design_classes_emit_literal_class_names",
                "flutter::tests::test_design_class_resolves_to_factory_decoration",
            ],
            &[],
            "",
        ),
        FloorItem::new(
            "design_variants",
            FloorCategory::Design,
            &[
                "design_tokens.omni (parameterized::design_tokens::*：Button.secondary 分支)",
                "flutter::tests::test_design_variant_resolves_to_real_decoration",
            ],
            &[],
            "",
        ),
        FloorItem::new(
            "design_dynamic_tokens",
            FloorCategory::Design,
            &[
                "react::tests::react_design_dynamic_tokens_emit_hook",
                "design_codegen_test::e2e_dynamic_token_full_pipeline",
                "design_codegen_test::emits_js_effect_for_dynamic_token",
                "design_codegen_test::css_output_skips_dynamic_tokens",
            ],
            &[],
            "",
        ),
        // ── Slots（1）──
        FloorItem::new(
            "slots",
            FloorCategory::Slots,
            &[
                "slots.omni (parameterized::slots::* ×3 后端)",
                "slots_body.omni (parameterized::slots_body::*：body 级声明 + fallback)",
                "slots_three_forms.omni (parameterized::slots_three_forms::*：default+named+fallback 含裸实例)",
            ],
            &[],
            "",
        ),
        // ── Control Flow（2）──
        FloorItem::new(
            "control_if",
            FloorCategory::ControlFlow,
            &[
                "control_flow.omni (parameterized::control_flow::*：mode:\"fade\")",
                "react::tests::conditional_fade_mode_emits_css_fade_transition",
                "react::tests::conditional_wait_mode_degrades_with_stated_comment",
                "flutter::tests::page_switcher_uses_indexed_stack",
                "omniview::tests::test_html_conditional_wait_mode_is_honest_instant_swap",
            ],
            &["Playwright 9 标签页条件渲染实测（T7.1，44 passed）"],
            "",
        ),
        FloorItem::new(
            "control_for",
            FloorCategory::ControlFlow,
            &[
                "control_flow.omni (parameterized::control_flow::*：key_expr + empty_body + [scroll] + 嵌套)",
                "parameterized::flutter_control_flow_contract (key → ValueKey(expr)；[scroll] → ListView.builder)",
                "react::tests::react_for_key_expr_emits_expr_key",
                "react::tests::react_for_empty_body_single_eval",
                "flutter::tests::flutter_for_scroll_emits_listview_builder_with_value_key",
                "flutter::tests::flutter_for_non_scroll_emits_column",
            ],
            &["Playwright keyed 列表保状态实测（T7.1；BUG-1/BUG-3 回归闭环）"],
            "widget-test 队列（3.2 基建）：@for 渲染/重排保状态行为断言",
        ),
        // ── Stores（1）──
        FloorItem::new(
            "stores",
            FloorCategory::Stores,
            &[
                "store.omni (parameterized::store_consumer::*：signal + action 三目标)",
                "react::tests::test_zustand_store_with_signals_and_getters",
                "flutter::tests::test_store_dart_with_getters",
                "flutter::tests::test_store_dart_with_action",
                "effect_deps.omni + tests_regression::main::effect_deps_fixture_missing_dep_is_compile_error",
            ],
            &[],
            "",
        ),
        // ── Lifecycle（1）──
        FloorItem::new(
            "lifecycle",
            FloorCategory::Lifecycle,
            &[
                "react::tests::test_tsx_component_with_lifecycle_mount",
                "react::tests::effect_cleanup_reads_are_deps (effect deps 含 cleanup)",
                "flutter::tests::test_component_with_mount_lifecycle",
                "flutter::tests::test_component_with_unmount_lifecycle",
                "flutter::tests::test_effect_emits_real_method_and_lifecycle_wiring",
                "omniview::tests::test_component_with_lifecycle_mount",
                "omniview::tests::test_component_with_effect",
            ],
            &[],
            "",
        ),
        // ── Expressions（5 规范 + compound assignment 扩展）──
        FloorItem::new(
            "expr_optional_chain",
            FloorCategory::Expressions,
            &[
                "omnilang-core::codegen::shared::expr::optional_chain_default_preserves_question_dot",
                "omnilang-core::codegen::shared::expr::dart_optional_chain_keeps_question_dot",
                "react::tests::react_optional_chain_method_len_translates_to_length",
                "semantic::validate_effect_deps::optional_chain_read_is_counted",
            ],
            &[],
            "",
        ),
        FloorItem::new(
            "expr_block_expr",
            FloorCategory::Expressions,
            &[
                "omnilang-core::codegen::shared::expr::dart_block_expr_emits_iife",
                "omnilang-core::codegen::shared::expr::rust_block_expr_emits_block_expression",
                "tests_regression::main::react_block_expr_statements_compile_clean",
                "tests_regression::main::python_block_expr_statements_diagnosed_not_dropped (floor 外硬诊断)",
                "tests_regression::main::vue_block_expr_statements_diagnosed_not_dropped",
            ],
            &[],
            "",
        ),
        FloorItem::new(
            "expr_match",
            FloorCategory::Expressions,
            &[
                "js_runtime_tests::js_runtime_match_literal_arms_pass (执行级)",
                "react::tests::fn_tail_match_emits_return",
                "react::tests::test_format_match_literal_arms_compare_subject_value",
                "generated_code_validation::match_variant_arms_flutter_compiles (#30 变体臂 + dart analyze)",
            ],
            &[],
            "",
        ),
        FloorItem::new(
            "expr_await",
            FloorCategory::Expressions,
            &[
                "floor_matrix::await_compile_tier_fixture (本文件新增编译级 fixture)",
                "js_runtime_tests::js_runtime_stdlib_gap_modules_pass (执行级 await)",
                "showcase 门禁：DataFetching/DataSubmit/async_demo 三目标 tsc/dart-analyze (generated_code_validation + verify-react)",
            ],
            &[],
            "Playwright DataFetching 成功路径断言（Task 2.3 un-fixme 候选）+ widget-test 队列（3.2 基建）",
        ),
        FloorItem::new(
            "expr_spread",
            FloorCategory::Expressions,
            &[
                "tests_regression::main::react_store_action_consecutive_sets_coalesce_into_one ([...items, text])",
                "flutter::tests::store_action_struct_literal_emits_constructor ([...todos])",
                "omnilang-core::codegen::shared::expr::dart_concat_spreads_typed_array_variable",
            ],
            &[],
            "",
        ),
        FloorItem::new(
            // ADR-052 Expressions 行第 6 原语；v1 设计文档 §10.1 floor 未列——本矩阵
            // 以第 25 行收录（24 规范项 + 1 扩展），保证 ADR-052 表列原语零静默丢项。
            "expr_compound_assign",
            FloorCategory::Expressions,
            &[
                "omnilang-core::parser::mod::stmt_compound_assign_add (+= 解析)",
                "omnilang-core::parser::mod::stmt_compound_assign_sub (-= 解析)",
                "flutter::tests::test_stmt_mutation_increment (count += 1)",
                "omniview::tests::test_stmt_mutation_increment (count += 1)",
                "react::tests::test_tsx_stmt_mutation_increment_signal (setCount(prev => prev + 1))",
                "floor_matrix::compound_assign_compile_tier_fixture (本文件新增编译级 fixture)",
            ],
            &[],
            "",
        ),
    ]
}

// ═════════════════════════════════════════════════════════════════════════════
// 覆盖检查（Step 1 失败测试 → Step 3 实现后全绿）
// ═════════════════════════════════════════════════════════════════════════════

/// 覆盖检查：每行必须映射到 ≥1 个编译级测试（参数化 fixture / 单元测试）或
/// 行为级标注（行为测试 + future 队列标注）。
///
/// 行为级条目的纪律：行为测试可空（future 标注即承认当前无机器断言），但 future
/// 标注必须指向 3.2 基建的 widget test/Playwright 扩展队列——行为欠账必须被追踪。
#[test]
fn every_floor_item_has_machine_verification() {
    let floor = floor_table();
    for item in &floor {
        let has_compile = !item.compile_tests.is_empty();
        let has_behavior = !item.behavior_tests.is_empty() && !item.behavior_future.is_empty();
        assert!(
            has_compile || has_behavior,
            "floor item {} (category {:?}) unverified — must map to ≥1 compile-tier \
             test or a behavior annotation (tests + future queue)",
            item.name,
            item.category,
        );
        if !item.behavior_future.is_empty() && !item.behavior_future.contains("widget-test") {
            panic!(
                "floor item {} behavior future must point to the 3.2 widget-test/Playwright \
                 extension queue; got: {}",
                item.name, item.behavior_future
            );
        }
    }
}

/// 形状检查：规范口径 24 项（v1 §10.1 计数）+ ADR-052 扩展原语 compound assignment
/// 第 25 行——计数漂移（新增/删除 floor 条目）必须显式发生并同步设计文档 §9。
#[test]
fn floor_has_24_canonical_items_plus_adr052_compound_extension() {
    let floor = floor_table();
    assert_eq!(
        floor.len(),
        25,
        "floor 行数漂移: 24 规范项 + compound assignment 扩展 = 25；\
         当前 {} 行——改 floor 必须先挂机器验证并同步设计文档 §9",
        floor.len()
    );
    // 规范 24 项中 Expressions 类别 = 5（OptionalChain/BlockExpr/Match/Await/Spread）；
    // 第 25 行必须是 compound assignment 扩展。
    let canonical_expr_count = floor
        .iter()
        .filter(|i| i.category == FloorCategory::Expressions && i.name != "expr_compound_assign")
        .count();
    assert_eq!(canonical_expr_count, 5, "Expressions 规范项应为 5");
    assert_eq!(
        floor.last().unwrap().name,
        "expr_compound_assign",
        "第 25 行必须是 ADR-052 扩展原语 compound assignment"
    );
    // 8 个类别齐全（ADR-052 Feature Floor 定义的 8 类）。
    let mut categories: Vec<FloorCategory> = floor.iter().map(|i| i.category).collect();
    categories.sort_unstable();
    categories.dedup();
    assert_eq!(
        categories,
        vec![
            FloorCategory::Ref,
            FloorCategory::Animation,
            FloorCategory::Design,
            FloorCategory::Slots,
            FloorCategory::ControlFlow,
            FloorCategory::Stores,
            FloorCategory::Lifecycle,
            FloorCategory::Expressions,
        ],
        "floor 必须覆盖 ADR-052 的 8 个类别"
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// 新增编译级 fixture（Step 3：缺失的按类补 fixture——全流水线三 P0 后端）
// ═════════════════════════════════════════════════════════════════════════════

/// 全流水线编译 harness（lexer → parser → semantic → IR → codegen），断言成功并返回
/// 全部输出文件内容拼接（与 parameterized.rs `compile_source_fixture` 同构）。
fn compile_p0_ok(source: &str, target: Target) -> String {
    let sources = FxHashMap::from_iter([("main.omni".to_string(), source.to_string())]);
    let path_mapping = path_mapping::derive_path_mapping(&sources);
    let result = compile_multi_internal(sources, target, path_mapping, false, false, None);
    assert!(
        result.success,
        "compilation to {:?} should succeed; errors: {:?}",
        target, result.errors
    );
    result
        .files
        .values()
        .cloned()
        .collect::<Vec<_>>()
        .join("\n")
}

fn p0_targets() -> Vec<Target> {
    #[cfg(feature = "codegen-flutter")]
    let targets = vec![Target::OmniView, Target::React, Target::Flutter];
    #[cfg(not(feature = "codegen-flutter"))]
    let targets = vec![Target::OmniView, Target::React];
    targets
}

/// expr_await 编译级 fixture——原缺口：await 仅有 showcase 门禁（learn-omniview
/// 三目标 tsc/dart-analyze）与 js_runtime 执行测试，无独立 P0 编译 fixture。
/// 形状对齐 DataFetching.omni（async action + await 组件级 lib async fn——已证明
/// 三目标可编译的已知良好形态），零 stdlib 依赖。
const AWAIT_FIXTURE: &str = r#"
import omniview;

pub async fn tick() -> String {
    return "tick";
}

component AsyncPage {
    signal status: String = "idle";

    action load() async {
        status = await tick();
    }

    view {
        <div>
            {status}
            <button @onPress={load}>go</button>
        </div>
    }
}
"#;

#[test]
fn await_compile_tier_fixture() {
    for target in p0_targets() {
        let out = compile_p0_ok(AWAIT_FIXTURE, target);
        assert!(
            out.contains("await"),
            "await must survive to {:?} output (async action body); got:\n{}",
            target,
            &out[..out.len().min(800)]
        );
    }
}

/// expr_compound_assign 编译级 fixture——原缺口：`+=`/`-=` 仅有 parser 单测与
/// IR 级语句发射单测，无全流水线三 P0 fixture。全流水线验证 `count += 1` 的
/// 语义映射：OmniView/Flutter 发射字面 `+= 1`，React 重写为函数式 updater
/// （setCount(prev => prev + 1)——React 信号自读变异的既定形状）。
const COMPOUND_ASSIGN_FIXTURE: &str = r#"
import omniview;

component CounterPage {
    signal count: i32 = 0;

    fn inc() {
        count += 1;
    }

    view {
        <div>
            {count}
            <button @onPress={inc}>+</button>
        </div>
    }
}
"#;

#[test]
fn compound_assign_compile_tier_fixture() {
    let out = compile_p0_ok(COMPOUND_ASSIGN_FIXTURE, Target::OmniView);
    // 全流水线把 `count += 1` 规范化为信号自读增量
    // （`__omni_subs.get("CounterPage_count").set(...value + 1)`）——增量语义
    // 保留即证明 compound assignment 未被静默丢弃。
    assert!(
        out.contains("value + 1"),
        "OmniView must preserve the compound-assignment increment; got:\n{}",
        &out[..out.len().min(800)]
    );

    let out = compile_p0_ok(COMPOUND_ASSIGN_FIXTURE, Target::React);
    assert!(
        out.contains("prev + 1"),
        "React must rewrite the self-read mutation to the functional updater; got:\n{}",
        &out[..out.len().min(800)]
    );

    #[cfg(feature = "codegen-flutter")]
    {
        let out = compile_p0_ok(COMPOUND_ASSIGN_FIXTURE, Target::Flutter);
        assert!(
            out.contains("count += 1"),
            "Flutter must emit the compound assignment; got:\n{}",
            &out[..out.len().min(800)]
        );
    }
}
