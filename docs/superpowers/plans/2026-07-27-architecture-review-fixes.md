# Architecture Review Fix Plan (2026-07-27 v2 Remediation)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix all CRITICAL and HIGH findings from the 12-expert architecture review (`.architecture/reviews/full-workspace-review-2026-07-27-v2.md`), restoring test suite, CI governance, security posture, and completing the P35 crate extraction.

**Architecture:** Phase 1 (Tasks 1-5) unblocks the test suite and fixes the critical security vulnerability. Phase 2 (Tasks 6-12) completes P35 crate extraction. Phase 3 (Tasks 13-21) addresses Rust quality issues. Phase 4 (Tasks 22-27) fixes TypeScript errors and frontend quality. Phase 5 (Tasks 28-36) hardens CI governance and documentation.

**Tech Stack:** Rust (cargo, clippy, rustfmt), TypeScript/React, Criterion benchmarks, ESLint v10, GitHub Actions CI

## Global Constraints

- `cargo test --workspace` must pass with zero errors
- `cargo clippy --workspace -- -D warnings` must pass
- `cargo fmt --check` on all crates must pass
- `tsc --noEmit` in `poiesis/` must pass with zero errors
- All security fixes must be applied to BOTH copies of debug-window HTML
- P35 completion requires omnilang to depend on omnilang-core, re-exporting types
- No new dependencies beyond Criterion (dev-only) and @types/node (dev-only)

---

## Phase 1: Unblock the Test Suite + Critical Security Fix

### Task 1: Fix test compilation errors in omnilang codegen backends

**Files:**
- Modify: `omnilang/src/codegen/rust/tests.rs` (~26 source_span additions)
- Modify: `omnilang/src/codegen/javascript/tests.rs` (~31 source_span additions)
- Modify: `omnilang/src/codegen/vue/tests.rs` (~6 source_span additions)
- Modify: `omnilang/src/codegen/dioxus/tests.rs` (~6 source_span additions)
- Modify: `omnilang/src/codegen/react/mod.rs` (~2 source_span additions)
- Modify: `omnilang/src/codegen/traits.rs` (~10 source_span additions)
- Modify: `omnilang/src/codegen/tests_regression/javascript.rs` (10 FxHashMap fixes)
- Modify: `omnilang/src/lib.rs` (1 source_span addition)

**Interfaces:**
- Consumes: IR struct definitions in `omnilang/src/ir/types.rs` (with `source_span: Option<Span>` field)
- Produces: Compilable test code — `cargo test -p omnilang --lib` passes

- [ ] **Step 1: Run tests to confirm current failure count**

```bash
cargo test -p omnilang --lib 2>&1 | grep "^error\[E0063\]" | wc -l
```

Expected: 73+ E0063 errors

- [ ] **Step 2: Add `source_span: None` to all IR struct initializers in test code**

For every struct literal construction in test files that builds an IR type with a `source_span` field, append `source_span: None`. Example pattern:

```rust
// Before (broken):
FunctionIR {
    name: "test_fn".to_string(),
    params: vec![],
    body: vec![],
    return_type: TypeKind::Void,
}

// After (fixed):
FunctionIR {
    name: "test_fn".to_string(),
    params: vec![],
    body: vec![],
    return_type: TypeKind::Void,
    source_span: None,
}
```

Focus files (in order of error count):
- `omnilang/src/codegen/javascript/tests.rs` — 31 sites
- `omnilang/src/codegen/rust/tests.rs` — 26 sites
- `omnilang/src/codegen/traits.rs` — 10 sites
- `omnilang/src/codegen/vue/tests.rs` — 6 sites
- `omnilang/src/codegen/dioxus/tests.rs` — 6 sites
- `omnilang/src/codegen/react/mod.rs` — 2 sites
- `omnilang/src/lib.rs` — 1 site

- [ ] **Step 3: Fix FxHashMap type mismatches in javascript regression tests**

```bash
# In omnilang/src/codegen/tests_regression/javascript.rs
# Replace all `HashMap::from([...])` with `FxHashMap::from_iter([...])`
# Add `use rustc_hash::FxHashMap;` at top of file (lines 7-12 area)
```

- [ ] **Step 4: Verify tests compile**

```bash
cargo test -p omnilang --lib --no-run
```

Expected: Compiles successfully (0 errors)

- [ ] **Step 5: Commit**

```bash
git add omnilang/src/codegen/
git commit -m "fix: add source_span: None to 73+ test struct initializers, fix FxHashMap imports"
```

---

### Task 2: Fix test compilation errors in integration tests + extracted crates

**Files:**
- Modify: `omnilang/tests/design_codegen_test.rs` (15 source_span additions)
- Modify: `omnilang/tests/compilation_test.rs` (add 6th arg `None` to `compile_multi_to_target`)
- Modify: `crates/omnilang-core/src/codegen/tests_regression/mod.rs` (6 source_span additions)
- Modify: `crates/omnilang-core/src/parser/mod.rs` (prefix unused vars with `_`)
- Modify: `crates/omnilang-codegen-omniview/src/view/element.rs` (8 source_span additions)
- Modify: `crates/omnilang-codegen-omniview/src/expr.rs` (5 source_span additions)
- Modify: `crates/omnilang-codegen-omniview/src/component_html.rs` (2 additions)
- Modify: `crates/omnilang-codegen-omniview/src/stmt.rs` (2 additions)
- Modify: `crates/omnilang-codegen-omniview/src/utils.rs` (2 additions)
- Modify: `crates/omnilang-codegen-omniview/src/view/mod.rs` (2 additions)
- Modify: `crates/omnilang-codegen-omniview/src/motion_config.rs` (2 additions)

**Interfaces:**
- Consumes: IR struct definitions from respective crates
- Produces: `cargo test --workspace` compiles successfully

- [ ] **Step 1: Fix design_codegen_test.rs — add source_span: None to 15 struct initializers**

```bash
# File: omnilang/tests/design_codegen_test.rs
# Add `source_span: None,` to every TokenIR, PseudoStateIR, DesignIR,
# VariantDimIR, ClassIR, VariantBranchIR, VariantIR constructor
```

- [ ] **Step 2: Fix compilation_test.rs — add 6th argument to compile_multi_to_target**

```bash
# File: omnilang/tests/compilation_test.rs, line ~85
# Change: compile_multi_to_target(&sources, target, &output_dir, &cache_dir, &entry)
# To:     compile_multi_to_target(&sources, target, &output_dir, &cache_dir, &entry, None)
```

- [ ] **Step 3: Fix omnilang-core codegen test regressions**

```bash
# File: crates/omnilang-core/src/codegen/tests_regression/mod.rs
# Add `source_span: None,` to all IR struct initializers (~6 sites)
```

- [ ] **Step 4: Fix omnilang-core parser unused variables**

```bash
# File: crates/omnilang-core/src/parser/mod.rs, lines 783, 849
# Change: `stmt` → `_stmt`, `expr` → `_expr`
```

- [ ] **Step 5: Fix omnilang-codegen-omniview test errors**

```bash
# Add source_span: None to struct initializers in:
# - view/element.rs (8 sites)
# - expr.rs (5 sites)
# - component_html.rs (2 sites)
# - stmt.rs (2 sites)
# - utils.rs (2 sites)
# - view/mod.rs (2 sites)
# - motion_config.rs (2 sites)
```

- [ ] **Step 6: Verify full workspace tests compile**

