# Batch 2: IR Module Split — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Split `ir/mod.rs` (3576 code lines) into `types.rs` + `lower_ast.rs` + `mod.rs`. The types file (~1050 lines) will be under 800 immediately; lower_ast.rs (~2500 lines) is tracked for follow-up phase splitting.

**Architecture:** Types (~30 structs/enums + 3 helper fns) move verbatim to `types.rs`. Lowering functions (~25 functions) move verbatim to `lower_ast.rs`. `mod.rs` becomes a thin re-export hub (~100 lines). Existing sub-modules (`logic.rs`, `lower_logic.rs`) are unchanged.

**Tech Stack:** Rust 2021, serde derives, cargo test

**Safety net:** `cargo test --lib` must return "375 passed; 0 failed" after every commit.

**Key mechanics:** Same `mv` + re-export pattern as Batch 1. `pub mod ir;` in `lib.rs` needs NO change.

---

### Task 1: Split ir/mod.rs → types.rs + lower_ast.rs + mod.rs

**Files:**
- Create: `src/ir/types.rs`
- Create: `src/ir/lower_ast.rs`
- Modify: `src/ir/mod.rs` (rewrite as re-export hub)
- Keep: `src/ir/logic.rs` (92 lines — unchanged)
- Keep: `src/ir/lower_logic.rs` (147 lines — unchanged)

- [ ] **Step 1: Backup and verify starting state**

```bash
cd omnilang && cargo test --lib 2>&1 | grep "test result:"
```
Expected: "test result: ok. 375 passed; 0 failed"

- [ ] **Step 2: Create types.rs — move all IR type definitions**

Move everything from `mod.rs` lines 1 through ~995 into `types.rs`:
- Type structs/enums: `ExprIR`, `MatchArmIR`, `MatchPatternIR`, `MatchPatternKindIR`, `TypeIR`, `SignalRefIR`, `BindingIR`, `EventHandlerIR`, `EventDataKind`, `EventFieldDef`, `BlockIR`, `StatementIR`, `StateIntent`, `MutationIntent`, `ComputedIntent`, `EffectIntent`, `LifecycleIntent`, `ConditionalBranch`, `ConditionalIntent`, `ListRenderingIntent`, `ElementIR`, `ViewTree`, `StyleRuleIR`, `ParamIR`, `ComponentIR`, `StoreIR`, `IRProgram`
- Motion types: `MotionIntent`, `MotionBodyIR`, `KeyframeIR`, `KeyframeStopIR`, `TimelineIR`, `TimelineItemIR`, `TimelineItemKindIR`, `DirectionIR`, `ConnectorIR`, `CallbackBodyIR`, `CallbackKindIR`, `PlaybackIR`, `EasingIR`, `HeroIntent`, `TriggerIntent`, `TriggerKind`, `StatesIR`, `StateTweenIR`, `MotionExprIR`, `TimelineIntent`, `LoopValue`, `DirectionValue`, `FillValue`, `InterruptValue`, `WhenValue`, `StaggerDirection`, `StaggerTimingIR`
- Helper functions: `event_data_schema`, `map_event_data_kind`, `event_data_kind_label`
- All `impl` blocks on types (e.g., `impl Default for IRProgram`, `impl IRProgram { fn all_components()... }`)

Create `src/ir/types.rs`:
```rust
//! Intent IR type definitions — all types used in the IR layer.
//! Serializes to/from JSON via serde.
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ... (all ~30 type definitions + impl blocks + 3 helper fns moved verbatim)
```

All items keep their existing visibility: `pub enum`, `pub struct`, `pub fn`.

- [ ] **Step 3: Create lower_ast.rs — move all lowering functions**

Move everything from `mod.rs` lines ~996 to end (before tests) into `lower_ast.rs`:
- `lower()`, `lower_component()`, `lower_view_block()`, `lower_view_node()`, `lower_stmt()`, `try_lower_mutation()`, `mutation_for_method()`, `lower_expr_to_json()`, `lower_expr_to_json_from_ir()`, `lower_store()`, `expr_to_json_value()`, `lower_motion_def()`, `lower_timeline_def()`, `lower_keyframe_body()`, `lower_timeline_body()`, `lower_timeline_items()`, `lower_timeline_item_kind_simple()`, `lower_callback_body()`, `lower_easing()`, `lower_playback_props()`, `lower_motion_expr()`, `lower_connector()`, `lower_stagger_direction()`, `format_position_value()`, `lower_hero()`, `lower_trigger()`
- `EventFieldCollector` struct + impl
- `backfill_fn_ref_event_fields()`, `collect_ir_event_fields()`, `walk_expr_event_fields()`

Create `src/ir/lower_ast.rs`:
```rust
//! AST → IR lowering: walks the parsed AST and produces IR types.
use super::types::*;  // pull in all IR types
use crate::parser::ast::*;
use crate::semantic::SemanticResult;
use std::collections::HashMap;

// ... (all ~25 lowering functions + EventFieldCollector moved verbatim)
```

All `fn` items become `pub(super) fn` (or keep `pub fn` for `lower` which is the public entry point).

- [ ] **Step 4: Rewrite mod.rs as re-export hub**

Replace the content of `src/ir/mod.rs`:
```rust
//! Intent IR — types and lowering logic.
//!
//! Pipeline: AST → IR types → codegen / JSON serialization.

pub mod logic;
pub mod lower_logic;
mod types;
mod lower_ast;

pub use lower_ast::lower;
pub use types::*;

// Any remaining IRProgram convenience methods that don't fit in types.rs
// (most should already be in the impl IRProgram block moved to types.rs)
```

Remove the test module (only ~67 lines) — if there are tests specific to the IR module that don't belong in types or lower_ast, keep them inline. Otherwise move to the appropriate file.

- [ ] **Step 5: Build and fix compilation errors**

```bash
cd omnilang && cargo check 2>&1 | head -30
```

Expected errors (fix iteratively):
- Import paths: `use crate::ir::*` callers need to resolve — but since `mod.rs` does `pub use types::*;`, all type paths should still work
- `use super::*` or `use crate::ir::*` in lowering functions — adjust to `use super::types::*;`
- Any `use crate::ir::lower` callers — still works via `pub use lower_ast::lower;`

Fix each error one at a time, re-running `cargo check` after each.

- [ ] **Step 6: Verify all tests pass**

```bash
cd omnilang && cargo test --lib 2>&1
```
Expected: "375 passed; 0 failed"

- [ ] **Step 7: Format and commit**

```bash
cd omnilang && cargo fmt
git add omnilang/src/ir/
git commit -m "refactor: split ir/mod.rs into types.rs + lower_ast.rs (3576→1050+2500+100)"
```

---

### Task 2: Verify file sizes and eval harness

- [ ] **Step 1: Check line counts**

```bash
cd omnilang && wc -l src/ir/*.rs src/ir/*/*.rs 2>/dev/null | sort -rn
```
Expected: `types.rs` ≤ 1050 lines, `lower_ast.rs` ~2500 lines, all others ≤ 800.

Note: `lower_ast.rs` at ~2500 lines remains over 800 — this is expected and tracked for follow-up.

- [ ] **Step 2: Run eval harness**

```bash
bash .claude/evals/eval-run-omnilang.sh
```
Expected: R4 file count drops (one fewer file >800 — mod.rs was replaced by lower_ast.rs which is still counted).

- [ ] **Step 3: Commit**

```bash
git commit -m "chore: verify IR split — 375 tests green, 1 file remaining >800 (lower_ast.rs)"
```
