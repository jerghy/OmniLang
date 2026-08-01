use super::{StdlibFn, ALL_TARGETS};
use crate::codegen::Target;

pub(super) fn register() -> Vec<StdlibFn> {
    vec![
        StdlibFn {
            module: "encode",
            name: "encode_base64_encode",
            min_args: 1,
            max_args: Some(1),
            available_targets: &ALL_TARGETS,
            codegen: codegen_encode_base64_encode,
            is_async: false,
        },
        StdlibFn {
            module: "encode",
            name: "encode_base64_decode",
            min_args: 1,
            max_args: Some(1),
            available_targets: &ALL_TARGETS,
            codegen: codegen_encode_base64_decode,
            is_async: false,
        },
        StdlibFn {
            module: "encode",
            name: "encode_hex_encode",
            min_args: 1,
            max_args: Some(1),
            available_targets: &ALL_TARGETS,
            codegen: codegen_encode_hex_encode,
            is_async: false,
        },
        StdlibFn {
            module: "encode",
            name: "encode_hex_decode",
            min_args: 1,
            max_args: Some(1),
            available_targets: &ALL_TARGETS,
            codegen: codegen_encode_hex_decode,
            is_async: false,
        },
        StdlibFn {
            module: "encode",
            name: "encode_url_encode",
            min_args: 1,
            max_args: Some(1),
            available_targets: &ALL_TARGETS,
            codegen: codegen_encode_url_encode,
            is_async: false,
        },
        StdlibFn {
            module: "encode",
            name: "encode_url_decode",
            min_args: 1,
            max_args: Some(1),
            available_targets: &ALL_TARGETS,
            codegen: codegen_encode_url_decode,
            is_async: false,
        },
    ]
}

pub(super) fn codegen_encode_base64_encode(args: &[String], target: Target) -> String {
    let s = &args[0];
    match target {
        Target::JavaScript
        | Target::TypeScript
        | Target::React
        | Target::Vue
        | Target::OmniView => {
            format!("btoa({s})")
        }
        Target::Python => format!("base64.b64encode({s}.encode()).decode()"),
        Target::Rust | Target::Dioxus => {
            format!("base64::engine::general_purpose::STANDARD.encode({s})")
        }
        Target::Dart | Target::Flutter => format!("base64Encode(utf8.encode({s}))"),
    }
}

pub(super) fn codegen_encode_base64_decode(args: &[String], target: Target) -> String {
    let s = &args[0];
    match target {
        Target::JavaScript
        | Target::TypeScript
        | Target::React
        | Target::Vue
        | Target::OmniView => {
            format!("atob({s})")
        }
        Target::Python => format!("base64.b64decode({s}).decode()"),
        Target::Rust | Target::Dioxus => format!(
            "String::from_utf8(base64::engine::general_purpose::STANDARD.decode({s}).unwrap_or_default()).unwrap_or_default()"
        ),
        Target::Dart | Target::Flutter => format!("utf8.decode(base64Decode({s}))"),
    }
}

pub(super) fn codegen_encode_hex_encode(args: &[String], target: Target) -> String {
    let s = &args[0];
    match target {
        Target::JavaScript
        | Target::TypeScript
        | Target::React
        | Target::Vue
        | Target::OmniView => {
            format!(
                "Array.from({s}, (c) => c.charCodeAt(0).toString(16).padStart(2, '0')).join('')"
            )
        }
        Target::Python => format!("{s}.encode().hex()"),
        Target::Rust | Target::Dioxus => format!("hex::encode({s})"),
        Target::Dart | Target::Flutter => {
            format!("{s}.codeUnits.map((c) => c.toRadixString(16).padLeft(2, '0')).join()")
        }
    }
}

pub(super) fn codegen_encode_hex_decode(args: &[String], target: Target) -> String {
    let s = &args[0];
    match target {
        Target::JavaScript
        | Target::TypeScript
        | Target::React
        | Target::Vue
        | Target::OmniView => {
            // Guard empty input: `"".match(...)` is null → `|| []` avoids a crash.
            format!(
                "(({s}).match(/.{{1,2}}/g) || []).map((b) => String.fromCharCode(parseInt(b, 16))).join('')"
            )
        }
        Target::Python => format!("bytes.fromhex({s}).decode()"),
        Target::Rust | Target::Dioxus => format!(
            "String::from_utf8(hex::decode({s}).unwrap_or_default()).unwrap_or_default()"
        ),
        Target::Dart | Target::Flutter => format!(
            "String.fromCharCodes(List.generate({s}.length ~/ 2, (i) => int.parse({s}.substring(i * 2, i * 2 + 2), radix: 16)))"
        ),
    }
}

pub(super) fn codegen_encode_url_encode(args: &[String], target: Target) -> String {
    let s = &args[0];
    match target {
        Target::JavaScript
        | Target::TypeScript
        | Target::React
        | Target::Vue
        | Target::OmniView => {
            format!("encodeURIComponent({s})")
        }
        Target::Python => format!("urllib.parse.quote({s})"),
        Target::Rust | Target::Dioxus => format!("urlencoding::encode({s})"),
        Target::Dart | Target::Flutter => format!("Uri.encodeComponent({s})"),
    }
}

pub(super) fn codegen_encode_url_decode(args: &[String], target: Target) -> String {
    let s = &args[0];
    match target {
        Target::JavaScript
        | Target::TypeScript
        | Target::React
        | Target::Vue
        | Target::OmniView => {
            format!("decodeURIComponent({s})")
        }
        Target::Python => format!("urllib.parse.unquote({s})"),
        Target::Rust | Target::Dioxus => {
            format!("urlencoding::decode({s}).unwrap_or_default()")
        }
        Target::Dart | Target::Flutter => format!("Uri.decodeComponent({s})"),
    }
}
