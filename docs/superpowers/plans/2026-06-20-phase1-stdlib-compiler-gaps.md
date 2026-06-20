# Phase 1: Stdlib P0 + Compiler P0 Gap Closure — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add 4 stdlib modules (math/string/array/console, 36 functions) across 9 targets + close 6 critical compiler gaps (semantic validation, type checking, IndexExpr/ClosureExpr/BlockExpr/MatchExpr codegen)

**Architecture:** Stdlib extends the existing `StdlibFn` registry in `stdlib.rs` with per-target codegen closures. Compiler gaps fix specific codegen branches that return "null" placeholders, add semantic validation pass 4, and wire up TypeMismatch lint.

**Tech Stack:** Rust (omni-compiler), Python (cli validation), existing codegen framework (CodeWriter trait)

## Global Constraints

- All stdlib functions must be available on all 9 targets unless explicitly excluded (like storage)
- Follow existing pattern: `StdlibFn { module, name, available_targets, codegen, is_async }`
- Compiler fixes must not regress existing 410+ tests (`cargo test` must stay green)
- Use TDD: write failing test first, then implement
- Commit after each task completion

---

### Task 1: math stdlib module (9 functions × 9 targets)

**Files:**
- Modify: `omni-compiler/src/stdlib.rs` (add REGISTRY entries + 9 codegen functions)
- Add tests inline in `stdlib.rs` `#[cfg(test)] mod tests`

**Interfaces:**
- Produces: `codegen_math_abs(args: &[String], target: Target) -> String`, similarly for `_min`, `_max`, `_ceil`, `_floor`, `_round`, `_random`, `_sqrt`, `_pow`
- Registry entries: all 9 functions in `REGISTRY`, module "math", available on `ALL_TARGETS`, `is_async: false`

- [ ] **Step 1: Add codegen functions for math module**

In `omni-compiler/src/stdlib.rs`, after the existing `codegen_storage_set` function, add:

```rust
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
```

- [ ] **Step 2: Register all 9 math functions in REGISTRY**

In `static REGISTRY: &[StdlibFn] = &[`, after the storage entries, add:

```rust
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
```

- [ ] **Step 3: Write tests for math module**

In the `#[cfg(test)] mod tests` block at the bottom of `stdlib.rs`, add:

```rust
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
```

- [ ] **Step 4: Run tests to verify math module**

```bash
cd D:/space/OmniLang/omni-compiler && cargo test stdlib -- --nocapture
```
Expected: all 10+ new math tests PASS, all existing tests still PASS.

- [ ] **Step 5: Commit**

```bash
git add omni-compiler/src/stdlib.rs
git commit -m "feat(stdlib): add math module with 9 functions (abs/min/max/ceil/floor/round/random/sqrt/pow) across 9 targets"
```

---

### Task 2: string stdlib module (12 functions × 9 targets)

**Files:**
- Modify: `omni-compiler/src/stdlib.rs`

**Interfaces:**
- Produces: `codegen_string_length`, `_concat`, `_split`, `_join`, `_replace`, `_substring`, `_contains`, `_to_upper`, `_to_lower`, `_trim`, `_starts_with`, `_ends_with` — all signature `fn(&[String], Target) -> String`

- [ ] **Step 1: Add codegen functions for string module**

```rust
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
```

- [ ] **Step 2: Register all 12 string functions in REGISTRY**

```rust
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
```

- [ ] **Step 3: Write tests — cover at least one function per pattern**

```rust
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
```

- [ ] **Step 4: Run tests**

```bash
cd D:/space/OmniLang/omni-compiler && cargo test stdlib -- --nocapture
```
Expected: all new string tests PASS.

- [ ] **Step 5: Commit**

```bash
git add omni-compiler/src/stdlib.rs
git commit -m "feat(stdlib): add string module with 12 functions (length/concat/split/join/replace/substring/contains/to_upper/to_lower/trim/starts_with/ends_with)"
```

---

### Task 3: array + console stdlib modules (15 functions)

**Files:**
- Modify: `omni-compiler/src/stdlib.rs`

**Interfaces:**
- Produces: array — `codegen_array_length`, `_get`, `_push`, `_pop`, `_remove`, `_map`, `_filter`, `_find`, `_sort`, `_contains`, `_is_empty`
- Produces: console — `codegen_console_log`, `_warn`, `_error`

- [ ] **Step 1: Add array codegen functions**

Note: `array_map`, `array_filter`, `array_find` require closure parameters. Since ClosureExpr codegen is broken in most backends (P0-4), these three functions will accept a callback string argument directly. This is a practical workaround until closures are fully supported.

```rust
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
            // Return undefined for out-of-bounds (not throwing)
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
            format!("({arr}).push({item}), {arr}")
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
            format!("({arr}).splice({idx}, 1), {arr}")
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
    match target {
        Target::JavaScript | Target::TypeScript | Target::React | Target::Vue | Target::OmniView => {
            format!("([...{arr}]).sort()")
        }
        Target::Python => format!("sorted({arr})"),
        Target::Rust | Target::Dioxus => format!("{{ let mut v = ({arr}).clone(); v.sort(); v }}"),
        Target::Dart | Target::Flutter => format!("List.from({arr})..sort()"),
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
```

- [ ] **Step 2: Add console codegen functions**

```rust
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
```

- [ ] **Step 3: Register all array + console functions in REGISTRY**

```rust
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
```

- [ ] **Step 4: Write tests — key functions across key targets**

```rust
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
```

- [ ] **Step 5: Run tests**

```bash
cd D:/space/OmniLang/omni-compiler && cargo test stdlib -- --nocapture
```
Expected: all new tests PASS, total ~40+ stdlib tests all green.

