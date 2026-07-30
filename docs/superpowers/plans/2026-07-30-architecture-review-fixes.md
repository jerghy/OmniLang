# Architecture Review Fixes Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix all Bucket 1 (real bugs) and Bucket 2.5 (hardenable) findings from the 2026-07-30 11-specialist architecture review of OmniView — ~30 fixes across 4 phases.

**Architecture:** Phase 1: 15 S-effort quick wins (~2h). Phase 2: 6 M-effort structural fixes (~4h). Phase 3: 4 runtime/testing fixes (~4h). Phase 4: 11 long-term items deferred to future sprint.

**Tech Stack:** Rust (omnilang-core, omnilang), TypeScript/JS (runtime, React codegen), Node.js (CI smoke tests). 853+ tests stay green throughout.

**Exclusions (Bucket 2 — intentional ADR trade-offs, do NOT fix):**
- Signal batching (ADR-028 D3, ADR-037 gates on profiling data)
- OmniView adopting FormatExpr trait (ADR-028 D2, ADR-012)
- Output minification (ADR-028 D4)
- Tree-shaking/code splitting (ADR-028 D5)

**Source:** `.architecture/reviews/omniview-framework-architecture-2026-07-30.md`

---

## Phase 1: Quick Wins (S effort, ~2 hours)

### Task 1: Fix production `.unwrap()` in component_html.rs

**Files:** `omnilang/src/codegen/omniview/component_html.rs:120-133`

- [ ] **Step 1: Replace `.keys()` + `.get().unwrap()` with `.iter()`**

Change:
```rust
for prop_name in child_comp.prop_defaults.keys() {
    let default_js = self.expr_to_js(
        child_comp.prop_defaults.get(prop_name).unwrap()
    );
```
To:
```rust
for (prop_name, default_value) in &child_comp.prop_defaults {
    let default_js = self.expr_to_js(default_value);
```

- [ ] **Step 2: Test** — `cargo test -p omnilang -- omniview` → PASS
- [ ] **Step 3: Commit** — `fix: replace production unwrap with iter() in component_html.rs`

---

### Task 2: Remove dead IR fields (is_copy, SignalRefIR.owner, StoreAction.event_data)

**Files:** `crates/omnilang-core/src/ir/signals.rs:16`, `types.rs:398-402,450-463`, `lower/component.rs:34`, `lower/stmt.rs`, `lower/view.rs:258,345`

- [ ] **Step 1: Remove `is_copy: bool`** from SignalIntent + `is_copy: false` from all constructions
- [ ] **Step 2: Remove `owner: Option<String>`** from SignalRefIR → `{ pub name: String }`. Update ~10 lowering sites + tests via `grep -rn "SignalRefIR" crates/`
- [ ] **Step 3: Remove `event_data: Option<EventDataKind>`** from StoreAction variant. Update accessor match arms (event_data(), needs_prevent_default(), needs_stop_propagation()). Remove `event_data: None` from lowering
- [ ] **Step 4: Build** — `cargo build` → fix compiler errors
- [ ] **Step 5: Test** — `cargo test` → PASS
- [ ] **Step 6: Commit** — `refactor: remove dead IR fields`

---

### Task 3: Add debug_assert! to format_block_expr default

**Files:** `crates/omnilang-core/src/codegen/shared/expr.rs:141-145`

- [ ] **Step 1: Add assertion before `self.format_expr(tail)`:**

```rust
fn format_block_expr(&self, statements: &[StatementIR], tail: &ExprIR) -> String {
    debug_assert!(
        statements.is_empty(),
        "format_block_expr with non-empty statements — backend must override"
    );
    self.format_expr(tail)
}
```

- [ ] **Step 2: Test** — `cargo test -p omnilang-core` → PASS
- [ ] **Step 3: Commit** — `fix: add debug_assert to format_block_expr default`

---

### Task 4: Gate typescript.rs behind feature flag

**Files:** `omnilang/src/codegen/omniview/mod.rs:19`, `omnilang/Cargo.toml`

- [ ] **Step 1: `#[cfg(feature = "codegen-dts")] pub mod typescript;`** in mod.rs
- [ ] **Step 2: `codegen-dts = []`** in [features] in Cargo.toml
- [ ] **Step 3: Verify** default build, `--features codegen-dts` build both succeed
- [ ] **Step 4: Commit** — `refactor: gate typescript.rs behind codegen-dts feature flag`

---

### Task 5: Remove ComponentOutput.html/.css reserved fields

