use super::{StdlibFn, ALL_TARGETS};
use crate::codegen::Target;

pub(super) fn register() -> Vec<StdlibFn> {
    vec![
        StdlibFn {
            module: "datetime",
            name: "datetime_now",
            min_args: 0,
            max_args: Some(0),
            available_targets: &ALL_TARGETS,
            codegen: codegen_datetime_now,
            is_async: false,
        },
        StdlibFn {
            module: "datetime",
            name: "datetime_format",
            min_args: 2,
            max_args: Some(2),
            available_targets: &ALL_TARGETS,
            codegen: codegen_datetime_format,
            is_async: false,
        },
        StdlibFn {
            module: "datetime",
            name: "datetime_from_timestamp",
            min_args: 1,
            max_args: Some(1),
            available_targets: &ALL_TARGETS,
            codegen: codegen_datetime_from_timestamp,
            is_async: false,
        },
        StdlibFn {
            module: "datetime",
            name: "datetime_year",
            min_args: 1,
            max_args: Some(1),
            available_targets: &ALL_TARGETS,
            codegen: codegen_datetime_year,
            is_async: false,
        },
        StdlibFn {
            module: "datetime",
            name: "datetime_month",
            min_args: 1,
            max_args: Some(1),
            available_targets: &ALL_TARGETS,
            codegen: codegen_datetime_month,
            is_async: false,
        },
        StdlibFn {
            module: "datetime",
            name: "datetime_day",
            min_args: 1,
            max_args: Some(1),
            available_targets: &ALL_TARGETS,
            codegen: codegen_datetime_day,
            is_async: false,
        },
        StdlibFn {
            module: "datetime",
            name: "datetime_hour",
            min_args: 1,
            max_args: Some(1),
            available_targets: &ALL_TARGETS,
            codegen: codegen_datetime_hour,
            is_async: false,
        },
        StdlibFn {
            module: "datetime",
            name: "datetime_minute",
            min_args: 1,
            max_args: Some(1),
            available_targets: &ALL_TARGETS,
            codegen: codegen_datetime_minute,
            is_async: false,
        },
        StdlibFn {
            module: "datetime",
            name: "datetime_second",
            min_args: 1,
            max_args: Some(1),
            available_targets: &ALL_TARGETS,
            codegen: codegen_datetime_second,
            is_async: false,
        },
    ]
}

pub(super) fn codegen_datetime_now(_args: &[String], target: Target) -> String {
    match target {
        Target::JavaScript
        | Target::TypeScript
        | Target::React
        | Target::Vue
        | Target::OmniView => "new Date()".to_string(),
        Target::Python => "datetime.datetime.now()".to_string(),
        Target::Rust | Target::Dioxus => "chrono::Local::now()".to_string(),
        Target::Dart | Target::Flutter => "DateTime.now()".to_string(),
    }
}

pub(super) fn codegen_datetime_format(args: &[String], target: Target) -> String {
    let dt = &args[0];
    let fmt = &args[1];
    match target {
        // Inline formatter supporting %Y %m %d %H %M %S (+ %% literal escape) —
        // self-contained so it works in the OmniView inline JS runtime with no
        // shared helpers. `%%` is stashed in a placeholder first so `%%Y` keeps
        // its literal `%` and is not consumed by the %Y replacement.
        Target::JavaScript
        | Target::TypeScript
        | Target::React
        | Target::Vue
        | Target::OmniView => {
            format!(
                "((_dt, _fmt) => {{ const _pad = (n) => String(n).padStart(2, '0'); const _d = new Date(_dt); return _fmt.replace(/%%/g, '\\u0000').replace(/%Y/g, String(_d.getFullYear()).padStart(4, '0')).replace(/%m/g, _pad(_d.getMonth() + 1)).replace(/%d/g, _pad(_d.getDate())).replace(/%H/g, _pad(_d.getHours())).replace(/%M/g, _pad(_d.getMinutes())).replace(/%S/g, _pad(_d.getSeconds())).replace(/\\u0000/g, '%'); }})({dt}, {fmt})"
            )
        }
        Target::Python => format!("{dt}.strftime({fmt})"),
        Target::Rust | Target::Dioxus => format!("{dt}.format({fmt}).to_string()"),
        Target::Dart | Target::Flutter => format!("DateFormat({fmt}).format({dt})"),
    }
}

pub(super) fn codegen_datetime_from_timestamp(args: &[String], target: Target) -> String {
    let ts = &args[0];
    match target {
        Target::JavaScript
        | Target::TypeScript
        | Target::React
        | Target::Vue
        | Target::OmniView => {
            format!("new Date({ts})")
        }
        Target::Python => format!("datetime.datetime.fromtimestamp({ts} / 1000)"),
        Target::Rust | Target::Dioxus => {
            format!("chrono::DateTime::from_timestamp_millis({ts}).unwrap_or(chrono::Local::now())")
        }
        Target::Dart | Target::Flutter => {
            format!("DateTime.fromMillisecondsSinceEpoch({ts})")
        }
    }
}

macro_rules! define_extractor {
    ($name:ident, $codegen:ident, $js:expr, $python:expr, $rust:expr, $dart:expr) => {
        pub(super) fn $codegen(args: &[String], target: Target) -> String {
            let dt = &args[0];
            match target {
                Target::JavaScript
                | Target::TypeScript
                | Target::React
                | Target::Vue
                | Target::OmniView => {
                    format!("new Date({dt}){}", $js)
                }
                Target::Python => format!("{dt}{}", $python),
                Target::Rust | Target::Dioxus => format!("{dt}{}", $rust),
                Target::Dart | Target::Flutter => format!("{dt}{}", $dart),
            }
        }
    };
}

define_extractor!(
    year,
    codegen_datetime_year,
    ".getFullYear()",
    ".year",
    ".year()",
    ".year"
);
define_extractor!(
    month,
    codegen_datetime_month,
    ".getMonth() + 1",
    ".month",
    ".month()",
    ".month"
);
define_extractor!(
    day,
    codegen_datetime_day,
    ".getDate()",
    ".day",
    ".day()",
    ".day"
);
define_extractor!(
    hour,
    codegen_datetime_hour,
    ".getHours()",
    ".hour",
    ".hour()",
    ".hour"
);
define_extractor!(
    minute,
    codegen_datetime_minute,
    ".getMinutes()",
    ".minute",
    ".minute()",
    ".minute"
);
define_extractor!(
    second,
    codegen_datetime_second,
    ".getSeconds()",
    ".second",
    ".second()",
    ".second"
);
