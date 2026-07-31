# Semantic-Level DSL Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Rewrite OmniLang compiler to use semantic-level code generation — .omni source expresses intent, compiler generates idiomatic platform-native code for React (Zustand v5 + React 19) and Flutter (Riverpod 3.0).

**Architecture:** Three-layer implementation: IR type changes (foundation) → semantic passes (intent recognition) → codegen rewrites (idiomatic output). Each layer depends on the previous. React and Flutter codegen are independent and can be parallelized.

**Tech Stack:** Rust (compiler), TypeScript/React 19/Zustand v5 (React target), Dart/Flutter/Riverpod 3.0 (Flutter target)

**Spec:** `docs/superpowers/specs/2026-07-31-semantic-level-dsl-syntax-spec.md`

---

## File Structure Map

```
crates/omnilang-core/src/
├── ir/
│   ├── types.rs              ← RefOpIR enum, ViewTree::Slot fix, PlaybackIR del
│   ├── types_motion.rs       ← TriggerKind::Focus, del paused_override
│   └── lower/
│       ├── view.rs           ← Slot lowering fix, semantic binding recognition
│       └── lower_inline_motions.rs  ← NEW: inline motion → named motion

├── semantic/
│   └── validate_ref_ops.rs   ← NEW: MethodCall on ref → RefOpExpr / error

└── codegen/shared/
    └── expr.rs               ← OptionalChain/BlockExpr/Match overrides

omnilang/src/codegen/
├── mod.rs                    ← Design gate: P0 remove, non-P0 warning→error
├── shared/design.rs          ← NEW: extract shared CSS emitter from omniview/
├── react/
│   ├── mod.rs                ← Zustand v5 rewrite, store gen
│   ├── view.rs               ← slots, ref bindings, semantic bindings
│   ├── expr.rs               ← OptionalChain/BlockExpr/Match overrides
│   ├── component/
│   │   ├── state.rs          ← useRef for refs, useState for signals
│   │   ├── effect.rs         ← @mount/@unmount/@effect → useEffect
│   │   └── imports.rs        ← Zustand v5 imports
│   ├── motion/
│   │   ├── mod.rs            ← Native ref + WAAPI (no DOM scanning)
│   │   └── keyframes.rs      ← Semantic property → WAAPI keyframes
│   └── design.rs             ← NEW: Design CSS emission
├── flutter/
│   ├── mod.rs                ← Riverpod 3.0 rewrite, store gen
│   ├── view.rs               ← slots, ref bindings, semantic bindings
│   ├── expr.rs               ← OptionalChain/BlockExpr/Match overrides
│   ├── motion.rs             ← TweenSequence, per-property Animatable
│   ├── ref.rs                ← NEW: GlobalKey + FocusNode + ScrollController gen
│   └── design.rs             ← NEW: ThemeData + ThemeExtension emission
└── tests_regression/
    └── parameterized.rs      ← Expand: design/slots/refs/key_expr/animation
```

---

## Phase 0: IR Foundation

### Task 0.1: Add RefOpIR enum to IR types

**Files:**
- Modify: `crates/omnilang-core/src/ir/types.rs`

- [ ] **Step 1: Add RefOpIR enum**

Insert after `ViewTree` definition (after line 678):

```rust
/// Platform-independent ref operations. The semantic layer rewrites
/// ref method calls into these variants so codegen knows the intent.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub enum RefOpIR {
    Focus,
    Blur,
    ScrollTo { x: Option<ExprIR>, y: Option<ExprIR> },
    ScrollBy { dx: Option<ExprIR>, dy: Option<ExprIR> },
    Measure,  // → { width, height, x, y }
    Select,
    SetSelection { start: ExprIR, end: ExprIR },
    Value,
    SetValue { value: ExprIR },
    CapturePointer { id: ExprIR },
    ReleasePointer { id: ExprIR },
}
```

- [ ] **Step 2: Add RefOpExpr variant to ExprIR**

In `ExprIR` enum (after `MethodCallExpr` variant):

```rust
/// A semantic ref operation — rewritten from ref.method() by validate_ref_ops.
/// Carries the ref variable name and the operation.
RefOpExpr { ref_name: String, op: RefOpIR },
```

- [ ] **Step 3: Add SemanticBinding variants to ElementIR**

In `ElementIR` struct (around line 619), add after `bindings`:

```rust
/// Semantic bindings (compiler-recognized, not generic props):
/// focus, scroll-x, scroll-y, value — driven by signals
pub semantic_bindings: FxHashMap<String, ExprIR>,
```

- [ ] **Step 4: Build and fix compilation errors**

Run: `cargo build -p omnilang-core 2>&1`
Expected: compile errors in all places that exhaustively match `ExprIR` and construct `ElementIR`.

- [ ] **Step 5: Add `#[non_exhaustive]` to ExprIR if not already present and fix all match arms**

For every `match` on `ExprIR` variants that fails, add `_ => {}` or the appropriate fallback. Focus on:
- `ir/lower/` — all lowering passes
- `ir/optimize.rs`
- `ir/walk.rs`
- `codegen/shared/expr.rs`
- Every codegen backend's expr.rs

Expected: `cargo build -p omnilang-core` compiles clean.

- [ ] **Step 6: Commit**

```bash
git add crates/omnilang-core/src/ir/types.rs
# plus all files with match arm fixes
git commit -m "feat(ir): add RefOpIR enum and ExprIR::RefOpExpr variant

Add platform-independent ref operation IR types for semantic-level
codegen. No semantic pass yet — just the types and compilation fixes."
```

### Task 0.2: Fix ViewTree::Slot to preserve fallback

**Files:**
- Modify: `crates/omnilang-core/src/ir/types.rs:673-675`
- Modify: `crates/omnilang-core/src/ir/lower/view.rs:205-208`
- Modify: all codegen consumers of `ViewTree::Slot`

- [ ] **Step 1: Change ViewTree::Slot variant**

Replace:
```rust
Slot(Option<String>),
```
with:
```rust
Slot {
    /// Slot name — None = default slot
    name: Option<String>,
    /// Fallback content rendered when parent provides no children
    fallback: Vec<ViewTree>,
    /// Optional type constraint: @slot header: HeaderData
    type_constraint: Option<TypeIR>,
    /// Optional data param name for typed slots
    data_param: Option<String>,
},
```

- [ ] **Step 2: Fix lowering to preserve default_content**

In `crates/omnilang-core/src/ir/lower/view.rs:205-208`, replace:
```rust
ViewNode::SlotDirective {
    name,
    default_content: _,
} => ViewTree::Slot(name.clone()),
```
with:
```rust
ViewNode::SlotDirective {
    name,
    default_content,
} => {
    let fallback = default_content
        .map(|nodes| nodes.into_iter().map(|n| lower_view_node(n, ...)).collect())
        .unwrap_or_default();
    ViewTree::Slot {
        name: name.clone(),
        fallback,
        type_constraint: None,   // populated later from semantic analysis
        data_param: None,
    }
},
```

- [ ] **Step 3: Fix all codegen consumers of ViewTree::Slot**

Grep for `ViewTree::Slot(` in `omnilang/src/codegen/` and update each match arm. For the default Omniview consumer (most critical):

```rust
// Before: ViewTree::Slot(name) => { ... }
// After:
ViewTree::Slot { name, fallback, .. } => {
    // use `name` and `fallback` instead of bare name
    // fallback is Vec<ViewTree> — render when no children provided
}
```

- [ ] **Step 4: Build and test**

Run: `cargo build -p omnilang 2>&1`
Expected: all match arms updated, compiles clean.

Run: `cargo test -p omnilang-core 2>&1`
Expected: existing tests pass (slot tests may change behavior — review snapshots).

- [ ] **Step 5: Commit**

