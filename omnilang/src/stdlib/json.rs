use super::{StdlibFn, ALL_TARGETS};
use crate::codegen::Target;

pub(super) fn register() -> Vec<StdlibFn> {
    vec![
        StdlibFn {
            module: "json",
            name: "json_parse",
            available_targets: &ALL_TARGETS,
            codegen: codegen_json_parse,
            is_async: false,
        },
        StdlibFn {
            module: "json",
            name: "json_stringify",
            available_targets: &ALL_TARGETS,
            codegen: codegen_json_stringify,
            is_async: false,
        },
    ]
}

pub(super) fn codegen_json_parse(args: &[String], target: Target) -> String {
    let text = &args[0];
    match target {
        Target::JavaScript
        | Target::TypeScript
        | Target::React
        | Target::Vue
        | Target::OmniView => {
            format!("JSON.parse({text})", text = text)
        }
        Target::Python => format!("json.loads({text})", text = text),
        Target::Rust | Target::Dioxus => {
            format!(
                "serde_json::from_str(&{text}).expect(\"JSON parse error\")",
                text = text
            )
        }
        Target::Dart | Target::Flutter => {
            format!("json.decode({text})", text = text)
        }
    }
}

pub(super) fn codegen_json_stringify(args: &[String], target: Target) -> String {
    let val = &args[0];
    match target {
        Target::JavaScript
        | Target::TypeScript
        | Target::React
        | Target::Vue
        | Target::OmniView => {
            format!("JSON.stringify({val})", val = val)
        }
        Target::Python => format!("json.dumps({val})", val = val),
        Target::Rust | Target::Dioxus => {
            format!(
                "serde_json::to_string(&{val}).expect(\"JSON serialize error\")",
                val = val
            )
        }
        Target::Dart | Target::Flutter => {
            format!("json.encode({val})", val = val)
        }
    }
}