**Files:** `omnilang/src/codegen/omniview/mod.rs`

- [ ] **Step 1: Remove `pub html: String,` and `pub css: String,`** from struct
- [ ] **Step 2: Remove all `html: String::new(), css: String::new()`** constructions
- [ ] **Step 3: Test** — `cargo test -p omnilang -- omniview` → PASS
- [ ] **Step 4: Commit** — `refactor: remove reserved ComponentOutput fields`

---

### Task 6: Fix @for key fallback for object items (React)

**Files:** `omnilang/src/codegen/react/view.rs:722-727`

- [ ] **Step 1: Change `key={item}` → `key={String(item)}`**

```rust
// BEFORE: format!(" key={{{}}}", item_name)
// AFTER:  format!(" key={{String({})}}", item_name)
```

- [ ] **Step 2: Test** — `cargo test -p omnilang -- react` → PASS
- [ ] **Step 3: Commit** — `fix: use String() coercion for @for key fallback`

---

### Task 7: Wrap store hooks return in useMemo (React)

**Files:** `omnilang/src/codegen/react/mod.rs` (gen_store_hook)

- [ ] **Step 1: Find return** — `grep -n "return {" omnilang/src/codegen/react/mod.rs`
- [ ] **Step 2: Wrap** — `return { increment, count }` → `return useMemo(() => ({ increment, count }), [count])`
- [ ] **Step 3: Test** — `cargo test -p omnilang -- react` → PASS
- [ ] **Step 4: Commit** — `fix: wrap store hooks return in useMemo`

---

### Task 8: Fix push_str(&format!(...)) → write!()

**Files:** ~17 files in crates/ and omnilang/

- [ ] **Step 1: Find** — `grep -rn 'push_str(&format!' omnilang/src/ crates/ --include="*.rs"`
- [ ] **Step 2: Convert** — each `s.push_str(&format!("...", a))` → `let _ = write!(s, "...", a)` (add `use std::fmt::Write`)
- [ ] **Step 3: Verify** — `cargo clippy -- -D clippy::format_push_string` → zero warnings
- [ ] **Step 4: Commit** — `perf: replace push_str(&format!(...)) with write!()`

---

### Task 9: Remove to_lowercase() allocation in TypeKind::from

**Files:** `crates/omnilang-core/src/ir/types.rs:298-312`

- [ ] **Step 1: Replace `s.to_lowercase().as_str()`** with direct match including PascalCase variants
- [ ] **Step 2: Test** — `cargo test -p omnilang-core` → PASS
- [ ] **Step 3: Commit** — `perf: avoid to_lowercase() allocation in TypeKind::from`

---

### Task 10: Inline MotionBodyIR single-variant enum

**Files:** `crates/omnilang-core/src/ir/types_motion.rs:22-24`, match sites

- [ ] **Step 1: `pub enum MotionBodyIR { Keyframe(KeyframeIR) }` → `pub type MotionBodyIR = KeyframeIR;`**
- [ ] **Step 2: Fix match** — `grep -rn "MotionBodyIR::Keyframe"` → simplify destructuring
- [ ] **Step 3: Test** — `cargo test` → PASS
- [ ] **Step 4: Commit** — `refactor: inline MotionBodyIR single-variant enum`

---

### Task 11: Remove Flutter from defaults + disable all-backends CI

**Files:** `omnilang/Cargo.toml`, `.github/workflows/ci.yml`

- [ ] **Step 1: Remove `codegen-flutter`** from `default = [...]`
- [ ] **Step 2: Add `if: false`** to test-all-backends CI job
- [ ] **Step 3: Verify** — `cargo build` → SUCCESS
- [ ] **Step 4: Commit** — `refactor: remove Flutter from defaults, disable all-backends CI`

---

### Task 12: Add content validation before editor eval()

**Files:** `omnilang/src/codegen/omniview/editor.rs`

- [ ] **Step 1: Before `(0, eval)(factoryCode)`**, insert:

```javascript
if (typeof factoryCode !== 'string' || !/^\s*function\s*\(/.test(factoryCode)) {
    __omni_reportError('warn', 'editor', 'Invalid factory code');
    return;
}
```

- [ ] **Step 2: Test** — `cargo test -p omnilang -- omniview` → PASS (update snapshots)
- [ ] **Step 3: Commit** — `security: add content validation before editor eval()`

---

### Task 13: Add minimum CSP to editor mode

**Files:** `omnilang/src/codegen/omniview/utils.rs`

