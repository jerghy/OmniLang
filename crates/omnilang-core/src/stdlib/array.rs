use super::{StdlibFn, ALL_TARGETS};
use crate::codegen::Target;

pub(super) fn register() -> Vec<StdlibFn> {
    vec![
        StdlibFn {
            module: "array",
            name: "array_length",
            min_args: 1,
            max_args: Some(1),
            available_targets: &ALL_TARGETS,
            codegen: codegen_array_length,
            is_async: false,
        },
        StdlibFn {
            module: "array",
            name: "array_get",
            min_args: 2,
            max_args: Some(2),
            available_targets: &ALL_TARGETS,
            codegen: codegen_array_get,
            is_async: false,
        },
        StdlibFn {
            module: "array",
            name: "array_push",
            min_args: 2,
            max_args: Some(2),
            available_targets: &ALL_TARGETS,
            codegen: codegen_array_push,
            is_async: false,
        },
        StdlibFn {
            module: "array",
            name: "array_pop",
            min_args: 1,
            max_args: Some(1),
            available_targets: &ALL_TARGETS,
            codegen: codegen_array_pop,
            is_async: false,
        },
        StdlibFn {
            module: "array",
            name: "array_remove",
            min_args: 2,
            max_args: Some(2),
            available_targets: &ALL_TARGETS,
            codegen: codegen_array_remove,
            is_async: false,
        },
        StdlibFn {
            module: "array",
            name: "array_map",
            min_args: 2,
            max_args: Some(2),
            available_targets: &ALL_TARGETS,
            codegen: codegen_array_map,
            is_async: false,
        },
        StdlibFn {
            module: "array",
            name: "array_filter",
            min_args: 2,
            max_args: Some(2),
            available_targets: &ALL_TARGETS,
            codegen: codegen_array_filter,
            is_async: false,
        },
        StdlibFn {
            module: "array",
            name: "array_find",
            min_args: 2,
            max_args: Some(2),
            available_targets: &ALL_TARGETS,
            codegen: codegen_array_find,
            is_async: false,
        },
        StdlibFn {
            module: "array",
            name: "array_sort",
            min_args: 1,
            max_args: Some(2),
            available_targets: &ALL_TARGETS,
            codegen: codegen_array_sort,
            is_async: false,
        },
        StdlibFn {
            module: "array",
            name: "array_contains",
            min_args: 2,
            max_args: Some(2),
            available_targets: &ALL_TARGETS,
            codegen: codegen_array_contains,
            is_async: false,
        },
        StdlibFn {
            module: "array",
            name: "array_is_empty",
            min_args: 1,
            max_args: Some(1),
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
            // Direct index (matches the JS family). The old bounds-guarded
            // form returned null on out-of-bounds — a nullable element type
            // that broke arithmetic downstream (double? in
            // `totalPrice - lastPrice`). Dart's typed List index is
            // non-nullable; out-of-bounds throws at runtime like JS's
            // undefined access. Task 5.8a: `idx as int` flagged
            // unnecessary_cast when the index is already statically int
            // (the language's i32 index contract) — `.toInt()` is
            // warning-free on int AND double/dynamic indexes.
            format!("({arr})[({idx}).toInt()]")
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
        // Task 5.8a: `idx as int` flagged unnecessary_cast for statically
        // int indexes (array_remove's i32 contract) — `.toInt()` is
        // warning-free on int AND double/dynamic indexes.
        Target::Dart | Target::Flutter => format!("({arr})..removeAt(({idx}).toInt())"),
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
            // firstWhere's orElse must return the ELEMENT type (non-nullable
            // lists reject a bare null — the analyzer reports Null not
            // returnable from String) — the dynamic cast keeps the missing-
            // element fallback compiling (Task 5.8).
            format!("({arr}).firstWhere({fn_name}, orElse: () => null as dynamic)")
        }
    }
}

