use super::{StdlibFn, ALL_TARGETS};
use crate::codegen::Target;

pub(super) fn register() -> Vec<StdlibFn> {
    vec![
        StdlibFn {
            module: "console",
            name: "console_log",
            available_targets: &ALL_TARGETS,
            codegen: codegen_console_log,
            is_async: false,
        },
        StdlibFn {
            module: "console",
            name: "console_warn",
            available_targets: &ALL_TARGETS,
            codegen: codegen_console_warn,
            is_async: false,
        },
        StdlibFn {
            module: "console",
            name: "console_error",
            available_targets: &ALL_TARGETS,
            codegen: codegen_console_error,
            is_async: false,
        },
    ]
}

pub(super) fn codegen_console_log(args: &[String], target: Target) -> String {
    let msg = &args[0];
    match target {
        Target::JavaScript
        | Target::TypeScript
        | Target::React
        | Target::Vue
        | Target::OmniView => {
            format!("console.log({msg})")
        }
        Target::Python => format!("print({msg})"),
        Target::Rust | Target::Dioxus => format!("println!(\"{{}}\", {msg})"),
        Target::Dart | Target::Flutter => format!("print({msg})"),
    }
}

pub(super) fn codegen_console_warn(args: &[String], target: Target) -> String {
    let msg = &args[0];
    match target {
        Target::JavaScript
        | Target::TypeScript
        | Target::React
        | Target::Vue
        | Target::OmniView => {
            format!("console.warn({msg})")
        }
        Target::Python => format!("import warnings; warnings.warn({msg})"),
        Target::Rust | Target::Dioxus => format!("eprintln!(\"WARN: {{}}\", {msg})"),
        Target::Dart | Target::Flutter => format!("print('WARN: ${{{msg}}}')"),
    }
}

pub(super) fn codegen_console_error(args: &[String], target: Target) -> String {
    let msg = &args[0];
    match target {
        Target::JavaScript
        | Target::TypeScript
        | Target::React
        | Target::Vue
        | Target::OmniView => {
            format!("console.error({msg})")
        }
        Target::Python => format!("import sys; print({msg}, file=sys.stderr)"),
        Target::Rust | Target::Dioxus => format!("eprintln!(\"ERROR: {{}}\", {msg})"),
        Target::Dart | Target::Flutter => format!("print('ERROR: ${{{msg}}}')"),
    }
}