```bash
git commit -m "fix(ir): preserve slot fallback content in ViewTree::Slot

Change ViewTree::Slot from leaf node to struct with name, fallback,
type_constraint, and data_param fields. Fix lowering to stop dropping
default_content (was bound to _)."
```

### Task 0.3: Motion IR type changes

**Files:**
- Modify: `crates/omnilang-core/src/ir/types_motion.rs`

- [ ] **Step 1: Add TriggerKind::Focus variant**

In `TriggerKind` enum (line 71-80), add:
```rust
Focus,
```

- [ ] **Step 2: Delete PlaybackIR.paused field**

In `PlaybackIR` struct (line 43-52), remove:
```rust
pub paused: bool,
```
And remove it from the struct literal construction everywhere (grep for `PlaybackIR {`).

- [ ] **Step 3: Delete MotionBehavior.paused_override field**

In `MotionBehavior` struct (line 111-120), remove:
```rust
pub paused_override: Option<ExprIR>,
```

- [ ] **Step 4: Delete `@paused` element attribute handling**

In `crates/omnilang-core/src/ir/lower/view.rs`, remove the `PausedOverride` element prop lowering (grep for `PausedOverride`). Remove `ElementProp::PausedOverride` from `parser/ast/view.rs`.

- [ ] **Step 5: Fix all compilation errors and tests**

Run: `cargo build -p omnilang 2>&1`
Fix all match exhaustiveness errors.
Run: `cargo test -p omnilang 2>&1`
Delete the `paused_override` snapshot file if it exists. Update any affected insta snapshots with `cargo insta review`.

- [ ] **Step 6: Commit**

```bash
git commit -m "refactor(ir): add TriggerKind::Focus, delete paused/paused_override

Per semantic-level DSL spec: @focus trigger re-added to feature floor.
paused (in PlaybackIR) and paused_override (in MotionBehavior) deleted
per ADR-052 — no .omni usage, replaced by conditional binding."
```

---

## Phase 1: Semantic Passes

### Task 1.1: Create validate_ref_ops semantic pass

**Files:**
- Create: `crates/omnilang-core/src/semantic/validate_ref_ops.rs`
- Modify: `crates/omnilang-core/src/semantic/mod.rs` (re-export)

- [ ] **Step 1: Write the pass skeleton**

```rust
//! Semantic pass: validate ref method calls and rewrite to RefOpExpr.
//!
//! Scans all expressions in component bodies (actions, fns, lifecycles)
//! for method calls on declared refs. Legal operations (focus, scrollTo, etc.)
//! are rewritten to ExprIR::RefOpExpr. Illegal operations (style.*, innerHTML,
//! querySelector, etc.) produce compile errors with migration hints.

use crate::ir::types::{ExprIR, RefOpIR};
use crate::semantic::SemanticDiagnostic;

pub fn validate_ref_ops(
    component: &mut ComponentIR,
    diagnostics: &mut Vec<SemanticDiagnostic>,
) {
    // Walk all expressions in the component
    walk_expressions_mut(&mut component.view, &mut |expr| {
        check_and_rewrite_ref_op(expr, &component.refs, diagnostics);
    });
    for effect in &mut component.effects {
        for stmt in &mut effect.body.statements {
            walk_stmt_expressions_mut(stmt, &mut |expr| {
                check_and_rewrite_ref_op(expr, &component.refs, diagnostics);
            });
        }
    }
    for action in &mut component.actions {
        for stmt in &mut action.body.statements {
            walk_stmt_expressions_mut(stmt, &mut |expr| {
                check_and_rewrite_ref_op(expr, &component.refs, diagnostics);
            });
        }
    }
}

const LEGAL_REF_METHODS: &[(&str, fn(Vec<ExprIR>) -> RefOpIR)] = &[
    ("focus",     |_| RefOpIR::Focus),
    ("blur",      |_| RefOpIR::Blur),
    ("scrollTo",  |a| RefOpIR::ScrollTo {
        x: a.get(0).cloned(), y: a.get(1).cloned()
    }),
    ("scrollBy",  |a| RefOpIR::ScrollBy {
        dx: a.get(0).cloned(), dy: a.get(1).cloned()
    }),
    ("select",    |_| RefOpIR::Select),
    ("setSelection", |a| RefOpIR::SetSelection {
        start: a.get(0).cloned().unwrap_or_default(),
        end: a.get(1).cloned().unwrap_or_default(),
    }),
    ("measure",   |_| RefOpIR::Measure),
    ("value",     |_| RefOpIR::Value),
    ("setValue",  |a| RefOpIR::SetValue {
        value: a.get(0).cloned().unwrap_or_default(),
    }),
    ("capturePointer",  |a| RefOpIR::CapturePointer {
        id: a.get(0).cloned().unwrap_or_default(),
    }),
    ("releasePointer",  |a| RefOpIR::ReleasePointer {
        id: a.get(0).cloned().unwrap_or_default(),
    }),
];

const ILLEGAL_REF_PATTERNS: &[(&str, &str)] = &[
    ("style",     "use style:{...} binding with signals instead"),
    ("innerHTML", "use text interpolation {value} instead"),
    ("addEventListener", "use @on* event binding syntax instead"),
    ("querySelector", "violates component encapsulation"),
    ("classList", "use class: binding syntax instead"),
    ("parentElement", "violates component encapsulation"),
    ("textContent", "use text interpolation {value} instead"),
    ("getBoundingClientRect", "use ref.measure() instead"),
    ("offsetWidth", "use ref.measure().width instead"),
    ("offsetHeight", "use ref.measure().height instead"),
];

fn check_and_rewrite_ref_op(
    expr: &mut ExprIR,
    refs: &[RefDeclIR],
    diagnostics: &mut Vec<SemanticDiagnostic>,
) {
    // ... implementation: match MethodCallExpr on ref, rewrite or error
}
```

- [ ] **Step 2: Implement the walk and rewrite logic**

Full implementation of `check_and_rewrite_ref_op`:
- Match `ExprIR::MethodCallExpr { object, method, args }` where `object` is a `IdentifierExpr` matching a declared ref name
- If method is in `LEGAL_REF_METHODS` → replace with `ExprIR::RefOpExpr { ref_name, op }`
- If method matches `ILLEGAL_REF_PATTERNS` → push diagnostic error with migration hint
- If method is unknown → push diagnostic error "unknown ref operation: {method}"

- [ ] **Step 3: Handle field access on refs (e.g., ref.width, ref.offsetWidth)**

Add handling for `ExprIR::FieldAccessExpr { object, field }`:
- `width` / `height` → rewrite to `RefOpExpr { op: Measure }` with field extraction
- `value` → rewrite to `RefOpExpr { op: Value }`
- `style` → error with migration hint
- others → error

- [ ] **Step 4: Wire the pass into the lowering pipeline**

In `crates/omnilang-core/src/ir/lower/mod.rs`, after `validate_required_props` call:

```rust
// Validate and rewrite ref operations
for module in &mut program.modules.values_mut() {
    for component in &mut module.components.values_mut() {
        semantic::validate_ref_ops::validate_ref_ops(component, diagnostics);
    }
}
```

- [ ] **Step 5: Add unit tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rewrites_ref_focus_to_refop() {
        // Setup: component with ref "inputBox", expression: inputBox.focus()
        // Assert: expression rewritten to RefOpExpr { ref_name: "inputBox", op: Focus }
    }

    #[test]
    fn errors_on_ref_style_access() {
        // Setup: expression: ref.style.color
        // Assert: diagnostic emitted with migration hint
    }

    #[test]
    fn passes_non_ref_method_call_through() {
        // Setup: expression: obj.toString()
        // Assert: unchanged (not a ref)
    }
}
```

- [ ] **Step 6: Build and run tests**

Run: `cargo test -p omnilang-core -- semantic::validate_ref_ops 2>&1`
Expected: tests pass.

Run: `cargo test -p omnilang 2>&1`
Expected: all existing tests pass.

- [ ] **Step 7: Commit**

```bash
git add crates/omnilang-core/src/semantic/validate_ref_ops.rs
git commit -m "feat(semantic): add validate_ref_ops pass