/// Count the parameters of a formatted JS-family closure string, if the
/// argument is one. JS-family closures follow the shared emission
/// convention `((a, b) => body)`. Returns `None` for non-closure strings
/// (identifiers, method calls, ...).
fn closure_param_count(s: &str) -> Option<usize> {
    let rest = s.strip_prefix("((")?;
    let close = rest.find(')')?;
    let params = rest[..close].trim();
    if params.is_empty() {
        Some(0)
    } else {
        Some(params.split(',').count())
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
                if closure_param_count(c) == Some(2) {
                    // Two-param closure = REAL comparator (documented
                    // contract `fn(T,T)->i32`). Numeric results are used
                    // directly — the comparator runs once per pair. Boolean
                    // results and key-style selectors (e.g. `|a, b| a.name`)
                    // fall back to the swapped-call comparison so the sort
                    // is never a silent no-op (a bare `(a, b) => a.name`
                    // comparator coerces to NaN and reorders nothing).
                    format!("([...{arr}]).sort(($0,$1) => {{ const $r={c}($0,$1); if (typeof $r === \"number\") {{ return $r; }} const $k={c}($1,$0); return $r < $k ? -1 : $r > $k ? 1 : 0; }})")
                } else {
                    // One-param closure (or a non-closure argument) = key
                    // extractor: compare the extracted keys.
                    format!("([...{arr}]).sort(($0,$1) => {{ const $ka={c}($0,$1); const $kb={c}($1,$0); return $ka < $kb ? -1 : $ka > $kb ? 1 : 0; }})")
                }
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
                if closure_param_count(c) == Some(2) {
                    // Dart's `List.sort` requires an `int Function(T, T)`
                    // comparator — the language's two-param closure may be a
                    // REAL comparator (numeric) OR a key-style selector
                    // (`|a, b| a.name` — sort by name). The JS family wraps
                    // with a swapped-call comparison; Dart needs the same
                    // (a bare `(a, b) => a.name` closure returns a String —
                    // return_of_invalid_type_from_closure, Task 5.8).
                    format!(
                        "List.from({arr})..sort(($0, $1) {{ final $ra = {c}($0, $1); final $rb = {c}($1, $0); if ($ra is num) {{ return $ra.compareTo($rb); }} return $ra.toString().compareTo($rb.toString()); }})"
                    )
                } else {
                    format!("List.from({arr})..sort({c})")
                }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codegen::try_format_stdlib_call;

    /// A two-param closure is a REAL comparator: it must be emitted through
    /// the comparator path (numeric results used directly — single call),
    /// NOT through the `$ka`/`$kb` key-extractor machinery.
    #[test]
    fn array_sort_two_param_closure_emits_comparator() {
        let out = try_format_stdlib_call(
            "array_sort",
            &["arr".into(), "((a, b) => (a.name > b.name))".into()],
            Target::JavaScript,
        )
        .expect("array_sort should format");
        assert!(
            out.contains("const $r=") && !out.contains("const $ka="),
            "two-param closure must emit the comparator path, got: {out}"
        );
        assert!(
            out.contains("typeof $r === \"number\""),
            "comparator path must fast-path numeric results (single evaluation), got: {out}"
        );
    }

    /// A one-param closure is a key extractor: the `$ka`/`$kb` machinery
    /// (compare the extracted keys) must be preserved.
    #[test]
    fn array_sort_one_param_closure_keeps_key_extractor() {
        let out = try_format_stdlib_call(
            "array_sort",
            &["arr".into(), "((x) => x.name)".into()],
            Target::JavaScript,
        )
        .expect("array_sort should format");
        assert!(
            out.contains("const $ka="),
            "one-param key extractor must keep the $ka/$kb machinery, got: {out}"
        );
    }

    /// Non-closure arguments (named functions, etc.) keep the machinery.
    #[test]
    fn array_sort_non_closure_arg_keeps_machinery() {
        let out = try_format_stdlib_call(
            "array_sort",
            &["arr".into(), "myCompare".into()],
            Target::JavaScript,
        )
        .expect("array_sort should format");
        assert!(
            out.contains("const $ka="),
            "non-closure args must keep the $ka/$kb machinery, got: {out}"
        );
    }

    /// No closure: default sort.
    #[test]
    fn array_sort_without_closure_emits_default_sort() {
        let out = try_format_stdlib_call("array_sort", &["arr".into()], Target::JavaScript)
            .expect("array_sort should format");
        assert_eq!(out, "([...arr]).sort()");
    }

    /// Task 5.8a: the Dart indexer cast `idx as int` flagged
    /// unnecessary_cast when the index is already statically int (the
    /// language's array_get/array_remove contract is i32) — `.toInt()` is
    /// warning-free for int AND double/dynamic indexes.
    #[test]
    fn array_get_dart_uses_to_int_coercion() {
        let out = try_format_stdlib_call(
            "array_get",
            &["prices".into(), "items.length - 1".into()],
            Target::Dart,
        )
        .expect("array_get should format");
        assert_eq!(out, "(prices)[(items.length - 1).toInt()]");
    }

    #[test]
    fn array_remove_dart_uses_to_int_coercion() {
        let out =
            try_format_stdlib_call("array_remove", &["arr".into(), "idx".into()], Target::Dart)
                .expect("array_remove should format");
        assert_eq!(out, "(arr)..removeAt((idx).toInt())");
    }

    #[test]
    fn array_get_js_keeps_plain_index() {
        let out =
            try_format_stdlib_call("array_get", &["arr".into(), "i".into()], Target::JavaScript)
                .expect("array_get should format");
        assert_eq!(out, "(arr)[i]");
    }
}
