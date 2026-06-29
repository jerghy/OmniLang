use super::{StdlibFn, ALL_TARGETS};
use crate::codegen::Target;

pub(super) fn register() -> Vec<StdlibFn> {
    vec![
        StdlibFn {
            module: "math",
            name: "math_abs",
            available_targets: &ALL_TARGETS,
            codegen: codegen_math_abs,
            is_async: false,
        },
        StdlibFn {
            module: "math",
            name: "math_min",
            available_targets: &ALL_TARGETS,
            codegen: codegen_math_min,
            is_async: false,
        },
        StdlibFn {
            module: "math",
            name: "math_max",
            available_targets: &ALL_TARGETS,
            codegen: codegen_math_max,
            is_async: false,
        },
        StdlibFn {
            module: "math",
            name: "math_ceil",
            available_targets: &ALL_TARGETS,
            codegen: codegen_math_ceil,
            is_async: false,
        },
        StdlibFn {
            module: "math",
            name: "math_floor",
            available_targets: &ALL_TARGETS,
            codegen: codegen_math_floor,
            is_async: false,
        },
        StdlibFn {
            module: "math",
            name: "math_round",
            available_targets: &ALL_TARGETS,
            codegen: codegen_math_round,
            is_async: false,
        },
        StdlibFn {
            module: "math",
            name: "math_random",
            available_targets: &ALL_TARGETS,
            codegen: codegen_math_random,
            is_async: false,
        },
        StdlibFn {
            module: "math",
            name: "math_sqrt",
            available_targets: &ALL_TARGETS,
            codegen: codegen_math_sqrt,
            is_async: false,
        },
        StdlibFn {
            module: "math",
            name: "math_pow",
            available_targets: &ALL_TARGETS,
            codegen: codegen_math_pow,
            is_async: false,
        },
    ]
}

pub(super) fn codegen_math_abs(args: &[String], target: Target) -> String {
    let n = &args[0];
    match target {
        Target::JavaScript
        | Target::TypeScript
        | Target::React
        | Target::Vue
        | Target::OmniView => {
            format!("Math.abs({n})")
        }
        Target::Python => format!("abs({n})"),
        Target::Rust | Target::Dioxus => format!("({n}).abs()"),
        Target::Dart | Target::Flutter => format!("({n}).abs()"),
    }
}

pub(super) fn codegen_math_min(args: &[String], target: Target) -> String {
    let a = &args[0];
    let b = &args[1];
    match target {
        Target::JavaScript
        | Target::TypeScript
        | Target::React
        | Target::Vue
        | Target::OmniView => {
            format!("Math.min({a}, {b})")
        }
        Target::Python => format!("min({a}, {b})"),
        Target::Rust | Target::Dioxus => format!("({a}).min({b})"),
        Target::Dart | Target::Flutter => format!("({a}).compareTo({b}) < 0 ? {a} : {b}"),
    }
}

pub(super) fn codegen_math_max(args: &[String], target: Target) -> String {
    let a = &args[0];
    let b = &args[1];
    match target {
        Target::JavaScript
        | Target::TypeScript
        | Target::React
        | Target::Vue
        | Target::OmniView => {
            format!("Math.max({a}, {b})")
        }
        Target::Python => format!("max({a}, {b})"),
        Target::Rust | Target::Dioxus => format!("({a}).max({b})"),
        Target::Dart | Target::Flutter => format!("({a}).compareTo({b}) > 0 ? {a} : {b}"),
    }
}

pub(super) fn codegen_math_ceil(args: &[String], target: Target) -> String {
    let n = &args[0];
    match target {
        Target::JavaScript
        | Target::TypeScript
        | Target::React
        | Target::Vue
        | Target::OmniView => {
            format!("Math.ceil({n})")
        }
        Target::Python => format!("math.ceil({n})"),
        Target::Rust | Target::Dioxus => format!("({n}).ceil()"),
        Target::Dart | Target::Flutter => format!("({n}).ceil()"),
    }
}

pub(super) fn codegen_math_floor(args: &[String], target: Target) -> String {
    let n = &args[0];
    match target {
        Target::JavaScript
        | Target::TypeScript
        | Target::React
        | Target::Vue
        | Target::OmniView => {
            format!("Math.floor({n})")
        }
        Target::Python => format!("math.floor({n})"),
        Target::Rust | Target::Dioxus => format!("({n}).floor()"),
        Target::Dart | Target::Flutter => format!("({n}).floor()"),
    }
}

pub(super) fn codegen_math_round(args: &[String], target: Target) -> String {
    let n = &args[0];
    match target {
        Target::JavaScript
        | Target::TypeScript
        | Target::React
        | Target::Vue
        | Target::OmniView => {
            format!("Math.round({n})")
        }
        Target::Python => format!("round({n})"),
        Target::Rust | Target::Dioxus => format!("({n}).round()"),
        Target::Dart | Target::Flutter => format!("({n}).round()"),
    }
}

pub(super) fn codegen_math_random(_args: &[String], target: Target) -> String {
    match target {
        Target::JavaScript
        | Target::TypeScript
        | Target::React
        | Target::Vue
        | Target::OmniView => "Math.random()".to_string(),
        Target::Python => "random.random()".to_string(),
        Target::Rust | Target::Dioxus => "rand::random::<f64>()".to_string(),
        Target::Dart | Target::Flutter => "Random().nextDouble()".to_string(),
    }
}

pub(super) fn codegen_math_sqrt(args: &[String], target: Target) -> String {
    let n = &args[0];
    match target {
        Target::JavaScript
        | Target::TypeScript
        | Target::React
        | Target::Vue
        | Target::OmniView => {
            format!("Math.sqrt({n})")
        }
        Target::Python => format!("math.sqrt({n})"),
        Target::Rust | Target::Dioxus => format!("({n}).sqrt()"),
        Target::Dart | Target::Flutter => format!("sqrt({n})"),
    }
}

pub(super) fn codegen_math_pow(args: &[String], target: Target) -> String {
    let base = &args[0];
    let exp = &args[1];
    match target {
        Target::JavaScript
        | Target::TypeScript
        | Target::React
        | Target::Vue
        | Target::OmniView => {
            format!("Math.pow({base}, {exp})")
        }
        Target::Python => format!("math.pow({base}, {exp})"),
        Target::Rust | Target::Dioxus => format!("({base}).powf({exp})"),
        Target::Dart | Target::Flutter => format!("pow({base}, {exp})"),
    }
}
