# Batch 5: Remaining Files Split — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Split the final 2 oversized files: `stdlib.rs` (1211 code) and `lib.rs` (970 code). Every resulting file ≤800 lines. With lsp.rs already deleted in Batch 1 Task 1, this batch completes the R4 elimination.

**Architecture:**
- **stdlib.rs → `stdlib/`:** One file per stdlib namespace (http, json, storage, math, string, array, console), each with its `codegen_*()` functions. `mod.rs` holds the registry and lookup functions.
- **lib.rs → flat files:** `wasm.rs` (WASM bindings), `ffi.rs` (C FFI), `validate.rs` (multi-file validation). `lib.rs` keeps the main pipeline.

**Tech Stack:** Rust 2021, serde, wasm-bindgen (conditional), cargo test

**Safety net:** `cargo test --lib` must return "375 passed; 0 failed" after every commit.

**Key mechanics:** `#[cfg(feature = "wasm")]` on the module declaration in `lib.rs` keeps WASM conditional. `#[no_mangle] extern "C"` FFI functions need no feature flag.

---

### Task 1: Split stdlib.rs → `stdlib/` directory

**Files:**
- Create: `src/stdlib/mod.rs`
- Create: `src/stdlib/http.rs`
- Create: `src/stdlib/json.rs`
- Create: `src/stdlib/storage.rs`
- Create: `src/stdlib/math.rs`
- Create: `src/stdlib/string.rs`
- Create: `src/stdlib/array.rs`
- Create: `src/stdlib/console.rs`
- Create: `src/stdlib/tests.rs`
- Delete: `src/stdlib.rs`

- [ ] **Step 1: Create directory and move**

```bash
mkdir -p omnilang/src/stdlib
mv omnilang/src/stdlib.rs omnilang/src/stdlib/mod.rs
```

- [ ] **Step 2: Extract http.rs** (~160 lines)

Move `codegen_http_send()`, `codegen_http_get()`, `codegen_http_post()` and their stdlib registration entries (the `StdlibFn` static instances for `http_get`, `http_post`, `http_send`).

```rust
//! std/http — HTTP request codegen for all targets.
use super::{StdlibFn, Target};

pub(super) fn codegen_http_send(args: &[String], target: Target) -> String { ... }
pub(super) fn codegen_http_get(args: &[String], target: Target) -> String { ... }
pub(super) fn codegen_http_post(args: &[String], target: Target) -> String { ... }

// Register in mod.rs: http::register(registry)
pub(super) fn register() -> Vec<(&'static str, StdlibFn)> {
    vec![
        ("http_get", StdlibFn { ... codegen: codegen_http_get ... }),
        ("http_post", StdlibFn { ... codegen: codegen_http_post ... }),
        ("http_send", StdlibFn { ... codegen: codegen_http_send ... }),
    ]
}
```

- [ ] **Step 3: Extract remaining namespaces**

Same pattern for each:

| File | Functions | ~Lines |
|------|-----------|--------|
| `json.rs` | `codegen_json_parse`, `codegen_json_stringify` | 70 |
| `storage.rs` | `codegen_storage_get`, `codegen_storage_set` | 70 |
| `math.rs` | `codegen_math_abs/min/max/ceil/floor/round/random/sqrt/pow` | 240 |
| `string.rs` | `codegen_string_length/concat/split/join/replace/substring/contains/to_upper/to_lower/trim/starts_with/ends_with` | 310 |
| `array.rs` | `codegen_array_length/push/pop/map/filter/find` | 200 |
| `console.rs` | `codegen_console_log/warn/error` | 70 |

Each file exports a `pub(super) fn register() -> Vec<(&'static str, StdlibFn)>`.

- [ ] **Step 4: Extract tests.rs**

Move `#[cfg(test)] mod tests { ... }` (~348 lines) to `tests.rs`.

- [ ] **Step 5: Rewrite mod.rs**