- [ ] **Step 1: When include_csp=false**, emit minimal `<meta>` CSP: `connect-src 'self'; frame-src 'none'; object-src 'none'`
- [ ] **Step 2: Test** — `cargo test -p omnilang -- omniview` → PASS (update snapshots)
- [ ] **Step 3: Commit** — `security: add minimum CSP to editor mode`

---

### Task 14: Add __mergeTouches usage check

**Files:** `omnilang/src/codegen/omniview/mod.rs`

- [ ] **Step 1: Add `project_uses_touch_events(ir)`** scanning event names for "touch"/"pointer"
- [ ] **Step 2: Gate `__mergeTouches` emission** with usage check
- [ ] **Step 3: Test** → PASS
- [ ] **Step 4: Commit** — `perf: conditionally emit __mergeTouches`

---

### Task 15: Extract const slices for duplicated attribute lists

**Files:** `omnilang/src/codegen/omniview/view/element.rs:102-315`

- [ ] **Step 1: Add module-level const slices:**

```rust
const HTML_BOOLEAN_ATTRS: &[&str] = &[
    "disabled", "checked", "required", "readonly", "selected",
    "autofocus", "multiple", "controls", "loop", "muted", "async", "defer",
];
const HTML_URL_ATTRS: &[&str] = &[
    "href", "src", "srcset", "action", "formaction", "poster",
];
```

- [ ] **Step 2: Replace inline lists** with `.contains()` calls
- [ ] **Step 3: Test** → PASS (behavior unchanged)
- [ ] **Step 4: Commit** — `refactor: extract const slices for attribute lists`

---

## Phase 2: Structural Fixes (M effort, ~4 hours)

### Task 16: Return all parse errors instead of truncating

**Files:** `crates/omnilang-core/src/parser/mod.rs:18-39`, `analyse.rs`

- [ ] **Step 1: Change `parse()` signature** — `Result<Program, ParseError>` → `(Program, Vec<ParseError>)`
- [ ] **Step 2: Update callers** in analyse.rs, WASM, CLI — collect all errors
- [ ] **Step 3: Test** → PASS
- [ ] **Step 4: Commit** — `feat: return all parse errors`

---

### Task 17: Populate suggestion field on diagnostics

**Files:** `error.rs`, `semantic/validate_types.rs`, `semantic/resolve.rs`

- [ ] **Step 1: Add `suggest_similar_name(attempt, known_names)`** with Levenshtein ≤ 3
- [ ] **Step 2: Add to type mismatches** — `.suggestion(format!("expected {}, got {}", ...))`
- [ ] **Step 3: Add to undefined refs** — `.suggestion("did you mean 'X'?")`
- [ ] **Step 4: Test** → PASS
- [ ] **Step 5: Commit** — `feat: populate diagnostic suggestion field`

---

### Task 18: Build reverse source map index

**Files:** `crates/omnilang-core/src/sourcemap/mod.rs`, `omnilang/src/codegen/omniview/mod.rs`

- [ ] **Step 1: Add `ReverseSourceMap`** types (generated_position → source_position)
- [ ] **Step 2: Build during codegen**, add to OmniViewOutput
- [ ] **Step 3: Test** → PASS
- [ ] **Step 4: Commit**

---

### Task 19: Build motion name index on IRProgram

**Files:** `crates/omnilang-core/src/ir/types.rs`, `lower/mod.rs`

- [ ] **Step 1: Add `motion_index: FxHashMap<String, MotionIntent>`** (skip serialization)
- [ ] **Step 2: Build during lowering** after all modules
- [ ] **Step 3: Replace `find_motion()` O(n×m)** → `self.motion_index.get(name)`
- [ ] **Step 4: Test** → PASS
- [ ] **Step 5: Commit**

---

### Task 20: Box ViewTree::Element variant

**Files:** `crates/omnilang-core/src/ir/types.rs:~667`, match sites

- [ ] **Step 1: `Element(ElementIR)` → `Element(Box<ElementIR>)`**, remove `#[allow(clippy::large_enum_variant)]`
- [ ] **Step 2: Fix match** — `ref el` patterns work with Box
- [ ] **Step 3: Test + clippy** → PASS, no large_enum_variant
- [ ] **Step 4: Commit**

---

### Task 21: Extract shared walk_expr/walk_stmt/walk_view

**Files:** Create: `crates/omnilang-core/src/ir/walk.rs`, Modify: `optimize.rs`, `lower/view.rs`