Scans ref method calls in component bodies, rewrites legal operations
(focus, scrollTo, select, measure, etc.) to ExprIR::RefOpExpr, and
emits compile errors for DOM-specific APIs with migration hints."
```

### Task 1.2: Create inline motion lowering pass

**Files:**
- Create: `crates/omnilang-core/src/ir/lower/lower_inline_motions.rs`
- Modify: `crates/omnilang-core/src/ir/lower/mod.rs` (wire in)

- [ ] **Step 1: Write the lowering pass**

```rust
//! Lowering pass: expand inline motion bodies into named MotionIntents.
//!
//! Scans all ViewTree nodes for TriggerIntent with inline_motion set,
//! synthesizes a unique name (__anon_m_N), creates a MotionIntent entry,
//! and rewrites the trigger to reference the named motion.

use crate::ir::types_motion::{MotionIntent, TriggerIntent, MotionBodyIR};

pub fn lower_inline_motions(module: &mut ModuleIR) {
    let mut counter = 0u32;
    let mut new_motions = Vec::new();

    // Walk all view trees in all components
    for component in module.components.values_mut() {
        walk_view_motions_mut(&mut component.view, &mut counter, &mut new_motions);
    }

    // Append synthesized motions to module
    module.motions.extend(new_motions);
}

fn walk_view_motions_mut(
    view: &mut Option<ViewTree>,
    counter: &mut u32,
    new_motions: &mut Vec<MotionIntent>,
) {
    // Recursive walk of ViewTree, looking for Element with motion triggers
    // If trigger.inline_motion is Some, synthesize name and rewrite
    // ...
}
```

- [ ] **Step 2: Implement the recursive ViewTree walk**

Traverse `ViewTree::Element`, `ViewTree::Conditional`, `ViewTree::ListRendering`, `ViewTree::Fragment`. For each `ElementIR` with `motion` triggers:
- Check each trigger's `inline_motion`
- If `Some(body)` → generate name `__anon_m_{counter}`, create `MotionIntent`, set `trigger.motion_ref = Some(name)`, set `trigger.inline_motion = None`

- [ ] **Step 3: Wire into lowering pipeline**

In `ir/lower/mod.rs`, after component lowering:

```rust
for module in &mut program.modules.values_mut() {
    lower_inline_motions::lower_inline_motions(module);
}
```

- [ ] **Step 4: Add test with inline motion annotation**

```rust
#[test]
fn expands_inline_motion_to_named() {
    // Input: component with @enter = { over: 200ms; from { opacity: 0 } to { opacity: 1 } }
    // Expected: module.motions contains __anon_m_0, trigger.motion_ref = "__anon_m_0"
}
```

- [ ] **Step 5: Build, test, commit**

```bash
cargo test -p omnilang-core 2>&1
cargo test -p omnilang 2>&1
git commit -m "feat(ir): add lower_inline_motions pass

Expands inline animation bodies into named MotionIntents so all
codegen backends only handle named motions. Synthesized names use
__anon_m_N pattern."
```

---

## Phase 2: Shared Codegen Fixes

### Task 2.1: Remove design block compile gate

**Files:**
- Modify: `omnilang/src/codegen/mod.rs`

- [ ] **Step 1: Remove gate for P0 targets (React, Flutter)**

Delete the `if has_design_blocks(ir)` blocks at:
- Lines 152-161 (React)
- Lines 192-201 (Flutter)

Keep the `has_design_blocks` check for non-P0 targets but change `warning` → `error`:

```rust
// Non-P0 targets: design blocks → hard error
if has_design_blocks(ir) {
    return CompileOutput {
        files: vec![],
        errors: vec!["Design blocks are not supported for this target. \
            Use --target react, --target flutter, or --target omniview.".into()],
        source_map: vec![],
        warnings: vec![],
    };
}
```

- [ ] **Step 2: Update the DESIGN_BLOCK_WARNING constant**

Replace with `DESIGN_BLOCK_ERROR` constant or inline the message.

- [ ] **Step 3: Build and verify**

Run: `cargo build -p omnilang 2>&1`
Expected: compiles clean. React and Flutter targets no longer return zero files for design blocks (they'll now fall through to codegen which may still emit incomplete output — that's Phase 3/4).

- [ ] **Step 4: Commit**

```bash
git commit -m "feat(codegen): remove design block gate for P0 targets

React and Flutter targets no longer return zero files when IR contains
design blocks. Non-P0 targets now emit hard error instead of warning.
Actual design codegen implementation follows in React/Flutter phases."
```

### Task 2.2: Fix shared OptionalChain and BlockExpr formatting

**Files:**
- Modify: `crates/omnilang-core/src/codegen/shared/expr.rs`
- Modify: `omnilang/src/codegen/react/expr.rs`
- Modify: `omnilang/src/codegen/flutter/expr.rs`

- [ ] **Step 1: Override format_optional_chain in ReactFormatExpr**

In `omnilang/src/codegen/react/expr.rs`, add/verify:

```rust
fn format_optional_chain(&self, object: &ExprIR, field: &str) -> String {
    format!("{}.{}", self.format_expr(object), field) // JS natively supports ?.
    // Actually: the shared default drops ?. — we need to detect that this
    // is an OptionalChainExpr from the caller context. The fix is:
    // In the shared format_expr, for OptionalChainExpr, call format_optional_chain
    // which React overrides to emit ?. correctly.
    format!("{}?.{}", self.format_expr(object), field)
}
```

- [ ] **Step 2: Override format_optional_chain in DartFormatExpr**

In `omnilang/src/codegen/flutter/expr.rs`:

```rust
fn format_optional_chain(&self, object: &ExprIR, field: &str) -> String {
    format!("{}?.{}", self.format_expr(object), field) // Dart natively supports ?.
}
```

- [ ] **Step 3: Fix format_block_expr in both backends**

React — emit IIFE:
```rust
fn format_block_expr(&self, block: &BlockIR) -> String {
    let stmts: Vec<String> = block.statements.iter()
        .map(|s| self.format_stmt(s))
        .collect();
    let tail = block.trailing_expr.as_ref()
        .map(|e| format!("return {};", self.format_expr(e)))
        .unwrap_or_default();
    format!("(() => {{ {} {} }})()", stmts.join(" "), tail)
}
```

Flutter — emit Dart IIFE:
```rust
fn format_block_expr(&self, block: &BlockIR) -> String {
    let stmts: Vec<String> = block.statements.iter()
        .map(|s| self.format_stmt(s))
        .collect();
    let tail = block.trailing_expr.as_ref()
        .map(|e| format!("return {};", self.format_expr(e)))
        .unwrap_or_default();
    format!("(() {{ {} {} }})()", stmts.join(" "), tail)
}
```

- [ ] **Step 4: Fix MatchExpr formatting for Flutter**

In `omnilang/src/codegen/flutter/expr.rs`, implement full `format_match`:
```rust
fn format_match(&self, matched: &ExprIR, arms: &[MatchArmIR]) -> String {
    let subject = self.format_expr(matched);
    let cases: Vec<String> = arms.iter().map(|arm| {
        let pattern = self.format_pattern(&arm.pattern);
        let body = self.format_expr(&arm.body);
        format!("case {}: return {};", pattern, body)
    }).collect();
    format!("(() {{ switch({}) {{ {} default: throw 'no match'; }} }})()",
        subject, cases.join(" "))
}
```

- [ ] **Step 5: Add regression tests**

In `omnilang/src/codegen/tests_regression/parameterized.rs`, add test cases:
```rust
#[test]
fn optional_chain_emits_correct_react() { /* ... */ }
#[test]
fn optional_chain_emits_correct_flutter() { /* ... */ }
#[test]
fn block_expr_emits_iife_react() { /* ... */ }
#[test]
fn block_expr_emits_iife_flutter() { /* ... */ }
```

- [ ] **Step 6: Run tests, review snapshots, commit**

```bash
cargo test -p omnilang 2>&1
cargo insta review  # accept updated snapshots
git commit -m "fix(codegen): override OptionalChain/BlockExpr/Match in React and Flutter