```bash
cargo test --workspace --no-run
```

Expected: 0 compilation errors

- [ ] **Step 7: Run tests and verify they pass**

```bash
cargo test --workspace 2>&1 | tail -5
```

Expected: `test result: ok. N passed; 0 failed`

- [ ] **Step 8: Commit**

```bash
git add omnilang/tests/ crates/
git commit -m "fix: fix remaining test compilation errors across workspace (90→0)"
```

---

### Task 3: Fix postMessage security vulnerabilities (CRITICAL)

**Files:**
- Modify: `poiesis/src/components/architecture/ArchitecturePreview.tsx:16` (add origin check)
- Modify: `poiesis/src-tauri/debug-window/index.html:316` (replace `"*"` with `e.origin`)
- Modify: `poiesis/src-tauri/debug-window/index.html:350-351` (replace `"*"` with `window.parent.origin`)
- Modify: `poiesis/public/debug-window/index.html:316,350-351` (sync identical fix)

**Interfaces:**
- Produces: All postMessage calls use explicit origin; ArchitecturePreview validates origin

- [ ] **Step 1: Fix ArchitecturePreview.tsx — add origin validation**

```typescript
// File: poiesis/src/components/architecture/ArchitecturePreview.tsx, line 16
// Before:
const handler = (e: MessageEvent) => {
  if (e.data?.type === "poiesis:element-click" && e.data.pid) {
    selectBubbleByPid(e.data.pid);
  }
};

// After:
const handler = (e: MessageEvent) => {
  if (e.origin !== window.location.origin) return;
  if (e.data?.type === "poiesis:element-click" && e.data.pid) {
    selectBubbleByPid(e.data.pid);
  }
};
```

- [ ] **Step 2: Fix debug-window bridge — replace `"*"` with `e.origin` on signal-state reply**

```javascript
// File: poiesis/src-tauri/debug-window/index.html, line ~316
// Before:
// e.source.postMessage({source:"omni-preview",type:"signal-state",signals:snap},"*");
// After:
// e.source.postMessage({source:"omni-preview",type:"signal-state",signals:snap}, e.origin);
```

- [ ] **Step 3: Fix debug-window bridge — replace `"*"` with `window.parent.origin` on element-click**

```javascript
// File: poiesis/src-tauri/debug-window/index.html, line ~350-351
// Before:
// window.parent.postMessage({source:"omni-preview",type:"element-click",nid:...,tag:...},"*");
// After:
// window.parent.postMessage({source:"omni-preview",type:"element-click",nid:...,tag:...}, window.parent.origin);
```

- [ ] **Step 4: Sync identical fixes to public/debug-window/index.html**

Apply Steps 2-3 identically to `poiesis/public/debug-window/index.html`.

- [ ] **Step 5: Commit**

```bash
git add poiesis/src/components/architecture/ArchitecturePreview.tsx \
        poiesis/src-tauri/debug-window/index.html \
        poiesis/public/debug-window/index.html
git commit -m "fix(security): add postMessage origin validation, replace '*' targetOrigin"
```

---

### Task 4: Add cargo test to CI pipeline

**Files:**
- Modify: `.github/workflows/ci.yml`

**Interfaces:**
- Produces: CI runs `cargo test --workspace` on every PR/push

- [ ] **Step 1: Add test job to CI**

In `.github/workflows/ci.yml`, add after the existing `rust` job (or modify the existing job to include tests):

```yaml
  test:
    name: Test (Rust)
    runs-on: ubuntu-latest
    needs: build
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - run: cargo test --workspace
```

- [ ] **Step 2: Add cargo fmt check to CI**

In the same job or a separate step:

```yaml
      - run: cargo fmt --check
```

- [ ] **Step 3: Add cargo clippy to CI**

```yaml
      - run: cargo clippy --workspace -- -D warnings
```

- [ ] **Step 4: Commit**

```bash
git add .github/workflows/ci.yml
git commit -m "ci: add cargo test, fmt, clippy to CI pipeline"
```

---

### Task 5: Fix cargo fmt — trailing whitespace in omnilang-codegen-omniview

**Files:**
- Modify: `crates/omnilang-codegen-omniview/src/store.rs:247,470,506`
- Modify: `crates/omnilang-codegen-omniview/src/design.rs:221` (formatting fix)

**Interfaces:**
- Produces: `cargo fmt --check` passes

- [ ] **Step 1: Run cargo fmt to auto-fix all formatting**

```bash
cargo fmt --all
```

- [ ] **Step 2: Verify fmt check passes**

```bash
cargo fmt --check --all
```

Expected: No output (all files properly formatted)

- [ ] **Step 3: Commit**

```bash
git add -u
git commit -m "style: apply cargo fmt across workspace"
```

---

## Phase 2: Complete P35 Crate Extraction

### Task 6: Align `#[non_exhaustive]` annotations between facade and core

**Files:**
- Modify: `crates/omnilang-core/src/ir/types.rs` (add 10 `#[non_exhaustive]` annotations)

**Interfaces:**
- Consumes: `omnilang/src/ir/types.rs` as reference for which enums need the attribute
- Produces: Both copies have identical `#[non_exhaustive]` on the same 10 enums

- [ ] **Step 1: Copy `#[non_exhaustive]` from facade to core**

In `crates/omnilang-core/src/ir/types.rs`, add `#[non_exhaustive]` to these enum definitions (matching the facade at `omnilang/src/ir/types.rs`):
- `BinaryOpIR` (~line 24)
- `ExprIR` (~line 129)
- `EventHandlerIR` (~line 257)
- `EventDataKind` (~line 279)
- `StatementIR` (~line 428)
- `MutationIntent` (~line 527)
- `MatchPatternKindIR` (~line 786)
- `ComponentIR` (~line 879)
- Plus 2 more from `ir/types.rs` lines 1052, 1093

- [ ] **Step 2: Verify both crates compile**

```bash
cargo check -p omnilang-core -p omnilang-codegen-omniview -p omnilang
```

- [ ] **Step 3: Commit**

```bash
git add crates/omnilang-core/src/ir/types.rs
git commit -m "fix: add #[non_exhaustive] to 10 IR enums in omnilang-core (align with facade)"
```

---

### Task 7: Move P39 optimize.rs to omnilang-core

**Files:**
- Create: `crates/omnilang-core/src/ir/optimize.rs` (copy from omnilang)
- Modify: `crates/omnilang-core/src/ir/mod.rs` (add `pub mod optimize;`)
- Modify: `crates/omnilang-core/src/analyse.rs` (add `optimize()` call in pipeline)
- Modify: `omnilang/src/ir/mod.rs` (change to `pub use omnilang_core::ir::optimize;`)

**Interfaces:**
- Consumes: `omnilang/src/ir/optimize.rs` (561 lines, existing implementation)
- Produces: `omnilang_core::ir::optimize::optimize()` is the canonical location

- [ ] **Step 1: Copy optimize.rs to omnilang-core**

```bash
cp omnilang/src/ir/optimize.rs crates/omnilang-core/src/ir/optimize.rs
```

- [ ] **Step 2: Add optimize module to omnilang-core's ir/mod.rs**

In `crates/omnilang-core/src/ir/mod.rs`, add:
```rust
pub mod optimize;
```

- [ ] **Step 3: Copy P39 optimize calls to omnilang-core's analyse.rs**

