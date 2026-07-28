# Breadth Review Critical Fixes Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix 5 HIGH-severity findings from the 2026-07-28 breadth architecture review, all confirmed genuine by deep verification against 10 ADRs.

**Architecture:** All 5 fixes are independent — each touches a different subsystem. C1 adds a missing optimizer call to the single-file compilation path. C2 fixes a data-loss bug in tagged union variant lowering. C3 guards WASM tests behind a target-arch cfg. C4 moves `unsafe-eval` from production CSP to dev-only. C5 wires the already-implemented `check_call` function into the resolve pass so function argument types are actually validated.

**Tech Stack:** Rust (omnilang-core, omnilang, omnilang-wasm), Tauri (tauri.conf.json)

**Prerequisites:** `cargo test --workspace` must pass before starting (currently all green, 0 failures).

---

## File Structure

| File | Role | Action |
|------|------|--------|
| `crates/omnilang-core/src/lib.rs` | Core pipeline entry point | C1: Add optimizer call after IR lowering |
| `omnilang/src/lib.rs` | Facade pipeline entry point | C1: Add optimizer call after IR lowering |
| `crates/omnilang-core/src/ir/logic.rs` | IR type definitions | C2: Change VariantIR.payload → fields |
| `crates/omnilang-core/src/ir/lower_logic.rs` | AST→IR lowering for logic | C2: Map all variant fields |
| `omnilang/src/codegen/rust/visitor.rs` | Rust codegen backend | C2: Update payload→fields |
| `omnilang/src/codegen/javascript/visitor.rs` | JS codegen backend | C2: Update payload→fields |
| `omnilang-wasm/src/lib.rs` | WASM bindings + tests | C3: Add wasm32 cfg gate |
| `poiesis/src-tauri/tauri.conf.json` | Tauri security config | C4: Move unsafe-eval to devCsp |
| `crates/omnilang-core/src/semantic/resolve.rs` | Semantic resolver | C5: Wire check_call into FuncCall |
| `crates/omnilang-core/src/semantic/validate_types.rs` | Type checker | C5: Remove dead_code allow |

---

### Task 1: C1 — Wire optimizer into single-file compilation path

**Files:**
- Modify: `crates/omnilang-core/src/lib.rs:115-116`
- Modify: `omnilang/src/lib.rs:61-62`

**Background:** The IR optimizer (`ir/optimize.rs`) runs in `analyse_multi` at `analyse.rs:322` but is never called in `run_pipeline`. This means single-file compilation (used by WASM/editor via `compile()`, `compile_omniview()`, `resolve_styles()`) produces unoptimized IR with dead computeds and unreachable code intact. Two `run_pipeline` functions exist — one in core, one in facade — both need the fix.

- [ ] **Step 1: Add optimizer call to core crate's `run_pipeline`**

In `crates/omnilang-core/src/lib.rs`, change lines 115-116 from:

```rust
    let ir = ir::lower(&program, &mut semantics);
    Ok(PipelineOutput {
```

To:

```rust
    let mut ir = ir::lower(&program, &mut semantics);
    crate::ir::optimize::optimize(&mut ir);
    Ok(PipelineOutput {
```

- [ ] **Step 2: Add optimizer call to facade crate's `run_pipeline`**

In `omnilang/src/lib.rs`, change lines 61-62 from:

```rust
    let ir = ir::lower(&program, &mut semantics);
    Ok(PipelineOutput {
```

To:

```rust
    let mut ir = ir::lower(&program, &mut semantics);
    omnilang_core::ir::optimize::optimize(&mut ir);
    Ok(PipelineOutput {
```

- [ ] **Step 3: Verify compilation**

Run: `cargo build --workspace`
Expected: Compiles successfully (0 errors). The optimizer module is already public and the function signature accepts `&mut IRProgram`.

- [ ] **Step 4: Run full test suite**

Run: `cargo test --workspace`
Expected: All tests pass (0 failures). The optimizer is a pure removal pass — it should not change behavior of any existing test.

- [ ] **Step 5: Commit**

```bash
git add crates/omnilang-core/src/lib.rs omnilang/src/lib.rs
git commit -m "fix: run IR optimizer in single-file compilation path

The optimizer (dead computed elimination + unreachable code removal)
was only called in analyse_multi, not in run_pipeline. This meant
single-file compilation via compile() / compile_omniview() / WASM
produced unoptimized IR.

Found by Compiler Architect in 2026-07-28 breadth review (C1)."
```

---

### Task 2: C2 — Fix multi-field tagged union variant payload truncation

