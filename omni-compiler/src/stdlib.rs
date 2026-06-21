//! Standard library intrinsic registry.
//!
//! Maps `import { fn } from "std/<module>"` to per-target native implementations.
//! Functions not available on a target produce compile-time errors.

use crate::codegen::Target;

/// A registered standard library function.
#[derive(Debug, Clone)]
pub struct StdlibFn {
    pub module: &'static str,
    pub name: &'static str,
    pub available_targets: &'static [Target],
    /// Emit target-native code for this function call.
    /// Receives the formatted argument strings and target.
    pub codegen: fn(args: &[String], target: Target) -> String,
    /// Whether the call must be wrapped in `await`.
    pub is_async: bool,
}

/// Look up a stdlib function by module and name.
pub fn lookup(module: &str, name: &str) -> Option<&'static StdlibFn> {
    REGISTRY
        .iter()
        .find(|f| f.module == module && f.name == name)
}

/// Check if a stdlib module is available on a target.
pub fn module_available(module: &str, target: Target) -> bool {
    REGISTRY
        .iter()
        .filter(|f| f.module == module)
        .all(|f| f.available_targets.contains(&target))
}

/// List available targets for a module (for error messages).
pub fn available_targets_for(module: &str) -> Vec<Target> {
    let mut targets: Vec<Target> = REGISTRY
        .iter()
        .filter(|f| f.module == module)
        .flat_map(|f| f.available_targets.iter())
        .copied()
        .collect();
    targets.sort_by_key(|t| format!("{:?}", t));
    targets.dedup();
    targets
}

// ═══════════════════════════════════════════════════════════
// Registry
// ═══════════════════════════════════════════════════════════

static REGISTRY: &[StdlibFn] = &[
    // ── std/http ──────────────────────────────────────
    StdlibFn {
        module: "http",
        name: "http_send",
        available_targets: &ALL_TARGETS,
        codegen: codegen_http_send,
        is_async: true,
    },
    StdlibFn {
        module: "http",
        name: "http_get",
        available_targets: &ALL_TARGETS,
        codegen: codegen_http_get,
        is_async: true,
    },
    StdlibFn {
        module: "http",
        name: "http_post",
        available_targets: &ALL_TARGETS,
        codegen: codegen_http_post,
        is_async: true,
    },
    // ── std/json ──────────────────────────────────────
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
    // ── std/storage ───────────────────────────────────
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
    // ── std/math ──────────────────────────────────────
    StdlibFn {
        module: "math", name: "math_abs",
        available_targets: &ALL_TARGETS,
        codegen: codegen_math_abs, is_async: false,
    },
    StdlibFn {
        module: "math", name: "math_min",
        available_targets: &ALL_TARGETS,
        codegen: codegen_math_min, is_async: false,
    },
    StdlibFn {
        module: "math", name: "math_max",
        available_targets: &ALL_TARGETS,
        codegen: codegen_math_max, is_async: false,
    },
    StdlibFn {
        module: "math", name: "math_ceil",
        available_targets: &ALL_TARGETS,
        codegen: codegen_math_ceil, is_async: false,
    },
    StdlibFn {
        module: "math", name: "math_floor",
        available_targets: &ALL_TARGETS,
        codegen: codegen_math_floor, is_async: false,
    },
    StdlibFn {
        module: "math", name: "math_round",
        available_targets: &ALL_TARGETS,
        codegen: codegen_math_round, is_async: false,
    },
    StdlibFn {
        module: "math", name: "math_random",
        available_targets: &ALL_TARGETS,
        codegen: codegen_math_random, is_async: false,
    },
    StdlibFn {
        module: "math", name: "math_sqrt",
        available_targets: &ALL_TARGETS,
        codegen: codegen_math_sqrt, is_async: false,
    },
    StdlibFn {
        module: "math", name: "math_pow",
        available_targets: &ALL_TARGETS,
        codegen: codegen_math_pow, is_async: false,
    },
    // ── std/string ────────────────────────────────────
    StdlibFn { module: "string", name: "string_length", available_targets: &ALL_TARGETS, codegen: codegen_string_length, is_async: false },
    StdlibFn { module: "string", name: "string_concat", available_targets: &ALL_TARGETS, codegen: codegen_string_concat, is_async: false },
    StdlibFn { module: "string", name: "string_split", available_targets: &ALL_TARGETS, codegen: codegen_string_split, is_async: false },
    StdlibFn { module: "string", name: "string_join", available_targets: &ALL_TARGETS, codegen: codegen_string_join, is_async: false },
    StdlibFn { module: "string", name: "string_replace", available_targets: &ALL_TARGETS, codegen: codegen_string_replace, is_async: false },
    StdlibFn { module: "string", name: "string_substring", available_targets: &ALL_TARGETS, codegen: codegen_string_substring, is_async: false },
    StdlibFn { module: "string", name: "string_contains", available_targets: &ALL_TARGETS, codegen: codegen_string_contains, is_async: false },
    StdlibFn { module: "string", name: "string_to_upper", available_targets: &ALL_TARGETS, codegen: codegen_string_to_upper, is_async: false },
    StdlibFn { module: "string", name: "string_to_lower", available_targets: &ALL_TARGETS, codegen: codegen_string_to_lower, is_async: false },
    StdlibFn { module: "string", name: "string_trim", available_targets: &ALL_TARGETS, codegen: codegen_string_trim, is_async: false },
    StdlibFn { module: "string", name: "string_starts_with", available_targets: &ALL_TARGETS, codegen: codegen_string_starts_with, is_async: false },
    StdlibFn { module: "string", name: "string_ends_with", available_targets: &ALL_TARGETS, codegen: codegen_string_ends_with, is_async: false },
    // ── std/array ─────────────────────────────────────
    StdlibFn { module: "array", name: "array_length", available_targets: &ALL_TARGETS, codegen: codegen_array_length, is_async: false },
    StdlibFn { module: "array", name: "array_get", available_targets: &ALL_TARGETS, codegen: codegen_array_get, is_async: false },
    StdlibFn { module: "array", name: "array_push", available_targets: &ALL_TARGETS, codegen: codegen_array_push, is_async: false },
    StdlibFn { module: "array", name: "array_pop", available_targets: &ALL_TARGETS, codegen: codegen_array_pop, is_async: false },
    StdlibFn { module: "array", name: "array_remove", available_targets: &ALL_TARGETS, codegen: codegen_array_remove, is_async: false },
    StdlibFn { module: "array", name: "array_map", available_targets: &ALL_TARGETS, codegen: codegen_array_map, is_async: false },
    StdlibFn { module: "array", name: "array_filter", available_targets: &ALL_TARGETS, codegen: codegen_array_filter, is_async: false },
    StdlibFn { module: "array", name: "array_find", available_targets: &ALL_TARGETS, codegen: codegen_array_find, is_async: false },
    StdlibFn { module: "array", name: "array_sort", available_targets: &ALL_TARGETS, codegen: codegen_array_sort, is_async: false },
    StdlibFn { module: "array", name: "array_contains", available_targets: &ALL_TARGETS, codegen: codegen_array_contains, is_async: false },
    StdlibFn { module: "array", name: "array_is_empty", available_targets: &ALL_TARGETS, codegen: codegen_array_is_empty, is_async: false },
    // ── std/console ───────────────────────────────────
    StdlibFn { module: "console", name: "console_log", available_targets: &ALL_TARGETS, codegen: codegen_console_log, is_async: false },
    StdlibFn { module: "console", name: "console_warn", available_targets: &ALL_TARGETS, codegen: codegen_console_warn, is_async: false },
    StdlibFn { module: "console", name: "console_error", available_targets: &ALL_TARGETS, codegen: codegen_console_error, is_async: false },
];

