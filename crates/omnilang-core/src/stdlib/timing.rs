use super::{StdlibFn, ALL_TARGETS};
use crate::codegen::Target;

/// std/timing 注册（spec §2.5）：debounce/throttle 为纯语义时间函数——
/// 时间语义跨端一致，非视图意图。import 形态 `from "std/timing"` 由 IR 层
/// strip 前缀后按裸模块名 "timing" 解析（async_mod 同款先例）。
/// 本任务注册类型/arity 供类型检查与三端 codegen 统一解析（V9 语义已在
/// 批次 4 Task 4 验证）；三端真实映射由 Task 7/9/11 落地，此处非 JS 族
/// 以可调用占位形态保证 import + 调用可完整到达 codegen（async_spawn 的
/// Dart 占位 `task.clone()` 同款先例）。
pub(super) fn register() -> Vec<StdlibFn> {
    vec![
        StdlibFn {
            module: "timing",
            name: "debounce",
            min_args: 2,
            max_args: Some(2),
            available_targets: &ALL_TARGETS,
            codegen: codegen_debounce,
            is_async: false,
        },
        StdlibFn {
            module: "timing",
            name: "throttle",
            min_args: 2,
            max_args: Some(2),
            available_targets: &ALL_TARGETS,
            codegen: codegen_throttle,
            is_async: false,
        },
    ]
}

/// debounce(fn, ms) → 防抖后的可调用函数。JS 族为自包含 setTimeout 包装
/// （无运行时依赖，表达式形态可直接作 @onClick/@onInput 处理器）；其余
/// 族为占位形态（真实映射见 Task 7/9/11）。
///
/// 调用形态 `(_fn.current ?? _fn)(...)`（Task 12）：React 端时序包装的
/// 首参可能被编译成 `{fn}_ref`（useRef 稳定引用——见 React view.rs
/// TIMING_FUNCS 臂）。普通 fn 直传时 `.current` 为 undefined，`??` 落回
/// 原 fn，两种形态统一可调。
pub(super) fn codegen_debounce(args: &[String], target: Target) -> String {
    let fn_expr = &args[0];
    let ms = &args[1];
    match target {
        Target::JavaScript
        | Target::TypeScript
        | Target::React
        | Target::Vue
        | Target::OmniView => {
            format!(
                "((_fn, _ms) => {{ let _t; return (..._a) => {{ clearTimeout(_t); _t = setTimeout(() => (_fn.current ?? _fn)(..._a), _ms); }}; }})({fn_expr}, {ms})"
            )
        }
        // 占位：保持可调用形态（透传调用，ms 暂未参与语义——Task 7/9/11 映射）
        Target::Python => {
            format!("(lambda _fn, _ms: (lambda *_a: _fn(*_a)))({fn_expr}, {ms})")
        }
        // 占位：返回原 fn（无节流语义——Task 7/9/11 映射）
        Target::Rust | Target::Dioxus | Target::Dart | Target::Flutter => fn_expr.clone(),
    }
}

/// throttle(fn, ms) → 节流后的可调用函数（trailing-only 语义：窗口内
/// 至多触发一次，且取窗口内最后一次调用——批次 4 Task 5 minor-1 拍板
/// P6「trailing-only 默认」）。JS 族为自包含 setTimeout 包装（窗口首调
/// 起计时，窗口尾以最新参数触发一次）；其余族占位同上。
///
/// 调用形态 `(_fn.current ?? _fn)(...)` 同上（React 端 `{fn}_ref` 稳定
/// 引用兼容）。
pub(super) fn codegen_throttle(args: &[String], target: Target) -> String {
    let fn_expr = &args[0];
    let ms = &args[1];
    match target {
        Target::JavaScript
        | Target::TypeScript
        | Target::React
        | Target::Vue
        | Target::OmniView => {
            format!(
                "((_fn, _ms) => {{ let _t; let _args; return (..._a) => {{ _args = _a; if (_t === undefined) {{ _t = setTimeout(() => {{ _t = undefined; (_fn.current ?? _fn)(..._args); }}, _ms); }} }}; }})({fn_expr}, {ms})"
            )
        }
        // 占位：保持可调用形态（透传调用，ms 暂未参与语义——Task 7/9/11 映射）
        Target::Python => {
            format!("(lambda _fn, _ms: (lambda *_a: _fn(*_a)))({fn_expr}, {ms})")
        }
        // 占位：返回原 fn（无节流语义——Task 7/9/11 映射）
        Target::Rust | Target::Dioxus | Target::Dart | Target::Flutter => fn_expr.clone(),
    }
}
