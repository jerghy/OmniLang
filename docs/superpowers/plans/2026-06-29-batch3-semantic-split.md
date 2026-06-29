# Batch 3: Semantic Module Split — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Split `semantic/lints.rs` (2025 code) into a `lints/` directory with per-rule files, and `semantic/mod.rs` (2942 code) into per-pass files. Every resulting file ≤800 lines.

**Architecture:**
- **lints.rs → `lints/`:** One file per lint rule, each containing the rule struct + `impl LintRule` + `#[cfg(test)] mod tests`. Shared helpers stay in `helpers.rs`. The `mod.rs` holds the `LintRule` trait, runner, and `all_rules()` registry.
- **mod.rs → per-pass:** `collect.rs` (Pass 1), `resolve.rs` (Pass 2), `validate.rs` (Pass 3-4). Standalone motion helpers go to `motion.rs`. `mod.rs` becomes the entry point.

**Tech Stack:** Rust 2021, cargo test

**Safety net:** `cargo test --lib` must return "375 passed; 0 failed" after every commit.

**Key mechanics:** Same `mv` + re-export pattern. Tests move WITH their rule (not to a separate tests.rs) since each rule file is small enough.

---

### Task 1: Split lints.rs — shared infrastructure

**Files:**
- Create: `src/semantic/lints/mod.rs`
- Create: `src/semantic/lints/helpers.rs`
- Delete: `src/semantic/lints.rs`

- [ ] **Step 1: Create directory and move file**

```bash
mkdir -p omnilang/src/semantic/lints
mv omnilang/src/semantic/lints.rs omnilang/src/semantic/lints/mod.rs
```

- [ ] **Step 2: Extract helpers.rs**

From `mod.rs`, move the following shared helper sections to `helpers.rs`:
- AST name reference collectors (lines ~1185-1435): functions that collect identifiers from AST nodes for use by multiple rules
- Store/Prop/Computed validation helpers (lines ~1436-2026): shared validation logic

Create `src/semantic/lints/helpers.rs`:
```rust
//! Shared helpers for lint rules: AST collectors, store/prop validation.
use std::collections::{HashMap, HashSet};
use crate::ir::IRProgram;
use crate::parser::ast::*;

// ... (all shared helper functions moved verbatim, each made pub(super))
```

- [ ] **Step 3: Update mod.rs**

In the new `mod.rs`:
- Keep: module doc, imports, `LintSeverity`, `LintDiagnostic`, `LintRule` trait, `run_all()`, `all_rules()` (~120 lines)
- Remove: all rule implementations, test modules, helper functions
- Add at top: `mod helpers;`
- Update `all_rules()` references: change `Box::new(rules::UndefinedRef)` to `Box::new(undefined_ref::UndefinedRef)` etc.

```rust
fn all_rules() -> Vec<Box<dyn LintRule>> {
    vec![
        Box::new(undefined_ref::UndefinedRef),
        Box::new(type_mismatch::TypeMismatch),
        Box::new(store_field::StoreFieldDirectMutation),
        Box::new(prop_immutability::PropImmutability),
        Box::new(computed_purity::ComputedPurity),
        Box::new(missing_return::MissingReturn),
        Box::new(dead_code::DeadCode),
        Box::new(unused_import::UnusedImport),
        Box::new(invalid_event_field::InvalidEventField),
        Box::new(store_action_event::StoreActionEventData),
    ]
}
```

- [ ] **Step 4: Verify compilation fails (expected — missing rule modules)**

```bash
cd omnilang && cargo check 2>&1 | grep "^error" | head -10
```
Expected: errors about unresolved modules `undefined_ref`, `type_mismatch`, etc.

- [ ] **Step 5: Commit partial**

```bash
cd omnilang && cargo fmt
git add omnilang/src/semantic/lints/
git commit -m "refactor: split lints.rs — extract mod.rs + helpers.rs (rules to follow)"
```

---

### Task 2: Split lints — extract each rule (UndefinedRef)

**Files:**
- Create: `src/semantic/lints/undefined_ref.rs`

- [ ] **Step 1: Create undefined_ref.rs**

Extract from the original `mod.rs` (or from git history / backup):
- The `UndefinedRef` struct
- `impl LintRule for UndefinedRef { ... }`
- `#[cfg(test)] mod tests { ... }` for UndefinedRef (lines ~2056-2137)

