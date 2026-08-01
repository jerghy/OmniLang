#[allow(unused_imports)]
use super::array::{codegen_array_length, codegen_array_map, codegen_array_push};
#[allow(unused_imports)]
use super::async_mod::{
    codegen_async_all, codegen_async_race, codegen_async_sleep, codegen_async_spawn,
    codegen_async_timeout,
};
#[allow(unused_imports)]
use super::console::{codegen_console_error, codegen_console_log};
#[allow(unused_imports)]
use super::datetime::{
    codegen_datetime_day, codegen_datetime_format, codegen_datetime_from_timestamp,
    codegen_datetime_hour, codegen_datetime_minute, codegen_datetime_month, codegen_datetime_now,
    codegen_datetime_second, codegen_datetime_year,
};
#[allow(unused_imports)]
use super::encode::{
    codegen_encode_base64_decode, codegen_encode_base64_encode, codegen_encode_hex_decode,
    codegen_encode_hex_encode, codegen_encode_url_decode, codegen_encode_url_encode,
};
#[allow(unused_imports)]
use super::http::{codegen_http_get, codegen_http_post, codegen_http_send};
#[allow(unused_imports)]
use super::json::{codegen_json_parse, codegen_json_stringify};
#[allow(unused_imports)]
use super::math::{
    codegen_math_abs, codegen_math_ceil, codegen_math_clamp, codegen_math_cos, codegen_math_e,
    codegen_math_floor, codegen_math_log, codegen_math_log10, codegen_math_log2, codegen_math_max,
    codegen_math_min, codegen_math_pi, codegen_math_pow, codegen_math_random,
    codegen_math_random_int, codegen_math_round, codegen_math_sin, codegen_math_sqrt,
    codegen_math_tan,
};
#[allow(unused_imports)]
use super::regex::{codegen_regex_match, codegen_regex_replace, codegen_regex_search};
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

// ── Async module tests ─────────────────────────────

#[test]
fn test_lookup_async_sleep() {
    let f = lookup("async", "async_sleep").expect("async_sleep should exist");
    assert_eq!(f.module, "async");
    assert!(f.is_async, "async_sleep should be async");
    assert!(f.available_targets.contains(&Target::OmniView));
}

#[test]
fn test_lookup_async_all_functions() {
    for name in [
        "async_sleep",
        "async_spawn",
        "async_all",
        "async_race",
        "async_timeout",
    ] {
        let f = lookup("async", name).unwrap_or_else(|| panic!("{name} should exist"));
        assert!(f.is_async, "{name} should be async");
    }
}

#[test]
fn test_codegen_async_sleep_js() {
    let result = codegen_async_sleep(&["20".into()], Target::JavaScript);
    assert!(result.contains("setTimeout"), "Got: {}", result);
    assert!(result.contains("20"), "Got: {}", result);
}

#[test]
fn test_codegen_async_spawn_js() {
    let result = codegen_async_spawn(&["task_a()".into()], Target::OmniView);
    assert_eq!(result, "Promise.resolve(task_a())");
}

#[test]
fn test_codegen_async_all_js() {
    let result = codegen_async_all(&["[a, b]".into()], Target::JavaScript);
    assert_eq!(result, "Promise.all([a, b])");
}

#[test]
fn test_codegen_async_race_js() {
    let result = codegen_async_race(&["[a, b]".into()], Target::JavaScript);
    assert_eq!(result, "Promise.race([a, b])");
}

#[test]
fn test_codegen_async_timeout_js() {
    let result = codegen_async_timeout(&["task".into(), "50".into()], Target::JavaScript);
    assert!(result.contains("Promise.race"), "Got: {}", result);
    assert!(result.contains("new Error('timeout')"), "Got: {}", result);
    assert!(result.contains("50"), "Got: {}", result);
}

#[test]
fn test_codegen_async_sleep_python() {
    let result = codegen_async_sleep(&["20".into()], Target::Python);
    assert_eq!(result, "asyncio.sleep(20 / 1000)");
}

#[test]
fn test_codegen_async_timeout_dart() {
    let result = codegen_async_timeout(&["task".into(), "50".into()], Target::Dart);
    assert_eq!(result, "task.timeout(Duration(milliseconds: 50))");
}

#[test]
fn test_async_available_on_all_targets() {
    for target in &ALL_TARGETS {
        assert!(
            module_available("async", *target),
            "async should be available on {:?}",
            target
        );
    }
}

// ── Datetime module tests ──────────────────────────

#[test]
fn test_lookup_datetime_now() {
    let f = lookup("datetime", "datetime_now").expect("datetime_now should exist");
    assert_eq!(f.module, "datetime");
    assert!(!f.is_async);
}

#[test]
fn test_lookup_datetime_all_functions() {
    for name in [
        "datetime_now",
        "datetime_format",
        "datetime_from_timestamp",
        "datetime_year",
        "datetime_month",
        "datetime_day",
        "datetime_hour",
        "datetime_minute",
        "datetime_second",
    ] {
        assert!(
            lookup("datetime", name).is_some(),
            "{name} should exist in std/datetime"
        );
    }
}