const ALL_TARGETS: [Target; 9] = [
    Target::JavaScript,
    Target::React,
    Target::Vue,
    Target::OmniView,
    Target::Python,
    Target::Rust,
    Target::Dart,
    Target::Flutter,
    Target::Dioxus,
];

const STORAGE_TARGETS: [Target; 5] = [
    Target::JavaScript,
    Target::React,
    Target::Vue,
    Target::OmniView,
    Target::Flutter,
];

// ═══════════════════════════════════════════════════════════
// HTTP codegen
// ═══════════════════════════════════════════════════════════

fn codegen_http_send(args: &[String], target: Target) -> String {
    let req = &args[0]; // HttpRequest struct variable
    match target {
        Target::JavaScript | Target::TypeScript | Target::React | Target::Vue | Target::OmniView => {
            // fetch() returns a Response; extract text body so callers get String
            format!(
                "fetch({req}.url, {{ method: {req}.method, headers: {req}.headers, body: {req}.body, signal: AbortSignal.timeout({req}.timeout) }}).then(r => r.text())",
                req = req
            )
        }
        Target::Python => {
            format!(
                "requests.request({req}.get('method'), {req}.get('url'), headers={req}.get('headers'), params={req}.get('query'), data={req}.get('body'), timeout={req}.get('timeout', 0)/1000)",
                req = req
            )
        }
        Target::Rust | Target::Dioxus => {
            format!(
                "reqwest::Client::new().request(reqwest::Method::from_str(&{req}.method).unwrap(), &{req}.url).headers({req}.headers.clone()).query(&{req}.query).body({req}.body.clone()).timeout(std::time::Duration::from_millis({req}.timeout as u64)).send()",
                req = req
            )
        }
        Target::Dart | Target::Flutter => {
            format!(
                "await http.Request({req}.method, Uri.parse({req}.url)).send()",
                req = req
            )
        }
    }
}

fn codegen_http_get(args: &[String], target: Target) -> String {
    let url = &args[0];
    match target {
        Target::JavaScript | Target::TypeScript | Target::React | Target::Vue | Target::OmniView => {
            // fetch() returns a Response; extract text body so callers get String
            format!("fetch({url}).then(r => r.text())", url = url)
        }
        Target::Python => format!("requests.get({url})", url = url),
        Target::Rust | Target::Dioxus => {
            format!("reqwest::blocking::get({url})", url = url)
        }
        Target::Dart | Target::Flutter => {
            format!("await http.get(Uri.parse({url}))", url = url)
        }
    }
}

