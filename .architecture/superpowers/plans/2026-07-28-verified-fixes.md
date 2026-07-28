# Architecture Review Fix Plan — Verified HIGH + Critical MEDIUM Findings

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix all 5 confirmed HIGH findings from the 2026-07-28 comprehensive breadth review + deep verification, plus selected MEDIUM findings that are small-effort/high-impact.

**Architecture:** Two-phase plan. Phase 1 fixes the 2 missing DCE fixes (preventing `activeStyle is not defined` / `fullName is not defined` runtime errors) + 3 type checker gaps + 1 CI contradiction + 1 dead code removal — all verified as real issues. Phase 2 adds regression tests and defense-in-depth improvements.

**Tech Stack:** Rust (omnilang-core + omnilang crates), cargo test/clippy/fmt for verification.

**Prerequisite knowledge:** ADR-011 (type checker architecture), ADR-003 (codegen target prioritization), ADR-015 (deep verification methodology).

---

### Task 1: Fix DCE — Track CallExpr function name in walk_expr_for_idents

**Files:**
- Modify: `crates/omnilang-core/src/ir/optimize.rs:28-32`

**Context:** The dead code eliminator's `walk_expr_for_idents` ignores the function name in `CallExpr`, so computed properties that reference functions (e.g., `computed greeting: String = fullName()` where `fullName` is a fn) have their function calls not tracked as dependencies. This causes the DCE to incorrectly eliminate computed properties that depend on function calls.

- [ ] **Step 1: Add name tracking to CallExpr**

In `crates/omnilang-core/src/ir/optimize.rs`, change line 28 from:
```rust
ExprIR::CallExpr { args, .. } => {
    for arg in args {
        walk_expr_for_idents(arg, used);
    }
}
```
To:
```rust
ExprIR::CallExpr { name, args } => {
    used.insert(name.clone());
    for arg in args {
        walk_expr_for_idents(arg, used);
    }
}
```

- [ ] **Step 2: Build and run existing tests**

