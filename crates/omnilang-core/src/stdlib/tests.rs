#[allow(unused_imports)]
use super::array::{codegen_array_length, codegen_array_map, codegen_array_push};
#[allow(unused_imports)]
use super::console::{codegen_console_error, codegen_console_log};
#[allow(unused_imports)]
use super::http::{codegen_http_get, codegen_http_post, codegen_http_send};
#[allow(unused_imports)]
use super::json::{codegen_json_parse, codegen_json_stringify};
#[allow(unused_imports)]
use super::math::{
    codegen_math_abs, codegen_math_ceil, codegen_math_floor, codegen_math_max, codegen_math_min,
    codegen_math_pow, codegen_math_random, codegen_math_round, codegen_math_sqrt,
};
#[allow(unused_imports)]
use super::storage::{codegen_storage_get, codegen_storage_set};
#[allow(unused_imports)]
use super::string::{
    codegen_string_concat, codegen_string_contains, codegen_string_ends_with, codegen_string_join,
    codegen_string_length, codegen_string_replace, codegen_string_split,
    codegen_string_starts_with, codegen_string_substring, codegen_string_to_lower,
    codegen_string_to_upper, codegen_string_trim,
};
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
    assert!(
        result.contains("fetch"),
        "JS http_get should use fetch. Got: {}",
        result
    );
}

#[test]
fn test_codegen_http_get_python() {
    let result = codegen_http_get(&["\"url\"".into()], Target::Python);
    assert!(
        result.contains("requests.get"),
        "Python http_get should use requests. Got: {}",
        result
    );
}

#[test]
fn test_codegen_http_get_rust() {
    let result = codegen_http_get(&["\"url\"".into()], Target::Rust);
    assert!(
        result.contains("reqwest::blocking::get"),
        "Rust http_get should use reqwest. Got: {}",
        result
    );
}

#[test]
fn test_codegen_http_get_dart() {
    let result = codegen_http_get(&["\"url\"".into()], Target::Dart);
    assert!(
        result.contains("http.get"),
        "Dart http_get should use http.get. Got: {}",
        result
    );
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
fn test_codegen_storage_get_compile_error_on_unsupported_target() {
    let result = codegen_storage_get(&["\"key\"".into()], Target::Rust);
    assert!(
        result.contains("compile_error!"),
        "Expected compile_error! for unsupported target, got: {}",
        result
    );
    assert!(
        result.contains("storage_get not available"),
        "Expected error message about storage_get, got: {}",
        result
    );
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
            "math should be available on {:?}",
            target
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
    assert_eq!(
        codegen_string_length(&["s".into()], Target::JavaScript),
        "(s).length"
    );
}

#[test]
fn test_codegen_string_length_python() {
    assert_eq!(
        codegen_string_length(&["s".into()], Target::Python),
        "len(s)"
    );
}

#[test]
fn test_codegen_string_concat_js() {
    assert_eq!(
        codegen_string_concat(&["a".into(), "b".into()], Target::JavaScript),
        "(a) + (b)"
    );
}

#[test]
fn test_codegen_string_contains_js() {
    assert_eq!(
        codegen_string_contains(&["s".into(), "\"x\"".into()], Target::JavaScript),
        "(s).includes(\"x\")"
    );
}

#[test]
fn test_codegen_string_trim_js() {
    assert_eq!(
        codegen_string_trim(&["s".into()], Target::JavaScript),
        "(s).trim()"
    );
}

#[test]
fn test_codegen_string_split_python() {
    assert_eq!(
        codegen_string_split(&["s".into(), "\",\"".into()], Target::Python),
        "(s).split(\",\")"
    );
}

#[test]
fn test_string_available_on_all_targets() {
    for target in &ALL_TARGETS {
        assert!(
            module_available("string", *target),
            "string should be available on {:?}",
            target
        );
    }
}

// ── Array + Console module tests ──────────────────

#[test]
fn test_codegen_array_length_js() {
    assert_eq!(
        codegen_array_length(&["arr".into()], Target::JavaScript),
        "(arr).length"
    );
}

#[test]
fn test_codegen_array_push_js() {
    let result = codegen_array_push(&["arr".into(), "item".into()], Target::JavaScript);
    // Now uses immutable spread: [...arr, item]
    assert!(result.contains("..."), "Got: {}", result);
    assert!(result.contains("arr"), "Got: {}", result);
    assert!(result.contains("item"), "Got: {}", result);
}

#[test]
fn test_codegen_array_map_python() {
    let result = codegen_array_map(&["arr".into(), "f".into()], Target::Python);
    assert_eq!(result, "[f(x) for x in arr]");
}

#[test]
fn test_codegen_console_log_js() {
    assert_eq!(
        codegen_console_log(&["\"hi\"".into()], Target::JavaScript),
        "console.log(\"hi\")"
    );
}

#[test]
fn test_codegen_console_log_python() {
    assert_eq!(
        codegen_console_log(&["\"hi\"".into()], Target::Python),
        "print(\"hi\")"
    );
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