```rust
//! Standard library registry — maps stdlib function names to codegen implementations.
pub use super::codegen::Target;

mod http;
mod json;
mod storage;
mod math;
mod string;
mod array;
mod console;
mod tests;

/// A registered standard library function.
pub struct StdlibFn {
    pub module: &'static str,
    pub name: &'static str,
    pub available_targets: &'static [Target],
    pub codegen: fn(&[String], Target) -> String,
}

/// Look up a stdlib function by module and name.
pub fn lookup(module: &str, name: &str) -> Option<&'static StdlibFn> {
    // Build registry once, cache in a static
    static REGISTRY: std::sync::OnceLock<Vec<StdlibFn>> = std::sync::OnceLock::new();
    let registry = REGISTRY.get_or_init(|| {
        let mut entries = Vec::new();
        for (name, entry) in http::register() { entries.push(entry); }
        for (name, entry) in json::register() { entries.push(entry); }
        // ... etc for all modules
        entries
    });
    registry.iter().find(|e| e.module == module && e.name == name)
}

pub fn module_available(module: &str, target: Target) -> bool { ... }
pub fn available_targets_for(module: &str) -> Vec<Target> { ... }
```

The `mod.rs` should be ~120 lines.

- [ ] **Step 6: Verify and commit**

```bash
cd omnilang && cargo test --lib 2>&1 | grep "test result:"
```
Expected: "375 passed; 0 failed"

```bash
cargo fmt
git add omnilang/src/stdlib/
git commit -m "refactor: split stdlib.rs into per-namespace sub-modules (1211→~120+submodules)"
```

---

### Task 2: Split lib.rs — extract wasm.rs

**Files:**
- Create: `src/wasm.rs`
- Modify: `src/lib.rs`

- [ ] **Step 1: Create wasm.rs**

Extract the WASM section (lines ~164-231 in current `lib.rs`):

```rust
//! WASM bindings — compiled only when feature "wasm" is enabled.
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

#[derive(Debug, Serialize, Deserialize, Default)]
struct CompileOptions {
    #[serde(default)]
    editor_mode: bool,
}

#[wasm_bindgen]
pub fn compile_to_omniview(source: &str, options_json: &str) -> String {
    let opts: CompileOptions = serde_json::from_str(options_json).unwrap_or_default();
    let result = crate::compile_omniview(source, opts.editor_mode);
    serde_json::to_string(&result).unwrap_or_else(|e| {
        serde_json::json!({
            "success": false,
            "files": null,
            "components": null,
            "source_map": null,
            "errors": [{ "message": e.to_string() }]
        }).to_string()
    })
}

#[wasm_bindgen]
pub fn compile_omniview_component(source: &str, component_name: &str, options_json: &str) -> String {
    // ... (moved verbatim)
}

#[wasm_bindgen]
pub fn compile_to_ir(source: &str) -> String {
    let result = crate::compile(source);
    serde_json::to_string(&result).unwrap_or_else(|e| {
        serde_json::json!({
            "success": false,
            "ir": null,
            "errors": [{ "message": e.to_string() }]
        }).to_string()
    })
}

#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}
```

- [ ] **Step 2: Update lib.rs**