fn codegen_http_post(args: &[String], target: Target) -> String {
    let url = &args[0];
    let body = args.get(1).map(|s| s.as_str()).unwrap_or("null");
    match target {
        Target::JavaScript | Target::TypeScript | Target::React | Target::Vue | Target::OmniView => {
            // fetch() returns a Response; extract text body so callers get String
            format!(
                "fetch({url}, {{ method: 'POST', body: {body} }}).then(r => r.text())",
                url = url,
                body = body
            )
        }
        Target::Python => {
            format!("requests.post({url}, data={body})", url = url, body = body)
        }
        Target::Rust | Target::Dioxus => {
            format!(
                "reqwest::blocking::Client::new().post({url}).body({body}).send()",
                url = url,
                body = body
            )
        }
        Target::Dart | Target::Flutter => {
            format!(
                "await http.post(Uri.parse({url}), body: {body}).send()",
                url = url,
                body = body
            )
        }
    }
}

// ═══════════════════════════════════════════════════════════
// JSON codegen
// ═══════════════════════════════════════════════════════════

fn codegen_json_parse(args: &[String], target: Target) -> String {
    let text = &args[0];
    match target {
        Target::JavaScript | Target::TypeScript | Target::React | Target::Vue | Target::OmniView => {
            format!("JSON.parse({text})", text = text)
        }
        Target::Python => format!("json.loads({text})", text = text),
        Target::Rust | Target::Dioxus => {
            format!("serde_json::from_str(&{text}).unwrap()", text = text)
        }
        Target::Dart | Target::Flutter => {
            format!("json.decode({text})", text = text)
        }
    }
}

fn codegen_json_stringify(args: &[String], target: Target) -> String {
    let val = &args[0];
    match target {
        Target::JavaScript | Target::TypeScript | Target::React | Target::Vue | Target::OmniView => {
            format!("JSON.stringify({val})", val = val)
        }
        Target::Python => format!("json.dumps({val})", val = val),
        Target::Rust | Target::Dioxus => {
            format!("serde_json::to_string(&{val}).unwrap()", val = val)
        }
        Target::Dart | Target::Flutter => {
            format!("json.encode({val})", val = val)
        }
    }
}

// ═══════════════════════════════════════════════════════════
// Storage codegen
// ═══════════════════════════════════════════════════════════

fn codegen_storage_get(args: &[String], target: Target) -> String {
    let key = &args[0];
    match target {
        Target::JavaScript | Target::TypeScript | Target::React | Target::Vue | Target::OmniView => {
            format!("localStorage.getItem({key})", key = key)
        }
        Target::Flutter => {
            format!("(await SharedPreferences.getInstance()).getString({key})", key = key)
        }
        _ => unreachable!("storage_get not available on {:?}", target),
    }
}