#[test]
fn test_codegen_datetime_now_js() {
    let result = codegen_datetime_now(&[], Target::OmniView);
    assert_eq!(result, "new Date()");
}

#[test]
fn test_codegen_datetime_format_js() {
    let result = codegen_datetime_format(
        &["now".into(), "\"%Y-%m-%d %H:%M:%S\"".into()],
        Target::OmniView,
    );
    assert!(result.contains("getFullYear"), "Got: {}", result);
    assert!(result.contains("getMonth"), "Got: {}", result);
    assert!(result.contains("getSeconds"), "Got: {}", result);
    assert!(result.contains("now"), "Got: {}", result);
}

#[test]
fn test_codegen_datetime_format_python() {
    let result = codegen_datetime_format(&["now".into(), "\"%Y-%m-%d\"".into()], Target::Python);
    assert_eq!(result, "now.strftime(\"%Y-%m-%d\")");
}

#[test]
fn test_codegen_datetime_from_timestamp_js() {
    let result = codegen_datetime_from_timestamp(&["0".into()], Target::JavaScript);
    assert_eq!(result, "new Date(0)");
}

#[test]
fn test_codegen_datetime_extractors_js() {
    assert_eq!(
        codegen_datetime_year(&["now".into()], Target::JavaScript),
        "new Date(now).getFullYear()"
    );
    assert_eq!(
        codegen_datetime_month(&["now".into()], Target::JavaScript),
        "new Date(now).getMonth() + 1"
    );
    assert_eq!(
        codegen_datetime_day(&["now".into()], Target::JavaScript),
        "new Date(now).getDate()"
    );
    assert_eq!(
        codegen_datetime_hour(&["now".into()], Target::JavaScript),
        "new Date(now).getHours()"
    );
    assert_eq!(
        codegen_datetime_minute(&["now".into()], Target::JavaScript),
        "new Date(now).getMinutes()"
    );
    assert_eq!(
        codegen_datetime_second(&["now".into()], Target::JavaScript),
        "new Date(now).getSeconds()"
    );
}

#[test]
fn test_codegen_datetime_year_python() {
    assert_eq!(
        codegen_datetime_year(&["now".into()], Target::Python),
        "now.year"
    );
}

#[test]
fn test_codegen_datetime_now_dart() {
    assert_eq!(codegen_datetime_now(&[], Target::Flutter), "DateTime.now()");
}

#[test]
fn test_datetime_available_on_all_targets() {
    for target in &ALL_TARGETS {
        assert!(
            module_available("datetime", *target),
            "datetime should be available on {:?}",
            target
        );
    }
}

// ── Encode module tests ────────────────────────────

#[test]
fn test_lookup_encode_all_functions() {
    for name in [
        "encode_base64_encode",
        "encode_base64_decode",
        "encode_hex_encode",
        "encode_hex_decode",
        "encode_url_encode",
        "encode_url_decode",
    ] {
        let f = lookup("encode", name).unwrap_or_else(|| panic!("{name} should exist"));
        assert!(!f.is_async, "{name} should be sync");
    }
}

#[test]
fn test_codegen_encode_base64_js() {
    assert_eq!(
        codegen_encode_base64_encode(&["s".into()], Target::OmniView),
        "btoa(s)"
    );
    assert_eq!(
        codegen_encode_base64_decode(&["s".into()], Target::OmniView),
        "atob(s)"
    );
}

#[test]
fn test_codegen_encode_hex_js() {
    let enc = codegen_encode_hex_encode(&["s".into()], Target::JavaScript);
    assert!(enc.contains("charCodeAt"), "Got: {}", enc);
    let dec = codegen_encode_hex_decode(&["s".into()], Target::JavaScript);
    assert!(dec.contains("fromCharCode"), "Got: {}", dec);
}

#[test]
fn test_codegen_encode_url_js() {
    assert_eq!(
        codegen_encode_url_encode(&["s".into()], Target::JavaScript),
        "encodeURIComponent(s)"
    );
    assert_eq!(
        codegen_encode_url_decode(&["s".into()], Target::JavaScript),
        "decodeURIComponent(s)"
    );
}

#[test]
fn test_codegen_encode_base64_python() {
    let enc = codegen_encode_base64_encode(&["s".into()], Target::Python);
    assert!(enc.contains("base64.b64encode"), "Got: {}", enc);
    let dec = codegen_encode_base64_decode(&["s".into()], Target::Python);
    assert!(dec.contains("base64.b64decode"), "Got: {}", dec);
}

#[test]
fn test_codegen_encode_hex_rust() {
    let enc = codegen_encode_hex_encode(&["s".into()], Target::Rust);
    assert_eq!(enc, "hex::encode(s)");
}

#[test]
fn test_encode_available_on_all_targets() {
    for target in &ALL_TARGETS {
        assert!(
            module_available("encode", *target),
            "encode should be available on {:?}",
            target
        );
    }
}