**Files:**
- Modify: `crates/omnilang-core/src/ir/logic.rs:49-55`
- Modify: `crates/omnilang-core/src/ir/lower_logic.rs:56-58`
- Modify: `omnilang/src/codegen/rust/visitor.rs:54`
- Modify: `omnilang/src/codegen/javascript/visitor.rs:88`

**Background:** The AST's `Variant` has `fields: Vec<StructField>` — supporting multiple named fields per variant. But the IR's `VariantIR` has `payload: Option<TypeIR>` — a single optional type. During lowering at `lower_logic.rs:58`, only `v.fields.first()` is mapped — all subsequent fields are silently dropped. This is a data-loss bug for tagged unions like `variant Ok { value: i32, message: String }`.

- [ ] **Step 1: Change `VariantIR` to use a `Vec<FieldIR>`**

In `crates/omnilang-core/src/ir/logic.rs`, replace lines 49-55:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariantIR {
    pub name: String,
    pub payload: Option<TypeIR>,
    #[serde(default)]
    pub source_span: Option<Span>,
}
```

With:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariantIR {
    pub name: String,
    pub fields: Vec<FieldIR>,
    #[serde(default)]
    pub source_span: Option<Span>,
}
```

- [ ] **Step 2: Update lowering to map all variant fields**

In `crates/omnilang-core/src/ir/lower_logic.rs`, replace lines 56-58:

```rust
            .map(|v| VariantIR {
                name: v.name.clone(),
                payload: v.fields.first().map(|f| lower_type(&f.type_ann)),
                source_span: None,
            })
```

With:

```rust
            .map(|v| VariantIR {
                name: v.name.clone(),
                fields: v.fields.iter().map(|f| FieldIR {
                    name: f.name.clone(),
                    r#type: lower_type(&f.type_ann),
                    visibility: Visibility::Public,
                    source_span: None,
                }).collect(),
                source_span: None,
            })
```

- [ ] **Step 3: Update Rust codegen visitor**

In `omnilang/src/codegen/rust/visitor.rs`, find the reference to `variant.payload` at line 54. The code currently looks like:

```rust
if let Some(payload) = &variant.payload {
```

Change to iterate over `variant.fields`:

```rust
if !variant.fields.is_empty() {
```

Adjust the body to handle `Vec<FieldIR>` instead of a single `TypeIR`. The surrounding context determines what the payload was used for — likely generating a tuple struct variant like `VariantName(PayloadType)`. With multiple fields, this becomes `VariantName { field1: Type1, field2: Type2 }`.

- [ ] **Step 4: Update JavaScript codegen visitor**

In `omnilang/src/codegen/javascript/visitor.rs`, find the reference to `variant.payload.is_some()` at line 88. Change the check to `!variant.fields.is_empty()` and update any payload-type access to iterate over fields.

- [ ] **Step 5: Check for all remaining `.payload` references**

Run: `grep -rn "\.payload" --include="*.rs" omnilang/src/codegen/ crates/omnilang-core/src/codegen/`
Expected: Find all remaining references. Update any codegen backend that accesses `variant.payload` to use `variant.fields` instead.

- [ ] **Step 6: Verify compilation**

Run: `cargo build --workspace`
Expected: Compiles successfully after fixing all `.payload` references.

- [ ] **Step 7: Run full test suite**

Run: `cargo test --workspace`
Expected: All tests pass. Any test that constructs a `VariantIR` with `payload: Some(...)` must be updated to use `fields: vec![FieldIR { ... }]`.

- [ ] **Step 8: Commit**

```bash
git add crates/omnilang-core/src/ir/logic.rs \
        crates/omnilang-core/src/ir/lower_logic.rs \
        omnilang/src/codegen/rust/visitor.rs \
        omnilang/src/codegen/javascript/visitor.rs
git commit -m "fix: support multi-field tagged union variant payloads

VariantIR previously stored only a single payload: Option<TypeIR>.
The lowering mapped only v.fields.first(), silently dropping all
subsequent fields. Changed to fields: Vec<FieldIR> to match the
AST's Variant.fields: Vec<StructField> and the expression-level
ExprIR::VariantLiteral { fields: Vec<(String, ExprIR)> }.

Found by Domain Expert in 2026-07-28 breadth review (C2)."
```

---

### Task 3: C3 — Guard WASM tests with wasm32 target gate

**Files:**
- Modify: `omnilang-wasm/src/lib.rs:660-661`

**Background:** The `omnilang-wasm` crate's `#[cfg(test)] mod tests` calls `serde_wasm_bindgen::to_value()` and `serde_wasm_bindgen::from_value()` unconditionally. These panic with `STATUS_STACK_BUFFER_OVERRUN` on native x86_64 because wasm-bindgen imported functions don't exist outside wasm32. This breaks `cargo test --workspace` for all developers on native platforms. The CI correctly runs these tests via `wasm-pack test --node` (Linux-only, non-blocking).