- [ ] **Step 1: Create walk.rs** with 3 callback-based functions covering all IR variants
- [ ] **Step 2: Update optimize.rs** — replace 3 duplicate walkers
- [ ] **Step 3: Update lower/view.rs** — replace walk_expr_event_fields
- [ ] **Step 4: Test** → PASS (snapshots unchanged)
- [ ] **Step 5: Commit**

---

## Phase 3: Runtime & Testing (M-L effort)

### Task 22: Fix React motion effects for conditional rendering

**Files:** `omnilang/src/codegen/react/motion/mod.rs`, `transforms.rs`

- [ ] **Step 1: Add MutationObserver** fallback after mount-only useEffect for new `[data-motion-*]` elements
- [ ] **Step 2: Test** → PASS
- [ ] **Step 3: Commit**

---

### Task 23: @for fragment batching (DocumentFragment)

**Files:** `omnilang/src/codegen/omniview/list.rs:357-360`

- [ ] **Step 1: Replace per-item `insertBefore`** with `DocumentFragment` accumulation + single insert
- [ ] **Step 2: Update snapshots** → PASS
- [ ] **Step 3: Commit**

---

### Task 24: Add OmniView runtime smoke test to CI

**Files:** Create: `tests/omniview_runtime_smoke.rs`, `tests/smoke_test.js`, Modify: `.github/workflows/ci.yml`

- [ ] **Step 1: Create Node.js smoke test** using jsdom — verify no errors, root has content
- [ ] **Step 2: Add Rust test** compiling fixture + running smoke test
- [ ] **Step 3: Add CI job**
- [ ] **Step 4: Test locally** → PASS
- [ ] **Step 5: Commit**

---

### Task 25: Extract NameResolver from OmniViewGenerator

**Files:** Create: `omnilang/src/codegen/omniview/name_resolver.rs`, Modify: `mod.rs` + all references

- [ ] **Step 1: Create NameResolver struct** with 8 fields + `take()`/`restore()`
- [ ] **Step 2: Replace 9 fields in OmniViewGenerator** with `name_resolver: NameResolver`
- [ ] **Step 3: Update all** `self.field` → `self.name_resolver.field` across codebase
- [ ] **Step 4: Test — snapshots byte-identical** → PASS
- [ ] **Step 5: Commit**

---

## Phase 4: Batch Structural Cleanups (S effort)

- [ ] **Task 26: Rename ConstructionKind::TagMapping → Component** — ~21 files, mechanical rename
- [ ] **Task 27: Add disambiguating doc comments** to both VariantIR types (logic.rs vs design.rs)
- [ ] **Task 28: Move design.rs test block** (279 lines) to `tests_regression/design.rs`
- [ ] **Task 29: Extract raw JS bridge code** into `include_str!()` files (editor.rs, debug.rs)
- [ ] **Task 30: Add error message truncation** — `String(error).substring(0, 200)` in runtime.rs

---

## Phase 5: Long-Term (defer to future sprint)

| # | Task | Effort |
|---|------|--------|
| L1 | Split generate_component (421 lines) into 8-10 methods | M |
| L2 | Split emit_list_dom (390 lines) into 4-5 methods | M |
| L3 | Formalize 6-buffer phase machine with OutputPhase enum | M |
| L4 | Refactor React motion to ref-based approach | L |
| L5 | Add view-aware error recovery for nested view trees | M |
| L6 | Move ViewTree transformations to shared infra | S |
| L7 | Add typed StyleClassValue/InlineStyleEntry IR structs | M |
| L8 | Add Array/Map native variants to TypeKind | M-L |
| L9 | Re-enable TypeScript declaration generation (opt-in) | S |
| L10 | Generate CSS module .d.ts files alongside .module.css | S |
| L11 | Emit event handler parameter types (not unknown) | S |

---

## Execution Strategy

**Before starting:** `cargo test` passes clean (853+ tests).

**Phase 1:** All 15 tasks independent — can parallelize. **Phase 2:** Tasks 16-21 sequential (IR dependencies). **Phase 3:** Independent. **Phase 4:** Independent.

**After each task:** `cargo test && cargo clippy` stays green.

**Pure refactors** (10, 15, 20, 21, 26-29): snapshots must be byte-identical.

**Generated-JS changes** (6, 7, 12-14, 22, 23): verify learn-omniview renders correctly via `cargo run -p omnic -- compile test/projects/learn-omniview/src/main.omni -t omniview`

---

*Plan generated 2026-07-30 from 11-specialist review at `.architecture/reviews/omniview-framework-architecture-2026-07-30.md`*