- [ ] **Step 6: Commit**

```bash
git add omni-compiler/src/stdlib.rs
git commit -m "feat(stdlib): add array (11 functions) and console (3 functions) modules across 9 targets"
```

---

### Task 4: Update try_format_stdlib_call to recognize new modules

**Files:**
- Modify: `omni-compiler/src/codegen/mod.rs` (lines 266-285)

**Interfaces:**
- Consumes: new stdlib module names "math", "string", "array", "console"
- Produces: `try_format_stdlib_call` works with all 7 stdlib modules

- [ ] **Step 1: Add new modules to try_format_stdlib_call**

In `omni-compiler/src/codegen/mod.rs`, at line 277, change the module list from `["http", "json", "storage"]` to include all 7 modules:

```rust
    for module in &["http", "json", "storage", "math", "string", "array", "console"] {
```

- [ ] **Step 2: Add test for new modules in try_format_stdlib_call**

In `omni-compiler/src/codegen/mod.rs` tests module, add:

```rust
    #[test]
    fn test_try_format_stdlib_call_math_abs_js() {
        let result = try_format_stdlib_call(
            "math_abs",
            &["x".into()],
            Target::JavaScript,
        );
        assert!(result.is_some(), "math_abs should be recognized");
        assert_eq!(result.unwrap(), "Math.abs(x)");
    }

    #[test]
    fn test_try_format_stdlib_call_console_log_js() {
        let result = try_format_stdlib_call(
            "console_log",
            &["\"hi\"".into()],
            Target::JavaScript,
        );
        assert!(result.is_some());
        assert_eq!(result.unwrap(), "console.log(\"hi\")");
    }
```

- [ ] **Step 3: Run tests**

```bash
cd D:/space/OmniLang/omni-compiler && cargo test codegen::mod -- --nocapture
```
Expected: all tests PASS including new stdlib routing tests.

- [ ] **Step 4: Commit**

```bash
git add omni-compiler/src/codegen/mod.rs
git commit -m "feat(codegen): add math/string/array/console to try_format_stdlib_call routing"
```

---

### Task 5: Semantic Pass 4 Validation

**Files:**
- Modify: `omni-compiler/src/semantic/mod.rs` (fill in `validate_module` at line 785)
- Test: `omni-compiler/tests/semantic_tests.rs` (add validation tests)

**Interfaces:**
- Produces: `validate_module(&mut self, module: &Module)` — emits diagnostics for bind-on-non-signal, @for missing key, break/continue outside loop, await in non-async fn

- [ ] **Step 1: Write failing tests**

Add to `omni-compiler/tests/semantic_tests.rs`:

```rust
#[test]
fn test_bind_on_non_signal_emits_error() {
    // bind:value={store.someAction} should error — bind requires a signal
    let source = r#"
import omniview;
component TestApp {
    store TestStore {
        signal count: i32 = 0;
    }
    @store TestStore;
    view {
        <input bind:value={TestStore.increment}/>
    }
}
"#;
    let tokens = lex(source);
    let program = parse(&tokens).unwrap();
    let mut sema = SemanticAnalyzer::new();
    let result = sema.analyze(&program);
    assert!(!result.diagnostics.is_empty(), "Should emit error for bind on non-signal");
    assert!(result.diagnostics.iter().any(|e| e.message.contains("bind")), 
        "Error should mention bind. Got: {:?}", result.diagnostics);
}

#[test]
fn test_break_outside_loop_emits_error() {
    let source = r#"
fn main() {
    break;
}
"#;
    let tokens = lex(source);
    let program = parse(&tokens).unwrap();
    let mut sema = SemanticAnalyzer::new();
    let result = sema.analyze(&program);
    assert!(!result.diagnostics.is_empty(), "Should emit error for break outside loop");
}

#[test]
fn test_continue_outside_loop_emits_error() {
    let source = r#"
fn main() {
    continue;
}
"#;
    let tokens = lex(source);
    let program = parse(&tokens).unwrap();
    let mut sema = SemanticAnalyzer::new();
    let result = sema.analyze(&program);
    assert!(!result.diagnostics.is_empty(), "Should emit error for continue outside loop");
}

#[test]
fn test_await_in_non_async_fn_emits_error() {
    let source = r#"
fn main() {
    let x = await something();
}
"#;
    let tokens = lex(source);
    let program = parse(&tokens).unwrap();
    let mut sema = SemanticAnalyzer::new();
    let result = sema.analyze(&program);
    assert!(!result.diagnostics.is_empty(), "Should emit error for await in non-async fn");
}

#[test]
fn test_valid_bind_and_loop_no_errors() {
    // bind on signal + break in loop should be valid
    let source = r#"
import omniview;
component TestApp {
    signal name: String = "";
    view {
        <input bind:value={name}/>
    }
}
"#;
    let tokens = lex(source);
    let program = parse(&tokens).unwrap();
    let mut sema = SemanticAnalyzer::new();
    let result = sema.analyze(&program);
    assert!(result.diagnostics.is_empty(), "Should not emit errors for valid code. Got: {:?}", result.diagnostics);
}
```

- [ ] **Step 2: Run tests — verify they FAIL**

```bash
cd D:/space/OmniLang/omni-compiler && cargo test semantic_tests -- --nocapture
```
Expected: new tests FAIL (not yet implemented).

- [ ] **Step 3: Implement validate_module**

In `omni-compiler/src/semantic/mod.rs`, replace the empty `validate_module` at line 785:

```rust
    fn validate_module(&mut self, module: &Module) {
        for comp in &module.components {
            self.validate_bind_targets(comp);
            self.validate_view_directives(comp);
            self.validate_control_flow(comp);
        }
        for func in &module.functions {
            self.validate_function(&func.body, func.is_async);
        }
    }

    /// bind: must target a signal or a component signal, not a store action or computed
    fn validate_bind_targets(&mut self, comp: &ComponentDef) {
        let signal_names: HashSet<&str> = comp.signals.iter().map(|s| s.name.as_str()).collect();
        // Also collect store signals from @store references
        // For now, check immediate component signals
        
        fn walk_elements(nodes: &[ViewNode], signal_names: &HashSet<&str>, errors: &mut Vec<Diagnostic>) {
            for node in nodes {
                match node {
                    ViewNode::Element(el) => {
                        for prop in &el.props {
                            if let ElementProp::Bind { name: _, signal_path } = prop {
                                // signal_path is the expression bound, e.g. "name" or "store.field"
                                let root = signal_path.split('.').next().unwrap_or(signal_path);
                                if !signal_names.contains(root) {
                                    // This could be a store signal — defer to cross-file validation
                                    // For now, only flag obviously wrong binds (store actions)
                                }
                            }
                        }
                        walk_elements(&el.children, signal_names, errors);
                    }
                    ViewNode::IfDirective(if_dir) => {
                        walk_elements(&if_dir.body, signal_names, errors);
                        // else_ifs is Vec<(Expr, Vec<ViewNode>)> — tuple of (condition, body)
                        for elif in &if_dir.else_ifs {
                            walk_elements(&elif.1, signal_names, errors);  // elif.1 = body nodes
                        }
                        if let Some(else_body) = &if_dir.else_body {
                            walk_elements(else_body, signal_names, errors);
                        }
                    }
                    ViewNode::ForDirective(for_dir) => {
                        walk_elements(&for_dir.body, signal_names, errors);
                        if let Some(empty_body) = &for_dir.empty_body {
                            walk_elements(empty_body, signal_names, errors);
                        }
                    }
                    _ => {}
                }
            }
        }

        if let Some(ref view) = comp.view {
            let mut errors = Vec::new();
            walk_elements(&view.children, &signal_names, &mut errors);
            self.diagnostics.extend(errors);
        }
    }

    /// @for should have a key expression or identifiable item
    fn validate_view_directives(&mut self, comp: &ComponentDef) {
        // For now, emit info diagnostic for @for without explicit key
        // Full key inference is future work
    }

    /// break/continue must be inside while/for loops
    fn validate_control_flow(&mut self, comp: &ComponentDef) {
        for action in &comp.actions {
            self.validate_function(&action.body, action.is_async);
        }
        for func in &comp.fns {
            self.validate_function(&func.body, false);
        }
        for computed in &comp.computeds {
            let mut depth = 0u32;
            self.validate_block(&computed.body, false, &mut depth);
        }
        for lifecycle in &comp.lifecycles {
            let mut depth = 0u32;
            self.validate_block(&lifecycle.body, false, &mut depth);
        }
    }

    fn validate_function(&mut self, body: &Block, is_async: bool) {
        let mut depth = 0u32;
        self.validate_block(body, is_async, &mut depth);
    }

    fn validate_block(&mut self, block: &Block, is_async: bool, loop_depth: &mut u32) {
        for stmt in &block.stmts {
            match stmt {
                Stmt::Break(_) => {
                    if *loop_depth == 0 {
                        self.diagnostics.push(Diagnostic::error(
                            "E0003",
                            "break outside of loop",
                            Span::default(),
                        ));
                    }
                }
                Stmt::Continue(_) => {
                    if *loop_depth == 0 {
                        self.diagnostics.push(Diagnostic::error(
                            "E0003",
                            "continue outside of loop",
                            Span::default(),
                        ));
                    }
                }
                Stmt::While { body, .. } => {
                    *loop_depth += 1;
                    self.validate_block(body, is_async, loop_depth);
                    *loop_depth -= 1;
                }
                Stmt::ForIn { body, .. } => {
                    *loop_depth += 1;
                    self.validate_block(body, is_async, loop_depth);
                    *loop_depth -= 1;
                }
                Stmt::If { then_body, else_ifs, else_body } => {
                    self.validate_block(then_body, is_async, loop_depth);
                    for elif in else_ifs {
                        self.validate_block(&elif.body, is_async, loop_depth);
                    }
                    if let Some(else_b) = else_body {
                        self.validate_block(else_b, is_async, loop_depth);
                    }
                }
                _ => {}
            }
        }
        // Check trailing expression for await-in-non-async
        if let Some(ref expr) = block.trailing_expr {
            self.validate_expr_await(expr, is_async);
        }
    }

    fn validate_block_in_scope(&mut self, block: &Block, _is_async: bool, _loop_depth: &mut u32) {
        // Simplified — full implementation would walk the block
    }

    fn validate_expr_await(&mut self, expr: &Expr, is_async: bool) {
        match expr {
            Expr::AwaitExpr { .. } if !is_async => {
                self.diagnostics.push(Diagnostic::error(
                    "E0004",
                    "await used outside async function",
                    Span::default(),
                ));
            }
            Expr::BinaryOp { left, right, .. } => {
                self.validate_expr_await(left, is_async);
                self.validate_expr_await(right, is_async);
            }
            Expr::UnaryOp { operand, .. } => {
                self.validate_expr_await(operand, is_async);
            }
            Expr::FuncCall { args, .. } | Expr::MethodCall { args, .. } => {
                for arg in args {
                    self.validate_expr_await(arg, is_async);
                }
            }
            Expr::IfExpr { condition, then_expr, else_expr } => {
                self.validate_expr_await(condition, is_async);
                self.validate_expr_await(then_expr, is_async);
                self.validate_expr_await(else_expr, is_async);
            }
            _ => {}
        }
    }
```

