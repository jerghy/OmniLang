use super::{StdlibFn, STORAGE_TARGETS};
use crate::codegen::Target;

pub(super) fn register() -> Vec<StdlibFn> {
    vec![
        StdlibFn {
            module: "storage",
            name: "storage_get",
            available_targets: &STORAGE_TARGETS,
            codegen: codegen_storage_get,
            is_async: false,
        },
        StdlibFn {
            module: "storage",
            name: "storage_set",
            available_targets: &STORAGE_TARGETS,
            codegen: codegen_storage_set,
            is_async: false,
        },
    ]
}

pub(super) fn codegen_storage_get(args: &[String], target: Target) -> String {
    let key = &args[0];
    match target {
        Target::JavaScript
        | Target::TypeScript
        | Target::React
        | Target::Vue
        | Target::OmniView => {
            format!("localStorage.getItem({key})", key = key)
        }
        Target::Flutter => {
            format!(
                "(await SharedPreferences.getInstance()).getString({key})",
                key = key
            )
        }
        _ => unreachable!("storage_get not available on {:?}", target),
    }
}

pub(super) fn codegen_storage_set(args: &[String], target: Target) -> String {
    let key = &args[0];
    let val = args.get(1).map(|s| s.as_str()).unwrap_or("\"\"");
    match target {
        Target::JavaScript
        | Target::TypeScript
        | Target::React
        | Target::Vue
        | Target::OmniView => {
            format!("localStorage.setItem({key}, {val})", key = key, val = val)
        }
        Target::Flutter => {
            format!(
                "(await SharedPreferences.getInstance()).setString({key}, {val})",
                key = key,
                val = val
            )
        }
        _ => unreachable!("storage_set not available on {:?}", target),
    }
}