- [ ] **Step 1: Add wasm32 target gate to the test module**

In `omnilang-wasm/src/lib.rs`, change line 660-661 from:

```rust
#[cfg(test)]
mod tests {
```

To:

```rust
#[cfg(all(test, target_arch = "wasm32"))]
mod tests {
```

- [ ] **Step 2: Verify `cargo test --workspace` no longer crashes on native**

Run: `cargo test --workspace`
Expected: All tests pass. No `STATUS_STACK_BUFFER_OVERRUN` crash. The omnilang-wasm test executable should either not be built or report 0 tests (filtered out by cfg).

- [ ] **Step 3: Verify WASM tests still compile under wasm-pack**

Run: `wasm-pack test --node omnilang-wasm` (on a system with wasm-pack installed, or verify via CI)
Expected: Tests compile and pass under wasm32 target.

- [ ] **Step 4: Commit**

```bash
git add omnilang-wasm/src/lib.rs
git commit -m "fix: guard WASM tests behind #[cfg(target_arch = \"wasm32\")]

serde-wasm-bindgen panics on native targets because wasm-bindgen
imported functions don't exist outside wasm32. This crashed
cargo test --workspace for all developers on Windows/Mac/Linux.

Found by Systems Architect in 2026-07-28 breadth review (C3)."
```

---

### Task 4: C4 — Move `unsafe-eval` from production CSP to devCsp

**Files:**
- Modify: `poiesis/src-tauri/tauri.conf.json:22`

**Background:** The production CSP in `tauri.conf.json` includes `'unsafe-eval'` in `script-src`. Combined with the broad Tauri FS capabilities (`D:/**`, `C:/**` — intentional per ADR-004 Decision 2), an eval()-based XSS in the main webview could lead to full drive access. The `devCsp` field already has `'unsafe-eval'` for development. The production CSP should restrict to `'self'` only for scripts.

- [ ] **Step 1: Remove `unsafe-eval` from production CSP**

In `poiesis/src-tauri/tauri.conf.json`, change line 22 from:

```json
      "csp": "default-src 'self'; script-src 'self' 'unsafe-eval'; style-src 'self' 'unsafe-inline'; connect-src 'self'; img-src 'self' data:; font-src 'self'; form-action 'none'; frame-ancestors 'none'; base-uri 'self'; object-src 'none'",
```

To:

```json
      "csp": "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; connect-src 'self'; img-src 'self' data:; font-src 'self'; form-action 'none'; frame-ancestors 'none'; base-uri 'self'; object-src 'none'",
```

The `devCsp` on line 23 already has `'unsafe-eval'` — no change needed there:

```json
      "devCsp": "default-src 'self'; script-src 'self' 'unsafe-inline' 'unsafe-eval'; style-src 'self' 'unsafe-inline'; connect-src 'self'; img-src 'self' data:; font-src 'self'; form-action 'none'; frame-ancestors 'none'; base-uri 'self'; object-src 'none'"
```

- [ ] **Step 2: Verify the production build works without eval**

Build the Poiesis production frontend and check for any CSP violations:

```bash
cd poiesis && pnpm build
```

Check the Vite/React production output does not require `eval()`. React 19+ production builds do not use `eval()` — only dev builds use it for source maps and hot reload. If the build succeeds without eval-related console errors, the change is safe.

- [ ] **Step 3: Commit**

```bash
git add poiesis/src-tauri/tauri.conf.json
git commit -m "fix(security): remove unsafe-eval from production CSP

Production CSP had 'unsafe-eval' in script-src, enabling eval()-based
XSS in the Tauri main webview. Combined with broad FS capabilities,
this created a drive-access escalation path.

Dev CSP already has 'unsafe-eval' — no dev workflow impact.

Found by Security Specialist in 2026-07-28 breadth review (C4)."
```

---

### Task 5: C5 — Wire `check_call` into `resolve_expr_refs`

**Files:**
- Modify: `crates/omnilang-core/src/semantic/resolve.rs:359-363`
- Modify: `crates/omnilang-core/src/semantic/validate_types.rs:240`

**Background:** The `check_call` function in `validate_types.rs` has a complete implementation for function argument type validation (parameter count check, per-argument type compatibility), but it is guarded by `#[allow(dead_code)]` and never called. The `resolve_expr_refs` handler for `Expr::FuncCall` (line 359-363) resolves references within the function expression and arguments, but never checks argument types against parameter types. To wire it, we need to: look up the function name in the symbol table, find the `fn:<name>` scope containing parameter symbols, extract their types, infer argument types, and call `check_call`.

