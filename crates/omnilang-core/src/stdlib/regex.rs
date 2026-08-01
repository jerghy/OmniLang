use super::{StdlibFn, ALL_TARGETS};
use crate::codegen::Target;

pub(super) fn register() -> Vec<StdlibFn> {
    vec![
        StdlibFn {
            module: "regex",
            name: "regex_match",
            available_targets: &ALL_TARGETS,
            codegen: codegen_regex_match,
            is_async: false,
        },
        StdlibFn {
            module: "regex",
            name: "regex_search",
            available_targets: &ALL_TARGETS,
            codegen: codegen_regex_search,
            is_async: false,
        },
        StdlibFn {
            module: "regex",
            name: "regex_replace",
            available_targets: &ALL_TARGETS,
            codegen: codegen_regex_replace,
            is_async: false,
        },
    ]
}

pub(super) fn codegen_regex_match(args: &[String], target: Target) -> String {
    let text = &args[0];
    let pattern = &args[1];
    match target {
        Target::JavaScript
        | Target::TypeScript
        | Target::React
        | Target::Vue
        | Target::OmniView => {
            format!("new RegExp({pattern}).test({text})")
        }
        Target::Python => format!("re.fullmatch({pattern}, {text}) is not None"),
        Target::Rust | Target::Dioxus => {
            format!("regex::Regex::new({pattern}).map(|re| re.is_match({text})).unwrap_or(false)")
        }
        Target::Dart | Target::Flutter => format!("RegExp({pattern}).hasMatch({text})"),
    }
}

pub(super) fn codegen_regex_search(args: &[String], target: Target) -> String {
    let text = &args[0];
    let pattern = &args[1];
    match target {
        Target::JavaScript
        | Target::TypeScript
        | Target::React
        | Target::Vue
        | Target::OmniView => {
            format!(
                "Array.from(({text}).matchAll(new RegExp({pattern}, 'g')), (m) => m[0])"
            )
        }
        Target::Python => format!("re.findall({pattern}, {text})"),
        Target::Rust | Target::Dioxus => format!(
            "regex::Regex::new({pattern}).map(|re| re.find_iter({text}).map(|m| m.as_str().to_string()).collect::<Vec<String>>()).unwrap_or_default()"
        ),
        Target::Dart | Target::Flutter => format!(
            "RegExp({pattern}).allMatches({text}).map((m) => m.group(0) ?? '').toList()"
        ),
    }
}

pub(super) fn codegen_regex_replace(args: &[String], target: Target) -> String {
    let text = &args[0];
    let pattern = &args[1];
    let replacement = &args[2];
    match target {
        Target::JavaScript
        | Target::TypeScript
        | Target::React
        | Target::Vue
        | Target::OmniView => {
            format!("({text}).replace(new RegExp({pattern}, 'g'), {replacement})")
        }
        Target::Python => format!("re.sub({pattern}, {replacement}, {text})"),
        Target::Rust | Target::Dioxus => format!(
            "regex::Regex::new({pattern}).map(|re| re.replace_all({text}, {replacement}).to_string()).unwrap_or_else(|_| {text}.to_string())"
        ),
        Target::Dart | Target::Flutter => {
            format!("({text}).replaceAll(RegExp({pattern}), {replacement})")
        }
    }
}
