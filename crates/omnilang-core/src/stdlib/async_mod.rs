use super::{StdlibFn, ALL_TARGETS};
use crate::codegen::Target;

pub(super) fn register() -> Vec<StdlibFn> {
    vec![
        StdlibFn {
            module: "async",
            name: "async_sleep",
            available_targets: &ALL_TARGETS,
            codegen: codegen_async_sleep,
            is_async: true,
        },
        StdlibFn {
            module: "async",
            name: "async_spawn",
            available_targets: &ALL_TARGETS,
            codegen: codegen_async_spawn,
            is_async: true,
        },
        StdlibFn {
            module: "async",
            name: "async_all",
            available_targets: &ALL_TARGETS,
            codegen: codegen_async_all,
            is_async: true,
        },
        StdlibFn {
            module: "async",
            name: "async_race",
            available_targets: &ALL_TARGETS,
            codegen: codegen_async_race,
            is_async: true,
        },
        StdlibFn {
            module: "async",
            name: "async_timeout",
            available_targets: &ALL_TARGETS,
            codegen: codegen_async_timeout,
            is_async: true,
        },
    ]
}

pub(super) fn codegen_async_sleep(args: &[String], target: Target) -> String {
    let ms = &args[0];
    match target {
        Target::JavaScript
        | Target::TypeScript
        | Target::React
        | Target::Vue
        | Target::OmniView => {
            format!("new Promise((_resolve) => setTimeout(_resolve, {ms}))")
        }
        Target::Python => format!("asyncio.sleep({ms} / 1000)"),
        Target::Rust | Target::Dioxus => {
            format!("tokio::time::sleep(std::time::Duration::from_millis({ms}))")
        }
        Target::Dart | Target::Flutter => {
            format!("Future.delayed(Duration(milliseconds: {ms}))")
        }
    }
}

pub(super) fn codegen_async_spawn(args: &[String], target: Target) -> String {
    let task = &args[0];
    match target {
        Target::JavaScript
        | Target::TypeScript
        | Target::React
        | Target::Vue
        | Target::OmniView => {
            format!("Promise.resolve({task})")
        }
        Target::Python => format!("asyncio.create_task({task})"),
        Target::Rust | Target::Dioxus => format!("tokio::spawn({task})"),
        Target::Dart | Target::Flutter => task.clone(),
    }
}

pub(super) fn codegen_async_all(args: &[String], target: Target) -> String {
    let tasks = &args[0];
    match target {
        Target::JavaScript
        | Target::TypeScript
        | Target::React
        | Target::Vue
        | Target::OmniView => {
            format!("Promise.all({tasks})")
        }
        Target::Python => format!("asyncio.gather(*{tasks})"),
        Target::Rust | Target::Dioxus => format!("futures::future::join_all({tasks})"),
        Target::Dart | Target::Flutter => format!("Future.wait({tasks})"),
    }
}

pub(super) fn codegen_async_race(args: &[String], target: Target) -> String {
    let tasks = &args[0];
    match target {
        Target::JavaScript
        | Target::TypeScript
        | Target::React
        | Target::Vue
        | Target::OmniView => {
            format!("Promise.race({tasks})")
        }
        Target::Python => format!(
            "(await asyncio.wait({tasks}, return_when=asyncio.FIRST_COMPLETED))[0].pop().result()"
        ),
        Target::Rust | Target::Dioxus => format!("futures::future::select_all({tasks}).await.0"),
        Target::Dart | Target::Flutter => format!("Future.any({tasks})"),
    }
}

pub(super) fn codegen_async_timeout(args: &[String], target: Target) -> String {
    let task = &args[0];
    let ms = &args[1];
    match target {
        Target::JavaScript
        | Target::TypeScript
        | Target::React
        | Target::Vue
        | Target::OmniView => {
            format!(
                "Promise.race([{task}, new Promise((_, _reject) => setTimeout(() => _reject(new Error('timeout')), {ms}))])"
            )
        }
        Target::Python => format!("asyncio.wait_for({task}, {ms} / 1000)"),
        Target::Rust | Target::Dioxus => {
            format!("tokio::time::timeout(std::time::Duration::from_millis({ms}), {task})")
        }
        Target::Dart | Target::Flutter => {
            format!("{task}.timeout(Duration(milliseconds: {ms}))")
        }
    }
}