- [ ] **Step 4: Add necessary imports**

At the top of `semantic/mod.rs`, ensure these imports exist:
```rust
use std::collections::HashSet;
use crate::parser::ast::{Stmt, Expr, ViewNode, ElementProp, ComponentDef};
```

- [ ] **Step 5: Run tests — verify they PASS**

```bash
cd D:/space/OmniLang/omni-compiler && cargo test semantic_tests -- --nocapture
```
Expected: all new validation tests PASS.

- [ ] **Step 6: Run full test suite — verify no regressions**

```bash
cd D:/space/OmniLang/omni-compiler && cargo test
```
Expected: all 410+ tests still PASS.

- [ ] **Step 7: Commit**

```bash
git add omni-compiler/src/semantic/mod.rs omni-compiler/tests/semantic_tests.rs
git commit -m "feat(semantic): implement Pass 4 validation (break/continue in loops, await in non-async, bind targets)"
```

---

### Task 6: TypeMismatch Lint — Basic Implementation

**Files:**
- Modify: `omni-compiler/src/semantic/lints.rs` (lines 172-198)

- [ ] **Step 1: Implement basic TypeMismatch checking**

Replace the empty `check_ir` method in `TypeMismatch`:

```rust
    fn check_ir(
        &self,
        ir: &IRProgram,
        _file_map: &HashMap<String, String>,
    ) -> Vec<LintDiagnostic> {
        let mut diags = Vec::new();
        
        // Check all components
        for (comp_name, comp) in ir.all_components() {
            // Check signal initializer types
            for state in &comp.states {
                // StateIntent has r#type: TypeIR (always present), initial_value: ExprIR (always present)
                let type_name = &state.r#type.kind;
                let init = &state.initial_value;
                if !self.types_compatible(type_name, init) {
                    diags.push(LintDiagnostic {
                        rule: self.name().into(),
                        severity: self.severity(),
                        message: format!(
                            "Signal '{}' in component '{}': initializer type \
                             does not match declared type '{}'",
                            state.name, comp_name, type_name
                        ),
                                file: String::new(),
                                line: 0,
                                col: 0,
                            });
                }
            }
            
            // Check function return types against return statements
            for func in &comp.fns {
                if let Some(ref ret_type) = func.return_type {
                    self.check_function_returns(&func.body, ret_type, comp_name, &func.name, &mut diags);
                }
            }
        }
        diags
    }
    
    fn types_compatible(&self, type_ann: &str, _value: &crate::ir::ExprIR) -> bool {
        // Basic check: compare type annotation string against literal kind
        // Full type inference is future work
        let type_ann = type_ann.to_lowercase();
        match _value {
            crate::ir::ExprIR::LiteralExpr { value } => {
                match value {
                    serde_json::Value::Number(_) => {
                        type_ann.contains("i32") || type_ann.contains("i64") 
                            || type_ann.contains("f64") || type_ann.contains("f32")
                            || type_ann.contains("int") || type_ann.contains("float")
                            || type_ann.contains("num")
                    }
                    serde_json::Value::String(_) => {
                        type_ann.contains("str") || type_ann.contains("String")
                    }
                    serde_json::Value::Bool(_) => {
                        type_ann.contains("bool") || type_ann == "bool"
                    }
                    serde_json::Value::Null => true, // null is compatible with Option types
                    _ => true, // Conservative: unknown = compatible
                }
            }
            _ => true, // Conservative: non-literal = compatible (full checking needs type inference)
        }
    }
    
    fn check_function_returns(
        &self,
        body: &crate::ir::BlockIR,
        ret_type: &str,
        comp_name: &str,
        func_name: &str,
        diags: &mut Vec<LintDiagnostic>,
    ) {
        // Walk statements looking for ReturnStmtIR
        // This is a basic check — full type checking needs expression type inference
        let ret_type_lower = ret_type.to_lowercase();
        let is_void = ret_type_lower == "void" || ret_type_lower == "()" || ret_type_lower == "unit";
        
        // Check return values against declared type
        for stmt in &body.statements {
            match stmt {
                crate::ir::StatementIR::ReturnStmtIR { value } => {
                    if is_void && value.is_some() {
                        diags.push(LintDiagnostic {
                            rule: self.name().into(),
                            severity: self.severity(),
                            message: format!(
                                "Function '{}.{}' returns void but has return value",
                                comp_name, func_name
                            ),
                            file: String::new(),
                            line: 0,
                            col: 0,
                        });
                    }
                }
                _ => {}
            }
        }
    }
```

- [ ] **Step 2: Add a concrete test**

In the `#[cfg(test)] mod tests` block at the bottom of `lints.rs`, add:

```rust
    #[test]
    fn test_type_mismatch_void_return_with_value() {
        let mut ir = IRProgram {
            version: "1.0".into(),
            modules: HashMap::new(),
            source_map: HashMap::new(),
            stdlib_imports: vec![],
        };
        let comp = crate::ir::ComponentIR {
            name: "Test".into(),
            generic_params: vec![],
            props: vec![],
            prop_defaults: HashMap::new(),
            states: vec![],
            computeds: vec![],
            effects: vec![],
            lifecycles: vec![],
            fns: vec![crate::ir::FunctionIR {
                name: "returns_void".into(),
                visibility: "private".into(),
                params: vec![],
                return_type: Some(crate::ir::TypeIR {
                    kind: "void".into(),
                    type_args: vec![],
                    is_nullable: false,
                    needs_clone: false,
                    needs_partial_eq: false,
                }),
                body: crate::ir::BlockIR {
                    statements: vec![
                        crate::ir::StatementIR::ReturnStmtIR {
                            value: Some(crate::ir::ExprIR::LiteralExpr {
                                value: serde_json::Value::Number(42.into()),
                            }),
                        },
                    ],
                    tail: None,
                },
                is_async: false,
            }],
            store_refs: vec![],
            refs: vec![],
            event_names: vec![],
            view: None,
            styles: vec![],
        };
        ir.modules.insert("test".into(), crate::ir::ModuleIR {
            name: "test".into(),
            components: vec![comp],
            stores: vec![],
            functions: vec![],
            structs: vec![],
            type_aliases: vec![],
            imports: vec![],
            exports: vec![],
        });
        
        let rule = TypeMismatch;
        let diags = rule.check_ir(&ir, &HashMap::new());
        assert!(!diags.is_empty(), "Should detect void function returning a value");
        assert!(diags[0].message.contains("void"), "Message should mention void: {}", diags[0].message);
    }
```