Create `src/semantic/lints/undefined_ref.rs`:
```rust
//! UndefinedRef lint: detects references to undefined symbols.
use super::{LintDiagnostic, LintRule, LintSeverity};
use super::helpers::*;
use crate::ir::IRProgram;
use crate::parser::ast::Program;
use std::collections::{HashMap, HashSet};

pub struct UndefinedRef;

impl LintRule for UndefinedRef {
    fn name(&self) -> &str { "undefined-ref" }
    fn category(&self) -> &str { "correctness" }
    fn severity(&self) -> LintSeverity { LintSeverity::Error }
    // ... (check_ast and check_ir implementations moved verbatim)
}

#[cfg(test)]
mod tests {
    use super::*;
    // ... (all UndefinedRef tests moved verbatim)
}
```

- [ ] **Step 2: Verify tests pass for this rule**

```bash
cd omnilang && cargo test undefined_ref 2>&1
```
Expected: all UndefinedRef tests pass.

- [ ] **Step 3: Commit**

```bash
cd omnilang && cargo fmt
git add omnilang/src/semantic/lints/undefined_ref.rs
git commit -m "refactor: extract UndefinedRef lint rule to its own file"
```

---

### Task 3: Split lints — extract remaining rules

Repeat the pattern from Task 2 for each remaining rule. Each rule follows the EXACT same structure.

**Files to create (one per commit):**
- `src/semantic/lints/type_mismatch.rs` — `TypeMismatch` struct + impl + tests (~280 lines)
- `src/semantic/lints/store_field.rs` — `StoreFieldDirectMutation` + tests (~285 lines)
- `src/semantic/lints/prop_immutability.rs` — `PropImmutability` + tests (~250 lines)
- `src/semantic/lints/computed_purity.rs` — `ComputedPurity` + tests (~250 lines)
- `src/semantic/lints/missing_return.rs` — `MissingReturn` + tests (~250 lines)
- `src/semantic/lints/dead_code.rs` — `DeadCode` + tests (~360 lines)
- `src/semantic/lints/unused_import.rs` — `UnusedImport` + tests (~280 lines)
- `src/semantic/lints/invalid_event_field.rs` — `InvalidEventField` + tests (~280 lines)
- `src/semantic/lints/store_action_event.rs` — `StoreActionEventData` + tests (~280 lines)

**For each rule:**

- [ ] **Step 1: Create the rule file** with the rule struct, `impl LintRule`, and `#[cfg(test)] mod tests` moved verbatim. Change `use crate::semantic::lints::*` to `use super::*;` and `use super::super::*;` as needed.

- [ ] **Step 2: Run the rule's tests** — `cargo test <rule_name>` — expected: PASS

- [ ] **Step 3: Commit** with message `refactor: extract <RuleName> lint to its own file`

**After all rules are extracted:**
- `mod.rs` should be ~120 lines (only the trait, runner, and registry)
- Delete all rule code and test code from `mod.rs`
- Run full test suite: `cargo test --lib` — expected: 375 passed

---

### Task 4: Split semantic/mod.rs — extract collect.rs (Pass 1)

**Files:**
- Modify: `src/semantic/mod.rs`
- Create: `src/semantic/collect.rs`

- [ ] **Step 1: Identify Pass 1 boundaries**

Pass 1 (Collect symbols) in `mod.rs` is marked by `// ── Pass 1: Collect symbols ──` and spans from approximately line 100 to line 387. It contains methods on `SemanticAnalyzer` that collect module-level symbols (functions, components, stores, structs, types).

- [ ] **Step 2: Create collect.rs**

Move all Pass 1 methods to a separate `impl SemanticAnalyzer` block:

```rust
//! Pass 1: Collect symbols from the AST.
use super::{SemanticAnalyzer, SemanticResult};
use crate::parser::ast::*;

impl SemanticAnalyzer {
    pub(super) fn collect_symbols(&mut self, program: &Program) {
        // ... (moved from mod.rs)
    }
    // ... (all other Pass 1 helper methods)
}
```

- [ ] **Step 3: Update mod.rs**

In `mod.rs`:
- Remove the Pass 1 methods
- Add `mod collect;` at top
- Add `use collect::*;` to bring methods into scope
- The `analyze()` entry point calls `analyzer.collect_symbols(program);` — this still works because `collect.rs` adds methods to the same `SemanticAnalyzer` type

- [ ] **Step 4: Verify tests**

```bash
cd omnilang && cargo test --lib 2>&1 | grep "test result:"
```
Expected: "375 passed; 0 failed"

- [ ] **Step 5: Commit**

```bash
cd omnilang && cargo fmt
git add omnilang/src/semantic/
git commit -m "refactor: extract semantic Pass 1 (collect) to collect.rs"
```

---

### Task 5: Split semantic/mod.rs — extract resolve.rs (Pass 2)