In `crates/omnilang-core/src/analyse.rs`, after the IR merge step (~line 310), add:
```rust
crate::ir::optimize::optimize(&mut merged_ir);
```
And in the cached path (~line 543), add the same call after `merge_file_ir_into`.

- [ ] **Step 4: Update omnilang facade to re-export from core**

In `omnilang/src/ir/mod.rs`, change:
```rust
pub mod optimize;
```
To:
```rust
pub use omnilang_core::ir::optimize;
```

- [ ] **Step 5: Verify compilation**

```bash
cargo check --workspace
```

- [ ] **Step 6: Commit**

```bash
git add crates/omnilang-core/src/ir/optimize.rs \
        crates/omnilang-core/src/ir/mod.rs \
        crates/omnilang-core/src/analyse.rs \
        omnilang/src/ir/mod.rs
git commit -m "refactor: move P39 optimize.rs to omnilang-core, re-export from facade"
```

---

### Task 8: Add omnilang-core dependency to omnilang facade

**Files:**
- Modify: `omnilang/Cargo.toml` (add `omnilang-core` dependency)

**Interfaces:**
- Consumes: `crates/omnilang-core` as workspace member
- Produces: `omnilang` crate can `use omnilang_core::*`

- [ ] **Step 1: Add dependency to omnilang/Cargo.toml**

```toml
# In omnilang/Cargo.toml, under [dependencies]:
omnilang-core = { path = "../crates/omnilang-core" }
```

- [ ] **Step 2: Verify the dependency resolves**

```bash
cargo check -p omnilang
```

Expected: Compiles without circular dependency errors

- [ ] **Step 3: Commit**

```bash
git add omnilang/Cargo.toml
git commit -m "refactor: add omnilang-core dependency to omnilang facade"
```

---

### Task 9: Re-export IR types from omnilang-core in omnilang facade

**Files:**
- Modify: `omnilang/src/lib.rs` (replace `pub mod ir;` with `pub use omnilang_core::ir;`)
- Modify: `omnilang/src/lib.rs` (same for lexer, parser, semantic, error, sourcemap, etc.)

**Interfaces:**
- Consumes: `omnilang-core` public modules
- Produces: `omnilang::ir::*` resolves to `omnilang_core::ir::*` (transparent re-export)

- [ ] **Step 1: Replace module declarations with re-exports in omnilang/src/lib.rs**

For each module that exists identically in both crates, replace:
```rust
pub mod ir;
pub mod lexer;
pub mod parser;
pub mod semantic;
pub mod error;
pub mod sourcemap;
pub mod stdlib;
pub mod path_mapping;
pub mod discovery;
pub mod validate;
```
With:
```rust
pub use omnilang_core::ir;
pub use omnilang_core::lexer;
pub use omnilang_core::parser;
pub use omnilang_core::semantic;
pub use omnilang_core::error;
pub use omnilang_core::sourcemap;
pub use omnilang_core::stdlib;
pub use omnilang_core::path_mapping;
pub use omnilang_core::discovery;
pub use omnilang_core::validate;
```

- [ ] **Step 2: Handle modules NOT in omnilang-core**

Keep `pub mod` for modules that only exist in the facade:
```rust
pub mod codegen;  // Backend implementations live here
// ir/optimize.rs was moved in Task 7
```

- [ ] **Step 3: Fix omnilang/src/ir/mod.rs**

Replace its content with:
```rust
pub use omnilang_core::ir::*;
pub use omnilang_core::ir::optimize;  // From Task 7
```

- [ ] **Step 4: Verify compilation**

```bash
cargo check --workspace
```

Expected: All crates compile. Fix any broken `crate::ir::` paths in codegen backends — they should still work via re-exports.

- [ ] **Step 5: Run tests**

```bash
cargo test --workspace
```

- [ ] **Step 6: Commit**

```bash
git add omnilang/src/lib.rs omnilang/src/ir/mod.rs
git commit -m "refactor: re-export IR types from omnilang-core in omnilang facade"
```

---

### Task 10: Remove duplicated source files from omnilang facade

**Files:**
- Delete: `omnilang/src/ir/` (except `mod.rs` which now re-exports)
- Delete: `omnilang/src/lexer/`
- Delete: `omnilang/src/parser/`
- Delete: `omnilang/src/semantic/`
- Delete: `omnilang/src/error.rs`
- Delete: `omnilang/src/sourcemap/`
- Delete: `omnilang/src/stdlib/`
- Delete: `omnilang/src/path_mapping.rs`
- Delete: `omnilang/src/discovery.rs`
- Delete: `omnilang/src/validate.rs`

**Interfaces:**
- Consumes: Re-exports from Task 9 must resolve all `pub use crate::*` paths in codegen backends
- Produces: ~57 fewer files in omnilang/src/; single source of truth in omnilang-core

- [ ] **Step 1: Verify all imports resolve through re-exports**

```bash
# Search for any `crate::ir::` paths that might break
grep -rn "crate::ir::" omnilang/src/codegen/ | head -20
# These should resolve via the re-export in omnilang/src/ir/mod.rs
```

- [ ] **Step 2: Delete duplicated source files**

```bash
# Remove IR types (keep mod.rs with re-exports)
rm -rf omnilang/src/ir/types.rs omnilang/src/ir/logic.rs omnilang/src/ir/design.rs \
       omnilang/src/ir/types_motion.rs omnilang/src/ir/visibility.rs \
       omnilang/src/ir/lower_logic.rs omnilang/src/ir/lower/

# Remove lexer
rm -rf omnilang/src/lexer/

# Remove parser
rm -rf omnilang/src/parser/

# Remove semantic
rm -rf omnilang/src/semantic/

# Remove other duplicated modules
rm omnilang/src/error.rs omnilang/src/sourcemap/ omnilang/src/stdlib/ \
   omnilang/src/path_mapping.rs omnilang/src/discovery.rs omnilang/src/validate.rs
```

- [ ] **Step 3: Verify compilation with deleted files**

```bash
cargo check --workspace
```

Expected: Compiles — all types resolve from omnilang-core via re-exports.

- [ ] **Step 4: Run full test suite**

```bash
cargo test --workspace
```

- [ ] **Step 5: Commit**

```bash
git add -u
git commit -m "refactor: remove 57 duplicated source files from omnilang (now re-exported from core)"
```

---

### Task 11: Wire omnilang-codegen-omniview through the facade

**Files:**
- Modify: `omnilang/Cargo.toml` (add omnilang-codegen-omniview dependency)
- Modify: `omnilang/src/codegen/mod.rs` (delegate OmniView target to extracted crate)

**Interfaces:**
- Consumes: `omnilang-codegen-omniview` crate, `OmniViewCodegen` struct
- Produces: OmniView codegen path uses extracted crate (no duplicate logic)

- [ ] **Step 1: Add dependency**

```toml
# In omnilang/Cargo.toml:
omnilang-codegen-omniview = { path = "../crates/omnilang-codegen-omniview" }
```

- [ ] **Step 2: Update codegen dispatch for OmniView target**

In `omnilang/src/codegen/mod.rs`, when target is `Target::OmniView`, delegate to `omnilang_codegen_omniview::OmniViewCodegen` instead of the local `crate::codegen::omniview` module.

- [ ] **Step 3: Remove duplicated omnilang/src/codegen/omniview/ directory**

```bash
rm -rf omnilang/src/codegen/omniview/
```

