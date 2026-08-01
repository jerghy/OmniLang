use super::{StdlibFn, ALL_TARGETS};
use crate::codegen::Target;

pub(super) fn register() -> Vec<StdlibFn> {
    vec![
        StdlibFn {
            module: "math",
            name: "math_abs",
            min_args: 1,
            max_args: Some(1),
            available_targets: &ALL_TARGETS,
            codegen: codegen_math_abs,
            is_async: false,
        },
        StdlibFn {
            module: "math",
            name: "math_min",
            min_args: 2,
            max_args: Some(2),
            available_targets: &ALL_TARGETS,
            codegen: codegen_math_min,
            is_async: false,
        },
        StdlibFn {
            module: "math",
            name: "math_max",
            min_args: 2,
            max_args: Some(2),
            available_targets: &ALL_TARGETS,
            codegen: codegen_math_max,
            is_async: false,
        },
        StdlibFn {
            module: "math",
            name: "math_ceil",
            min_args: 1,
            max_args: Some(1),
            available_targets: &ALL_TARGETS,
            codegen: codegen_math_ceil,
            is_async: false,
        },
        StdlibFn {
            module: "math",
            name: "math_floor",
            min_args: 1,
            max_args: Some(1),
            available_targets: &ALL_TARGETS,
            codegen: codegen_math_floor,
            is_async: false,
        },
        StdlibFn {
            module: "math",
            name: "math_round",
            min_args: 1,
            max_args: Some(1),
            available_targets: &ALL_TARGETS,
            codegen: codegen_math_round,
            is_async: false,
        },
        StdlibFn {
            module: "math",
            name: "math_random",
            min_args: 0,
            max_args: Some(0),
            available_targets: &ALL_TARGETS,
            codegen: codegen_math_random,
            is_async: false,
        },
        StdlibFn {
            module: "math",
            name: "math_sqrt",
            min_args: 1,
            max_args: Some(1),
            available_targets: &ALL_TARGETS,
            codegen: codegen_math_sqrt,
            is_async: false,
        },
        StdlibFn {
            module: "math",
            name: "math_pow",
            min_args: 2,
            max_args: Some(2),
            available_targets: &ALL_TARGETS,
            codegen: codegen_math_pow,
            is_async: false,
        },
        StdlibFn {
            module: "math",
            name: "math_sin",
            min_args: 1,
            max_args: Some(1),
            available_targets: &ALL_TARGETS,
            codegen: codegen_math_sin,
            is_async: false,
        },
        StdlibFn {
            module: "math",
            name: "math_cos",
            min_args: 1,
            max_args: Some(1),
            available_targets: &ALL_TARGETS,
            codegen: codegen_math_cos,
            is_async: false,
        },
        StdlibFn {
            module: "math",
            name: "math_tan",
            min_args: 1,
            max_args: Some(1),
            available_targets: &ALL_TARGETS,
            codegen: codegen_math_tan,
            is_async: false,
        },
        StdlibFn {
            module: "math",
            name: "math_log",
            min_args: 1,
            max_args: Some(1),
            available_targets: &ALL_TARGETS,
            codegen: codegen_math_log,
            is_async: false,
        },
        StdlibFn {
            module: "math",
            name: "math_log2",
            min_args: 1,
            max_args: Some(1),
            available_targets: &ALL_TARGETS,
            codegen: codegen_math_log2,
            is_async: false,
        },
        StdlibFn {
            module: "math",
            name: "math_log10",
            min_args: 1,
            max_args: Some(1),
            available_targets: &ALL_TARGETS,
            codegen: codegen_math_log10,
            is_async: false,
        },
        StdlibFn {
            module: "math",
            name: "math_pi",
            min_args: 0,
            max_args: Some(0),
            available_targets: &ALL_TARGETS,
            codegen: codegen_math_pi,
            is_async: false,
        },
        StdlibFn {
            module: "math",
            name: "math_e",
            min_args: 0,
            max_args: Some(0),
            available_targets: &ALL_TARGETS,
            codegen: codegen_math_e,
            is_async: false,
        },
        StdlibFn {
            module: "math",
            name: "math_clamp",
            min_args: 3,
            max_args: Some(3),
            available_targets: &ALL_TARGETS,
            codegen: codegen_math_clamp,
            is_async: false,
        },
        StdlibFn {
            module: "math",
            name: "math_random_int",
            min_args: 2,
            max_args: Some(2),
            available_targets: &ALL_TARGETS,
            codegen: codegen_math_random_int,
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

macro_rules! define_unary {
    ($name:ident, $codegen:ident, $js:expr, $python:expr, $rust:expr, $dart:expr) => {
        pub(super) fn $codegen(args: &[String], target: Target) -> String {
            let n = &args[0];
            match target {
                Target::JavaScript
                | Target::TypeScript
                | Target::React
                | Target::Vue
                | Target::OmniView => {
                    format!($js, n = n)
                }
                Target::Python => format!($python, n = n),
                Target::Rust | Target::Dioxus => format!($rust, n = n),
                Target::Dart | Target::Flutter => format!($dart, n = n),
            }
        }
    };
}

define_unary!(
    sin,
    codegen_math_sin,
    "Math.sin({n})",
    "math.sin({n})",
    "({n}).sin()",
    "sin({n})"
);
define_unary!(
    cos,
    codegen_math_cos,
    "Math.cos({n})",
    "math.cos({n})",
    "({n}).cos()",
    "cos({n})"
);
define_unary!(
    tan,
    codegen_math_tan,
    "Math.tan({n})",
    "math.tan({n})",
    "({n}).tan()",
    "tan({n})"
);
define_unary!(
    log,
    codegen_math_log,
    "Math.log({n})",
    "math.log({n})",
    "({n}).ln()",
    "log({n})"
);
define_unary!(
    log2,
    codegen_math_log2,
    "Math.log2({n})",
    "math.log2({n})",
    "({n}).log2()",
    "log({n}) / ln2"
);
define_unary!(
    log10,
    codegen_math_log10,
    "Math.log10({n})",
    "math.log10({n})",
    "({n}).log10()",
    "log({n}) / ln10"
);

pub(super) fn codegen_math_pi(_args: &[String], target: Target) -> String {
    match target {
        Target::JavaScript
        | Target::TypeScript
        | Target::React
        | Target::Vue
        | Target::OmniView => "Math.PI".to_string(),
        Target::Python => "math.pi".to_string(),
        Target::Rust | Target::Dioxus => "std::f64::consts::PI".to_string(),
        Target::Dart | Target::Flutter => "pi".to_string(),
    }
}

pub(super) fn codegen_math_e(_args: &[String], target: Target) -> String {
    match target {
        Target::JavaScript
        | Target::TypeScript
        | Target::React
        | Target::Vue
        | Target::OmniView => "Math.E".to_string(),
        Target::Python => "math.e".to_string(),
        Target::Rust | Target::Dioxus => "std::f64::consts::E".to_string(),
        Target::Dart | Target::Flutter => "e".to_string(),
    }
}

pub(super) fn codegen_math_clamp(args: &[String], target: Target) -> String {
    let val = &args[0];
    let lo = &args[1];
    let hi = &args[2];
    match target {
        Target::JavaScript
        | Target::TypeScript
        | Target::React
        | Target::Vue
        | Target::OmniView => {
            format!("Math.min(Math.max({val}, {lo}), {hi})")
        }
        Target::Python => format!("max({lo}, min({hi}, {val}))"),
        Target::Rust | Target::Dioxus => format!("({val}).clamp({lo}, {hi})"),
        Target::Dart | Target::Flutter => format!("({val}).clamp({lo}, {hi})"),
    }
}

pub(super) fn codegen_math_random_int(args: &[String], target: Target) -> String {
    let lo = &args[0];
    let hi = &args[1];
    match target {
        Target::JavaScript
        | Target::TypeScript
        | Target::React
        | Target::Vue
        | Target::OmniView => {
            format!("Math.floor(Math.random() * ({hi} - {lo} + 1)) + {lo}")
        }
        Target::Python => format!("random.randint({lo}, {hi})"),
        Target::Rust | Target::Dioxus => {
            format!("rand::random::<i32>().rem_euclid({hi} - {lo} + 1) + {lo}")
        }
        Target::Dart | Target::Flutter => {
            format!("Random().nextInt({hi} - {lo} + 1) + {lo}")
        }
    }
}