```bash
cargo test -p omnilang-core -- ir::optimize 2>&1 | tail -5
```
Expected: all existing optimizer tests pass (should pass — this only adds tracking, doesn't remove anything).

- [ ] **Step 3: Commit**

```bash
git add crates/omnilang-core/src/ir/optimize.rs
git commit -m "fix(ir): track CallExpr function name in DCE walk_expr_for_idents

Previously the dead code eliminator ignored function names in CallExpr
nodes, causing computed properties that referenced user-defined functions
to be incorrectly eliminated. This produced 'fullName is not defined'
runtime errors in learn-omniview.

Fixes ADR-014 bridge point D1: DCE + CallExpr interaction."
```

---

### Task 2: Fix DCE — Walk JSON style fields for $signal/$concat/$call references

**Files:**
- Modify: `crates/omnilang-core/src/ir/optimize.rs` — add `walk_json_for_signals` function + call sites

**Context:** Dynamic style bindings (`style: { color: someSignal }`, `styleClasses: { active: isActive }`) are stored as JSON objects in `ElementIR.inline_style` and `ElementIR.style_classes`. The DCE only walks `ExprIR` trees for identifier references but ignores these JSON fields, so signals used ONLY in style bindings are incorrectly marked as dead and eliminated. This produces `activeStyle is not defined` runtime errors.

- [ ] **Step 1: Add `walk_json_for_signals` function**

In `crates/omnilang-core/src/ir/optimize.rs`, after `walk_expr_for_idents` (after line ~95), add:

```rust
/// Recursively walk a JSON value looking for `$signal` and `$signal_concat`
/// references, collecting the signal names into `used`. This is needed because
/// dynamic style bindings (style: { key: expr }) are stored as JSON, not ExprIR.
fn walk_json_for_signals(value: &serde_json::Value, used: &mut FxHashSet<String>) {
    match value {
        serde_json::Value::Object(map) => {
            if let Some(signal_name) = map.get("$signal").and_then(|v| v.as_str()) {
                used.insert(signal_name.to_string());
                return;
            }
            if let Some(arr) = map.get("$signal_concat").and_then(|v| v.as_array()) {
                if let Some(first) = arr.first() {
                    walk_json_for_signals(first, used);
                }
                return;
            }
            if let Some(arr) = map.get("$concat").and_then(|v| v.as_array()) {
                for item in arr {
                    walk_json_for_signals(item, used);
                }
                return;
            }
            if let Some(call) = map.get("$call") {
                if let Some(args) = call.get("args").and_then(|v| v.as_array()) {
                    for arg in args {
                        walk_json_for_signals(arg, used);
                    }
                }
                // Also track the function name as a dependency
                if let Some(name) = call.get("name").and_then(|v| v.as_str()) {
                    used.insert(name.to_string());
                }
                return;
            }
            for v in map.values() {
                walk_json_for_signals(v, used);
            }
        }
        serde_json::Value::Array(arr) => {
            for item in arr {
                walk_json_for_signals(item, used);
            }
        }
        _ => {}
    }
}
```

- [ ] **Step 2: Add call sites in `walk_view_for_idents`**

In the same file, inside `walk_view_for_idents`, after the motion walk block (after line 268, before the `if let Some(ref scroll)` line), add:

```rust
// Walk inline_style and style_classes JSON for $signal / $signal_concat refs.
// Dynamic styles like style: { color: mySignal } are stored as JSON,
// not ExprIR, so the standard walk_expr_for_idents path misses them.
if let Some(ref style) = el.inline_style {
    walk_json_for_signals(style, used);
}
if let Some(ref classes) = el.style_classes {
    walk_json_for_signals(classes, used);
}
```

- [ ] **Step 3: Build and run existing tests**

```bash
cargo test -p omnilang-core -- ir::optimize 2>&1 | tail -5
```
Expected: all existing optimizer tests pass.

- [ ] **Step 4: Commit**

```bash
git add crates/omnilang-core/src/ir/optimize.rs
git commit -m "fix(ir): walk JSON style fields for signal refs in DCE

Dynamic style bindings (style: { color: signalName }) and styleClasses
are stored as serde_json::Value in ElementIR, not ExprIR. The DCE's
walk_view_for_idents only traversed ExprIR trees, missing signals
referenced exclusively through $signal/$signal_concat/$concat/$call
JSON tags. This caused 'activeStyle is not defined' errors.

Adds walk_json_for_signals to recursively search JSON for signal refs.

Fixes ADR-014 bridge point D5: DCE + JSON style representation interaction."
```

---

### Task 3: Fix assignment target existence validation

**Files:**
- Modify: `crates/omnilang-core/src/semantic/resolve.rs:260`

**Context:** `Stmt::Assign { value, .. }` ignores the `target` field — the variable name being assigned to is never checked against the symbol table. A typo like `coutn = 5` (for `count = 5`) compiles silently. Note: ADR-011 line 70 exempts assignment TYPE CHECKING but does NOT exempt target existence validation — these are separate concerns.

- [ ] **Step 1: Add target existence check**

In `crates/omnilang-core/src/semantic/resolve.rs`, change line 260 from:
```rust
Stmt::Assign { value, .. } => self.resolve_expr_refs(value, vctx),
```
To:
```rust
Stmt::Assign { target, value, span } => {
    // Validate assignment target exists in symbol table.
    // This is separate from type checking (deferred per ADR-011).
    match target {
        AssignTarget::Name(name) => {
            if !self.symbols.contains(name) {
                self.diagnostics.push(Diagnostic::error(
                    DiagnosticKind::E0001,
                    &format!("Assignment to undeclared variable '{}'", name),
                    span.clone(),
                ));
            }
        }
        AssignTarget::Field { object, field: _ } => {
            self.resolve_expr_refs(object, vctx);
        }
        AssignTarget::Index { object, index } => {
            self.resolve_expr_refs(object, vctx);
            self.resolve_expr_refs(index, vctx);
        }
    }
    self.resolve_expr_refs(value, vctx);
}
```

- [ ] **Step 2: Build and run semantic tests**

```bash
cargo test -p omnilang-core -- semantic 2>&1 | tail -10
```
Expected: existing semantic tests pass. New behavior: `coutn = 5` (undeclared variable) now produces E0001 diagnostic.

- [ ] **Step 3: Verify with a quick parser test that undeclared assignment is caught**

```bash
cargo test -p omnilang-core 2>&1 | tail -5
```
Expected: all 514 tests pass. Verify no regression.

- [ ] **Step 4: Commit**

```bash
git add crates/omnilang-core/src/semantic/resolve.rs
git commit -m "fix(semantic): validate assignment target existence in resolve pass

Previously Stmt::Assign ignored the target field, so typos like
'coutn = 5' (for 'count = 5') compiled without error. Now validates
AssignTarget::Name against the symbol table and emits E0001 for
undeclared variables.

Note: assignment type checking remains deferred per ADR-011 known
limitations. This fix only adds existence validation, which is a
separate concern not excused by ADR-011."
```

---

### Task 4: Fix signal initializer type checking

**Files:**
- Modify: `crates/omnilang-core/src/semantic/resolve.rs:50-53`

**Context:** Signal initializers (e.g., `signal count: i32 = computeLabel()` where `computeLabel()` returns String) have their references resolved but never type-checked. `VarDecl` initializers ARE type-checked (line 219 calls `check_assignment`), but signal inits are in a separate loop that only calls `resolve_expr_refs`.

- [ ] **Step 1: Add type check after signal init resolution**

In `crates/omnilang-core/src/semantic/resolve.rs`, change lines 50-53 from:
```rust
// Resolve signal initializers
for sig in &comp.signals {
    self.resolve_expr_refs(&sig.init, &mut vctx);
}
```
To:
```rust
// Resolve signal initializers
for sig in &comp.signals {
    self.resolve_expr_refs(&sig.init, &mut vctx);
    // Type-check: signal initializer must match declared type
    let scope = self.symbols.current_scope_index();
    let expected = SemanticType::from_type_annotation(&sig.type_ann);
    let actual = infer_expr_type(&sig.init, &self.symbols, scope);
    if let Err(msg) = check_assignment(&expected, &actual) {
        self.diagnostics.push(Diagnostic::error(
            DiagnosticKind::E0010,
            &format!(
                "Signal '{}' type mismatch: {}",
                sig.name, msg
            ),
            sig.span.clone(),
        ));
    }
}
```

- [ ] **Step 2: Build and run all tests**

```bash
cargo test -p omnilang-core 2>&1 | tail -5
```
Expected: all 514 tests pass. New behavior: signal initializers with type mismatches now produce E0010 errors.

- [ ] **Step 3: Commit**

```bash
git add crates/omnilang-core/src/semantic/resolve.rs
git commit -m "fix(semantic): add type checking for signal initializers

Previously signal initializer expressions had references resolved but
were never type-checked against the declared signal type. This allowed
'signal count: i32 = computeLabel()' (where computeLabel returns String)
to compile without error.

Now calls check_assignment after resolving signal init expressions,
matching the existing behavior for VarDecl initializers at line 219.

This gap was not covered by ADR-011's known limitations."
```

---

### Task 5: Remove dead O(M*N) scan in project_uses_motion

**Files:**
- Modify: `omnilang/src/codegen/omniview/mod.rs:428-431`

**Context:** The `project_uses_motion()` function scans all modules for motions/timelines at line 425-426, returning true if found. Lines 429-430 re-scan all modules from inside a per-component loop — but by the time execution reaches that point, ALL modules are guaranteed to have empty motions/timelines (otherwise line 426 would have already returned). The `.all()` scans on 429-430 always return `true`, `!true` is `false`, so these conditions never trigger. Only `hero_defs` on line 431 has any effect.

- [ ] **Step 1: Remove dead lines**

In `omnilang/src/codegen/omniview/mod.rs`, change lines 428-433 from:
```rust
        for comp in module.components.values() {
            if !ir.modules.values().all(|m| m.motions.is_empty())
                || !ir.modules.values().all(|m| m.timelines.is_empty())
                || !comp.hero_defs.is_empty()
            {
```
To:
```rust
        for comp in module.components.values() {
            if !comp.hero_defs.is_empty()
            {
```

- [ ] **Step 2: Build and run OmniView tests**

```bash
cargo test -p omnilang --features codegen-omniview 2>&1 | tail -5
```
Expected: all OmniView tests pass.

- [ ] **Step 3: Commit**

```bash
git add omnilang/src/codegen/omniview/mod.rs
git commit -m "fix(codegen): remove dead O(M*N) scan in project_uses_motion

Lines 429-430 re-scanned all modules for motions/timelines from inside
a per-component loop. However, line 425-426 already returns true if ANY
module has non-empty motions/timelines. When execution reaches the
component loop, all modules are guaranteed empty — making the .all()
scans always return true, !true = false, never triggering.

Only hero_defs check was live. Removed the 2 dead lines."
```

---

### Task 6: Remove all-backends CI job (ADR-003 compliance)

**Files:**
- Modify: `.github/workflows/ci.yml:161-171`

**Context:** ADR-003 (accepted 2026-07-26) explicitly decided to remove `all-backends` from CI. The `test-all-backends` job still runs on every push, contradicting the decision and wasting CI minutes testing 9K lines of inactive backend code.

- [ ] **Step 1: Remove or comment out the job**

In `.github/workflows/ci.yml`, remove or comment out lines 161-171:
```yaml
  # test-all-backends:
  #   runs-on: ubuntu-latest
  #   needs: rust
  #   steps:
  #     - uses: actions/checkout@v4
  #     - uses: actions-rust-lang/setup-rust-toolchain@v1
  #       with:
  #         toolchain: stable
  #     - uses: Swatinem/rust-cache@v2
  #     - name: Test all backends
  #       run: cargo test -p omnilang --features all-backends
```

- [ ] **Step 2: Commit**

```bash
git add .github/workflows/ci.yml
git commit -m "fix(ci): remove all-backends CI job per ADR-003

ADR-003 explicitly decided to remove all-backends from CI to reduce
IR change blast radius and CI time. The job was left in the workflow,
contradicting the accepted decision. Non-P0 backends remain in-tree
behind feature flags but are no longer tested on every push."
```

---

### Task 7: Add regression tests for ADR-014 bridge-point fixes

**Files:**
- Create: `crates/omnilang-core/src/ir/optimize_tests.rs` (or add to existing `optimize.rs` test module)

**Context:** ADR-014 documented 8 cross-subsystem bugs found only by manual browser testing of learn-omniview. All 789 unit tests passed while these bugs existed. None of the 8 bugs have regression tests. This task adds targeted tests for the 2 DCE bugs fixed in Tasks 1 & 2.

- [ ] **Step 1: Add DCE regression test for JSON style signal references**

Add to the test module in `crates/omnilang-core/src/ir/optimize.rs` (in the existing `#[cfg(test)] mod tests` block):

```rust
/// Regression: DCE must not eliminate computeds whose signals are referenced
/// only through JSON style bindings ($signal in inline_style/style_classes).
#[test]
fn test_dce_preserves_computed_referenced_by_json_style() {
    use crate::ir::types::*;
    use crate::ir::optimize::optimize_component;

    let mut comp = ComponentIR {
        name: "Test".into(),
        computed: {
            let mut map = FxHashMap::default();
            map.insert(
                "activeStyle".into(),
                ComputedIR {
                    body: Some(ExprIR::IdentifierExpr {
                        name: "isActive".into(),
                        type_ann: None,
                        source_span: None,
                    }),
                    deps: vec![],
                    is_pure: true,
                    source_span: None,
                },
            );
            map
        },
        view: Some(ViewTree::Element(ElementIR {
            tag: Some("div".into()),
            inline_style: Some(serde_json::json!({
                "color": {"$signal": "activeStyle"}
            })),
            style_classes: None,
            children: vec![],
            construction_kind: ConstructionKind::Static,
            class_map_bindings: FxHashMap::default(),
            source_span: None,
            motion: None,
            scroll: None,
            drag: None,
            hero: None,
        })),
        signals: {
            let mut map = FxHashMap::default();
            map.insert(
                "isActive".into(),
                SignalIR {
                    type_ann: TypeIR {
                        kind: TypeKind::Bool,
                        is_nullable: false,
                    },
                    init: Some(ExprIR::LiteralExpr {
                        value: serde_json::Value::Bool(false),
                    }),
                    source_span: None,
                },
            );
            map
        },
        ..Default::default()
    };

    let optimized = optimize_component(&comp);
    // activeStyle must NOT be eliminated — it's referenced via JSON $signal
    assert!(
        optimized.computed.contains_key("activeStyle"),
        "activeStyle computed was incorrectly eliminated; its signal 'isActive' is referenced only through JSON inline_style $signal"
    );
}
```

- [ ] **Step 2: Run the new test**

```bash
cargo test -p omnilang-core -- test_dce_preserves_computed_referenced_by_json_style
```
Expected: PASS (verifies the fix from Task 2 works).

- [ ] **Step 3: Add DCE regression test for CallExpr function name tracking**

In the same test module:

```rust
/// Regression: DCE must not eliminate computeds that depend on user-defined
/// function calls. The CallExpr name must be tracked as a dependency.
#[test]
fn test_dce_preserves_computed_with_fn_call_dependency() {
    use crate::ir::types::*;
    use crate::ir::optimize::optimize_component;

    let mut comp = ComponentIR {
        name: "Test".into(),
        fns: {
            let mut map = FxHashMap::default();
            map.insert(
                "fullName".into(),
                FunctionIR {
                    params: vec![],
                    return_type: TypeIR {
                        kind: TypeKind::String,
                        is_nullable: false,
                    },
                    body: BlockIR {
                        statements: vec![],
                        source_span: None,
                    },
                    is_async: false,
                    source_span: None,
                },
            );
            map
        },
        computed: {
            let mut map = FxHashMap::default();
            map.insert(
                "greeting".into(),
                ComputedIR {
                    body: Some(ExprIR::CallExpr {
                        name: "fullName".into(),
                        args: vec![],
                        type_ann: None,
                        source_span: None,
                    }),
                    deps: vec![],
                    is_pure: true,
                    source_span: None,
                }),
            );
            map
        },
        ..Default::default()
    };

    let optimized = optimize_component(&comp);
    // greeting must NOT be eliminated — it depends on fn fullName
    assert!(
        optimized.computed.contains_key("greeting"),
        "greeting computed was incorrectly eliminated; it depends on fn 'fullName' via CallExpr"
    );
}
```

- [ ] **Step 4: Run the new test**

```bash
cargo test -p omnilang-core -- test_dce_preserves_computed_with_fn_call_dependency
```
Expected: PASS (verifies the fix from Task 1 works).

- [ ] **Step 5: Run full test suite**

```bash
cargo test --workspace --exclude poiesis 2>&1 | tail -5
```
Expected: all 789+ tests pass, 0 failures.

- [ ] **Step 6: Run clippy**

```bash
cargo clippy --workspace --exclude poiesis -- -D warnings 2>&1 | tail -5
```
Expected: no warnings (0 errors).

- [ ] **Step 7: Commit**

```bash
git add crates/omnilang-core/src/ir/optimize.rs
git commit -m "test(ir): add regression tests for ADR-014 DCE bridge-point bugs

Adds two regression tests:
1. DCE preserves computed properties whose signals are referenced only
   through JSON style bindings ($signal in inline_style)
2. DCE preserves computed properties that depend on user-defined
   function calls (CallExpr name tracking)

These cover the two bridge-point bugs documented in ADR-014 that
previously required manual browser verification to detect."
```

---

### Task 8: Add recursion depth guard in lower_expr_to_json_from_ir

**Files:**
- Modify: `crates/omnilang-core/src/ir/lower/expr.rs:543-597`

**Context:** `lower_expr_to_json_from_ir` is recursive without depth guard. While the parser's `MAX_RECURSION_DEPTH=256` bounds AST depth, this function operates post-lowering on IR and could theoretically encounter deeply nested JSON from style expressions. Defense-in-depth improvement.

- [ ] **Step 1: Add depth counter with fallback**

Change the function signature and body in `crates/omnilang-core/src/ir/lower/expr.rs`:

At line 543, change:
```rust
pub(super) fn lower_expr_to_json_from_ir(expr: &ExprIR) -> serde_json::Value {
```
To:
```rust
pub(super) fn lower_expr_to_json_from_ir(expr: &ExprIR) -> serde_json::Value {
    lower_expr_to_json_from_ir_depth(expr, 0)
}

/// Internal implementation with recursion depth guard.
/// Returns Value::Null if max depth (256) is exceeded.
fn lower_expr_to_json_from_ir_depth(expr: &ExprIR, depth: u32) -> serde_json::Value {
    const MAX_DEPTH: u32 = 256;
    if depth > MAX_DEPTH {
        return serde_json::Value::Null;
    }
    let recurse = |e: &ExprIR| lower_expr_to_json_from_ir_depth(e, depth + 1);
```

Then update all recursive calls to use `recurse`:
- Line 552: `let obj = recurse(object);`
- Line 559-560: `recurse(left)`, `recurse(right)`
- Line 568: `args.iter().map(|a| recurse(a)).collect::<Vec<_>>()`
- Line 591: `map.insert(key, recurse(v));`

And update the final `match` at line 595 to close the inner function:
```rust
    match expr {
        // ... all existing arms using recurse instead of direct recursion ...
        other => serde_json::Value::String(format!("{:?}", other)),
    }
}
```

- [ ] **Step 2: Build and run lowering tests**

```bash
cargo test -p omnilang-core -- ir::lower 2>&1 | tail -5
```
Expected: all lowering tests pass.

- [ ] **Step 3: Commit**

```bash
git add crates/omnilang-core/src/ir/lower/expr.rs
git commit -m "fix(ir): add recursion depth guard to lower_expr_to_json_from_ir

Adds a depth counter (max 256) to prevent theoretical stack overflow
from deeply nested JSON expressions. While the parser's own depth limit
provides the primary guard, this defense-in-depth prevents crash if the
IR contains unexpectedly deep expression trees from style bindings.

Returns serde_json::Value::Null when depth exceeded rather than
panicking."
```

---

### Task 9: Final verification — full test suite + clippy + fmt

**Files:** None (verification only)

- [ ] **Step 1: Run full test suite**

```bash
cargo test --workspace --exclude poiesis 2>&1 | tail -5
```
Expected: all tests pass, 0 failures.

- [ ] **Step 2: Run clippy with deny warnings**

```bash
cargo clippy --workspace --exclude poiesis -- -D warnings 2>&1 | tail -5
```
Expected: 0 errors.

- [ ] **Step 3: Check formatting**

```bash
cargo fmt --all -- --check
```
Expected: no formatting issues.

- [ ] **Step 4: Report final status**

Expected output:
```
All quality gates pass:
- cargo test --workspace: PASS (789+ tests, 0 failures)
- cargo clippy --workspace -- -D warnings: PASS (0 warnings)
- cargo fmt --all -- --check: PASS
```

---

## Execution Order Summary

| Order | Task | Priority | Effort | Depends On |
|-------|------|----------|--------|------------|
| 1 | DCE: CallExpr name tracking | CRITICAL | 5 min | — |
| 2 | DCE: JSON style signal walk | CRITICAL | 15 min | — |
| 3 | Assignment target validation | CRITICAL | 10 min | — |
| 4 | Signal init type checking | CRITICAL | 10 min | — |
| 5 | Dead code removal in project_uses_motion | CRITICAL | 2 min | — |
| 6 | Remove all-backends CI | CRITICAL | 5 min | — |
| 7 | ADR-014 regression tests | HIGH | 30 min | 1, 2 |
| 8 | Recursion depth guard | MEDIUM | 15 min | — |
| 9 | Final verification | — | 5 min | 1-8 |

**Total estimated effort**: ~1.5 hours for all 9 tasks.

**Commit strategy**: One commit per task (9 commits). Each task produces a self-contained, revertible change.