- [ ] **Step 1: Remove `#[allow(dead_code)]` from `check_call`**

In `crates/omnilang-core/src/semantic/validate_types.rs`, change line 240 from:

```rust
#[allow(dead_code)]
pub fn check_call(
```

To:

```rust
pub fn check_call(
```

- [ ] **Step 2: Wire `check_call` into the `FuncCall` handler**

In `crates/omnilang-core/src/semantic/resolve.rs`, replace lines 359-363:

```rust
            Expr::FuncCall { func, args, .. } => {
                self.resolve_expr_refs(func, vctx);
                for a in args {
                    self.resolve_expr_refs(a, vctx);
                }
            }
```

With:

```rust
            Expr::FuncCall { func, args, .. } => {
                self.resolve_expr_refs(func, vctx);
                for a in args {
                    self.resolve_expr_refs(a, vctx);
                }

                // Validate function argument types against parameter types
                if let Expr::VarRef { name: fn_name, .. } = func.as_ref() {
                    if let Some(fn_sym) = vctx.symbols.lookup_in_current_scope(fn_name) {
                        if fn_sym.kind == SymbolKind::Function
                            || fn_sym.kind == SymbolKind::Action
                        {
                            // Collect parameter types from the fn:<name> scope
                            let fn_scope_name = format!(
                                "{}:{}",
                                if fn_sym.kind == SymbolKind::Action { "action" } else { "fn" },
                                fn_name
                            );
                            if let Some(scope_idx) =
                                vctx.symbols.find_scope_by_name(&fn_scope_name)
                            {
                                let param_types: Vec<SemanticType> = vctx
                                    .symbols
                                    .get_scope(scope_idx)
                                    .map(|scope| {
                                        scope
                                            .symbols
                                            .values()
                                            .filter(|s| s.kind == SymbolKind::Param)
                                            .map(|s| {
                                                s.type_ann
                                                    .as_ref()
                                                    .map(|t| SemanticType::parse_type_string(t))
                                                    .unwrap_or(SemanticType::Any)
                                            })
                                            .collect()
                                    })
                                    .unwrap_or_default();

                                // Infer argument types
                                let arg_types: Vec<SemanticType> = args
                                    .iter()
                                    .map(|a| infer_expr_type(a, &vctx.symbols))
                                    .collect();

                                // Get declared return type from function symbol
                                let return_type = fn_sym
                                    .type_ann
                                    .as_ref()
                                    .map(|t| SemanticType::parse_type_string(t))
                                    .unwrap_or(SemanticType::Void);

                                if let Err(msg) = check_call(
                                    fn_name,
                                    &param_types,
                                    &arg_types,
                                    &return_type,
                                ) {
                                    vctx.diagnostics.push(Diagnostic::error(
                                        msg,
                                        DiagnosticKind::TypeMismatch,
                                        // Use the span of the first argument, or func span
                                        args.first()
                                            .and_then(|a| self.expr_span(a))
                                            .unwrap_or(Span::default()),
                                    ));
                                }
                            }
                        }
                    }
                }
            }
```

- [ ] **Step 3: Add the `lookup_in_current_scope` helper if missing**

Check if `SymbolTable::lookup_in_current_scope` exists:

```bash
grep -n "fn lookup_in_current_scope\|fn lookup\b" crates/omnilang-core/src/semantic/symbols.rs
```

If it doesn't exist, add it:

```rust
/// Look up a symbol name in the current scope only (not parent scopes).
pub fn lookup_in_current_scope(&self, name: &str) -> Option<&Symbol> {
    self.scopes[self.current_scope].symbols.get(name)
}
```

- [ ] **Step 4: Add `find_scope_by_name` helper if missing**

Check if it exists:

```bash
grep -n "fn find_scope_by_name" crates/omnilang-core/src/semantic/symbols.rs
```

If it doesn't exist, add it:

```rust
/// Find a scope index by its name.
pub fn find_scope_by_name(&self, name: &str) -> Option<usize> {
    self.scope_by_name.get(name).copied()
}
```

- [ ] **Step 5: Verify compilation**

Run: `cargo build --workspace`
Expected: Compiles successfully. Fix any issues with imports (add `use crate::semantic::types::SemanticType;` and `use crate::semantic::symbols::SymbolKind;` at the top of resolve.rs if not already imported).

- [ ] **Step 6: Run full test suite**

Run: `cargo test --workspace`
Expected: All tests pass. The type checker is additive — existing tests that don't have type errors should be unaffected.