- [ ] **Step 4: Verify tests pass**

```bash
cargo test --workspace
```

- [ ] **Step 5: Commit**

```bash
git add omnilang/Cargo.toml omnilang/src/codegen/mod.rs
git commit -m "refactor: wire omnilang-codegen-omniview crate through facade, remove duplicate"
```

---

### Task 12: Remove unused codegen feature flags from omnilang-core

**Files:**
- Modify: `crates/omnilang-core/Cargo.toml:25-33` (remove 8 unused feature flags)

**Interfaces:**
- Produces: Only `codegen-omniview` feature remains; future backends add flags on creation

- [ ] **Step 1: Remove unused feature flags**

In `crates/omnilang-core/Cargo.toml`, remove these features from `[features]`:
```toml
codegen-react = []
codegen-vue = []
codegen-javascript = []
codegen-dart = []
codegen-flutter = []
codegen-rust = []
codegen-dioxus = []
codegen-python = []
```

Keep only:
```toml
codegen-omniview = []
```

- [ ] **Step 2: Verify compilation**

```bash
cargo check --workspace
```

- [ ] **Step 3: Commit**

```bash
git add crates/omnilang-core/Cargo.toml
git commit -m "refactor: remove 8 unused codegen feature flags from omnilang-core (YAGNI)"
```

---

## Phase 3: Rust Quality — Clippy, Dead Code, Unreachable Patterns

### Task 13: Fix 14 clippy errors in omnilang

