use super::{StdlibFn, ALL_TARGETS};
use crate::codegen::Target;

pub(super) fn register() -> Vec<StdlibFn> {
    vec![
        StdlibFn {
            module: "string",
            name: "string_length",
            min_args: 1,
            max_args: Some(1),
            available_targets: &ALL_TARGETS,
            codegen: codegen_string_length,
            is_async: false,
        },
        StdlibFn {
            module: "string",
            name: "string_concat",
            min_args: 2,
            max_args: Some(2),
            available_targets: &ALL_TARGETS,
            codegen: codegen_string_concat,
            is_async: false,
        },
        StdlibFn {
            module: "string",
            name: "string_split",
            min_args: 2,
            max_args: Some(2),
            available_targets: &ALL_TARGETS,
            codegen: codegen_string_split,
            is_async: false,
        },
        StdlibFn {
            module: "string",
            name: "string_join",
            min_args: 2,
            max_args: Some(2),
            available_targets: &ALL_TARGETS,
            codegen: codegen_string_join,
            is_async: false,
        },
        StdlibFn {
            module: "string",
            name: "string_replace",
            min_args: 3,
            max_args: Some(3),
            available_targets: &ALL_TARGETS,
            codegen: codegen_string_replace,
            is_async: false,
        },
        StdlibFn {
            module: "string",
            name: "string_substring",
            min_args: 2,
            max_args: Some(3),
            available_targets: &ALL_TARGETS,
            codegen: codegen_string_substring,
            is_async: false,
        },
        StdlibFn {
            module: "string",
            name: "string_contains",
            min_args: 2,
            max_args: Some(2),
            available_targets: &ALL_TARGETS,
            codegen: codegen_string_contains,
            is_async: false,
        },
        StdlibFn {
            module: "string",
            name: "string_to_upper",
            min_args: 1,
            max_args: Some(1),
            available_targets: &ALL_TARGETS,
            codegen: codegen_string_to_upper,
            is_async: false,
        },
        StdlibFn {
            module: "string",
            name: "string_to_lower",
            min_args: 1,
            max_args: Some(1),
            available_targets: &ALL_TARGETS,
            codegen: codegen_string_to_lower,
            is_async: false,
        },
        StdlibFn {
            module: "string",
            name: "string_trim",
            min_args: 1,
            max_args: Some(1),
            available_targets: &ALL_TARGETS,
            codegen: codegen_string_trim,
            is_async: false,
        },
        StdlibFn {
            module: "string",
            name: "string_starts_with",
            min_args: 2,
            max_args: Some(2),
            available_targets: &ALL_TARGETS,
            codegen: codegen_string_starts_with,
            is_async: false,
        },
        StdlibFn {
            module: "string",
            name: "string_ends_with",
            min_args: 2,
            max_args: Some(2),
            available_targets: &ALL_TARGETS,
            codegen: codegen_string_ends_with,
            is_async: false,
        },
    ]
}

pub(super) fn codegen_string_length(args: &[String], target: Target) -> String {
    let s = &args[0];
    match target {
        Target::JavaScript
        | Target::TypeScript
        | Target::React
        | Target::Vue
        | Target::OmniView => {
            format!("({s}).length")
        }
        Target::Python => format!("len({s})"),
        Target::Rust | Target::Dioxus => format!("({s}).len()"),
        Target::Dart | Target::Flutter => format!("({s}).length"),
    }
}

pub(super) fn codegen_string_concat(args: &[String], target: Target) -> String {
    let a = &args[0];
    let b = &args[1];
    match target {
        Target::JavaScript
        | Target::TypeScript
        | Target::React
        | Target::Vue
        | Target::OmniView => {
            format!("({a}) + ({b})")
        }
        Target::Python => format!("({a}) + ({b})"),
        Target::Rust | Target::Dioxus => format!("({a}).clone() + ({b})"),
        Target::Dart | Target::Flutter => format!("({a}) + ({b})"),
    }
}

pub(super) fn codegen_string_split(args: &[String], target: Target) -> String {
    let s = &args[0];
    let sep = &args[1];
    match target {
        Target::JavaScript
        | Target::TypeScript
        | Target::React
        | Target::Vue
        | Target::OmniView => {
            format!("({s}).split({sep})")
        }
        Target::Python => format!("({s}).split({sep})"),
        Target::Rust | Target::Dioxus => format!("({s}).split({sep}).collect::<Vec<_>>()"),
        Target::Dart | Target::Flutter => format!("({s}).split({sep}).toList()"),
    }
}

pub(super) fn codegen_string_join(args: &[String], target: Target) -> String {
    let arr = &args[0];
    let sep = &args[1];
    match target {
        Target::JavaScript
        | Target::TypeScript
        | Target::React
        | Target::Vue
        | Target::OmniView => {
            format!("({arr}).join({sep})")
        }
        Target::Python => format!("({sep}).join({arr})"),
        Target::Rust | Target::Dioxus => format!("({arr}).join({sep})"),
        Target::Dart | Target::Flutter => format!("({arr}).join({sep})"),
    }
}