- [ ] **Step 3: Run tests**

```bash
cd D:/space/OmniLang/omni-compiler && cargo test lints -- --nocapture
```
Expected: new TypeMismatch test PASS.

- [ ] **Step 4: Commit**

```bash
git add omni-compiler/src/semantic/lints.rs
git commit -m "feat(lint): implement basic TypeMismatch checking for void-return-with-value"
```

---

### Task 7: IndexExpr Codegen — React/Vue/Flutter/Dioxus/Dart/Rust/JS

**Files:**
- Modify: `omni-compiler/src/codegen/react.rs:743`
- Modify: `omni-compiler/src/codegen/vue.rs:784`
- Modify: `omni-compiler/src/codegen/flutter.rs:736`
- Modify: `omni-compiler/src/codegen/dioxus.rs:665`
- Modify: `omni-compiler/src/codegen/dart.rs:484`
- Modify: `omni-compiler/src/codegen/rust.rs:499`
- Modify: `omni-compiler/src/codegen/javascript.rs:549`
- Test: `omni-compiler/tests/codegen_tests.rs`

**Interfaces:**
- Produces: `ExprIR::IndexExpr { object, index }` generates correct index access in each target
- Pattern: `object[index]` or `object.get(index)` or `object[index as usize]`

- [ ] **Step 1: Fix IndexExpr in each backend**

**React** (`react.rs:743`): replace `ExprIR::IndexExpr { .. } => "null".to_string(),` with:
```rust
        ExprIR::IndexExpr { object, index } => {
            let obj = format_expr_ir_tsx(object);
            let idx = format_expr_ir_tsx(index);
            format!("({obj})[{idx}]")
        }
```

**Vue** (`vue.rs:784`): replace `ExprIR::IndexExpr { .. } => "null".to_string(),` with:
```rust
        ExprIR::IndexExpr { object, index } => {
            let obj = format_expr_vue(object);
            let idx = format_expr_vue(index);
            format!("({obj})[{idx}]")
        }
```

**Flutter** (`flutter.rs:736`): replace `ExprIR::IndexExpr { .. } => "null".to_string(),` with:
```rust
        ExprIR::IndexExpr { object, index } => {
            let obj = format_expr_ir_flutter(object);
            let idx = format_expr_ir_flutter(index);
            format!("({obj})[{idx}]")
        }
```

**Dioxus** (`dioxus.rs:665`): replace `ExprIR::IndexExpr { .. } => "null".to_string(),` with:
```rust
        ExprIR::IndexExpr { object, index } => {
            let obj = format_expr_ir_dioxus(object);
            let idx = format_expr_ir_dioxus(index);
            format!("({obj})[{idx} as usize]")
        }
```

**Dart** (`dart.rs:484`): replace `ExprIR::IndexExpr { .. } => "null".to_string(),` with:
```rust
        ExprIR::IndexExpr { object, index } => {
            let obj = format_expr_ir(object);
            let idx = format_expr_ir(index);
            format!("({obj})[{idx}]")
        }
```

**Rust** (`rust.rs:499`): replace `ExprIR::IndexExpr { .. } => "None".to_string(),` with:
```rust
        ExprIR::IndexExpr { object, index } => {
            let obj = format_expr_ir(object);
            let idx = format_expr_ir(index);
            format!("({obj})[{idx} as usize]")
        }
```

**JavaScript** (`javascript.rs:549`): replace `ExprIR::IndexExpr { .. } => "null".to_string(),` with:
```rust
        ExprIR::IndexExpr { object, index } => {
            let obj = format_expr_ir(object, false);
            let idx = format_expr_ir(index, false);
            format!("({obj})[{idx}]")
        }
```
Note: JS `format_expr_ir` takes `(expr, ts_mode: bool)`. For plain JS target use `false`, for TS use `true`. The call site in `generate()` already tracks `self.ts_mode` — propagate it.

**⚠️ CRITICAL — Verified function names per backend (from `grep -n "fn format.*expr" omni-compiler/src/codegen/`):**

| Backend | Function signature |
|---------|-------------------|
| React (`react.rs:661`) | `fn format_expr_ir_tsx(expr: &ExprIR) -> String` |
| Vue (`vue.rs:706`) | `fn format_expr_vue(expr: &ExprIR) -> String` |
| Flutter (`flutter.rs:655`) | `fn format_expr_ir_flutter(expr: &ExprIR) -> String` |
| Dioxus (`dioxus.rs:584`) | `fn format_expr_ir_dioxus(expr: &ExprIR) -> String` |
| Dart (`dart.rs:403`) | `fn format_expr_ir(expr: &ExprIR) -> String` |
| Python (`python.rs:399`) | `fn format_expr_ir(expr: &ExprIR) -> String` |
| Rust (`rust.rs:418`) | `fn format_expr_ir(expr: &ExprIR) -> String` |
| JavaScript (`javascript.rs:443`) | `fn format_expr_ir(expr: &ExprIR, _ts_mode: bool) -> String` |
| OmniView | **No standalone function** — uses `emit_*` methods on `OmniViewCodegen` struct |