**Files:**
- Modify: `omnilang/src/codegen/omniview/view/element.rs:731` (too_many_arguments — extract arg struct)
- Modify: `omnilang/src/codegen/omniview/view/element.rs:1021` (let_and_return)
- Modify: `omnilang/src/codegen/react/component/function.rs:281-285` (collapsible_match)
- Modify: `omnilang/src/codegen/react/expr.rs:394-423` (collapsible_match)
- Modify: `omnilang/src/discovery.rs:26` (double_must_use — remove #[must_use])
- Modify: `omnilang/src/lib.rs` (double_must_use on 2 functions)

**Interfaces:**
- Produces: `cargo clippy -p omnilang -- -D warnings` passes

- [ ] **Step 1: Fix collapsible_match (4 instances)**

Merge nested `if let` patterns into single matches. Example for `function.rs:281-285`:
```rust
// Before:
if let ExprIR::SomeVariant(x) = &expr {
    if let Some(y) = &x.field {
        // ...
    }
}

// After:
if let ExprIR::SomeVariant(x) = &expr
    && let Some(y) = &x.field
{
    // ...
}
```

Apply same pattern to `expr.rs:394-423` (3 instances).

- [ ] **Step 2: Fix let_and_return**

In `element.rs:1021`, change:
```rust
let result = some_expression;
result
```
To just:
```rust
some_expression
```

- [ ] **Step 3: Fix double_must_use (3 instances)**

Remove `#[must_use]` from functions that already return `Result` or complex structs at `discovery.rs:26` and `lib.rs` (2 sites).

- [ ] **Step 4: Fix too_many_arguments — extract argument struct**

For the 8-parameter function at `element.rs:731`, extract an argument struct:
```rust
struct EmitElementHtmlCloseArgs<'a> {
    element: &'a ElementIR,
    writer: &'a mut CodeWriter,
    // ... remaining 6 fields
}
```

- [ ] **Step 5: Fix omnilang-core clippy errors (3)**

```bash
# unused import in crates/omnilang-core
# empty lines after doc comments (3 sites — remove blank lines)
```

- [ ] **Step 6: Verify clippy passes**

```bash
cargo clippy --workspace -- -D warnings
```

- [ ] **Step 7: Commit**

```bash
git add -u
git commit -m "fix: resolve 14 clippy errors — collapsible_match, let_and_return, double_must_use, too_many_arguments"
```

---

### Task 14: Remove 12 unreachable catch-all match arms

**Files:**
- Modify: `crates/omnilang-codegen-omniview/src/component_html.rs:230`
- Modify: `crates/omnilang-codegen-omniview/src/expr.rs:231,305,327`
- Modify: `crates/omnilang-codegen-omniview/src/stmt.rs:335`
- Modify: `crates/omnilang-codegen-omniview/src/utils.rs:115`
- Modify: `crates/omnilang-codegen-omniview/src/view/element.rs:420,434,465,526`
- Modify: `crates/omnilang-codegen-omniview/src/view/mod.rs:191`
- Modify: `omnilang/src/codegen/omniview/expr.rs:231`

**Interfaces:**
- Produces: All match arms are exhaustive; compiler enforces coverage on new variants

- [ ] **Step 1: Remove `_ => unreachable!()` from exhaustive matches**

For each file listed, find the pattern:
```rust
match value {
    VariantA => { ... }
    VariantB => { ... }
    // ... all variants covered ...
    _ => unreachable!(),
}
```

Remove the `_ => unreachable!(),` line. The compiler will now enforce exhaustiveness.

- [ ] **Step 2: Verify compilation and tests**

```bash
cargo check --workspace
cargo test --workspace
```

Expected: Compiler verifies all matches are exhaustive; tests pass.

- [ ] **Step 3: Commit**

```bash
git add crates/omnilang-codegen-omniview/src/ omnilang/src/codegen/omniview/
git commit -m "fix: remove 12 unreachable catch-all match arms — let compiler enforce exhaustiveness"
```

---

### Task 15: Remove dead code

**Files:**
- Delete functions from: `crates/omnilang-codegen-omniview/src/motion_config.rs` (`keyframe_to_config`, `timeline_body_to_config`, `callback_to_config`)
- Delete functions from: `omnilang/src/codegen/react/mod.rs` (`block_mutates_store_signal`, `stmt_mutates_store_signal`)
- Remove or justify: `poiesis/src-tauri/src/lib.rs` (`diagnostic_to_json`)

**Interfaces:**
- Produces: Zero dead code warnings in production code

- [ ] **Step 1: Remove dead functions from motion_config.rs**

```bash
# Delete keyframe_to_config, timeline_body_to_config, callback_to_config
# (verified: zero callers via grep audit)
```

- [ ] **Step 2: Remove dead functions from react/mod.rs**

```bash
# Delete block_mutates_store_signal, stmt_mutates_store_signal
```

- [ ] **Step 3: Fix diagnostic_to_json — remove #[allow(dead_code)] if unused**

If `diagnostic_to_json` has no callers, remove the function. If kept for FFI, add `#[expect(dead_code, reason = "exported via FFI")]`.

- [ ] **Step 4: Verify**

```bash
cargo check --workspace
cargo clippy --workspace -- -D warnings
```

- [ ] **Step 5: Commit**

```bash
git add -u
git commit -m "chore: remove 6 dead code items (zero callers confirmed)"
```

---

### Task 16: Remove or feature-gate compile_multi_target + rayon

**Files:**
- Modify: `omnilang/Cargo.toml` (gate rayon behind feature flag or remove)
- Modify: `omnilang/src/lib.rs:573-607` (gate or remove `compile_multi_target`)

**Interfaces:**
- Produces: `rayon` dependency only compiled when `multi-target` feature is enabled

- [ ] **Step 1: Gate compile_multi_target behind feature flag**

In `omnilang/Cargo.toml`:
```toml
[features]
multi-target = ["dep:rayon"]

[dependencies]
rayon = { version = "1", optional = true }
```

In `omnilang/src/lib.rs`, wrap `compile_multi_target` in `#[cfg(feature = "multi-target")]`.

- [ ] **Step 2: Gate rayon in omnilang-core**

In `crates/omnilang-core/Cargo.toml`:
```toml
rayon = { version = "1", optional = true }
```

Gate `par_iter()` usage in `crates/omnilang-core/src/analyse.rs` behind `#[cfg(feature = "rayon")]` with sequential fallback.

- [ ] **Step 3: Verify default build (without multi-target)**

```bash
cargo check --workspace
cargo test --workspace
```

- [ ] **Step 4: Commit**

```bash
git add omnilang/Cargo.toml omnilang/src/lib.rs crates/omnilang-core/Cargo.toml crates/omnilang-core/src/analyse.rs
git commit -m "refactor: gate compile_multi_target + rayon behind 'multi-target' feature flag"
```

---

### Task 17: Fix TypeKind normalization — `"String"` → `TypeKind::String`

**Files:**
- Modify: `crates/omnilang-core/src/ir/types.rs:304-318` (TypeKind::from)

**Interfaces:**
- Produces: Case-insensitive primitive type matching; `"String"` and `"string"` both → `TypeKind::String`

- [ ] **Step 1: Add case-insensitive normalization**

In `TypeKind::from(&str)`, add case-insensitive matching for known primitives:
```rust
impl From<&str> for TypeKind {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "string" => TypeKind::String,
            "int" | "i32" | "i64" => TypeKind::Int,
            "float" | "f32" | "f64" => TypeKind::Float,
            "bool" => TypeKind::Bool,
            "void" => TypeKind::Void,
            "any" => TypeKind::Any,
            _ => TypeKind::Named {
                name: s.to_string(),  // Preserve original casing
                type_args: vec![],
            },
        }
    }
}
```

- [ ] **Step 2: Add test for normalization**

```rust
#[test]
fn typekind_case_insensitive() {
    assert_eq!(TypeKind::from("String"), TypeKind::String);
    assert_eq!(TypeKind::from("string"), TypeKind::String);
    assert_eq!(TypeKind::from("MyType"), TypeKind::Named { name: "MyType".into(), type_args: vec![] });
}
```

- [ ] **Step 3: Verify tests pass**

```bash
cargo test -p omnilang-core
```

- [ ] **Step 4: Commit**

```bash
git add crates/omnilang-core/src/ir/types.rs
git commit -m "fix: normalize type names case-insensitively in TypeKind::from()"
```

---

### Task 18: Rename ComponentIR.states → signals, StateIntent → SignalIntent

**Files:**
- Modify: `crates/omnilang-core/src/ir/types.rs` (rename fields and struct)
- Modify: `crates/omnilang-core/src/ir/lower/component.rs` (update lowering code)
- Modify: All codegen backends that reference `states`/`StateIntent` (~8 files)

**Interfaces:**
- Produces: Ubiquitous Language consistency — "signal" is the domain term everywhere

- [ ] **Step 1: Rename in IR type definitions**

In `crates/omnilang-core/src/ir/types.rs`:
- Rename `StateIntent` struct → `SignalIntent`
- Rename `ComponentIR.states: Vec<StateIntent>` → `ComponentIR.signals: Vec<SignalIntent>`

- [ ] **Step 2: Update lowering code**

In `crates/omnilang-core/src/ir/lower/component.rs`: replace all `StateIntent` with `SignalIntent`, all `.states` with `.signals`.

- [ ] **Step 3: Update codegen backends**

Search and replace across `omnilang/src/codegen/*/`:
```bash
grep -rn "\.states" omnilang/src/codegen/
grep -rn "StateIntent" omnilang/src/codegen/
```

Replace all occurrences. Update React, Flutter, OmniView, Vue, Dioxus, JavaScript, Rust, Dart, Python backends.

- [ ] **Step 4: Update lint rules**

In `crates/omnilang-core/src/semantic/lints/`: replace `StateIntent` → `SignalIntent`.

- [ ] **Step 5: Verify tests pass**

```bash
cargo test --workspace
```

- [ ] **Step 6: Commit**

```bash
git add -u
git commit -m "refactor: rename StateIntent→SignalIntent, ComponentIR.states→signals (ubiquitous language)"
```

---

### Task 19: Remove motions/timelines from ComponentIR (single source of truth)

**Files:**
- Modify: `crates/omnilang-core/src/ir/types.rs` (remove `motions` and `timelines` from `ComponentIR`)
- Modify: `crates/omnilang-core/src/ir/lower/component.rs` (stop populating component-level motions)
- Modify: `omnilang/src/codegen/*/` (all codegen backends: resolve motion refs via IRProgram lookup)

**Interfaces:**
- Consumes: `ModuleIR.motions`, `ModuleIR.timelines` as canonical location
- Produces: `ComponentIR` no longer duplicates motion ownership; codegen resolves via `IRProgram::find_motion()`

- [ ] **Step 1: Add find_motion/find_timeline lookup methods to IRProgram**

```rust
impl IRProgram {
    pub fn find_motion(&self, name: &str) -> Option<&MotionIntent> {
        self.modules.values().find_map(|m| m.motions.iter().find(|mo| mo.name == name))
    }

    pub fn find_timeline(&self, name: &str) -> Option<&TimelineIntent> {
        self.modules.values().find_map(|m| m.timelines.iter().find(|t| t.name == name))
    }
}
```

- [ ] **Step 2: Remove motions/timelines from ComponentIR**

In `crates/omnilang-core/src/ir/types.rs`, remove the lines:
```rust
pub motions: Vec<MotionIntent>,
pub timelines: Vec<TimelineIntent>,
```
from the `ComponentIR` struct definition.

- [ ] **Step 3: Update lowering code**

In `crates/omnilang-core/src/ir/lower/component.rs`, remove code that populates `ComponentIR.motions` and `ComponentIR.timelines`.

- [ ] **Step 4: Update all codegen backends**

Replace all `component.motions` / `component.timelines` accesses with `program.find_motion(name)` / `program.find_timeline(name)` lookups. Thread `&IRProgram` through codegen functions where needed.

- [ ] **Step 5: Update tests**

Remove `motions: vec![]` and `timelines: vec![]` from `ComponentIR` test initializers.

- [ ] **Step 6: Verify tests pass**

```bash
cargo test --workspace
```

- [ ] **Step 7: Commit**

```bash
git add -u
git commit -m "refactor: remove motions/timelines from ComponentIR — single source in ModuleIR"
```

---

### Task 21: Replace build_component_map cloning with references

**Files:**
- Modify: `crates/omnilang-core/src/codegen/shared/generator.rs:12-16` (return refs)
- Modify: `omnilang/src/codegen/flutter/mod.rs:45` (accept `&FxHashMap<&str, &ComponentIR>`)
- Modify: `omnilang/src/codegen/react/mod.rs:72` (same)

**Interfaces:**
- Produces: `build_component_map` returns borrowed references instead of clones

- [ ] **Step 1: Change build_component_map return type**

```rust
// Before:
pub fn build_component_map(ir: &IRProgram) -> FxHashMap<String, ComponentIR> {
    ir.all_components().map(|(k, v)| (k.to_string(), v.clone())).collect()
}

// After:
pub fn build_component_map(ir: &IRProgram) -> FxHashMap<&str, &ComponentIR> {
    ir.all_components()
}
```

- [ ] **Step 2: Update Flutter backend to accept references**

Change function signatures from `FxHashMap<String, ComponentIR>` to `FxHashMap<&str, &ComponentIR>` with appropriate lifetimes.

- [ ] **Step 3: Update React backend similarly**

Same pattern — accept references instead of owned values.

- [ ] **Step 4: Verify tests pass**

```bash
cargo test --workspace
```

- [ ] **Step 5: Commit**

```bash
git add crates/omnilang-core/src/codegen/shared/generator.rs \
        omnilang/src/codegen/flutter/mod.rs \
        omnilang/src/codegen/react/mod.rs
git commit -m "perf: replace ComponentIR cloning with references in build_component_map"
```

---

### Task 21: Remove Poiesis toast.ts wrapper (YAGNI)

**Files:**
- Modify: `poiesis/src/components/error/ErrorBoundary.tsx` (inline toast call)
- Delete: `poiesis/src/lib/toast.ts`

**Interfaces:**
- Produces: Single-call-site sonner wrapper removed; direct `toast.error()` call in ErrorBoundary

- [ ] **Step 1: Inline toast call at ErrorBoundary.tsx**

```typescript
// Remove: import { showToast, ToastType } from "../../lib/toast";
// Add: import { toast as sonnerToast } from "sonner";
// Replace: showToast(ToastType.error, message);
// With: sonnerToast.error(message);
```

- [ ] **Step 2: Delete toast.ts**

```bash
rm poiesis/src/lib/toast.ts
```

- [ ] **Step 3: Verify**

```bash
cd poiesis && npx tsc --noEmit --pretty false 2>&1 | grep -c "error"
```

Expected count decreases by any toast-related errors.

- [ ] **Step 4: Commit**

```bash
git add poiesis/src/components/error/ErrorBoundary.tsx
git rm poiesis/src/lib/toast.ts
git commit -m "refactor: inline sonner toast call, remove toast.ts wrapper (YAGNI)"
```

---

## Phase 4: TypeScript Quality — Zero-Error Baseline

### Task 22: Fix critical TypeScript type errors

**Files:**
- Modify: `poiesis/src/components/architecture/ArchitecturePreview.tsx` (already fixed in Task 3)
- Modify: `poiesis/src/components/design/ComponentDefPanel.tsx:400-418` (renderRow type narrowing)
- Modify: `poiesis/src/services/codegen/code-op-parser.ts:164,173,284,298,307` (SlotValue types)
- Modify: `poiesis/src/services/codegen/pipeline.ts:128` (el.style type narrowing)
- Modify: `poiesis/src/store/undo-apply.ts:210` (branches type narrowing)
- Modify: `poiesis/src/store/slices/design-system.ts:113-114` (variantRef narrowing)
- Modify: `poiesis/src/components/design/VariantEditor.tsx:1157-1203` (readonly → mutable)
- Modify: `poiesis/src/commands/rustCompiler.ts:177` (wasm export type)
- Modify: `poiesis/src/commands/compile-api.ts:90` (Window type cast)

**Interfaces:**
- Produces: Real TS2322/TS2345/TS4104 errors fixed; type narrowing applied

- [ ] **Step 1: Fix ComponentDefPanel.tsx renderRow type narrowing**

Add type guard at lines 400-418:
```typescript
// Before: (p: DefEntry) => ...
// Fix: narrow to DefEntry before calling handler
const renderRow = (entry: DefEntry | ComputedEntry) => {
  if ('type' in entry) {
    return renderPropRow(entry);    // DefEntry
  } else {
    return renderComputedRow(entry); // ComputedEntry
  }
};
```

- [ ] **Step 2: Fix code-op-parser.ts SlotValue mismatches**

At lines 164, 173, 284, 298, 307: ensure `SlotValue` type is used where expected, or explicitly cast string values through the SlotValue constructor.

- [ ] **Step 3: Fix pipeline.ts el.style narrowing**

At line 128, add type guard:
```typescript
if ('style' in el) {
  // el is now narrowed to the variant that has style
}
```

- [ ] **Step 4: Fix undo-apply.ts + design-system.ts type narrowing**

Add discriminated union checks on `nodeType` before accessing variant-specific properties.

- [ ] **Step 5: Fix VariantEditor.tsx readonly array mutations**

Replace mutations like `branches[0].values = [...]` with spread copies:
```typescript
branches[0] = { ...branches[0], values: [...newValues] };
```

- [ ] **Step 6: Fix rustCompiler.ts wasm export**

Replace `resolve_styles_multi_cached` (non-existent) with `resolve_styles_multi` or add runtime existence check.

- [ ] **Step 7: Fix compile-api.ts Window cast**

Add intermediate `unknown` cast:
```typescript
const win = window as unknown as Record<string, unknown>;
```

- [ ] **Step 8: Verify type errors reduced**

```bash
cd poiesis && npx tsc --noEmit 2>&1 | grep "^error" | wc -l
```

Target: fewer than 50 errors remaining (down from 108).

- [ ] **Step 9: Commit**

```bash
git add poiesis/src/
git commit -m "fix: resolve critical TypeScript type errors in codegen pipeline and components"
```

---

### Task 23: Fix unused variable/import TypeScript errors (39 TS6133)

**Files:**
- Multiple files across `poiesis/src/` with TS6133 errors

**Interfaces:**
- Produces: 39 fewer errors; zero unused declarations

- [ ] **Step 1: List all TS6133 errors**

```bash
cd poiesis && npx tsc --noEmit 2>&1 | grep "TS6133"
```

- [ ] **Step 2: Fix each — prefix with `_` or remove**

For each unused variable/import:
- If intentionally unused (e.g., type import for documentation), prefix with `_`
- If truly dead, remove the declaration entirely

Key files:
- `BubbleNode.tsx:35` — `size` unused
- `LockIcon.tsx:5` — entire import unused
- `ClassEditorSidebar.tsx:4` — `useMemo` unused
- `ClassPreview.tsx:9,63` — `ResizeHandles`, `HANDLE_SIZE` unused
- `DesignTabContent.tsx:43` — `activeDesign` unused
- `ResizeHandles.tsx:11` — `HANDLE_SIZE` unused

- [ ] **Step 3: Verify errors reduced**

```bash
cd poiesis && npx tsc --noEmit 2>&1 | grep "TS6133" | wc -l
```

Expected: 0

- [ ] **Step 4: Commit**

```bash
git add poiesis/src/
git commit -m "fix: remove 39 unused TypeScript declarations (TS6133)"
```

---

### Task 24: Fix remaining TypeScript errors

**Files:**
- Various files in `poiesis/src/`

**Interfaces:**
- Produces: `tsc --noEmit` passes with zero errors

- [ ] **Step 1: Fix missing CLEAR_BTN in BackgroundCard.tsx and BorderCard.tsx**

```typescript
// Add CLEAR_BTN constant or import:
const CLEAR_BTN = "Clear";
```

- [ ] **Step 2: Fix TokenEditor.tsx font TokenType**

Add `"font"` to the `TokenType` union or change to a valid member.

- [ ] **Step 3: Fix TokenEditor.tsx JSX namespace**

Add `import { JSX } from "react";` or replace `JSX.Element` with `React.ReactNode`.

- [ ] **Step 4: Fix remaining `readonly` array mutations**

Any remaining TS4104 errors: use spread copies instead of mutation.

- [ ] **Step 5: Verify zero TS errors**

```bash
cd poiesis && npx tsc --noEmit
```

Expected: Clean exit, zero errors.

- [ ] **Step 6: Commit**

```bash
git add poiesis/src/
git commit -m "fix: resolve remaining TypeScript errors — zero-error baseline established"
```

---

### Task 25: Fix ESLint v10 configuration

**Files:**
- Create: `poiesis/eslint.config.js`
- Modify: `poiesis/package.json` (remove legacy `eslintConfig`)

**Interfaces:**
- Produces: ESLint v10 functional; `pnpm lint` passes

- [ ] **Step 1: Create eslint.config.js**

```javascript
import tseslint from "typescript-eslint";
import reactHooks from "eslint-plugin-react-hooks";
import reactRefresh from "eslint-plugin-react-refresh";

export default tseslint.config(
  { ignores: ["dist", "node_modules"] },
  {
    extends: [
      ...tseslint.configs.recommended,
    ],
    files: ["**/*.{ts,tsx}"],
    plugins: {
      "react-hooks": reactHooks,
      "react-refresh": reactRefresh,
    },
    rules: {
      ...reactHooks.configs.recommended.rules,
      "react-refresh/only-export-components": ["warn", { allowConstantExport: true }],
      "@typescript-eslint/no-unused-vars": ["error", { argsIgnorePattern: "^_" }],
    },
  }
);
```

- [ ] **Step 2: Remove legacy eslintConfig from package.json**

Delete the `"eslintConfig"` block (lines 55-58).

- [ ] **Step 3: Verify lint passes**

```bash
cd poiesis && npx eslint . --ext .ts,.tsx
```

- [ ] **Step 4: Commit**

```bash
git add poiesis/eslint.config.js poiesis/package.json
git commit -m "fix: migrate ESLint config to v10 flat config format"
```

---

### Task 26: Replace 6 direct console.* calls with Logger

**Files:**
- Modify: `poiesis/src/pages/DesignPage.tsx:104,118,129`
- Modify: `poiesis/src/components/design/DesignTabContent.tsx:121`
- Modify: `poiesis/src/components/design/DesignTabBar.tsx:259,268,273,294`
- Modify: `poiesis/src/components/panels/inspector/ExpressionEditor.tsx:52,55`
- Modify: `poiesis/src/services/monaco/diagnostics.ts:59,99,159`
- Modify: `poiesis/src/components/canvas/code/CodeBlockEditor.tsx:147`

**Interfaces:**
- Produces: All logging goes through Logger abstraction

- [ ] **Step 1: Replace each direct console call**

Pattern:
```typescript
// Before:
console.warn("DesignPage: something happened", error);

// After:
import { Logger } from "../../lib/utils/logger";
const logger = new Logger("DesignPage");
logger.warn("something happened", error);
```

Apply to all 6 files at the listed line locations.

- [ ] **Step 2: Verify compilation**

```bash
cd poiesis && npx tsc --noEmit
```

- [ ] **Step 3: Commit**

```bash
git add poiesis/src/
git commit -m "fix: replace 6 direct console.* calls with Logger abstraction"
```

---

### Task 27: Add @types/node devDependency for test files

**Files:**
- Modify: `poiesis/package.json`

**Interfaces:**
- Produces: Node globals (`fs`, `path`, `__dirname`) resolve in test files

- [ ] **Step 1: Install @types/node**

```bash
cd poiesis && npm install --save-dev @types/node
```

- [ ] **Step 2: Mark Node-environment tests**

In test files using Node globals, add:
```typescript
// @vitest-environment node
```

Or in `vitest.config.js`, configure separate projects for node vs jsdom.

- [ ] **Step 3: Verify tests run**

```bash
cd poiesis && npx vitest run
```

- [ ] **Step 4: Commit**

```bash
git add poiesis/package.json poiesis/package-lock.json
git commit -m "fix: add @types/node for test Node globals"
```

---

## Phase 5: CI Governance + Documentation

### Task 28: Integrate eval framework into CI

**Files:**
- Modify: `.github/workflows/ci.yml`

**Interfaces:**
- Produces: CI runs `bash .claude/evals/eval-run-omnilang.sh`

- [ ] **Step 1: Add eval job to CI**

```yaml
  evals:
    name: AI Evals
    runs-on: ubuntu-latest
    needs: test
    steps:
      - uses: actions/checkout@v4
      - name: Run OmniLang evals
        run: bash .claude/evals/eval-run-omnilang.sh
        continue-on-error: true  # Report but don't block on eval failures initially
```

- [ ] **Step 2: Commit**

```bash
git add .github/workflows/ci.yml
git commit -m "ci: add AI eval framework to CI pipeline"
```

---

### Task 29: Add TypeScript type check to CI

**Files:**
- Modify: `.github/workflows/ci.yml`

**Interfaces:**
- Produces: CI runs `tsc --noEmit` on Poiesis

- [ ] **Step 1: Add TypeScript check job**

```yaml
  typescript:
    name: Type Check (Poiesis)
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with:
          node-version: 22
      - run: cd poiesis && npm ci
      - run: cd poiesis && npx tsc --noEmit
```

- [ ] **Step 2: Commit**

```bash
git add .github/workflows/ci.yml
git commit -m "ci: add TypeScript type check for Poiesis"
```

---

### Task 30: Add debug-window HTML sync check to CI

**Files:**
- Modify: `.github/workflows/ci.yml`

**Interfaces:**
- Produces: CI fails if the two debug-window HTML files diverge

- [ ] **Step 1: Add diff check**

```yaml
      - name: Check debug-window HTML sync
        run: |
          diff poiesis/src-tauri/debug-window/index.html poiesis/public/debug-window/index.html || \
            (echo "ERROR: debug-window HTML files differ! Sync them." && exit 1)
```

- [ ] **Step 2: Commit**

```bash
git add .github/workflows/ci.yml
git commit -m "ci: add debug-window HTML sync check"
```

---

### Task 31: Add Criterion benchmarks

**Files:**
- Create: `benches/lexer_bench.rs`
- Create: `benches/pipeline_bench.rs`
- Create: `benches/codegen_bench.rs`
- Modify: `Cargo.toml` (add `[[bench]]` targets + criterion dev-dependency)

**Interfaces:**
- Produces: `cargo bench` runs lexer, pipeline, and codegen benchmarks

- [ ] **Step 1: Add Criterion to workspace Cargo.toml**

```toml
[dev-dependencies]
criterion = { version = "0.5", features = ["html_reports"] }

[[bench]]
name = "lexer_bench"
harness = false

[[bench]]
name = "pipeline_bench"
harness = false

[[bench]]
name = "codegen_bench"
harness = false
```

- [ ] **Step 2: Create lexer benchmark**

```rust
// benches/lexer_bench.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use omnilang_core::lexer::tokenize;

fn bench_lexer_small(c: &mut Criterion) {
    let source = include_str!("../test/fixtures/golden/view-counter.omni");
    c.bench_function("lexer_small", |b| b.iter(|| tokenize(black_box(source))));
}

criterion_group!(benches, bench_lexer_small);
criterion_main!(benches);
```

- [ ] **Step 3: Create pipeline benchmark**

Benchmark full parse-to-IR for a 5-component project.

- [ ] **Step 4: Create codegen benchmark**

Benchmark React + OmniView codegen on representative IR.

- [ ] **Step 5: Verify benchmarks run**

```bash
cargo bench
```

- [ ] **Step 6: Commit**

```bash
git add benches/ Cargo.toml
git commit -m "perf: add Criterion benchmarks for lexer, pipeline, and codegen"
```

---

### Task 32: Add optimize.rs test coverage

**Files:**
- Modify: `crates/omnilang-core/src/ir/optimize.rs` (add test module)

**Interfaces:**
- Produces: 8+ test cases covering all dead-computed elimination edge cases

- [ ] **Step 1: Add edge case tests**

Add tests for:
- Empty component (no computeds)
- All computeds used → none eliminated
- All computeds unused → all eliminated
- Nested expression references (computed inside conditional/list)
- Computeds referenced through effects
- Computeds referenced through lifecycles
- Computeds referenced through view tree
- Circular reference: computed A references computed B, B references A

- [ ] **Step 2: Verify tests pass**

```bash
cargo test -p omnilang-core optimize
```

- [ ] **Step 3: Commit**

```bash
git add crates/omnilang-core/src/ir/optimize.rs
git commit -m "test: add 8+ edge case tests for optimize.rs dead-computed elimination"
```

---

### Task 33: Fix stdlib.md — document missing 5 modules

**Files:**
- Modify: `docs/stdlib.md`

**Interfaces:**
- Produces: Complete API reference for all 8 stdlib modules

- [ ] **Step 1: Document omni/json**

Add section covering `JSON.parse()`, `JSON.stringify()`.

- [ ] **Step 2: Document omni/http**

Add section covering `http.get()`, `http.post()`, `http.put()`, `http.delete()`.

- [ ] **Step 3: Document omni/array**

Add section covering `array.map()`, `array.filter()`, `array.reduce()`, `array.find()`, `array.concat()`.

- [ ] **Step 4: Document omni/console**

Add section covering `console.log()`, `console.warn()`, `console.error()`.

- [ ] **Step 5: Document omni/storage**

Add section covering `storage.get()`, `storage.set()`, `storage.remove()`, `storage.clear()`.

- [ ] **Step 6: Update docs/README.md line 35**

Change "待补充" status to "✅ Complete (8/8)".

- [ ] **Step 7: Commit**

```bash
git add docs/stdlib.md docs/README.md
git commit -m "docs: complete stdlib API reference (8/8 modules)"
```

---

### Task 34: Fix ADR-005 title + archive Rete.js docs

**Files:**
- Modify: `.architecture/decisions/adrs/ADR-005-ts-pipeline-parser-emitter-not-compiler.md:1`
- Create or modify: `docs/rete-docs/README.md` (add deprecation notice)

**Interfaces:**
- Produces: ADR-005 header correct; Rete.js docs marked as deprecated

- [ ] **Step 1: Fix ADR-005 header**

Change line 1 from `# ADR 1:` to `# ADR-005:`.

- [ ] **Step 2: Mark Rete.js docs as deprecated**

Create/update `docs/rete-docs/README.md`:
```markdown
# ⚠️ DEPRECATED — Rete.js Documentation

Rete.js has been removed from the Poiesis tech stack. These 71 files are retained
only as historical reference. See `docs/design/DESIGN_DOCS_INDEX.md` for current architecture.
```

- [ ] **Step 3: Align README.md and QUICKSTART.md example projects**

Audit `test/projects/` for active projects, update both docs to reference the same set.

- [ ] **Step 4: Commit**

```bash
git add .architecture/decisions/adrs/ADR-005-ts-pipeline-parser-emitter-not-compiler.md \
        docs/rete-docs/README.md docs/README.md QUICKSTART.md README.md
git commit -m "docs: fix ADR-005 title, mark Rete.js as deprecated, align example projects"
```

---

### Task 35: Fix remaining low-priority documentation issues

**Files:**
- Modify: `docs/CODE_REVIEW_REPORT_2026-07-06.md` (add `[History]` prefix)
- Modify: `docs/CODE_REVIEW_REPORT_2026-07-06_SUPPLEMENT.md` (add `[History]` prefix)
- Create: `docs/superpowers/README.md` (navigation guide)

**Interfaces:**
- Produces: Clean docs/ namespace

- [ ] **Step 1: Add history prefix to old review reports**

```markdown
# [History] Code Review Report — 2026-07-06
```

- [ ] **Step 2: Create docs/superpowers/README.md**

```markdown
# Superpowers Docs

- `specs/` — Design specifications and feature specs
- `plans/` — Implementation plans with task lists
- `reports/` — Generated reports and analysis
- `fixes/` — Fix plans and remediation docs
- `sdd/` — Task briefs (Software Design Documents)
```

- [ ] **Step 3: Commit**

```bash
git add docs/
git commit -m "docs: archive old review reports, add superpowers README"
```

---

### Task 36: Final verification — full workspace CI simulation

**Files:**
- None (verification only)

**Interfaces:**
- Consumes: All previous tasks
- Produces: Confirmation that ALL quality gates pass

- [ ] **Step 1: Run cargo test --workspace**

```bash
cargo test --workspace 2>&1 | tail -5
```

Expected: `test result: ok. N passed; 0 failed; 0 ignored`

- [ ] **Step 2: Run cargo clippy --workspace**

```bash
cargo clippy --workspace -- -D warnings
```

Expected: Clean exit (0 errors, 0 warnings)

- [ ] **Step 3: Run cargo fmt --check**

```bash
cargo fmt --check --all
```

Expected: No diff output

- [ ] **Step 4: Run TypeScript type check**

```bash
cd poiesis && npx tsc --noEmit
```

Expected: Clean exit (0 errors)

- [ ] **Step 5: Run ESLint**

```bash
cd poiesis && npx eslint . --ext .ts,.tsx
```

Expected: Clean exit (0 errors, 0 warnings)

- [ ] **Step 6: Run eval framework**

```bash
bash .claude/evals/eval-run-omnilang.sh
```

Expected: All PASS or WARN, no FAIL

- [ ] **Step 7: Commit final state (if any changes)**

```bash
git status
# Commit any remaining changes
```

---

## Execution Order & Dependencies

```
Phase 1: Unblock Tests (parallel where possible)
  Task 1 ──┬── Task 2 ──┬── Phase 2
  Task 3 ──┘            │
  Task 4 ───────────────┤
  Task 5 ───────────────┘

Phase 2: Complete P35
  Task 5 ── Task 6 ── Task 7 ── Task 8 ── Task 9 ── Task 10 ── Task 11
                                                                   │
  Task 12 (parallel)                                               │
                                                                   │
Phase 3: Rust Quality                                               │
  Task 13 ── Task 14 ── Task 15 ── Task 16 ── Task 17 ── Task 18 ── Task 19 ── Task 20
                                                                        │
  Task 21 (parallel — Poiesis)                                           │
                                                                        │
Phase 4: TypeScript Quality                                              │
  Task 22 ── Task 23 ── Task 24 ── Task 25 ── Task 26 ── Task 27       │
                                                                        │
Phase 5: CI + Docs (all parallel)                                        │
  Task 28 ── Task 29 ── Task 30 ── Task 31 ── Task 32 ── Task 33 ── Task 34 ── Task 35
                                                                                    │
  Task 36 (final verification) ◄────────────────────────────────────────────────────┘
```

**Total tasks**: 36 | **Estimated total effort**: ~2-3 days | **Phases**: 5