// ── Regex module tests ─────────────────────────────

#[test]
fn test_lookup_regex_all_functions() {
    for name in ["regex_match", "regex_search", "regex_replace"] {
        let f = lookup("regex", name).unwrap_or_else(|| panic!("{name} should exist"));
        assert!(!f.is_async, "{name} should be sync");
    }
}

#[test]
fn test_codegen_regex_match_js() {
    let result = codegen_regex_match(&["s".into(), "\"^[a-z]+$\"".into()], Target::OmniView);
    assert!(result.contains(".test("), "Got: {}", result);
    assert!(result.contains("new RegExp"), "Got: {}", result);
}

#[test]
fn test_codegen_regex_search_js() {
    let result = codegen_regex_search(&["s".into(), "\"\\\\d+\"".into()], Target::JavaScript);
    assert!(result.contains("matchAll"), "Got: {}", result);
    assert!(result.contains("'g'"), "Got: {}", result);
}

#[test]
fn test_codegen_regex_replace_js() {
    let result = codegen_regex_replace(
        &["s".into(), "\"\\\\d+\"".into(), "\"X\"".into()],
        Target::JavaScript,
    );
    assert!(result.contains(".replace("), "Got: {}", result);
    assert!(result.contains("'g'"), "Got: {}", result);
}

#[test]
fn test_codegen_regex_match_python() {
    let result = codegen_regex_match(&["s".into(), "p".into()], Target::Python);
    assert!(result.contains("re.fullmatch"), "Got: {}", result);
}

#[test]
fn test_codegen_regex_search_rust() {
    let result = codegen_regex_search(&["s".into(), "p".into()], Target::Rust);
    assert!(result.contains("find_iter"), "Got: {}", result);
}

#[test]
fn test_regex_available_on_all_targets() {
    for target in &ALL_TARGETS {
        assert!(
            module_available("regex", *target),
            "regex should be available on {:?}",
            target
        );
    }
}

// ── Math extension tests ───────────────────────────

#[test]
fn test_lookup_math_extensions() {
    for name in [
        "math_sin",
        "math_cos",
        "math_tan",
        "math_log",
        "math_log2",
        "math_log10",
        "math_pi",
        "math_e",
        "math_clamp",
        "math_random_int",
    ] {
        let f = lookup("math", name).unwrap_or_else(|| panic!("{name} should exist"));
        assert!(!f.is_async, "{name} should be sync");
    }
}

#[test]
fn test_codegen_math_trig_js() {
    assert_eq!(
        codegen_math_sin(&["x".into()], Target::JavaScript),
        "Math.sin(x)"
    );
    assert_eq!(
        codegen_math_cos(&["x".into()], Target::JavaScript),
        "Math.cos(x)"
    );
    assert_eq!(
        codegen_math_tan(&["x".into()], Target::JavaScript),
        "Math.tan(x)"
    );
}

#[test]
fn test_codegen_math_logs_js() {
    assert_eq!(
        codegen_math_log(&["x".into()], Target::JavaScript),
        "Math.log(x)"
    );
    assert_eq!(
        codegen_math_log2(&["x".into()], Target::JavaScript),
        "Math.log2(x)"
    );
    assert_eq!(
        codegen_math_log10(&["x".into()], Target::JavaScript),
        "Math.log10(x)"
    );
}

#[test]
fn test_codegen_math_constants_js() {
    assert_eq!(codegen_math_pi(&[], Target::JavaScript), "Math.PI");
    assert_eq!(codegen_math_e(&[], Target::JavaScript), "Math.E");
}

#[test]
fn test_codegen_math_constants_python() {
    assert_eq!(codegen_math_pi(&[], Target::Python), "math.pi");
    assert_eq!(codegen_math_e(&[], Target::Python), "math.e");
}

#[test]
fn test_codegen_math_clamp_js() {
    assert_eq!(
        codegen_math_clamp(&["v".into(), "lo".into(), "hi".into()], Target::JavaScript),
        "Math.min(Math.max(v, lo), hi)"
    );
}

#[test]
fn test_codegen_math_clamp_rust() {
    assert_eq!(
        codegen_math_clamp(&["v".into(), "lo".into(), "hi".into()], Target::Rust),
        "(v).clamp(lo, hi)"
    );
}

#[test]
fn test_codegen_math_random_int_js() {
    let result = codegen_math_random_int(&["lo".into(), "hi".into()], Target::JavaScript);
    assert!(
        result.contains("Math.floor(Math.random()"),
        "Got: {}",
        result
    );
    assert!(result.contains("hi - lo + 1"), "Got: {}", result);
}

#[test]
fn test_codegen_math_random_int_python() {
    assert_eq!(
        codegen_math_random_int(&["lo".into(), "hi".into()], Target::Python),
        "random.randint(lo, hi)"
    );
}

#[test]
fn test_codegen_math_log2_dart() {
    assert_eq!(
        codegen_math_log2(&["x".into()], Target::Dart),
        "log(x) / ln2"
    );
}