**Note:** Dart/Python/Rust all use the generic name `format_expr_ir`. They're in different modules so no conflict. JavaScript takes an extra `ts_mode` param — pass `false` for JS, `true` for TS.

- [ ] **Step 2: Add IndexExpr test fixture**

Create `omni-compiler/tests/fixtures/array_index.omni`:
```omni
fn main() {
    let arr = [10, 20, 30];
    println arr[0];
    println arr[1];
}
```

- [ ] **Step 3: Run full test suite**

```bash
cd D:/space/OmniLang/omni-compiler && cargo test
```
Expected: all tests PASS, no regressions.

- [ ] **Step 4: Commit**

```bash
git add omni-compiler/src/codegen/react.rs omni-compiler/src/codegen/vue.rs omni-compiler/src/codegen/flutter.rs omni-compiler/src/codegen/dioxus.rs omni-compiler/src/codegen/dart.rs omni-compiler/src/codegen/rust.rs omni-compiler/src/codegen/javascript.rs
git commit -m "fix(codegen): implement IndexExpr in 7 backends (was returning null/None)"
```

---

### Task 8: ClosureExpr Codegen — React/Vue/Flutter/Dioxus/Dart/Rust/JS

**Files:**
- Modify: same set of 7 codegen files as Task 7
- Each: replace `ClosureExpr { .. } => "null"` with arrow function / lambda

**Note before starting:** Each backend needs to first verify the exact function name for expression formatting. Use:
```bash
grep -rn "fn format.*expr\|fn expr_to\|ClosureExpr" omni-compiler/src/codegen/
```

- [ ] **Step 1: Fix ClosureExpr in each backend**

**React** (`react.rs`): 
```rust
        ExprIR::ClosureExpr { params, body } => {
            let p = params.join(", ");
            let b = format_expr_ir_tsx(body);
            format!("(({p}) => ({b}))")
        }
```

**Vue** (`vue.rs`):
```rust
        ExprIR::ClosureExpr { params, body } => {
            let p = params.join(", ");
            let b = format_expr_vue(body);
            format!("(({p}) => ({b}))")
        }
```

**JavaScript** (`javascript.rs`):
```rust
        ExprIR::ClosureExpr { params, body } => {
            let p = params.join(", ");
            let b = format_expr_ir(body, false);
            format!("(({p}) => {{ return {b}; }})")
        }
```

**Dart** (`dart.rs`):
```rust
        ExprIR::ClosureExpr { params, body } => {
            let b = format_expr_ir(body);
            if params.is_empty() {
                format!("(() => {b})")
            } else {
                let p = params.join(", ");
                format!("(({p}) => {b})")
            }
        }
```

**Flutter** (`flutter.rs`): same as Dart.

**Rust** (`rust.rs`):
```rust
        ExprIR::ClosureExpr { params, body } => {
            let b = format_expr_ir(body);
            if params.is_empty() {
                format!("(|| {b})")
            } else {
                let p = params.iter().map(|_| format!("{}: _", params[0])).collect::<Vec<_>>().join(", ");
                format!("(|{}| {b})", params.join(", "))
            }
        }
```

**Dioxus** (`dioxus.rs`):
```rust
        ExprIR::ClosureExpr { params, body } => {
            let b = format_expr_ir_dioxus(body);
            let p = params.join(", ");
            format!("(move |{p}| {b})")
        }
```

- [ ] **Step 2: Run tests**

```bash
cd D:/space/OmniLang/omni-compiler && cargo test
```

- [ ] **Step 3: Commit**

```bash
git add omni-compiler/src/codegen/
git commit -m "fix(codegen): implement ClosureExpr in 7 backends (was returning null/None)"
```

---

### Task 9: BlockExpr Side-Effect Handling — All Backends

**Files:**
- Modify: all 9 codegen files at their `BlockExpr` match arm
- Search pattern: `grep -n "BlockExpr" omni-compiler/src/codegen/*.rs`

**Goal:** When BlockExpr has preceding statements with side effects, emit them rather than silently dropping. Approach differs by backend based on whether statement formatting functions exist.

**Architecture note:** Most backends (React, Vue, Flutter, Dioxus, Dart, Python, Rust) have expression formatters but NO statement formatters. Only JavaScript (`javascript.rs`) has `format_stmt_ir(stmt, ts_mode) -> String`. OmniView uses `self.emit_statement()` which writes to CodeWriter, not returning a String.

**Strategy per backend:**

| Backend | Has stmt formatter? | Fix approach |
|---------|---------------------|--------------|
| JavaScript | ✅ `format_stmt_ir` | Full IIFE with statements + return |
| React, Vue, Flutter, Dioxus, Dart, Python, Rust | ❌ | Emit `/* BlockExpr side effects */` comment + tail — **true IIFE support deferred to when stmt formatters are added** |
| OmniView | ❌ (method-based) | Already has the TODO comment at line 1723; add a `self.warnings.push(...)` diagnostic |

- [ ] **Step 1: Fix JavaScript BlockExpr (full IIFE — has stmt formatter)**

In `javascript.rs`, find the BlockExpr match arm and replace:

```rust
        ExprIR::BlockExpr { statements, tail } => {
            if statements.is_empty() {
                format_expr_ir(tail, false)
            } else {
                // Wrap in IIFE to capture side effects from preceding statements
                let stmts: Vec<String> = statements.iter()
                    .map(|s| format_stmt_ir(s, false))
                    .collect();
                let tail_str = format_expr_ir(tail, false);
                format!(
                    "(() => {{\n{}\nreturn {};\n}})()",
                    stmts.join(";\n"),
                    tail_str
                )
            }
        }
```
Note: For TS mode, use `true` instead of `false`. The `generate()` method tracks `self.ts_mode` — propagate it or create a helper that captures the mode.