- [ ] **Step 7: Write a quick smoke test for function call type checking**

Create a minimal `.omni` file with a type error and verify the compiler now catches it:

```bash
echo 'fn add(a: i32, b: i32) -> i32 { a + b }
component App {
    view {
        Text { "{{add("hello", true)}}" }
    }
}' > /tmp/test_type_check.omni
cargo run -p omnic -- compile /tmp/test_type_check.omni
```

Expected: The compiler should report a type mismatch error for argument 1 (`"hello"` is `String`, expected `i32`).

- [ ] **Step 8: Commit**

```bash
git add crates/omnilang-core/src/semantic/resolve.rs \
        crates/omnilang-core/src/semantic/validate_types.rs \
        crates/omnilang-core/src/semantic/symbols.rs
git commit -m "feat(semantic): wire check_call into resolve_expr_refs

The check_call function (validate_types.rs) had a complete
implementation for function argument type validation but was
guarded by #[allow(dead_code)] and never called. This meant
users could pass wrong argument types to functions silently.

Now FuncCall resolution looks up the function's parameter types
from the symbol table, infers argument types, and validates
compatibility. Parameter types are collected from the fn:<name>
scope where they are stored as SymbolKind::Param entries.

Found independently by 4 specialists in 2026-07-28 breadth review (C5)."
```

---

### Task 6: Verification — Run full quality gates

**Files:**
- None (verification only)

- [ ] **Step 1: Run full test suite**

Run: `cargo test --workspace`
Expected: All tests pass, 0 failures.

- [ ] **Step 2: Run clippy**

Run: `cargo clippy --workspace -- -D warnings`
Expected: 0 warnings.

- [ ] **Step 3: Check formatting**

Run: `cargo fmt --check`
Expected: No formatting issues (or fix the one pre-existing issue in `semantic/collect.rs:227`).

- [ ] **Step 4: Run eval baseline check**

Run: `bash .claude/evals/eval-run-omnilang.sh`
Expected: All checks pass. Some baselines are outdated (BASELINE_TESTS=227 vs actual ~737+) — this is tracked as I11 for a separate fix.

- [ ] **Step 5: Final commit (if any format fixes)**

If cargo fmt produced changes:

```bash
git add -u
git commit -m "style: cargo fmt after critical fixes"
```

---

## Self-Review

### 1. Spec coverage

| Finding | Task | Status |
|---------|------|--------|
| C1: Optimizer bypass | Task 1 | ✅ Covered — adds `optimize()` call to both `run_pipeline` copies |
| C2: Variant payload truncation | Task 2 | ✅ Covered — changes `payload: Option<TypeIR>` to `fields: Vec<FieldIR>` |
| C3: WASM test crash | Task 3 | ✅ Covered — adds `#[cfg(target_arch = "wasm32")]` |
| C4: CSP unsafe-eval | Task 4 | ✅ Covered — removes from production CSP, devCsp already has it |
| C5: check_call dead | Task 5 | ✅ Covered — wires into `resolve_expr_refs` with symbol table lookup |

### 2. Placeholder scan

- Task 2 Step 3-4: The Rust and JavaScript visitor updates say "adjust the body" — this is not a placeholder, it's guidance to read the actual file and adapt. The exact code depends on how `payload` was used in context (the grep showed 2 references, both manageable).
- Task 5 Step 3-4: Conditionally adds `lookup_in_current_scope` and `find_scope_by_name` if missing — the plan checks whether they exist first.
- No TBD, TODO, or "add appropriate error handling" patterns found.

### 3. Type consistency

- `VariantIR.fields: Vec<FieldIR>` — `FieldIR` is defined in the same module (`logic.rs:32-38`) with fields `name: String`, `r#type: TypeIR`, `visibility: Visibility`, `source_span: Option<Span>`. Used consistently in Task 2 lowering code.
- `check_call` signature: `(&str, &[SemanticType], &[SemanticType], &SemanticType) -> Result<SemanticType, String>` — matched in Task 5 call site.
- `SymbolTable::lookup_in_current_scope` returns `Option<&Symbol>` — matched in Task 5 with `if let Some(fn_sym)`.
- `SymbolTable::find_scope_by_name` returns `Option<usize>` — matched with `if let Some(scope_idx)`.
- `SemanticType::parse_type_string` takes `&str` and returns `SemanticType` — matched with `fn_sym.type_ann.as_ref()`.
- `Diagnostic::error` takes `(String, DiagnosticKind, Span)` — matched in Task 5.
- `Span::default()` exists — fallback for when expr span can't be determined.
