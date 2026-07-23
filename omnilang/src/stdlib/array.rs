use super::{StdlibFn, ALL_TARGETS};
use crate::codegen::Target;

pub(super) fn register() -> Vec<StdlibFn> {
    vec![
        StdlibFn {
            module: "array",
            name: "array_length",
            available_targets: &ALL_TARGETS,
            codegen: codegen_array_length,
            is_async: false,
        },
        StdlibFn {
            module: "array",
            name: "array_get",
            available_targets: &ALL_TARGETS,
            codegen: codegen_array_get,
            is_async: false,
        },
        StdlibFn {
            module: "array",
            name: "array_push",
            available_targets: &ALL_TARGETS,
            codegen: codegen_array_push,
            is_async: false,
        },
        StdlibFn {
            module: "array",
            name: "array_pop",
            available_targets: &ALL_TARGETS,
            codegen: codegen_array_pop,
            is_async: false,
        },
        StdlibFn {
            module: "array",
            name: "array_remove",
            available_targets: &ALL_TARGETS,
            codegen: codegen_array_remove,
            is_async: false,
        },
        StdlibFn {
            module: "array",
            name: "array_map",
            available_targets: &ALL_TARGETS,
            codegen: codegen_array_map,
            is_async: false,
        },
        StdlibFn {
            module: "array",
            name: "array_filter",
            available_targets: &ALL_TARGETS,
            codegen: codegen_array_filter,
            is_async: false,
        },
        StdlibFn {
            module: "array",
            name: "array_find",
            available_targets: &ALL_TARGETS,
            codegen: codegen_array_find,
            is_async: false,
        },
        StdlibFn {
            module: "array",
            name: "array_sort",
            available_targets: &ALL_TARGETS,
            codegen: codegen_array_sort,
            is_async: false,
        },
        StdlibFn {
            module: "array",
            name: "array_contains",
            available_targets: &ALL_TARGETS,
            codegen: codegen_array_contains,
            is_async: false,
        },
        StdlibFn {
            module: "array",
            name: "array_is_empty",
            available_targets: &ALL_TARGETS,
            codegen: codegen_array_is_empty,
            is_async: false,
        },
    ]
}

pub(super) fn codegen_array_length(args: &[String], target: Target) -> String {
    let arr = &args[0];
    match target {
        Target::JavaScript
        | Target::TypeScript
        | Target::React
        | Target::Vue
        | Target::OmniView => {
            format!("({arr}).length")
        }
        Target::Python => format!("len({arr})"),
        Target::Rust | Target::Dioxus => format!("({arr}).len()"),
        Target::Dart | Target::Flutter => format!("({arr}).length"),
    }
}

pub(super) fn codegen_array_get(args: &[String], target: Target) -> String {
    let arr = &args[0];
    let idx = &args[1];
    match target {
        Target::JavaScript
        | Target::TypeScript
        | Target::React
        | Target::Vue
        | Target::OmniView => {
            format!("({arr})[{idx}]")
        }
        Target::Python => format!("({arr})[{idx}] if 0 <= ({idx}) < len({arr}) else None"),
        Target::Rust | Target::Dioxus => format!("({arr}).get({idx} as usize).cloned()"),
        Target::Dart | Target::Flutter => {
            format!("({idx}) >= 0 && ({idx}) < ({arr}).length ? ({arr})[{idx} as int] : null")
        }
    }
}

pub(super) fn codegen_array_push(args: &[String], target: Target) -> String {
    let arr = &args[0];
    let item = &args[1];
    match target {
        Target::JavaScript
        | Target::TypeScript
        | Target::React
        | Target::Vue
        | Target::OmniView => {
            format!("[...({arr}), {item}]")
        }
        Target::Python => format!("({arr}).append({item}) or {arr}"),
        Target::Rust | Target::Dioxus => format!("{{ ({arr}).push({item}); {arr} }}"),
        Target::Dart | Target::Flutter => format!("({arr})..add({item})"),
    }
}

pub(super) fn codegen_array_pop(args: &[String], target: Target) -> String {
    let arr = &args[0];
    match target {
        Target::JavaScript
        | Target::TypeScript
        | Target::React
        | Target::Vue
        | Target::OmniView => {
            // Return new array without last element (immutable)
            format!("({arr}).slice(0, -1)")
        }
        Target::Python => format!("({arr}).pop() or {arr} if len({arr}) > 0 else []"),
        Target::Rust | Target::Dioxus => format!("{{ let mut _v = {arr}.clone(); _v.pop(); _v }}"),
        Target::Dart | Target::Flutter => {
            format!("({arr}).toList()..removeLast()")
        }
    }
}