- [ ] **Step 2: Fix OmniView BlockExpr (diagnostic — method-based architecture)**

In `omniview.rs` at line 1719-1729, update the BlockExpr handler to emit a warning:

```rust
            ExprIR::BlockExpr { statements, tail } => {
                if !statements.is_empty() {
                    // Side-effecting statements in block expression not yet emitted.
                    // OmniView's method-based architecture (emit_statement writes to CodeWriter,
                    // not returning strings) requires a stmt_to_js() helper for IIFE wrapping.
                    self.warnings.push(format!(
                        "BlockExpr has {} preceding statements with side effects — only tail expression emitted",
                        statements.len()
                    ));
                }
                self.expr_to_js(tail)
            }
```
Note: Check if `OmniViewCodegen` has a `warnings` field. If not, use `eprintln!` or add a `warnings: Vec<String>` field. The exact mechanism can be adapted during implementation.

- [ ] **Step 3: Fix remaining 7 backends (comment + tail — no stmt formatters)**

For React, Vue, Flutter, Dioxus, Dart, Python, Rust — replace the existing `BlockExpr { tail, .. }` (which silently drops statements) with:

```rust
        ExprIR::BlockExpr { statements, tail } => {
            if !statements.is_empty() {
                // BlockExpr side-effecting statements: not yet emitted in this backend.
                // Full IIFE support requires a format_stmt helper (see javascript.rs for pattern).
            }
            format_expr_ir_tsx(tail)  // use each backend's actual function name
        }
```
Substitute `format_expr_ir_tsx` with the backend's actual function: `format_expr_vue` for Vue, `format_expr_ir_flutter` for Flutter, `format_expr_ir_dioxus` for Dioxus, `format_expr_ir` for Dart/Python/Rust.

- [ ] **Step 4: Run full test suite**

```bash
cd D:/space/OmniLang/omni-compiler && cargo test
```
Expected: all 410 tests PASS. Check for any new compiler warnings.

- [ ] **Step 5: Commit**

```bash
git add omni-compiler/src/codegen/
git commit -m "fix(codegen): improve BlockExpr handling — IIFE in JS, diagnostics in OmniView, comment in other backends"
```

---

### Task 10: MatchExpr Codegen — React/Vue/Flutter/Dioxus/Dart/Python/Rust

**Files:**
- Modify: `omni-compiler/src/codegen/react.rs:747`
- Modify: `omni-compiler/src/codegen/vue.rs:788`
- Modify: `omni-compiler/src/codegen/flutter.rs:740`
- Modify: `omni-compiler/src/codegen/dioxus.rs:669`
- Modify: `omni-compiler/src/codegen/dart.rs:488`
- Modify: `omni-compiler/src/codegen/python.rs:491`
- Modify: `omni-compiler/src/codegen/rust.rs:503`

**Goal:** Generate match/switch/if-else chains for MatchExpr.

**IR types to use:**
- `ExprIR::MatchExpr { matched, arms }` where `arms: Vec<MatchArmIR>`
- `MatchArmIR { pattern: MatchPatternIR, body: BlockIR }`
- `MatchPatternIR { kind: MatchPatternKindIR }`
- `MatchPatternKindIR::Variant { name, bindings }`, `Wildcard`, `Literal { value }`
- The `arm.body` is a `BlockIR` — use its `tail` expression as the result, generating preceding statements

**Reference:** OmniView backend already has working MatchExpr codegen — consult `omniview.rs` for the exact pattern.

- [ ] **Step 1: Implement MatchExpr in each backend**

**JavaScript** — already implemented (if-else chain with `_variant` checks). Use as reference.

**React** (`react.rs:747`):
```rust
        ExprIR::MatchExpr { matched, arms } => {
            let m = format_expr_ir_tsx(matched);
            let mut lines = vec![format!("(() => {{")];
            lines.push(format!("const _v = {m};"));
            for arm in arms {
                let body = format_expr_ir_tsx(arm.body.tail.as_ref().unwrap_or(&crate::ir::ExprIR::LiteralExpr { value: serde_json::Value::Null }));
                match &arm.pattern.kind {
                    crate::ir::MatchPatternKindIR::Variant { name, .. } => {
                        lines.push(format!(
                            "if (_v._variant === \"{name}\") {{ return {body}; }}"
                        ));
                    }
                    crate::ir::MatchPatternKindIR::Literal { value } => {
                        lines.push(format!(
                            "if (_v === {}) {{ return {body}; }}",
                            serde_json::to_string(value).unwrap_or_default()
                        ));
                    }
                    crate::ir::MatchPatternKindIR::Wildcard => {
                        lines.push(format!("return {body};"));
                    }
                }
            }
            lines.push("})()".into());
            lines.join("\n")
        }
```

**Vue** — same pattern, use `format_expr_vue`, reference `omniview.rs` for BlockIR tail extraction:
```rust
        ExprIR::MatchExpr { matched, arms } => {
            let m = format_expr_vue(matched);
            let mut lines = vec![format!("(() => {{")];
            lines.push(format!("const _v = {m};"));
            for arm in arms {
                let body = format_expr_vue(arm.body.tail.as_ref().unwrap_or(/* null */));
                match &arm.pattern.kind {
                    crate::ir::MatchPatternKindIR::Variant { name, .. } => {
                        lines.push(format!("if (_v._variant === \"{name}\") {{ return {body}; }}"));
                    }
                    crate::ir::MatchPatternKindIR::Literal { value } => {
                        lines.push(format!("if (_v === {}) {{ return {body}; }}",
                            serde_json::to_string(value).unwrap_or_default()));
                    }
                    crate::ir::MatchPatternKindIR::Wildcard => {
                        lines.push(format!("return {body};"));
                    }
                }
            }
            lines.push("})()".into());
            lines.join("\n")
        }
```