React: emit ?. for optional chains, IIFE for block expressions.
Flutter: emit ?. for optional chains, Dart IIFE, switch for matches.
Fixes H2, H3, M9 from cross-target feature gap review."
```

---

## Phase 3: React Codegen

### Task 3.1: Zustand v5 store rewrite

**Files:**
- Modify: `omnilang/src/codegen/react/mod.rs` (store generation)
- Modify: `omnilang/src/codegen/react/component/state.rs` (signal reads)
- Modify: `omnilang/src/codegen/react/component/imports.rs` (imports)
- Delete: Hooks mode (`ReactStoreMode::Hooks`)

- [ ] **Step 1: Replace store generation with Zustand v5 pattern**

Rewrite `gen_zustand_store` (around line 352) to emit:

```typescript
import { create } from 'zustand';

interface TodoStoreState {
  items: Todo[];
  filter: string;
}

export const useTodoStore = create<TodoStoreState>()((set) => ({
  items: [],
  filter: 'all',
}));

// Getters → standalone selector hooks
export function useFilteredItems() {
  const items = useTodoStore(s => s.items);
  const filter = useTodoStore(s => s.filter);
  return useMemo(
    () => items.filter(item => filter === 'all' || item.status === filter),
    [items, filter]
  );
}

// Actions → exported functions using setState
export function useAddTodo() {
  return useCallback((text: string) => {
    useTodoStore.setState((s) => ({
      items: [...s.items, { id: Date.now(), text, status: 'active' }],
    }));
  }, []);
}
```

Key changes from current Zustand v4 output:
- `create<T>()()` — double-call (v5 requirement)
- Getters → standalone hooks (not store-internal)
- Actions → exported `useCallback` functions (not store-internal)
- Remove `get()` usage from store body (v5 removed it from create callback)

- [ ] **Step 2: Update signal read formatting**

In `component/state.rs`, for store signal reads, emit:
```typescript
// Before: const items = TodoStore.items;  (wrong — not reactive)
// After:  const items = useTodoStore(s => s.items);
```

- [ ] **Step 3: Update action call formatting**

For `StoreName.action(args)`:
```typescript
// Before: TodoStore.add(text)  (undefined in component scope)
// After:  const addTodo = useAddTodo(); addTodo(text)
```

- [ ] **Step 4: Delete Hooks mode**

Remove `ReactStoreMode::Hooks` variant. Remove the CLI flag's "hooks" option. Update `omnic/src/main.rs`:

```rust
// Before:
#[arg(long, default_value = "hooks", value_parser = ["hooks", "zustand"])]
// After:
#[arg(long, default_value = "zustand")]  // only option, no choice
```

- [ ] **Step 5: Update test snapshots**

Run: `cargo test -p omnilang -- codegen::react 2>&1`
Expected: snapshot mismatches. Review with `cargo insta review` and accept updated Zustand v5 output.

- [ ] **Step 6: Commit**

```bash
git commit -m "refactor(react): rewrite store codegen to Zustand v5

- create<T>()() double-call pattern (v5 requirement)
- Getters → standalone selector hooks with useMemo
- Actions → exported useCallback functions
- Delete ReactStoreMode::Hooks (Zustand v5 only)
- Signal reads → useStore(s => s.field) selectors"
```

### Task 3.2: React slot codegen

**Files:**
- Modify: `omnilang/src/codegen/react/view.rs:827-830`

- [ ] **Step 1: Replace slot comment with props.children**

Replace the current slot comment emission with:
```rust
ViewTree::Slot { name, fallback, .. } => {
    if let Some(slot_name) = name {
        // Named slot: {header ?? <DefaultHeader />}
        let fallback_jsx = render_fallback(fallback);
        format!("{{props.{0} ?? ({1})}}", slot_name, fallback_jsx)
    } else {
        // Default slot: {props.children}
        "{props.children}".to_string()
    }
}
```

- [ ] **Step 2: Add slot props to component interface**

In component generation, add named slot props to the TypeScript interface:
```typescript
interface ModalProps {
  header?: ReactNode;
  footer?: ReactNode;
  children?: ReactNode;
}
```

- [ ] **Step 3: Update tests and snapshots**

Run: `cargo test -p omnilang -- codegen::react::view 2>&1`
Expected: slots now emit props.children/props.{name} instead of comments.

- [ ] **Step 4: Commit**

```bash
git commit -m "feat(react): implement slot codegen with props.children

Default slot → {props.children}, named slot → {props.{name}}.
Named slot props added to component TypeScript interface.
Fallback content rendered with ?? operator."
```

### Task 3.3: React ref semantic binding codegen

**Files:**
- Modify: `omnilang/src/codegen/react/view.rs`
- Modify: `omnilang/src/codegen/react/component/state.rs`

- [ ] **Step 1: Generate useRef for ref declarations**

In `component/state.rs`, for each `RefDeclIR`:
```typescript
const searchBoxRef = useRef<HTMLInputElement>(null);
const scrollPaneRef = useRef<HTMLDivElement>(null);
```

- [ ] **Step 2: Wire @ref={name} to JSX ref prop**

In `view.rs`, for elements with `ref_binding`:
```typescript
<input ref={searchBoxRef} ... />
```

- [ ] **Step 3: Generate semantic binding effects**

For `focus={isSearchActive}`:
```typescript
useEffect(() => {
  if (isSearchActive) {
    searchBoxRef.current?.focus();
  } else {
    searchBoxRef.current?.blur();
  }
}, [isSearchActive]);
```

For `scroll-y={scrollPosition}`:
```typescript
useEffect(() => {
  scrollPaneRef.current?.scrollTo({ top: scrollPosition, behavior: 'smooth' });
}, [scrollPosition]);
```

For `value={searchText}`:
```typescript
// Already handled by bind:value pattern — reuse existing onChange handler
```

- [ ] **Step 4: Handle RefOpExpr in expression formatting**

In `react/expr.rs`, add match arm:
```rust
ExprIR::RefOpExpr { ref_name, op } => match op {
    RefOpIR::Select => format!("{}Ref.current?.select()", ref_name),
    RefOpIR::SetSelection { start, end } => 
        format!("{}Ref.current?.setSelectionRange({}, {})", ref_name, 
            self.format_expr(start), self.format_expr(end)),
    RefOpIR::Measure => 
        format!("{}Ref.current?.getBoundingClientRect()", ref_name),
    RefOpIR::CapturePointer { id } =>
        format!("{}Ref.current?.setPointerCapture({})", ref_name, self.format_expr(id)),
    RefOpIR::ReleasePointer { id } =>
        format!("{}Ref.current?.releasePointerCapture({})", ref_name, self.format_expr(id)),
    _ => format!("/* RefOp {:?} not applicable in React */", op),
}
```

- [ ] **Step 5: Add tests and commit**

```bash
cargo test -p omnilang 2>&1
git commit -m "feat(react): implement ref semantic binding codegen