pub(super) fn codegen_array_remove(args: &[String], target: Target) -> String {
    let arr = &args[0];
    let idx = &args[1];
    match target {
        Target::JavaScript
        | Target::TypeScript
        | Target::React
        | Target::Vue
        | Target::OmniView => {
            format!("({arr}).filter((_, _i) => _i !== {idx})")
        }
        Target::Python => format!("({arr}).pop({idx}) or {arr}"),
        Target::Rust | Target::Dioxus => format!("{{ ({arr}).remove({idx} as usize); {arr} }}"),
        Target::Dart | Target::Flutter => format!("({arr})..removeAt({idx} as int)"),
    }
}

pub(super) fn codegen_array_map(args: &[String], target: Target) -> String {
    let arr = &args[0];
    let fn_name = &args[1];
    match target {
        Target::JavaScript
        | Target::TypeScript
        | Target::React
        | Target::Vue
        | Target::OmniView => {
            format!("({arr}).map({fn_name})")
        }
        Target::Python => format!("[{fn_name}(x) for x in {arr}]"),
        Target::Rust | Target::Dioxus => {
            format!("({arr}).iter().map({fn_name}).collect::<Vec<_>>()")
        }
        Target::Dart | Target::Flutter => format!("({arr}).map({fn_name}).toList()"),
    }
}

pub(super) fn codegen_array_filter(args: &[String], target: Target) -> String {
    let arr = &args[0];
    let fn_name = &args[1];
    match target {
        Target::JavaScript
        | Target::TypeScript
        | Target::React
        | Target::Vue
        | Target::OmniView => {
            format!("({arr}).filter({fn_name})")
        }
        Target::Python => format!("[x for x in {arr} if {fn_name}(x)]"),
        Target::Rust | Target::Dioxus => {
            format!("({arr}).iter().filter(|x| {fn_name}(x)).cloned().collect::<Vec<_>>()")
        }
        Target::Dart | Target::Flutter => format!("({arr}).where({fn_name}).toList()"),
    }
}

pub(super) fn codegen_array_find(args: &[String], target: Target) -> String {
    let arr = &args[0];
    let fn_name = &args[1];
    match target {
        Target::JavaScript
        | Target::TypeScript
        | Target::React
        | Target::Vue
        | Target::OmniView => {
            format!("({arr}).find({fn_name})")
        }
        Target::Python => format!("next((x for x in {arr} if {fn_name}(x)), None)"),
        Target::Rust | Target::Dioxus => format!("({arr}).iter().find(|x| {fn_name}(x)).cloned()"),
        Target::Dart | Target::Flutter => {
            format!("({arr}).firstWhere({fn_name}, orElse: () => null)")
        }
    }
}

pub(super) fn codegen_array_sort(args: &[String], target: Target) -> String {
    let arr = &args[0];
    let closure = args.get(1);
    match target {
        Target::JavaScript
        | Target::TypeScript
        | Target::React
        | Target::Vue
        | Target::OmniView => {
            if let Some(c) = closure {
                format!("([...{arr}]).sort(($0,$1) => {{ let $ka={c}($0,$1); let $kb={c}($1,$0); return $ka < $kb ? -1 : $ka > $kb ? 1 : 0; }})")
            } else {
                format!("([...{arr}]).sort()")
            }
        }
        Target::Python => {
            if let Some(c) = closure {
                format!("sorted({arr}, key={c})")
            } else {
                format!("sorted({arr})")
            }
        }
        Target::Rust | Target::Dioxus => {
            if let Some(c) = closure {
                format!("{{ let mut v = ({arr}).clone(); v.sort_by(|a, b| {c}); v }}")
            } else {
                format!("{{ let mut v = ({arr}).clone(); v.sort(); v }}")
            }
        }
        Target::Dart | Target::Flutter => {
            if let Some(c) = closure {
                format!("List.from({arr})..sort({c})")
            } else {
                format!("List.from({arr})..sort()")
            }
        }
    }
}

pub(super) fn codegen_array_contains(args: &[String], target: Target) -> String {
    let arr = &args[0];
    let item = &args[1];
    match target {
        Target::JavaScript
        | Target::TypeScript
        | Target::React
        | Target::Vue
        | Target::OmniView => {
            format!("({arr}).includes({item})")
        }
        Target::Python => format!("({item}) in ({arr})"),
        Target::Rust | Target::Dioxus => format!("({arr}).contains(&{item})"),
        Target::Dart | Target::Flutter => format!("({arr}).contains({item})"),
    }
}

pub(super) fn codegen_array_is_empty(args: &[String], target: Target) -> String {
    let arr = &args[0];
    match target {
        Target::JavaScript
        | Target::TypeScript
        | Target::React
        | Target::Vue
        | Target::OmniView => {
            format!("({arr}).length === 0")
        }
        Target::Python => format!("len({arr}) == 0"),
        Target::Rust | Target::Dioxus => format!("({arr}).is_empty()"),
        Target::Dart | Target::Flutter => format!("({arr}).isEmpty"),
    }
}