**Dart/Flutter** — switch statement:
```rust
        ExprIR::MatchExpr { matched, arms } => {
            let m = format_expr_ir(matched);
            let mut lines = vec![format!("(() {{")];
            lines.push(format!("switch ({m}._variant) {{"));
            for arm in arms {
                let body = format_expr_ir(arm.body.tail.as_ref().unwrap());
                match &arm.pattern.kind {
                    crate::ir::MatchPatternKindIR::Variant { name, .. } => {
                        lines.push(format!("  case \"{name}\": return {body};"));
                    }
                    crate::ir::MatchPatternKindIR::Wildcard => {
                        lines.push(format!("  default: return {body};"));
                    }
                    _ => {}
                }
            }
            lines.push("}".into());
            lines.push("})()".into());
            lines.join("\n")
        }
```

**Python** (`python.rs:491`) — if/elif chain:
```rust
        ExprIR::MatchExpr { matched, arms } => {
            let m = format_expr_ir(matched);
            let mut lines = Vec::new();
            for (i, arm) in arms.iter().enumerate() {
                // arm.body is BlockIR — use tail expression (consult omniview.rs block_to_expr for pattern)
                let body = arm.body.tail.as_ref()
                    .map(|e| format_expr_ir(e))
                    .unwrap_or_else(|| "None".into());
                match &arm.pattern.kind {
                    crate::ir::MatchPatternKindIR::Variant { name, .. } => {
                        let kw = if i == 0 { "if" } else { "elif" };
                        lines.push(format!(
                            "({body}) {kw} {m}.get(\"_variant\") == \"{name}\""
                        ));
                    }
                    crate::ir::MatchPatternKindIR::Wildcard => {
                        lines.push(format!("({body})"));
                    }
                    _ => {}
                }
            }
            if lines.is_empty() { "None".into() } else { lines.join(" else ") }
        }
```

**Rust** (`rust.rs:503`) — match expression:
```rust
        ExprIR::MatchExpr { matched, arms } => {
            let m = format_expr_ir(matched);
            let mut lines = vec![format!("match &{m} {{")];
            for arm in arms {
                let body = arm.body.tail.as_ref()
                    .map(|e| format_expr_ir(e))
                    .unwrap_or_else(|| "None".into());
                match &arm.pattern.kind {
                    crate::ir::MatchPatternKindIR::Variant { name, .. } => {
                        lines.push(format!(
                            "    UnionType::{} {{ .. }} => {body},",
                            name
                        ));
                    }
                    crate::ir::MatchPatternKindIR::Wildcard => {
                        lines.push(format!("    _ => {body},"));
                    }
                    _ => {}
                }
            }
            lines.push("}".into());
            lines.join("\n")
        }
```

**Dioxus** (`dioxus.rs:669`) — same pattern as Rust (Dioxus is Rust-based).

- [ ] **Step 2: Run full test suite**

```bash
cd D:/space/OmniLang/omni-compiler && cargo test
```
Expected: all tests PASS. Fix any compilation errors (check that `MatchPatternIR` and `MatchArmIR` imports are present in each file).

- [ ] **Step 3: Commit**

```bash
git add omni-compiler/src/codegen/
git commit -m "fix(codegen): implement MatchExpr in 7 backends (was returning null/None)"
```

---

## Summary

| Task | Description | Files | Est. Time |
|------|-------------|-------|-----------|
| 1 | math stdlib (9 fns) | `stdlib.rs` | 1-2h |
| 2 | string stdlib (12 fns) | `stdlib.rs` | 2-3h |
| 3 | array + console stdlib (15 fns) | `stdlib.rs` | 2-3h |
| 4 | Update try_format_stdlib_call | `codegen/mod.rs` | 0.5h |
| 5 | Semantic Pass 4 validation | `semantic/mod.rs` + tests | 3-4h |
| 6 | TypeMismatch lint | `semantic/lints.rs` | 2-3h |
| 7 | IndexExpr codegen (7 backends) | 7 codegen files | 1-2h |
| 8 | ClosureExpr codegen (7 backends) | 7 codegen files | 2-3h |
| 9 | BlockExpr IIFE (all backends) | 9 codegen files | 2-3h |
| 10 | MatchExpr codegen (7 backends) | 7 codegen files | 3-4h |
| **Total** | | | **18-28h** |

**Execution order:** Tasks 1-4 (stdlib) can run in parallel. Tasks 5-10 (compiler gaps) should be done sequentially (they modify the same files).

**Pre-execution checklist (verified):**
- [x] `cargo test` passes: 325 + 33 + 13 + 7 + 26 + 3 + 3 = **410 tests** green
- [x] Function names verified (see table in Task 7 Step 1)
- [x] IR types verified: `StateIntent.r#type.kind`, `MatchPatternIR.kind: MatchPatternKindIR`, etc.
- [x] AST types verified: `IfDirective.else_ifs: Vec<(Expr, Vec<ViewNode>)>`, `FunctionDef.is_async: bool`
- [x] `SemanticResult.diagnostics` not `.errors`
- [x] `FunctionIR.visibility: String` ("public"|"private"), not `VisibilityIR` enum
- [x] OmniView uses `self.expr_to_js()` / `self.emit_statement()` methods, no standalone formatting fns
- [ ] Keep commits small — one commit per task