- useRef for ref declarations
- @ref={name} → JSX ref={nameRef}
- focus/scroll-y/value semantic bindings → useEffect
- RefOpExpr formatting for select/setSelection/measure/capturePointer"
```

### Task 3.4: React animation — native refs, no DOM scanning

**Files:**
- Modify: `omnilang/src/codegen/react/motion/mod.rs`
- Modify: `omnilang/src/codegen/react/motion/keyframes.rs`
- Modify: `omnilang/src/codegen/react/component/motion_effects.rs`

- [ ] **Step 1: Replace document.querySelectorAll with element refs**

Remove all `document.querySelectorAll('[data-motion-enter]')` and `MutationObserver` on `document.body`. Instead, generate `useEffect` that calls `element.animate()` directly on the component's ref:

```typescript
// @enter={fadeIn} on element with ref={cardRef}
useEffect(() => {
  const el = cardRef.current;
  if (!el) return;
  const animation = el.animate(fadeIn_kf, fadeIn_opts);
  return () => animation.cancel();
}, []);
```

- [ ] **Step 2: Convert WAAPI keyframes to use semantic property names**

In `keyframes.rs`, map semantic properties to WAAPI:
```rust
fn semantic_to_waapi(prop: &str, value: &str) -> (String, String) {
    match prop {
        "opacity" => ("opacity".into(), value.into()),
        "scale" => ("transform".into(), format!("scale({})", value)),
        "translateX" => ("transform".into(), format!("translateX({}px)", value)),
        "translateY" => ("transform".into(), format!("translateY({}px)", value)),
        "rotate" => ("transform".into(), format!("rotate({}deg)", value)),
        "backgroundColor" => ("backgroundColor".into(), value.into()),
        "color" => ("color".into(), value.into()),
        _ => (prop.into(), value.into()),
    }
}
```

- [ ] **Step 3: Remove per-component MutationObservers**

Delete the `MutationObserver` code in `motion/mod.rs` for @enter, @hover, @press, @in-view effects. Replace each with direct ref-based effects.

- [ ] **Step 4: Handle @hover and @press with React event handlers**

```typescript
// @hover={scaleUp} on element with ref={cardRef}
const handleMouseEnter = useCallback(() => {
  cardRef.current?.animate(scaleUp_kf, { ...scaleUp_opts, direction: 'normal' });
}, []);
const handleMouseLeave = useCallback(() => {
  cardRef.current?.animate(scaleUp_kf, { ...scaleUp_opts, direction: 'reverse' });
}, []);
// JSX: <div ref={cardRef} onMouseEnter={handleMouseEnter} onMouseLeave={handleMouseLeave}>
```

- [ ] **Step 5: Commit**

```bash
git commit -m "refactor(react): rewrite animation codegen to use native refs

Replace document.querySelectorAll + MutationObserver (OmniView pattern)
with direct element refs + useEffect + useCallback.
@enter → useEffect + el.animate()
@hover → onMouseEnter/Leave + el.animate()
@press → onPointerDown/Up + el.animate()
@in-view → IntersectionObserver on ref element"
```

### Task 3.5: React design CSS emission

**Files:**
- Create: `omnilang/src/codegen/react/design.rs`
- Modify: `omnilang/src/codegen/react/mod.rs` (wire in)

- [ ] **Step 1: Extract shared design CSS emitter**

Copy `omnilang/src/codegen/omniview/design.rs` to `omnilang/src/codegen/shared/design.rs`. Remove OmniView-specific parts (editor bridges, debug CSS).

- [ ] **Step 2: Create React-specific design emitter**

In `react/design.rs`:
```rust
pub fn emit_react_design_css(designs: &[&DesignIR]) -> String {
    let mut css = String::new();
    for design in designs {
        // Theme-aware CSS custom properties
        css.push_str(&format!("/* Design: {} */\n", design.name));
        
        // Default theme tokens
        css.push_str(":root, [data-theme=\"light\"] {\n");
        for (name, token) in &design.tokens {
            css.push_str(&format!("  --{}: {};\n", name, token_css_value(token)));
        }
        css.push_str("}\n");
        
        // Theme overrides
        for theme in &design.themes {
            css.push_str(&format!("[data-theme=\"{}\"] {{\n", theme));
            for (name, token) in &design.tokens {
                if let Some(override_val) = token.theme_values.get(theme) {
                    css.push_str(&format!("  --{}: {};\n", name, token_css_value(override_val)));
                }
            }
            css.push_str("}\n");
        }
        
        // Classes
        for class in &design.classes {
            css.push_str(&format!(".{} {{ ", class.name));
            for (prop, val) in &class.properties {
                css.push_str(&format!("{}: {}; ", prop, val));
            }
            css.push_str("}\n");
        }
        
        // Variants
        for variant in &design.variants {
            for branch in &variant.branches {
                css.push_str(&format!(".{}-{} {{ ", variant.name, branch.name));
                // ...
                css.push_str("}\n");
            }
        }
    }
    css
}
```

- [ ] **Step 3: Wire into React codegen**

In `react/mod.rs`, after component generation, call `emit_react_design_css` and add `design.css` to output files.

- [ ] **Step 4: Commit**

```bash
git commit -m "feat(react): add design CSS emission

Generate design.css with CSS custom properties, theme-aware variables
(:root/[data-theme]), and class/variant CSS rules. Extracted shared
CSS emitter from omniview/design.rs."
```

---

## Phase 4: Flutter Codegen

### Task 4.1: Riverpod 3.0 store rewrite

**Files:**
- Modify: `omnilang/src/codegen/flutter/mod.rs`

- [ ] **Step 1: Replace StateProvider with Notifier + NotifierProvider**

Rewrite store generation from Riverpod 2.x style to 3.0:

```dart
// State class (immutable)
class TodoStoreState {
  final List<Todo> items;
  final String filter;
  const TodoStoreState({this.items = const [], this.filter = 'all'});
  TodoStoreState copyWith({List<Todo>? items, String? filter}) =>
    TodoStoreState(items: items ?? this.items, filter: filter ?? this.filter);
}

// Notifier
class TodoStoreNotifier extends Notifier<TodoStoreState> {
  @override
  TodoStoreState build() => const TodoStoreState();

  // Getters
  List<Todo> get filteredItems => state.items
    .where((item) => state.filter == 'all' || item.status == state.filter)
    .toList();
  int get activeCount => state.items
    .where((item) => item.status == 'active').length;

  // Actions
  void add(String text) {
    state = state.copyWith(items: [
      ...state.items,
      Todo(id: _nextId++, text: text, status: 'active'),
    ]);
  }
}

// Provider
final todoStoreProvider = NotifierProvider<TodoStoreNotifier, TodoStoreState>(
  TodoStoreNotifier.new,
);
```

- [ ] **Step 2: Fix C2 — generate real _runEffect_N methods**

Replace the inline effect body emission with a proper method:

```dart
void _runEffect_0() {
  // effect body
  saveToLocal(elapsed);
}

@override
void initState() {
  super.initState();
  _runEffect_0();  // initial run
}