fn codegen_storage_set(args: &[String], target: Target) -> String {
    let key = &args[0];
    let val = args.get(1).map(|s| s.as_str()).unwrap_or("\"\"");
    match target {
        Target::JavaScript | Target::TypeScript | Target::React | Target::Vue | Target::OmniView => {
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

// ═══════════════════════════════════════════════════════════
// Math codegen
// ═══════════════════════════════════════════════════════════

fn codegen_math_abs(args: &[String], target: Target) -> String {
    let n = &args[0];
    match target {
        Target::JavaScript | Target::TypeScript | Target::React | Target::Vue | Target::OmniView => {
            format!("Math.abs({n})")
        }
        Target::Python => format!("abs({n})"),
        Target::Rust | Target::Dioxus => format!("({n}).abs()"),
        Target::Dart | Target::Flutter => format!("({n}).abs()"),
    }
}

fn codegen_math_min(args: &[String], target: Target) -> String {
    let a = &args[0]; let b = &args[1];
    match target {
        Target::JavaScript | Target::TypeScript | Target::React | Target::Vue | Target::OmniView => {
            format!("Math.min({a}, {b})")
        }
        Target::Python => format!("min({a}, {b})"),
        Target::Rust | Target::Dioxus => format!("({a}).min({b})"),
        Target::Dart | Target::Flutter => format!("({a}).compareTo({b}) < 0 ? {a} : {b}"),
    }
}

fn codegen_math_max(args: &[String], target: Target) -> String {
    let a = &args[0]; let b = &args[1];
    match target {
        Target::JavaScript | Target::TypeScript | Target::React | Target::Vue | Target::OmniView => {
            format!("Math.max({a}, {b})")
        }
        Target::Python => format!("max({a}, {b})"),
        Target::Rust | Target::Dioxus => format!("({a}).max({b})"),
        Target::Dart | Target::Flutter => format!("({a}).compareTo({b}) > 0 ? {a} : {b}"),
    }
}

fn codegen_math_ceil(args: &[String], target: Target) -> String {
    let n = &args[0];
    match target {
        Target::JavaScript | Target::TypeScript | Target::React | Target::Vue | Target::OmniView => {
            format!("Math.ceil({n})")
        }
        Target::Python => format!("math.ceil({n})"),
        Target::Rust | Target::Dioxus => format!("({n}).ceil()"),
        Target::Dart | Target::Flutter => format!("({n}).ceil()"),
    }
}

fn codegen_math_floor(args: &[String], target: Target) -> String {
    let n = &args[0];
    match target {
        Target::JavaScript | Target::TypeScript | Target::React | Target::Vue | Target::OmniView => {
            format!("Math.floor({n})")
        }
        Target::Python => format!("math.floor({n})"),
        Target::Rust | Target::Dioxus => format!("({n}).floor()"),
        Target::Dart | Target::Flutter => format!("({n}).floor()"),
    }
}

fn codegen_math_round(args: &[String], target: Target) -> String {
    let n = &args[0];
    match target {
        Target::JavaScript | Target::TypeScript | Target::React | Target::Vue | Target::OmniView => {
            format!("Math.round({n})")
        }
        Target::Python => format!("round({n})"),
        Target::Rust | Target::Dioxus => format!("({n}).round()"),
        Target::Dart | Target::Flutter => format!("({n}).round()"),
    }
}

fn codegen_math_random(_args: &[String], target: Target) -> String {
    match target {
        Target::JavaScript | Target::TypeScript | Target::React | Target::Vue | Target::OmniView => {
            "Math.random()".to_string()
        }
        Target::Python => "random.random()".to_string(),
        Target::Rust | Target::Dioxus => "rand::random::<f64>()".to_string(),
        Target::Dart | Target::Flutter => "Random().nextDouble()".to_string(),
    }
}

fn codegen_math_sqrt(args: &[String], target: Target) -> String {
    let n = &args[0];
    match target {
        Target::JavaScript | Target::TypeScript | Target::React | Target::Vue | Target::OmniView => {
            format!("Math.sqrt({n})")
        }
        Target::Python => format!("math.sqrt({n})"),
        Target::Rust | Target::Dioxus => format!("({n}).sqrt()"),
        Target::Dart | Target::Flutter => format!("sqrt({n})"),
    }
}

fn codegen_math_pow(args: &[String], target: Target) -> String {
    let base = &args[0]; let exp = &args[1];
    match target {
        Target::JavaScript | Target::TypeScript | Target::React | Target::Vue | Target::OmniView => {
            format!("Math.pow({base}, {exp})")
        }
        Target::Python => format!("math.pow({base}, {exp})"),
        Target::Rust | Target::Dioxus => format!("({base}).powf({exp})"),
        Target::Dart | Target::Flutter => format!("pow({base}, {exp})"),
    }
}

// ═══════════════════════════════════════════════════════════
// String codegen
// ═══════════════════════════════════════════════════════════

fn codegen_string_length(args: &[String], target: Target) -> String {
    let s = &args[0];
    match target {
        Target::JavaScript | Target::TypeScript | Target::React | Target::Vue | Target::OmniView => {
            format!("({s}).length")
        }
        Target::Python => format!("len({s})"),
        Target::Rust | Target::Dioxus => format!("({s}).len()"),
        Target::Dart | Target::Flutter => format!("({s}).length"),
    }
}

fn codegen_string_concat(args: &[String], target: Target) -> String {
    let a = &args[0]; let b = &args[1];
    match target {
        Target::JavaScript | Target::TypeScript | Target::React | Target::Vue | Target::OmniView => {
            format!("({a}) + ({b})")
        }
        Target::Python => format!("({a}) + ({b})"),
        Target::Rust | Target::Dioxus => format!("({a}).clone() + ({b})"),
        Target::Dart | Target::Flutter => format!("({a}) + ({b})"),
    }
}

fn codegen_string_split(args: &[String], target: Target) -> String {
    let s = &args[0]; let sep = &args[1];
    match target {
        Target::JavaScript | Target::TypeScript | Target::React | Target::Vue | Target::OmniView => {
            format!("({s}).split({sep})")
        }
        Target::Python => format!("({s}).split({sep})"),
        Target::Rust | Target::Dioxus => format!("({s}).split({sep}).collect::<Vec<_>>()"),
        Target::Dart | Target::Flutter => format!("({s}).split({sep}).toList()"),
    }
}

fn codegen_string_join(args: &[String], target: Target) -> String {
    let arr = &args[0]; let sep = &args[1];
    match target {
        Target::JavaScript | Target::TypeScript | Target::React | Target::Vue | Target::OmniView => {
            format!("({arr}).join({sep})")
        }
        Target::Python => format!("({sep}).join({arr})"),
        Target::Rust | Target::Dioxus => format!("({arr}).join({sep})"),
        Target::Dart | Target::Flutter => format!("({arr}).join({sep})"),
    }
}

fn codegen_string_replace(args: &[String], target: Target) -> String {
    let s = &args[0]; let from = &args[1]; let to = &args[2];
    match target {
        Target::JavaScript | Target::TypeScript | Target::React | Target::Vue | Target::OmniView => {
            format!("({s}).replaceAll({from}, {to})")
        }
        Target::Python => format!("({s}).replace({from}, {to})"),
        Target::Rust | Target::Dioxus => format!("({s}).replace({from}, {to})"),
        Target::Dart | Target::Flutter => format!("({s}).replaceAll({from}, {to})"),
    }
}

fn codegen_string_substring(args: &[String], target: Target) -> String {
    let s = &args[0]; let start = &args[1];
    let end = args.get(2).map(|e| e.as_str()).unwrap_or("undefined");
    match target {
        Target::JavaScript | Target::TypeScript | Target::React | Target::Vue | Target::OmniView => {
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

fn codegen_string_contains(args: &[String], target: Target) -> String {
    let s = &args[0]; let sub = &args[1];
    match target {
        Target::JavaScript | Target::TypeScript | Target::React | Target::Vue | Target::OmniView => {
            format!("({s}).includes({sub})")
        }
        Target::Python => format!("({sub}) in ({s})"),
        Target::Rust | Target::Dioxus => format!("({s}).contains({sub})"),
        Target::Dart | Target::Flutter => format!("({s}).contains({sub})"),
    }
}

fn codegen_string_to_upper(args: &[String], target: Target) -> String {
    let s = &args[0];
    match target {
        Target::JavaScript | Target::TypeScript | Target::React | Target::Vue | Target::OmniView => {
            format!("({s}).toUpperCase()")
        }
        Target::Python => format!("({s}).upper()"),
        Target::Rust | Target::Dioxus => format!("({s}).to_uppercase()"),
        Target::Dart | Target::Flutter => format!("({s}).toUpperCase()"),
    }
}

fn codegen_string_to_lower(args: &[String], target: Target) -> String {
    let s = &args[0];
    match target {
        Target::JavaScript | Target::TypeScript | Target::React | Target::Vue | Target::OmniView => {
            format!("({s}).toLowerCase()")
        }
        Target::Python => format!("({s}).lower()"),
        Target::Rust | Target::Dioxus => format!("({s}).to_lowercase()"),
        Target::Dart | Target::Flutter => format!("({s}).toLowerCase()"),
    }
}

fn codegen_string_trim(args: &[String], target: Target) -> String {
    let s = &args[0];
    match target {
        Target::JavaScript | Target::TypeScript | Target::React | Target::Vue | Target::OmniView => {
            format!("({s}).trim()")
        }
        Target::Python => format!("({s}).strip()"),
        Target::Rust | Target::Dioxus => format!("({s}).trim()"),
        Target::Dart | Target::Flutter => format!("({s}).trim()"),
    }
}

fn codegen_string_starts_with(args: &[String], target: Target) -> String {
    let s = &args[0]; let prefix = &args[1];
    match target {
        Target::JavaScript | Target::TypeScript | Target::React | Target::Vue | Target::OmniView => {
            format!("({s}).startsWith({prefix})")
        }
        Target::Python => format!("({s}).startswith({prefix})"),
        Target::Rust | Target::Dioxus => format!("({s}).starts_with({prefix})"),
        Target::Dart | Target::Flutter => format!("({s}).startsWith({prefix})"),
    }
}

fn codegen_string_ends_with(args: &[String], target: Target) -> String {
    let s = &args[0]; let suffix = &args[1];
    match target {
        Target::JavaScript | Target::TypeScript | Target::React | Target::Vue | Target::OmniView => {
            format!("({s}).endsWith({suffix})")
        }
        Target::Python => format!("({s}).endswith({suffix})"),
        Target::Rust | Target::Dioxus => format!("({s}).ends_with({suffix})"),
        Target::Dart | Target::Flutter => format!("({s}).endsWith({suffix})"),
    }
}

// ═══════════════════════════════════════════════════════════
// Array codegen
// ═══════════════════════════════════════════════════════════

fn codegen_array_length(args: &[String], target: Target) -> String {
    let arr = &args[0];
    match target {
        Target::JavaScript | Target::TypeScript | Target::React | Target::Vue | Target::OmniView => {
            format!("({arr}).length")
        }
        Target::Python => format!("len({arr})"),
        Target::Rust | Target::Dioxus => format!("({arr}).len()"),
        Target::Dart | Target::Flutter => format!("({arr}).length"),
    }
}

fn codegen_array_get(args: &[String], target: Target) -> String {
    let arr = &args[0]; let idx = &args[1];
    match target {
        Target::JavaScript | Target::TypeScript | Target::React | Target::Vue | Target::OmniView => {
            format!("({arr})[{idx}]")
        }
        Target::Python => format!("({arr})[{idx}] if 0 <= ({idx}) < len({arr}) else None"),
        Target::Rust | Target::Dioxus => format!("({arr}).get({idx} as usize).cloned()"),
        Target::Dart | Target::Flutter => format!("({idx}) >= 0 && ({idx}) < ({arr}).length ? ({arr})[{idx} as int] : null"),
    }
}

fn codegen_array_push(args: &[String], target: Target) -> String {
    let arr = &args[0]; let item = &args[1];
    match target {
        Target::JavaScript | Target::TypeScript | Target::React | Target::Vue | Target::OmniView => {
            format!("(({arr}).push({item}), ({arr}).slice())")
        }
        Target::Python => format!("({arr}).append({item}) or {arr}"),
        Target::Rust | Target::Dioxus => format!("{{ ({arr}).push({item}); {arr} }}"),
        Target::Dart | Target::Flutter => format!("({arr})..add({item})"),
    }
}

fn codegen_array_pop(args: &[String], target: Target) -> String {
    let arr = &args[0];
    match target {
        Target::JavaScript | Target::TypeScript | Target::React | Target::Vue | Target::OmniView => {
            format!("({arr}).pop()")
        }
        Target::Python => format!("({arr}).pop() if {arr} else None"),
        Target::Rust | Target::Dioxus => format!("({arr}).pop()"),
        Target::Dart | Target::Flutter => format!("({arr}).isNotEmpty ? ({arr}).removeLast() : null"),
    }
}

fn codegen_array_remove(args: &[String], target: Target) -> String {
    let arr = &args[0]; let idx = &args[1];
    match target {
        Target::JavaScript | Target::TypeScript | Target::React | Target::Vue | Target::OmniView => {
            format!("(({arr}).splice({idx}, 1), ({arr}).slice())")
        }
        Target::Python => format!("({arr}).pop({idx}) or {arr}"),
        Target::Rust | Target::Dioxus => format!("{{ ({arr}).remove({idx} as usize); {arr} }}"),
        Target::Dart | Target::Flutter => format!("({arr})..removeAt({idx} as int)"),
    }
}

fn codegen_array_map(args: &[String], target: Target) -> String {
    let arr = &args[0]; let fn_name = &args[1];
    match target {
        Target::JavaScript | Target::TypeScript | Target::React | Target::Vue | Target::OmniView => {
            format!("({arr}).map({fn_name})")
        }
        Target::Python => format!("[{fn_name}(x) for x in {arr}]"),
        Target::Rust | Target::Dioxus => format!("({arr}).iter().map({fn_name}).collect::<Vec<_>>()"),
        Target::Dart | Target::Flutter => format!("({arr}).map({fn_name}).toList()"),
    }
}

fn codegen_array_filter(args: &[String], target: Target) -> String {
    let arr = &args[0]; let fn_name = &args[1];
    match target {
        Target::JavaScript | Target::TypeScript | Target::React | Target::Vue | Target::OmniView => {
            format!("({arr}).filter({fn_name})")
        }
        Target::Python => format!("[x for x in {arr} if {fn_name}(x)]"),
        Target::Rust | Target::Dioxus => format!("({arr}).iter().filter(|x| {fn_name}(x)).cloned().collect::<Vec<_>>()"),
        Target::Dart | Target::Flutter => format!("({arr}).where({fn_name}).toList()"),
    }
}

fn codegen_array_find(args: &[String], target: Target) -> String {
    let arr = &args[0]; let fn_name = &args[1];
    match target {
        Target::JavaScript | Target::TypeScript | Target::React | Target::Vue | Target::OmniView => {
            format!("({arr}).find({fn_name})")
        }
        Target::Python => format!("next((x for x in {arr} if {fn_name}(x)), None)"),
        Target::Rust | Target::Dioxus => format!("({arr}).iter().find(|x| {fn_name}(x)).cloned()"),
        Target::Dart | Target::Flutter => format!("({arr}).firstWhere({fn_name}, orElse: () => null)"),
    }
}

fn codegen_array_sort(args: &[String], target: Target) -> String {
    let arr = &args[0];
    let closure = args.get(1);
    match target {
        Target::JavaScript | Target::TypeScript | Target::React | Target::Vue | Target::OmniView => {
            if let Some(c) = closure {
                format!("([...{arr}]).sort({c})")
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

fn codegen_array_contains(args: &[String], target: Target) -> String {
    let arr = &args[0]; let item = &args[1];
    match target {
        Target::JavaScript | Target::TypeScript | Target::React | Target::Vue | Target::OmniView => {
            format!("({arr}).includes({item})")
        }
        Target::Python => format!("({item}) in ({arr})"),
        Target::Rust | Target::Dioxus => format!("({arr}).contains(&{item})"),
        Target::Dart | Target::Flutter => format!("({arr}).contains({item})"),
    }
}

fn codegen_array_is_empty(args: &[String], target: Target) -> String {
    let arr = &args[0];
    match target {
        Target::JavaScript | Target::TypeScript | Target::React | Target::Vue | Target::OmniView => {
            format!("({arr}).length === 0")
        }
        Target::Python => format!("len({arr}) == 0"),
        Target::Rust | Target::Dioxus => format!("({arr}).is_empty()"),
        Target::Dart | Target::Flutter => format!("({arr}).isEmpty"),
    }
}

// ═══════════════════════════════════════════════════════════
// Console codegen
// ═══════════════════════════════════════════════════════════

fn codegen_console_log(args: &[String], target: Target) -> String {
    let msg = &args[0];
    match target {
        Target::JavaScript | Target::TypeScript | Target::React | Target::Vue | Target::OmniView => {
            format!("console.log({msg})")
        }
        Target::Python => format!("print({msg})"),
        Target::Rust | Target::Dioxus => format!("println!(\"{{}}\", {msg})"),
        Target::Dart | Target::Flutter => format!("print({msg})"),
    }
}

fn codegen_console_warn(args: &[String], target: Target) -> String {
    let msg = &args[0];
    match target {
        Target::JavaScript | Target::TypeScript | Target::React | Target::Vue | Target::OmniView => {
            format!("console.warn({msg})")
        }
        Target::Python => format!("import warnings; warnings.warn({msg})"),
        Target::Rust | Target::Dioxus => format!("eprintln!(\"WARN: {{}}\", {msg})"),
        Target::Dart | Target::Flutter => format!("print('WARN: ${{{msg}}}')"),
    }
}

fn codegen_console_error(args: &[String], target: Target) -> String {
    let msg = &args[0];
    match target {
        Target::JavaScript | Target::TypeScript | Target::React | Target::Vue | Target::OmniView => {
            format!("console.error({msg})")
        }
        Target::Python => format!("import sys; print({msg}, file=sys.stderr)"),
        Target::Rust | Target::Dioxus => format!("eprintln!(\"ERROR: {{}}\", {msg})"),
        Target::Dart | Target::Flutter => format!("print('ERROR: ${{{msg}}}')"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Registry tests ─────────────────────────────────

    #[test]
    fn test_lookup_http_get() {
        let f = lookup("http", "http_get").expect("http_get should exist");
        assert_eq!(f.module, "http");
        assert_eq!(f.name, "http_get");
        assert!(f.is_async);
        assert!(f.available_targets.contains(&Target::JavaScript));
        assert!(f.available_targets.contains(&Target::Python));
        assert!(f.available_targets.contains(&Target::Rust));
    }

    #[test]
    fn test_lookup_json_parse() {
        let f = lookup("json", "json_parse").expect("json_parse should exist");
        assert_eq!(f.module, "json");
        assert!(!f.is_async);
    }

    #[test]
    fn test_lookup_nonexistent() {
        assert!(lookup("nonexistent", "fn").is_none());
        assert!(lookup("http", "nonexistent").is_none());
    }

    #[test]
    fn test_storage_not_available_on_rust() {
        assert!(!module_available("storage", Target::Rust));
        assert!(!module_available("storage", Target::Python));
        assert!(!module_available("storage", Target::Dart));
    }

    #[test]
    fn test_storage_available_on_web() {
        assert!(module_available("storage", Target::JavaScript));
        assert!(module_available("storage", Target::OmniView));
        assert!(module_available("storage", Target::Flutter));
    }

    #[test]
    fn test_http_available_everywhere() {
        for target in &ALL_TARGETS {
            assert!(
                module_available("http", *target),
                "http should be available on {:?}",
                target
            );
        }
    }

    // ── Codegen tests ──────────────────────────────────

    #[test]
    fn test_codegen_http_get_js() {
        let result = codegen_http_get(&["\"url\"".into()], Target::JavaScript);
        assert!(result.contains("fetch"), "JS http_get should use fetch. Got: {}", result);
    }

    #[test]
    fn test_codegen_http_get_python() {
        let result = codegen_http_get(&["\"url\"".into()], Target::Python);
        assert!(result.contains("requests.get"), "Python http_get should use requests. Got: {}", result);
    }

    #[test]
    fn test_codegen_http_get_rust() {
        let result = codegen_http_get(&["\"url\"".into()], Target::Rust);
        assert!(result.contains("reqwest::blocking::get"), "Rust http_get should use reqwest. Got: {}", result);
    }

    #[test]
    fn test_codegen_http_get_dart() {
        let result = codegen_http_get(&["\"url\"".into()], Target::Dart);
        assert!(result.contains("http.get"), "Dart http_get should use http.get. Got: {}", result);
    }

    #[test]
    fn test_codegen_json_parse_js() {
        let result = codegen_json_parse(&["text".into()], Target::JavaScript);
        assert_eq!(result, "JSON.parse(text)");
    }

    #[test]
    fn test_codegen_json_stringify_js() {
        let result = codegen_json_stringify(&["obj".into()], Target::JavaScript);
        assert_eq!(result, "JSON.stringify(obj)");
    }

    #[test]
    fn test_codegen_json_parse_python() {
        let result = codegen_json_parse(&["text".into()], Target::Python);
        assert_eq!(result, "json.loads(text)");
    }

    #[test]
    fn test_codegen_json_stringify_python() {
        let result = codegen_json_stringify(&["obj".into()], Target::Python);
        assert_eq!(result, "json.dumps(obj)");
    }

    #[test]
    fn test_codegen_json_parse_rust() {
        let result = codegen_json_parse(&["text".into()], Target::Rust);
        assert!(result.contains("serde_json::from_str"), "Got: {}", result);
    }

    #[test]
    fn test_codegen_storage_get_js() {
        let result = codegen_storage_get(&["\"key\"".into()], Target::JavaScript);
        assert!(result.contains("localStorage.getItem"), "Got: {}", result);
    }

    #[test]
    fn test_codegen_storage_set_js() {
        let result = codegen_storage_set(&["\"key\"".into(), "\"val\"".into()], Target::JavaScript);
        assert!(result.contains("localStorage.setItem"), "Got: {}", result);
    }

    #[test]
    #[should_panic(expected = "storage_get not available")]
    fn test_codegen_storage_get_panics_on_rust() {
        codegen_storage_get(&["\"key\"".into()], Target::Rust);
    }

    #[test]
    fn test_codegen_http_send_js() {
        let result = codegen_http_send(&["req".into()], Target::JavaScript);
        assert!(result.contains("fetch(req.url"), "Got: {}", result);
        assert!(result.contains("AbortSignal.timeout"), "Got: {}", result);
    }

    // ── Math module tests ─────────────────────────────

    #[test]
    fn test_lookup_math_abs() {
        let f = lookup("math", "math_abs").expect("math_abs should exist");
        assert_eq!(f.module, "math");
        assert!(!f.is_async);
        assert!(f.available_targets.contains(&Target::JavaScript));
    }

    #[test]
    fn test_codegen_math_abs_js() {
        let result = codegen_math_abs(&["x".into()], Target::JavaScript);
        assert_eq!(result, "Math.abs(x)");
    }

    #[test]
    fn test_codegen_math_abs_python() {
        let result = codegen_math_abs(&["x".into()], Target::Python);
        assert_eq!(result, "abs(x)");
    }

    #[test]
    fn test_codegen_math_abs_rust() {
        let result = codegen_math_abs(&["x".into()], Target::Rust);
        assert_eq!(result, "(x).abs()");
    }

    #[test]
    fn test_codegen_math_abs_dart() {
        let result = codegen_math_abs(&["x".into()], Target::Dart);
        assert_eq!(result, "(x).abs()");
    }

    #[test]
    fn test_codegen_math_min_js() {
        let result = codegen_math_min(&["a".into(), "b".into()], Target::JavaScript);
        assert_eq!(result, "Math.min(a, b)");
    }

    #[test]
    fn test_codegen_math_min_python() {
        let result = codegen_math_min(&["a".into(), "b".into()], Target::Python);
        assert_eq!(result, "min(a, b)");
    }

    #[test]
    fn test_codegen_math_random_js() {
        let result = codegen_math_random(&[], Target::JavaScript);
        assert_eq!(result, "Math.random()");
    }

    #[test]
    fn test_codegen_math_sqrt_js() {
        let result = codegen_math_sqrt(&["x".into()], Target::JavaScript);
        assert_eq!(result, "Math.sqrt(x)");
    }

    #[test]
    fn test_codegen_math_pow_js() {
        let result = codegen_math_pow(&["base".into(), "exp".into()], Target::JavaScript);
        assert_eq!(result, "Math.pow(base, exp)");
    }

    #[test]
    fn test_math_available_on_all_targets() {
        for target in &ALL_TARGETS {
            assert!(
                module_available("math", *target),
                "math should be available on {:?}", target
            );
        }
    }

    // ── String module tests ───────────────────────────

    #[test]
    fn test_lookup_string_length() {
        let f = lookup("string", "string_length").expect("string_length should exist");
        assert_eq!(f.module, "string");
        assert!(!f.is_async);
    }

    #[test]
    fn test_codegen_string_length_js() {
        assert_eq!(codegen_string_length(&["s".into()], Target::JavaScript), "(s).length");
    }

    #[test]
    fn test_codegen_string_length_python() {
        assert_eq!(codegen_string_length(&["s".into()], Target::Python), "len(s)");
    }

    #[test]
    fn test_codegen_string_concat_js() {
        assert_eq!(codegen_string_concat(&["a".into(), "b".into()], Target::JavaScript), "(a) + (b)");
    }

    #[test]
    fn test_codegen_string_contains_js() {
        assert_eq!(codegen_string_contains(&["s".into(), "\"x\"".into()], Target::JavaScript), "(s).includes(\"x\")");
    }

    #[test]
    fn test_codegen_string_trim_js() {
        assert_eq!(codegen_string_trim(&["s".into()], Target::JavaScript), "(s).trim()");
    }

    #[test]
    fn test_codegen_string_split_python() {
        assert_eq!(codegen_string_split(&["s".into(), "\",\"".into()], Target::Python), "(s).split(\",\")");
    }

    #[test]
    fn test_string_available_on_all_targets() {
        for target in &ALL_TARGETS {
            assert!(module_available("string", *target), "string should be available on {:?}", target);
        }
    }

    // ── Array + Console module tests ──────────────────

    #[test]
    fn test_codegen_array_length_js() {
        assert_eq!(codegen_array_length(&["arr".into()], Target::JavaScript), "(arr).length");
    }

    #[test]
    fn test_codegen_array_push_js() {
        let result = codegen_array_push(&["arr".into(), "item".into()], Target::JavaScript);
        assert!(result.contains("push"), "Got: {}", result);
        assert!(result.contains("arr"), "Got: {}", result);
    }

    #[test]
    fn test_codegen_array_map_python() {
        let result = codegen_array_map(&["arr".into(), "f".into()], Target::Python);
        assert_eq!(result, "[f(x) for x in arr]");
    }

    #[test]
    fn test_codegen_console_log_js() {
        assert_eq!(codegen_console_log(&["\"hi\"".into()], Target::JavaScript), "console.log(\"hi\")");
    }

    #[test]
    fn test_codegen_console_log_python() {
        assert_eq!(codegen_console_log(&["\"hi\"".into()], Target::Python), "print(\"hi\")");
    }

    #[test]
    fn test_codegen_console_error_rust() {
        let result = codegen_console_error(&["msg".into()], Target::Rust);
        assert!(result.contains("eprintln!"), "Got: {}", result);
    }

    #[test]
    fn test_array_and_console_available_on_all_targets() {
        for target in &ALL_TARGETS {
            assert!(module_available("array", *target));
            assert!(module_available("console", *target));
        }
    }
}