**Files:**
- Modify: `src/semantic/mod.rs`
- Create: `src/semantic/resolve.rs`

Same pattern as Task 4. Pass 2 (Resolve references) is marked by `// ── Pass 2: Resolve references ──` and spans approximately lines 388-681. Also includes view tree resolution (lines ~682-823).

- [ ] **Step 1: Create resolve.rs**

```rust
//! Pass 2: Resolve symbol references and view tree bindings.
use super::SemanticAnalyzer;
use crate::parser::ast::*;
use std::collections::HashMap;

impl SemanticAnalyzer {
    pub(super) fn resolve_references(&mut self, program: &Program) {
        // ... (moved from mod.rs)
    }
    // ... (all Pass 2 + view tree resolution methods)
}
```

- [ ] **Step 2: Update mod.rs** — remove Pass 2 methods, add `mod resolve;`

- [ ] **Step 3: Verify and commit**

```bash
cd omnilang && cargo test --lib 2>&1 && cargo fmt
git add omnilang/src/semantic/
git commit -m "refactor: extract semantic Pass 2 (resolve) to resolve.rs"
```

---

### Task 6: Split semantic/mod.rs — extract validate.rs (Pass 3-4) + motion.rs

**Files:**
- Modify: `src/semantic/mod.rs`
- Create: `src/semantic/validate.rs`
- Create: `src/semantic/motion.rs`
- Create: `src/semantic/helpers.rs`

- [ ] **Step 1: Extract motion.rs first** (standalone functions, no state dependency)

Move standalone motion validation helpers (lines ~2602-2942) to `motion.rs`:
- `collect_element_ids()`, `is_valid_easing_name()`, `is_keyword_value()`, `has_enter_leave_animation()`, `contains_velocity_ref()`, `contains_velocity_stmt()`, `parse_duration_ms()`, `collect_timeline_refs()`

```rust
//! Standalone motion validation helpers.
use crate::parser::ast::*;
use std::collections::HashSet;

pub(super) fn collect_element_ids(nodes: &[ViewNode]) -> HashSet<String> { ... }
pub(super) fn is_valid_easing_name(name: &str) -> bool { ... }
// ... etc
```

- [ ] **Step 2: Extract helpers.rs**

Move `type_to_string()`, `is_non_interpolatable()`, and `impl TypeAnnotation` (lines ~2855-2942) to `helpers.rs`.

- [ ] **Step 3: Extract validate.rs** (Pass 3-4, ~900 lines)

Move dependency collection + validation methods from `mod.rs` (lines ~824-1733, minus motion helpers already extracted):
- Pass 3 (dependencies): `collect_dependencies()`
- Pass 4 (validate): type checking, store reference validation
- Motion validation methods (lines ~1733-2601)

```rust
//! Pass 3-4: Dependency collection and validation.
use super::{SemanticAnalyzer, motion, helpers};
use crate::parser::ast::*;

impl SemanticAnalyzer {
    pub(super) fn collect_dependencies(&mut self, program: &Program) { ... }
    pub(super) fn validate(&mut self) { ... }
    // ... (all validation methods)
}
```

**Note:** If `validate.rs` exceeds 800 lines after extraction, further split into:
- `validate_types.rs` — type checking (~500 lines)
- `validate_motion.rs` — motion/timeline validation (~400 lines)
This split is determined during implementation by measuring the actual extracted line count.

- [ ] **Step 4: Update mod.rs**

`mod.rs` now contains only: module doc, imports, `SemanticResult` type, `analyze()` entry point, `SemanticAnalyzer` struct + `new()`. ~100 lines.

- [ ] **Step 5: Verify and commit**

```bash
cd omnilang && cargo test --lib 2>&1  # Expected: 375 passed
cargo fmt
git add omnilang/src/semantic/
git commit -m "refactor: extract semantic Pass 3-4 (validate) + motion + helpers"
```

---

### Task 7: Final verification

- [ ] **Step 1: Check all file sizes**

```bash
cd omnilang && find src/semantic -name "*.rs" -exec wc -l {} \; | sort -rn | awk '$1 > 800 {print $1, $2}'
```
Expected: no files over 800 lines (or at most `validate.rs` if the further split is needed — handle it now if so).

- [ ] **Step 2: Full test suite**

```bash
cd omnilang && cargo test --lib 2>&1 | grep "test result:"
```
Expected: "375 passed; 0 failed"

- [ ] **Step 3: Eval harness**

```bash
bash .claude/evals/eval-run-omnilang.sh
```

- [ ] **Step 4: Commit**

```bash
git commit -m "chore: verify semantic split — all 375 tests green, files ≤800 lines"
```