@override
void didUpdateWidget(covariant Timer oldWidget) {
  super.didUpdateWidget(oldWidget);
  if (elapsed != oldWidget.elapsed) {
    _runEffect_0();  // re-run when deps change
  }
}
```

- [ ] **Step 3: Fix C3 — emit correct constructor**

Replace `const {` with `const CompName({`:
```dart
// Before: const {
// After:  const CompName({
const CompName({
  super.key,
  required this.title,
  this.onSubmit,
}) : super(key: key);
```

- [ ] **Step 4: Fix C4 — store reads in views**

Thread store-alias→provider-name mapping. For `TodoStore.items` in view context:
```dart
// Before: TodoStore.items  (undefined)
// After:  ref.watch(todoStoreProvider.select((s) => s.items))
```

Create a `store_provider_map: FxHashMap<String, String>` mapping store aliases to provider variable names.

- [ ] **Step 5: Fix C5 — color hex conversion**

Add `css_hex_to_dart_color` utility:
```rust
fn css_hex_to_dart_color(hex: &str) -> String {
    let hex = hex.trim_start_matches('#');
    match hex.len() {
        6 => format!("Color(0xFF{})", hex.to_uppercase()),
        8 => format!("Color(0x{})", hex.to_uppercase()),
        _ => format!("Color(0xFF000000)"), // fallback
    }
}
```

- [ ] **Step 6: Fix C6 — per-widget param whitelist**

Replace blind prop passthrough with widget-specific parameter maps:
```rust
fn widget_params(tag: &str) -> &[&str] {
    match tag {
        "Container" => &["color", "padding", "margin", "width", "height", "decoration"],
        "TextField" => &["controller", "focusNode", "decoration", "onChanged"],
        "ElevatedButton" => &["onPressed", "child", "style"],
        // ...
        _ => &[],
    }
}
```
Unmapped props → compiler warning diagnostic.

- [ ] **Step 7: Update snapshots and commit**

```bash
cargo test -p omnilang -- codegen::flutter 2>&1
cargo insta review
git commit -m "refactor(flutter): rewrite store to Riverpod 3.0 + 6 CRITICAL fixes

Riverpod 3.0: Notifier + NotifierProvider replace StateProvider.
Fixes:
C2: generate real _runEffect_N() method with didUpdateWidget deps check
C3: emit const CompName({ instead of const {
C4: store-alias→provider-name map for view context reads
C5: CSS hex → Dart Color conversion
C6: per-widget parameter whitelist with diagnostic for unmapped props"
```

### Task 4.2: Flutter ref codegen

**Files:**
- Create: `omnilang/src/codegen/flutter/ref.rs`
- Modify: `omnilang/src/codegen/flutter/mod.rs`
- Modify: `omnilang/src/codegen/flutter/view.rs`

- [ ] **Step 1: Generate GlobalKey + controllers per ref declaration**

For each `RefDeclIR`, analyze which operations are used and generate only needed helpers:

```dart
// ref scrollPane: Element (used: scrollTo)
final GlobalKey _scrollPaneKey = GlobalKey();
ScrollController? _scrollPaneScrollCtrl;

// ref searchBox: Element (used: focus, value)
final GlobalKey _searchBoxKey = GlobalKey();
FocusNode? _searchBoxFocusNode;
TextEditingController? _searchBoxTextCtrl;
```

- [ ] **Step 2: Wire @ref={name} to Widget key**

In `view.rs`, for elements with `ref_binding`:
```dart
TextField(
  key: _searchBoxKey,
  focusNode: _searchBoxFocusNode,
  controller: _searchBoxTextCtrl,
  ...
)
```

- [ ] **Step 3: Generate semantic binding handling**

For `focus={isSearchActive}`:
```dart
// In build():
if (isSearchActive) {
  WidgetsBinding.instance.addPostFrameCallback((_) {
    _searchBoxFocusNode?.requestFocus();
  });
}
```

For `scroll-y={scrollPosition}`:
```dart
// In initState or build:
WidgetsBinding.instance.addPostFrameCallback((_) {
  _scrollPaneScrollCtrl?.animateTo(
    scrollPosition,
    duration: Duration(milliseconds: 300),
    curve: Curves.easeInOut,
  );
});
```

- [ ] **Step 4: Handle RefOpExpr in Dart expression formatting**

In `flutter/expr.rs`, add match arm for `ExprIR::RefOpExpr`.

- [ ] **Step 5: Commit**

```bash
git commit -m "feat(flutter): implement ref semantic codegen

Generate GlobalKey + on-demand FocusNode/ScrollController/TextEditingController
per ref declaration. Semantic bindings (focus, scroll-y, value) mapped to
post-frame callbacks. RefOpExpr formatting for select/setSelection/measure."
```

### Task 4.3: Flutter slot codegen

**Files:**
- Modify: `omnilang/src/codegen/flutter/view.rs`
- Modify: `omnilang/src/codegen/flutter/mod.rs`

- [ ] **Step 1: Add Widget? params for slots**

For each slot in component, generate widget parameter:
```dart
class Modal extends ConsumerStatefulWidget {
  final Widget? header;
  final Widget? footer;
  final Widget? child;  // default slot

  const Modal({
    super.key,
    this.header,
    this.footer,
    this.child,
  });
}
```

- [ ] **Step 2: Render slots in build**

Replace `SizedBox.shrink()` slot placeholder with:
```dart
// Default slot
widget.child ?? DefaultHeader()

// Named slot
widget.header ?? DefaultHeader()
```

- [ ] **Step 3: Commit**

```bash
git commit -m "feat(flutter): implement slot codegen

Add Widget? params for each slot declaration. Render slots in build
with ?? fallback operator. Fixes H1/H7 from cross-target review."
```

### Task 4.4: Flutter TweenSequence animation

**Files:**
- Modify: `omnilang/src/codegen/flutter/motion.rs`
- Modify: `omnilang/src/codegen/flutter/mod.rs`

- [ ] **Step 1: Generate per-property TweenSequence for multi-stop keyframes**

Replace single `Tween<double>` with per-property `Animatable<T>`:
```dart
// For motion fadeIn with keyframe { 0%: opacity:0, scale:0.95; 50%: opacity:0.5; 100%: opacity:1, scale:1 }

late final AnimationController _fadeIn_ctrl;
late final Animation<double> _fadeIn_opacity;
late final Animation<double> _fadeIn_scale;

void _initFadeIn(TickerProvider vsync) {
  _fadeIn_ctrl = AnimationController(
    duration: const Duration(milliseconds: 300),
    vsync: vsync,
  );
  
  // Per-property TweenSequence
  _fadeIn_opacity = TweenSequence<double>([
    TweenSequenceItem(tween: Tween(begin: 0.0, end: 0.5), weight: 50.0),
    TweenSequenceItem(tween: Tween(begin: 0.5, end: 1.0), weight: 50.0),
  ]).animate(_fadeIn_ctrl);
  
  _fadeIn_scale = Tween<double>(begin: 0.95, end: 1.0).animate(
    CurvedAnimation(parent: _fadeIn_ctrl, curve: Curves.easeOut),
  );
}
```

- [ ] **Step 2: Map semantic animation properties to Flutter widgets**

In `build_animated_builder_body`, for each semantic property:
```dart
// opacity → Opacity widget
// scale → Transform.scale
// translateX/Y → Transform.translate
// rotate → Transform.rotate
// backgroundColor → Container color
// color → TextStyle color
return AnimatedBuilder(
  animation: _fadeIn_ctrl,
  builder: (context, child) {
    return Opacity(
      opacity: _fadeIn_opacity.value,
      child: Transform.scale(
        scale: _fadeIn_scale.value,
        child: child,
      ),
    );
  },
  child: /* original widget */,
);
```

- [ ] **Step 3: Handle playback properties**

Implement `loop` (repeat), `direction` (forward/reverse/Controller.repeat), `fill` (Tween end value), `delay` (Future.delayed before forward):

```dart
// loop 3
_fadeIn_ctrl.repeat(count: 3);

// direction: alternate  
_fadeIn_ctrl.repeat(reverse: true);

// delay: 100ms
Future.delayed(Duration(milliseconds: 100), () => _fadeIn_ctrl.forward());
```

- [ ] **Step 4: Fix duration parsing**

Parse `300ms` / `1.5s` / `0.5` (seconds) correctly:
```rust
fn parse_duration_ms(s: &str) -> u64 {
    if s.ends_with("ms") {
        s.trim_end_matches("ms").parse().unwrap_or(0)
    } else if s.ends_with('s') {
        (s.trim_end_matches('s').parse::<f64>().unwrap_or(0.0) * 1000.0) as u64
    } else {
        (s.parse::<f64>().unwrap_or(0.0) * 1000.0) as u64
    }
}
```

- [ ] **Step 5: Add diagnostics**

Emit warning for `width`/`height` animation properties in Flutter.
Emit error for `steps()` easing.

- [ ] **Step 6: Commit**

```bash
git commit -m "feat(flutter): implement TweenSequence animation codegen

Per-property Animatable<T> for multi-stop keyframes. Semantic animation
properties mapped to Flutter widgets (Opacity, Transform.scale, etc.).
Playback properties (loop/direction/fill/delay) implemented.
Duration parsing fixed for ms/s/raw number formats."
```

### Task 4.5: Flutter design token / ThemeData codegen

**Files:**
- Create: `omnilang/src/codegen/flutter/design.rs`
- Modify: `omnilang/src/codegen/flutter/mod.rs`

- [ ] **Step 1: Generate Dart constants for static tokens**

```dart
abstract final class AppTokens {
  static const Color primaryColor = Color(0xFF6366F1);
  static const double spacingSm = 8.0;
  static const double radiusMd = 8.0;
  static const String fontMain = 'Inter';
  static const Duration durationNormal = Duration(milliseconds: 300);
  static const Cubic easeOut = Cubic(0.16, 1, 0.3, 1);
}
```

- [ ] **Step 2: Generate BoxDecoration factories for classes**

```dart
abstract final class AppDesign {
  static BoxDecoration cardDecoration() => BoxDecoration(
    color: AppTokens.primaryColor,
    borderRadius: BorderRadius.circular(AppTokens.radiusMd),
    boxShadow: [AppTokens.shadowCard],
  );
}
```

- [ ] **Step 3: Generate ThemeExtension for variants**

```dart
class AppSizeTheme extends ThemeExtension<AppSizeTheme> {
  final EdgeInsets padding;
  final double fontSize;
  const AppSizeTheme({required this.padding, required this.fontSize});
  
  static const sm = AppSizeTheme(padding: EdgeInsets.all(8), fontSize: 12);
  static const md = AppSizeTheme(padding: EdgeInsets.all(16), fontSize: 14);
  static const lg = AppSizeTheme(padding: EdgeInsets.all(24), fontSize: 18);
  
  @override AppSizeTheme copyWith({EdgeInsets? padding, double? fontSize}) =>
    AppSizeTheme(padding: padding ?? this.padding, fontSize: fontSize ?? this.fontSize);
  @override AppSizeTheme lerp(AppSizeTheme? other, double t) =>
    AppSizeTheme(
      padding: EdgeInsets.lerp(padding, other?.padding ?? padding, t)!,
      fontSize: lerpDouble(fontSize, other?.fontSize ?? fontSize, t)!,
    );
}

// Register in MaterialApp:
MaterialApp(
  theme: ThemeData(extensions: [AppSizeTheme.md]),
)
```

- [ ] **Step 4: Generate theme switching**

```dart
final themeModeProvider = StateProvider<ThemeMode>((ref) => ThemeMode.light);

// .omni: theme = "dark" → 
ref.read(themeModeProvider.notifier).state = ThemeMode.dark;
```

- [ ] **Step 5: Generate dynamic token handling**

```dart
final accentColorProvider = Provider<Color>((ref) {
  return ref.watch(dynamicTokenStoreProvider.select((s) => s.accentColor));
});

class MyApp extends ConsumerWidget {
  Widget build(BuildContext context, WidgetRef ref) {
    final accentColor = ref.watch(accentColorProvider);
    return MaterialApp(
      theme: ThemeData.light().copyWith(
        colorScheme: ColorScheme.fromSeed(seedColor: accentColor),
      ),
    );
  }
}
```

- [ ] **Step 6: Commit**

```bash
git commit -m "feat(flutter): implement design token codegen

Static tokens → typed Dart constants. Classes → BoxDecoration factories.
Variants → ThemeExtension<T>. Theme switching → ThemeMode + StateProvider.
Dynamic tokens → Riverpod Provider + ThemeData.copyWith."
```

---

## Phase 5: CI Gates & Testing

### Task 5.1: Expand parameterized test matrix

**Files:**
- Modify: `omnilang/src/codegen/tests_regression/parameterized.rs`

- [ ] **Step 1: Add test fixtures for each feature floor category**

Create `.omni` fixture files:
- `tests/fixtures/parameterized/design_tokens.omni` — design block with token + class + variant
- `tests/fixtures/parameterized/slots.omni` — slot default + named + fallback
- `tests/fixtures/parameterized/refs.omni` — semantic bindings + semantic methods
- `tests/fixtures/parameterized/animation.omni` — multi-stop keyframe + playback
- `tests/fixtures/parameterized/key_expr.omni` — @for with key + empty

- [ ] **Step 2: Add cross-target compilation tests**

For each fixture, compile to React and Flutter targets, verify:
```rust
#[test]
fn design_tokens_compiles_react() {
    let output = compile_fixture("design_tokens.omni", Target::React);
    assert!(output.errors.is_empty());
    assert!(!output.files.is_empty()); // No more zero-file gate
    // Verify design.css exists
    assert!(output.files.iter().any(|f| f.name == "design.css"));
}
```

- [ ] **Step 3: Commit**

```bash
git commit -m "test: expand parameterized test matrix for feature floor

Add fixtures for design tokens, slots, refs, animation, key_expr.
All fixtures compile to React and Flutter targets with assertions
on expected output files."
```

### Task 5.2: Add CI validation gates

**Files:**
- Create: `.github/workflows/codegen-validation.yml` (or modify existing CI)
- Modify: CI script to run TypeScript/Dart checks

- [ ] **Step 1: Add TypeScript check for React output**

```yaml
- name: Validate React output
  run: |
    cargo run -- build --target react tests/fixtures/parameterized/design_tokens.omni -o /tmp/react-out
    cd /tmp/react-out && npx tsc --noEmit --strict *.tsx *.ts
```

- [ ] **Step 2: Add Dart check for Flutter output**

```yaml
- name: Validate Flutter output
  run: |
    cargo run -- build --target flutter tests/fixtures/parameterized/design_tokens.omni -o /tmp/flutter-out
    cd /tmp/flutter-out && dart analyze
```

- [ ] **Step 3: Commit**

```bash
git commit -m "ci: add TypeScript and Dart validation gates

Run tsc --noEmit on React regression fixture output.
Run dart analyze on Flutter regression fixture output.
Prevents regression of C1-C6 class bugs (uncompilable output
encoded in golden snapshots)."
```

---

## Phase 6: Demo Project Migration

### Task 6.1: Migrate learn-omniview animation syntax

**Files:** Modify all 8 animation demo files + 5 react fixtures.

- [ ] **Step 1: Rewrite motion body syntax**

For each file, convert:
```
Old:  over 500 ease power2Out
New:  over: 500ms
      easing: ease-out

Old:  from { x: 0; y: 0 }
New:  from { translateX: 0; translateY: 0 }

Old:  from { opacity: 0; scale: 0.8 }
New:  from { opacity: 0; scale: 0.8 }  ← opacity/scale unchanged

Old:  @inView={motion} @in-view-once={false}
New:  @in-view={motion} once={false}
```

Files:
- `test/projects/learn-omniview/src/pages/MotionDemo.omni`
- `test/projects/learn-omniview/src/pages/ElementRefDemo.omni` (motion sections)
- `test/projects/learn-omniview/src/pages/MotionTriggersDemo.omni`
- `test/projects/learn-omniview/src/pages/KeyframeAnimationDemo.omni`
- `test/projects/learn-omniview/src/pages/EasingFunctionsDemo.omni`
- `test/projects/learn-omniview/src/pages/AnimPropertiesDemo.omni`
- `test/projects/learn-omniview/src/pages/CssVarsDemo.omni` (motion sections)
- `tests/fixtures/react/motion_*.omni`

- [ ] **Step 2: Rewrite trigger attribute names**

```
Old:  @inView={motion} @in-view-once={false} @in-view-repeat={true} @in-view-threshold={0.5}
New:  @in-view={motion} once={false} repeat={true} threshold={0.5}

Old:  @scroll-start="top 80%" @scroll-end="bottom 20%" @scroll-scrub={true}
New:  @scroll={motion} start=0.8 end=0.2 scrub={true}

Old:  @dragHorizontal={motion} @drag-constraints="left:0 right:300" @drag-elastic={0.2}
New:  @drag-horizontal={motion} constraints={300} elastic={0.2}
```

- [ ] **Step 3: Remove `@paused={...}` element attributes**

`KeyframeAnimationDemo.omni` L312: remove `@paused={isPaused}`, replace with conditional binding if needed.

- [ ] **Step 4: Fix `steps()` easing syntax**

`EasingFunctionsDemo.omni` L242:
```
Old:  ease steps(count: 6, direction: end)
New:  easing: steps(6, end)
```

- [ ] **Step 5: Commit**

```bash
git commit -m "refactor(demo): migrate learn-omniview animation syntax to semantic DSL

Motion body: over/ease → over:/easing: with units. Keyframe properties:
x/y → translateX/translateY. Trigger renames: @inView → @in-view,
@dragHorizontal → @drag-horizontal, sub-attrs → independent attrs.
Remove @paused attribute. Fix steps() easing syntax."
```

### Task 6.2: Migrate learn-omniview ref syntax

**Files:**
- Modify: `test/projects/learn-omniview/src/pages/ElementRefDemo.omni`

- [ ] **Step 1: Replace ref.style.* with style bindings or signals**

```
Old:  sizeBox.style.width = "300px"
      sizeBox.style.height = "120px"
New:  <div style: { width: sizeBoxWidth, height: sizeBoxHeight } />
```

- [ ] **Step 2: Replace ref.focus()/blur() with focus={signal} binding**

```
Old:  inputBox.focus()
New:  signal inputFocused = false
      <input @ref={inputBox} focus={inputFocused} />
      // in action: inputFocused = true
```

- [ ] **Step 3: Replace ref.scrollTo/scrollBy with scroll-y={signal}**

```
Old:  scrollArea.scrollTo(0, 0)
New:  signal scrollY = 0
      <ScrollView @ref={scrollPane} scroll-y={scrollY} />
      // in action: scrollY = 0
```

- [ ] **Step 4: Replace ref.getBoundingClientRect() with ref.measure()**

```
Old:  dragTarget.getBoundingClientRect()
New:  dragTarget.measure()
```

- [ ] **Step 5: Replace ref.offsetWidth/offsetHeight with ref.measure().width/.height**

```
Old:  sizeBox.offsetWidth
New:  sizeBox.measure().width
```

- [ ] **Step 6: Commit**

```bash
git commit -m "refactor(demo): migrate ElementRefDemo to semantic ref syntax

Replace DOM-style ref operations with semantic equivalents:
- ref.style.* → style:{...} signal binding
- ref.focus() → focus={sig} binding  
- ref.scrollTo() → scroll-y={sig} binding
- ref.getBoundingClientRect() → ref.measure()
- ref.offsetWidth/Height → ref.measure().width/.height"
```

### Task 6.3: Migrate learn-omniview slot syntax

**Files:**
- Modify: `test/projects/learn-omniview/src/components/Modal.omni`
- Modify: `test/projects/learn-omniview/src/components/NotificationsPanel.omni`
- Modify: `test/projects/learn-omniview/src/components/SlotDefaultDemo.omni`
- Modify: `test/projects/learn-omniview/src/pages/SignalPatternsDemo.omni`

- [ ] **Step 1: Move @slot declarations from view template to component body**

```
Old (inline in view):
  <div>
    @slot header { <DefaultHeader /> }
    @slot default
    {header}
    {children}
  </div>

New (body + view):
  component Modal {
    @slot header { <DefaultHeader /> }
    @slot default
    
    <div>
      {header}
      {children}
    </div>
  }
```

- [ ] **Step 2: Update consumer syntax from slot="name" to named props**

```
Old:  <Modal><div slot="header">Title</div>content</Modal>
New:  <Modal header={<div>Title</div>}>content</Modal>
```

- [ ] **Step 3: Update @slot(name) parenthesized form to @slot name**

```
Old:  @slot(header)
New:  @slot header
```

- [ ] **Step 4: Commit**

```bash
git commit -m "refactor(demo): migrate slot syntax to semantic DSL

Move @slot declarations from view template to component body.
Render via {header}/{children} interpolation. Consumer side:
slot= attribute → named props. @slot(name) → @slot name."
```

### Task 6.4: Update @if mode= syntax

**Files:**
- Modify: `test/projects/learn-omniview/src/pages/MotionDemo.omni`
- Modify: `test/projects/learn-omniview/src/pages/SignalPatternsDemo.omni`

- [ ] **Step 1: Fix syntax separator and values**

```
Old:  @if isOpen mode="sync"
New:  @if isOpen mode: "wait"   ("sync" removed, use "wait")

Old:  @if showPanel mode="wait"
New:  @if showPanel mode: "wait"  (= → :)
```

- [ ] **Step 2: Commit**

```bash
git commit -m "refactor(demo): fix @if mode syntax to semantic DSL

mode= → mode:. Remove 'sync' value (use 'wait' instead)."
```

---

## Phase 7: Governance Update

### Task 7.1: Update ADRs and deferrals

**Files:**
- Modify: `.architecture/decisions/adrs/ADR-020-flutter-state-management-riverpod.md`
- Modify: `.architecture/decisions/adrs/ADR-021-cross-target-feature-parity.md`
- Modify: `.architecture/deferrals.md`

- [ ] **Step 1: Update ADR-020 status to Riverpod 3.0 Implemented**

Update status section to reflect Riverpod 3.0 Notifier + NotifierProvider implementation.

- [ ] **Step 2: Update ADR-021 to behavior floor + diagnostics**

Replace "semantically equivalent output" with "semantically equivalent behavior for documented feature floor + hard diagnostics outside it."

- [ ] **Step 3: Update deferrals.md**

Remove completed items. Add any new deferred items discovered during implementation.

- [ ] **Step 4: Commit**

```bash
git commit -m "docs(adr): update ADR-020, ADR-021, deferrals

ADR-020: mark Riverpod 3.0 as Implemented.
ADR-021: redefine as behavior floor + diagnostics per ADR-052.
Deferrals: sync with post-implementation state."
```

---

## Implementation Order

```
Phase 0 (IR):  0.1 → 0.2 → 0.3        [sequential — each touches types.rs]
Phase 1 (Sem):  1.1, 1.2               [parallel — independent passes]
Phase 2 (Shared): 2.1 → 2.2            [sequential]
Phase 3 (React):  3.1 → 3.2 → 3.3 → 3.4 → 3.5  [sequential within, parallel with Flutter]
Phase 4 (Flutter): 4.1 → 4.2 → 4.3 → 4.4 → 4.5  [sequential within, parallel with React]
Phase 5 (CI):    5.1, 5.2              [parallel]
Phase 6 (Demo):  6.1 → 6.2 → 6.3 → 6.4  [sequential]
Phase 7 (Gov):   7.1                    [last]
```

**Critical path**: 0.1 → 0.2 → 0.3 → 1.1 → 2.2 → 3.1/4.1 → 5.1
**Parallelizable**: 3.x and 4.x can run in parallel after Phase 2 completes.