In `lib.rs`:
- Remove the WASM section (lines ~164-231)
- Add conditional module declaration: `#[cfg(feature = "wasm")] pub mod wasm;`
- Remove the `#[cfg(feature = "wasm")] use wasm_bindgen::prelude::*;` import (it's now in wasm.rs)

- [ ] **Step 3: Build with and without WASM feature**

```bash
cd omnilang && cargo check 2>&1 | grep "^error" | head -5
```
Expected: zero errors (WASM feature disabled — wasm module is conditionally compiled).

```bash
cd omnilang && cargo check --features wasm 2>&1 | grep "^error" | head -5
```
Expected: zero errors (WASM feature enabled — wasm module is compiled).

- [ ] **Step 4: Verify tests and commit**

```bash
cd omnilang && cargo test --lib 2>&1 | grep "test result:"
# Expected: 375 passed
cargo fmt
git add src/wasm.rs src/lib.rs
git commit -m "refactor: extract WASM bindings from lib.rs to wasm.rs"
```

---

### Task 3: Split lib.rs — extract ffi.rs

**Files:**
- Create: `src/ffi.rs`
- Modify: `src/lib.rs`

- [ ] **Step 1: Create ffi.rs** (~270 lines)

Extract the C FFI section (lines ~701-970):
- `rust_to_c_string()` (private helper)
- `omni_free_string()`, `omni_version()`, `omni_analyse()`, `omni_compile_multi_to_target()`

```rust
//! C FFI — extern "C" functions for Python ctypes/cffi consumers.
use std::collections::HashMap;
use std::ffi::CString;
use std::os::raw::c_char;
use serde::Deserialize;

fn rust_to_c_string(s: String) -> *mut c_char {
    let sanitized = s.replace('\0', "\\0");
    CString::new(sanitized)
        .unwrap_or_else(|_| CString::new("{\"error\":\"serialization failed\"}")
        .expect("hardcoded CString has no interior NUL bytes"))
        .into_raw()
}

#[no_mangle]
pub unsafe extern "C" fn omni_free_string(ptr: *mut c_char) {
    if ptr.is_null() { return; }
    // SAFETY: ptr is non-null (checked above) and was produced by
    // CString::into_raw() from a prior omni_* call.
    unsafe { let _ = CString::from_raw(ptr); }
}

#[no_mangle]
pub extern "C" fn omni_version() -> *mut c_char {
    rust_to_c_string(env!("CARGO_PKG_VERSION").to_string())
}

// ... (omni_analyse, omni_compile_multi_to_target — moved verbatim)
```

- [ ] **Step 2: Update lib.rs**

- Remove the FFI section
- Add: `pub mod ffi;`
- Remove: `use std::ffi::CString;` and `use std::os::raw::c_char;` (now in ffi.rs)
- Keep: `use serde::{Deserialize, Serialize};` and `use std::collections::HashMap;`

- [ ] **Step 3: Verify and commit**

```bash
cd omnilang && cargo check 2>&1 | grep "^error" | head -5
# Expected: zero errors
cargo test --lib 2>&1 | grep "test result:"
# Expected: 375 passed
cargo fmt
git add src/ffi.rs src/lib.rs
git commit -m "refactor: extract C FFI bindings from lib.rs to ffi.rs"
```

---

### Task 4: Split lib.rs — extract validate.rs

**Files:**
- Create: `src/validate.rs`
- Modify: `src/lib.rs`

- [ ] **Step 1: Create validate.rs** (~436 lines)

Extract the multi-file validation section (lines ~262-699):
- `MultiFileResult` struct
- `validate_multi()` (becomes `pub(crate) fn`)
- `validate_file_consistency()`
- `CrossFileSymbolKind` enum
- `build_pub_symbols()`
- `validate_cross_file()`
- `validate_named_import()`

```rust
//! Multi-file validation: cross-file imports, visibility, cycle detection.
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use crate::parser::ast::{ImportStmt, Program, Visibility};

#[derive(Debug, Serialize, Deserialize)]
pub struct MultiFileResult {
    pub success: bool,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub errors: Vec<String>,
}

pub(crate) fn validate_multi(
    sources: HashMap<String, String>,
    path_mapping: HashMap<String, String>,
) -> MultiFileResult {
    // ... (moved verbatim)
}

// ... (all helper functions moved verbatim)
```

- [ ] **Step 2: Update lib.rs**

- Remove the validation section
- Add: `mod validate;`
- In `compile_multi_to_target()`, change `validate_multi(...)` → `validate::validate_multi(...)`

- [ ] **Step 3: Verify lib.rs is now ~450 lines**

```bash
wc -l omnilang/src/lib.rs
```
Expected: ≤ 450 lines.

- [ ] **Step 4: Verify and commit**

```bash
cd omnilang && cargo check 2>&1 | grep "^error" | head -5
cargo test --lib 2>&1 | grep "test result:"
cargo fmt
git add src/validate.rs src/lib.rs
git commit -m "refactor: extract multi-file validation from lib.rs to validate.rs"
```

---

### Task 5: Final R4 verification

- [ ] **Step 1: Count all files over 800 lines (code only)**

```bash
cd omnilang && find src -name "*.rs" -not -name "tests.rs" -exec wc -l {} \; | sort -rn | awk '$1 > 800 {print $1, $2}'
```
Expected: only `src/ir/lower_ast.rs` appears (at ~2500 lines, tracked for follow-up). All other files ≤800.

- [ ] **Step 2: Run full eval harness**

```bash
bash .claude/evals/eval-run-omnilang.sh
```
Expected: C1-C5 pass, R1-R4 pass (R4 moves from 18 files → 1 file over 800).

- [ ] **Step 3: Final commit**

```bash
git add -A
git commit -m "chore: R4 file size reduction complete — 18→1 files over 800 lines"
```