pub(super) fn codegen_string_replace(args: &[String], target: Target) -> String {
    let s = &args[0];
    let from = &args[1];
    let to = &args[2];
    match target {
        Target::JavaScript
        | Target::TypeScript
        | Target::React
        | Target::Vue
        | Target::OmniView => {
            format!("({s}).replaceAll({from}, {to})")
        }
        Target::Python => format!("({s}).replace({from}, {to})"),
        Target::Rust | Target::Dioxus => format!("({s}).replace({from}, {to})"),
        Target::Dart | Target::Flutter => format!("({s}).replaceAll({from}, {to})"),
    }
}

pub(super) fn codegen_string_substring(args: &[String], target: Target) -> String {
    let s = &args[0];
    let start = &args[1];
    let end = args.get(2).map(|e| e.as_str()).unwrap_or("undefined");
    match target {
        Target::JavaScript
        | Target::TypeScript
        | Target::React
        | Target::Vue
        | Target::OmniView => {
            if args.len() >= 3 {
                format!("({s}).substring({start}, {end})")
            } else {
                format!("({s}).substring({start})")
            }
        }
        Target::Python => {
            if args.len() >= 3 {
                format!("({s})[{start}:{end}]")
            } else {
                format!("({s})[{start}:]")
            }
        }
        Target::Rust | Target::Dioxus => {
            if args.len() >= 3 {
                format!("({s})[{start}..{end}].to_string()")
            } else {
                format!("({s})[{start}..].to_string()")
            }
        }
        Target::Dart | Target::Flutter => {
            if args.len() >= 3 {
                format!("({s}).substring({start}, {end})")
            } else {
                format!("({s}).substring({start})")
            }
        }
    }
}

pub(super) fn codegen_string_contains(args: &[String], target: Target) -> String {
    let s = &args[0];
    let sub = &args[1];
    match target {
        Target::JavaScript
        | Target::TypeScript
        | Target::React
        | Target::Vue
        | Target::OmniView => {
            format!("({s}).includes({sub})")
        }
        Target::Python => format!("({sub}) in ({s})"),
        Target::Rust | Target::Dioxus => format!("({s}).contains({sub})"),
        Target::Dart | Target::Flutter => format!("({s}).contains({sub})"),
    }
}

pub(super) fn codegen_string_to_upper(args: &[String], target: Target) -> String {
    let s = &args[0];
    match target {
        Target::JavaScript
        | Target::TypeScript
        | Target::React
        | Target::Vue
        | Target::OmniView => {
            format!("({s}).toUpperCase()")
        }
        Target::Python => format!("({s}).upper()"),
        Target::Rust | Target::Dioxus => format!("({s}).to_uppercase()"),
        Target::Dart | Target::Flutter => format!("({s}).toUpperCase()"),
    }
}

pub(super) fn codegen_string_to_lower(args: &[String], target: Target) -> String {
    let s = &args[0];
    match target {
        Target::JavaScript
        | Target::TypeScript
        | Target::React
        | Target::Vue
        | Target::OmniView => {
            format!("({s}).toLowerCase()")
        }
        Target::Python => format!("({s}).lower()"),
        Target::Rust | Target::Dioxus => format!("({s}).to_lowercase()"),
        Target::Dart | Target::Flutter => format!("({s}).toLowerCase()"),
    }
}

pub(super) fn codegen_string_trim(args: &[String], target: Target) -> String {
    let s = &args[0];
    match target {
        Target::JavaScript
        | Target::TypeScript
        | Target::React
        | Target::Vue
        | Target::OmniView => {
            format!("({s}).trim()")
        }
        Target::Python => format!("({s}).strip()"),
        Target::Rust | Target::Dioxus => format!("({s}).trim()"),
        Target::Dart | Target::Flutter => format!("({s}).trim()"),
    }
}

pub(super) fn codegen_string_starts_with(args: &[String], target: Target) -> String {
    let s = &args[0];
    let prefix = &args[1];
    match target {
        Target::JavaScript
        | Target::TypeScript
        | Target::React
        | Target::Vue
        | Target::OmniView => {
            format!("({s}).startsWith({prefix})")
        }
        Target::Python => format!("({s}).startswith({prefix})"),
        Target::Rust | Target::Dioxus => format!("({s}).starts_with({prefix})"),
        Target::Dart | Target::Flutter => format!("({s}).startsWith({prefix})"),
    }
}

pub(super) fn codegen_string_ends_with(args: &[String], target: Target) -> String {
    let s = &args[0];
    let suffix = &args[1];
    match target {
        Target::JavaScript
        | Target::TypeScript
        | Target::React
        | Target::Vue
        | Target::OmniView => {
            format!("({s}).endsWith({suffix})")
        }
        Target::Python => format!("({s}).endswith({suffix})"),
        Target::Rust | Target::Dioxus => format!("({s}).ends_with({suffix})"),
        Target::Dart | Target::Flutter => format!("({s}).endsWith({suffix})"),
    }
}
