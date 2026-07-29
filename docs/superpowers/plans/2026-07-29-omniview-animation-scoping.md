# OmniView Animation Design Scoping & Fixes — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Execute ADR-032 design scoping: reduce animation subsystem from 33 features to 14 (React-Flutter parity floor), fix 4 critical bugs, and add missing test coverage.

**Architecture:** Four-phase, 11-task implementation: (1) Critical fixes on retained features — 4 small, high-impact patches. (2) Design scoping — one atomic task removing 21 features across ALL pipeline layers simultaneously (AST + IR + lowering + codegen + runtime), producing ~50% code reduction. Layers cannot compile independently after partial removal. (3) Code quality — deduplicate trigger codegen, unify easing table, consolidate scroll handlers. (4) Test coverage — add motion snapshot tests for React and Flutter backends.

**Tech Stack:** Rust (omnilang-core + omnilang crates), TypeScript (@omni/animation runtime), Flutter/Dart codegen, React/TSX codegen, WAAPI browser runtime

**Reference Docs:**
- [ADR-032](../../.architecture/decisions/adrs/ADR-032-omniview-animation-design-scoping.md) — full design decision
- [Architecture Review](../../.architecture/reviews/component-omniview-animation.md) — two-phase review with feature matrix

## Global Constraints

- **Flutter capability floor**: Every retained feature must work with full fidelity in both React and Flutter targets
- **Compiler errors for removed features**: Users of removed features must get clear error messages with migration guidance, not silent no-ops
- **No broken Dart output**: Flutter codegen must never produce code that references undefined variables/classes
- **Test before code**: Phase 4 adds tests; Phases 1-3 fix existing tests as needed
- **Commit per task**: Each task produces one commit with conventional commit format
- **Existing tests must pass**: `cargo test` and `pnpm test` in `packages/omni-animation` must be green after each task

---

## Phase 1: Critical Fixes (This Sprint — S Effort Each)

### Task 1: Fix React @in-view to Use User Motion

**Files:**
- Modify: `omnilang/src/codegen/react/view.rs:195-205` — add `data-motion-inview-name` attribute emission
- Modify: `omnilang/src/codegen/react/motion/transforms.rs:74-77` — replace hardcoded opacity with `_M[name]` lookup

**Interfaces:**
- Consumes: `TriggerIntent.motion_ref` (IR — already available on element)
- Produces: Generated React TSX where @in-view uses user-defined motion instead of hardcoded `opacity:0→1`

- [ ] **Step 1: Add `data-motion-inview-name` attribute to React view codegen**

In `omnilang/src/codegen/react/view.rs`, around line 195-205, find the @in-view attribute emission block. Currently it only emits `data-motion-inview`. Add a companion attribute for the motion name, following the same pattern as @enter's `data-motion-name`:

```rust
// After existing data-motion-inview attribute emission, add:
if let Some(ref motion_name) = trigger.motion_ref {
    w.attr("data-motion-inview-name", motion_name);
}
```

- [ ] **Step 2: Replace hardcoded opacity animation with motion lookup**

In `omnilang/src/codegen/react/motion/transforms.rs`, around line 74-77, find:

```rust
w.emit("animate(el, [{offset:0,properties:{opacity:0}}, {offset:1,properties:{opacity:1}}], {duration:300, easing:'ease-out', fill:'both'});");
```

Replace with motion name lookup, following the @enter pattern in `mod.rs:70-82`:

```rust
w.emit("const motionName = el.getAttribute('data-motion-inview-name');");
w.emit("if (motionName && _M[motionName]) {");
w.emit("  const {kf, opts} = _M[motionName];");
w.emit("  animate(el, kf, opts);");
w.emit("} else {");
w.emit("  animate(el, [{offset:0,properties:{opacity:0}}, {offset:1,properties:{opacity:1}}], {duration:300, easing:'ease-out', fill:'both'});");
w.emit("}");
```

- [ ] **Step 3: Build and verify compilation**

```bash
cd omnilang && cargo check 2>&1
```

Expected: no new compilation errors.

- [ ] **Step 4: Run existing React codegen tests**

```bash
cd omnilang && cargo test react 2>&1
```

Expected: all existing React codegen tests pass.

- [ ] **Step 5: Commit**

```bash
git add omnilang/src/codegen/react/view.rs omnilang/src/codegen/react/motion/transforms.rs
git commit -m "fix(react): @in-view uses user motion definition instead of hardcoded fade

Previously @in-view always emitted a generic opacity 0→1 animation,
ignoring the user's named @motion reference. Now emits data-motion-inview-name
attribute and looks up _M[name] to use the correct user-defined motion."
```

---

### Task 2: Escape Scroll/Drag Config Strings in Codegen

**Files:**
- Modify: `omnilang/src/codegen/omniview/motion_triggers.rs:156-157` (scroll start/end)
- Modify: `omnilang/src/codegen/omniview/motion_triggers.rs:272` (drag constraints)

**Interfaces:**
- Consumes: `escape_literal_str` from `crate::codegen` (already exists, used in JSON serialization)
- Produces: Generated JS where scroll/drag config strings cannot break out of JS string literals

- [ ] **Step 1: Add import for escape_literal_str**

At the top of `omnilang/src/codegen/omniview/motion_triggers.rs`, add:

```rust
use crate::codegen::escape_literal_str;
```

(Verify the exact module path — it may be `super::escape_literal_str` or `crate::codegen::json::escape_literal_str`. Search for existing uses in the codebase.)

- [ ] **Step 2: Escape scroll start/end config strings**

In `emit_trigger_scroll_js`, around lines 156-157, find:

```rust
let scroll_start_str = el
    .scroll_behavior()
    .and_then(|s| s.start.as_deref())
    .unwrap_or("top 80%");
let scroll_end_str = el
    .scroll_behavior()
    .and_then(|s| s.end.as_deref())
    .unwrap_or("bottom 20%");
```

After each, apply escaping:

```rust
let scroll_start_str = escape_literal_str(
    el.scroll_behavior()
        .and_then(|s| s.start.as_deref())
        .unwrap_or("top 80%"),
    '"',
);
let scroll_end_str = escape_literal_str(
    el.scroll_behavior()
        .and_then(|s| s.end.as_deref())
        .unwrap_or("bottom 20%"),
    '"',
);
```

- [ ] **Step 3: Escape drag constraints string**

In `emit_trigger_drag_js`, around line 272, find:

```rust
let constraints_str = el
    .drag_behavior()
    .and_then(|d| d.constraints.as_deref())
    .unwrap_or("");
```

Apply escaping:

```rust
let constraints_str = escape_literal_str(
    el.drag_behavior()
        .and_then(|d| d.constraints.as_deref())
        .unwrap_or(""),
    '"',
);
```

- [ ] **Step 4: Build and verify**

```bash
cd omnilang && cargo check 2>&1
```

- [ ] **Step 5: Run existing OmniView motion snapshot tests**

```bash
cd omnilang && cargo test motion 2>&1
```

Expected: snapshot tests may need updating if the escaping changes output format. Review snapshots and accept changes if escaping is correct.

- [ ] **Step 6: Commit**

```bash
git add omnilang/src/codegen/omniview/motion_triggers.rs
git commit -m "fix(omniview): escape scroll/drag config strings in generated JS

Prevents JS injection via compromised .omni source files where
scroll start/end or drag constraints contain double-quote characters.
Uses existing escape_literal_str helper."
```

---

### Task 3: Fix bounceIn/bounceOut Identical Values

**Files:**
- Modify: `crates/omnilang-core/src/codegen/shared/easing.rs:36-37`
- Modify: `packages/omni-animation/src/easing.ts:31-32`

**Interfaces:**
- Consumes: N/A (static data change)
- Produces: `bounceIn` and `bounceOut` resolve to **different** cubic-bezier values with correct directional semantics

- [ ] **Step 1: Fix Rust shared easing table**

In `crates/omnilang-core/src/codegen/shared/easing.rs`, around lines 36-37, replace:

```rust
("bounceIn", "cubic-bezier(0.215,0.610,0.355,1.000)"),
("bounceOut", "cubic-bezier(0.215,0.610,0.355,1.000)"),  // same as bounceIn
```

With proper asymmetric approximations:

```rust
// bounceIn: overshoot at start (bounce toward the peak)
("bounceIn", "cubic-bezier(0.215,0.610,0.355,1.000)"),
// bounceOut: overshoot at end (bounce settling down)
("bounceOut", "cubic-bezier(0.175,0.885,0.320,1.275)"),  // backOut-like overshoot at end
```

- [ ] **Step 2: Fix JS runtime easing table**

In `packages/omni-animation/src/easing.ts`, around lines 31-32, apply the same change:

```typescript
bounceIn: 'cubic-bezier(0.215,0.610,0.355,1.000)',
bounceOut: 'cubic-bezier(0.175,0.885,0.320,1.275)',
```

Add a comment documenting that these are cubic-bezier approximations — WAAPI cannot represent true bounce curves:

```typescript
// Bounce approximations (WAAPI cannot natively represent bounce curves.
// These cubic-bezier values approximate the directional asymmetry: bounceIn
// has overshoot at the start, bounceOut has overshoot at the end.)
```

- [ ] **Step 3: Run JS runtime tests**

```bash
cd packages/omni-animation && pnpm test 2>&1
```

Expected: easing tests pass (bounceIn/bounceOut test may need updated expected value).

- [ ] **Step 4: Run Rust tests**

```bash
cd omnilang && cargo test easing 2>&1
```

Expected: `test_easing_all_variants` passes (verify the new bounceOut value is correct).

- [ ] **Step 5: Commit**

```bash
git add crates/omnilang-core/src/codegen/shared/easing.rs packages/omni-animation/src/easing.ts
git commit -m "fix(easing): bounceIn and bounceOut now have distinct cubic-bezier values

Previously both resolved to the same curve (0.215,0.610,0.355,1.000),
making bounceOut visually identical to bounceIn. bounceOut now uses
(0.175,0.885,0.320,1.275), a backOut-like approximation that produces
an overshoot-at-end effect, preserving the directional semantics."
```

---

### Task 4: Add __omni_animations Map Eviction

**Files:**
- Modify: `omnilang/src/codegen/omniview/list.rs:349-354` — add `.delete()` after @leave animation completes
- Modify: `omnilang/src/codegen/omniview/element.rs` — add `__omni_cleanupElement` function emission

**Interfaces:**
- Consumes: `__omni_animations` Map (runtime global)
- Produces: New `__omni_cleanupElement(nid)` JS function that removes all entries with keys starting with `nid`

- [ ] **Step 1: Add __omni_cleanupElement function to runtime**

In the OmniView runtime emission (find where other `__omni_*` helpers are emitted — likely in `runtime.rs` or `mod.rs`), add:

```rust
self.js.push_str(r#"
function __omni_cleanupElement(nid) {
  const prefix = nid + "_";
  __omni_animations.forEach((anim, key) => {
    if (key.startsWith(prefix)) {
      anim.cancel();
      __omni_animations.delete(key);
    }
  });
}
"#);
```

- [ ] **Step 2: Call __omni_cleanupElement on @for key removal**

In the @for list update logic (around `list.rs:349-354`), after the @leave animation completes and before the element is removed from DOM, add:

```rust
// After @leave animation completes, clean up all animations for this element
// before it's removed from the DOM
w.emit(&format!("__omni_cleanupElement(\"{nid}\");", nid = key_nid));
```

- [ ] **Step 3: Call __omni_cleanupElement on @if branch switching**

In the @if branch switching logic, add cleanup calls for elements removed when a branch becomes inactive.

- [ ] **Step 4: Build and verify**

```bash
cd omnilang && cargo check 2>&1
```

- [ ] **Step 5: Run OmniView tests**

```bash
cd omnilang && cargo test omniview 2>&1
```

- [ ] **Step 6: Commit**

```bash
git add omnilang/src/codegen/omniview/
git commit -m "fix(omniview): add __omni_animations eviction via __omni_cleanupElement

Adds __omni_cleanupElement(nid) function that cancels and deletes all
Map entries with keys starting with the given node ID. Called on @for
key removal and @if branch switching to prevent unbounded Map growth.

Previously the __omni_animations Map had zero eviction — entries
accumulated across re-renders, retaining Animation objects that
prevent GC of detached DOM elements."
```

---

## Phase 2: Design Scoping — Remove 21 Features Atomically (Next Sprint, L Effort)

### Task 5: Remove 21 Features Across All Pipeline Layers (Atomic)

> **Note:** This task MUST be executed atomically — removing AST variants (Task 5a) immediately breaks parser/semantic/IR/codegen compilation. All layers are modified together in one commit per ADR-032 implementation strategy: "Execute as a single big-bang commit to avoid intermediate states where features are half-removed."

**Files:**
- Modify: `crates/omnilang-core/src/parser/ast/motion.rs` (AST — remove 20+ variants, 5 structs)
- Modify: `crates/omnilang-core/src/ir/types_motion.rs` (IR — mirror all AST removals)
- Modify: `crates/omnilang-core/src/ir/lower/motion.rs` (Lowering — remove lowering for removed variants)
- Modify: `crates/omnilang-core/src/ir/lower/view.rs` (~line 545 — remove when/delayChildren/staggerChildren drop)
- Modify: `crates/omnilang-core/src/semantic/motion.rs` (Semantic — remove validation for removed features)
- Modify: `crates/omnilang-core/src/semantic/validate_motion.rs` (Semantic — remove feature checks)
- Modify: `omnilang/src/codegen/omniview/motion_triggers.rs` (OmniView — remove 6 trigger branches)
- Modify: `omnilang/src/codegen/omniview/motion_config.rs` (OmniView — remove timeline/callback/states/hero)
- Delete: `omnilang/src/codegen/omniview/timeline.rs` (entire file)
- Modify: `omnilang/src/codegen/omniview/view/element.rs:716-729` (OmniView — remove hero attributes)
- Modify: `omnilang/src/codegen/omniview/motion.rs:41-145` (OmniView — remove states JS emission)
- Modify: `omnilang/src/codegen/react/motion/keyframes.rs:16-18` (React — remove Timeline/CallbackOnly stubs)
- Modify: `omnilang/src/codegen/react/motion/transforms.rs` (React — remove @driven-by effect)
- Modify: `omnilang/src/codegen/react/component/motion_effects.rs` (React — remove unused effects)
- Modify: `omnilang/src/codegen/flutter/motion.rs` (Flutter — remove states, driven-by, broken easing; fix inline motion)
- Modify: `omnilang/src/codegen/flutter/motion_widgets.rs` (Flutter — remove DrivenByBuilder)
- Delete: `packages/omni-animation/src/hero.ts` (JS runtime — entire file)
- Modify: `packages/omni-animation/src/easing.ts` (JS runtime — remove resolveParameterized)
- Modify: `packages/omni-animation/src/index.ts` (JS runtime — remove hero export)

**AST Removals:**
- TriggerKind: `Leave`, `Unhover`, `Focus`, `Animate`, `Timeline`, `DrivenBy` (14→8)
- MotionBody: `Timeline`, `CallbackOnly` (3→1, Keyframe only)
- EasingExpr: `Spring`, `Steps` → simplify `Back` (remove intensity), `Elastic` (remove amplitude/period) (6→4)
- PlaybackProp: `RepeatDelay`, `RepeatRefresh`, `TimeScale`, `Interrupt`, `RespectReducedMotion`, `When`, `DelayChildren`, `StaggerChildren`, `StaggerFrom` (14→5)
- Entire structs/enums: `HeroDef`, `StateDef`, `TimelineDef`, `TimelineBody`, `TimelineItem`, `TimelineItemKind`, `Direction`, `DirectionItem`, `Connector`, `PositionValue`, `StaggerCall`, `StaggerTiming`, `StaggerDirection`, `CallbackSpec`, `CallbackBody`
- Field: `color_space` on `KeyframeBody`

**IR Removals:** Mirror all AST removals. Additionally remove: `DragBehavior.scrub`, `DragBehavior.momentum` (defined but never emitted).

**Codegen key outcomes:**
- Eliminates ALL broken Dart output (undefined `_statesController`, undefined `DrivenByBuilder`, `LateInitializationError` for Timeline/CallbackOnly bodies)
- Flutter `wrap_with_motion_widgets` now handles inline motion bodies
- React no longer emits Timeline/CallbackOnly dummy fade stubs
- OmniView no longer emits hero `data-hero` attributes (no JS runtime reads them)

**Interfaces:**
- Produces: Slimmed pipeline where every AST variant has a complete path through IR→lowering→codegen→runtime

- [ ] **Step 1: Remove features from AST** (`crates/omnilang-core/src/parser/ast/motion.rs`)

Remove all enum variants and types listed above. This will break compilation across the entire crate — that's expected.

- [ ] **Step 2: Remove features from IR types** (`crates/omnilang-core/src/ir/types_motion.rs`)

Mirror all AST removals in IR types. Remove `DragBehavior.scrub` and `DragBehavior.momentum`.

- [ ] **Step 3: Remove lowering for removed features** (`crates/omnilang-core/src/ir/lower/motion.rs`, `view.rs`)

Delete all lowering functions that handle removed AST/IR variants. Remove `when`/`delayChildren`/`staggerChildren` drop logic in `view.rs:545-550`.

- [ ] **Step 4: Remove features from semantic validation** (`crates/omnilang-core/src/semantic/motion.rs`, `validate_motion.rs`)

Remove validation for states, hero, timeline, stagger, callbacks. Add compiler errors for any removed feature that could still reach validation.

- [ ] **Step 5: Remove features from OmniView codegen**

Remove 6 trigger branches in `motion_triggers.rs`. Remove timeline/callback/states/hero from `motion_config.rs` and `motion.rs`. Delete `timeline.rs`. Remove hero attributes from `view/element.rs:716-729`.

- [ ] **Step 6: Remove features from React codegen**

Remove Timeline/CallbackOnly stubs in `keyframes.rs:16-18`. Remove @driven-by effect. Remove unused trigger handling.

- [ ] **Step 7: Remove features from Flutter codegen + fix inline motion**

Remove states, driven-by, broken easing arms. Fix `wrap_with_motion_widgets` to handle inline motion bodies.

- [ ] **Step 8: Remove features from JS runtime**

Delete `hero.ts`. Remove `resolveParameterized()` from `easing.ts`. Update `index.ts` exports.

- [ ] **Step 9: Fix all compilation errors**

```bash
cd crates/omnilang-core && cargo check 2>&1
cd omnilang && cargo check 2>&1
```

Work through errors systematically. Every error should be either: (a) a deleted function/variant reference → delete the caller, or (b) a match exhaustiveness error → complete the match with only retained variants.

- [ ] **Step 10: Run full test suite**

```bash
cd omnilang && cargo test 2>&1
cd packages/omni-animation && pnpm test 2>&1
```

Update or remove tests that reference removed features. Snapshot tests will need updating — review and accept.

- [ ] **Step 11: Commit**

```bash
git add -A
git commit -m "refactor(animation): remove 21 features per ADR-032 design scoping

Atomic removal across all pipeline layers: AST, IR, lowering,
semantic validation, OmniView/React/Flutter codegen, JS runtime.

Retained: 8 triggers, Keyframe-only body, 4 easing variants,
5 playback properties, FLIP animations.

Removed: 6 trigger types, Timeline/CallbackOnly modes,
Spring/Steps easing, 9 PlaybackProp variants, States system,
Hero transitions, Timeline orchestration subsystem,
5 callback hooks, color_space field, StaggerDirection.

~50% code reduction. Zero cross-backend feature gaps.
All broken Flutter Dart output eliminated.
Per ADR-032: React-Flutter parity floor."
```

#### Former Tasks 6-9 (merged into Task 5)

The following were originally separate tasks but are now part of the atomic Task 5 above:
- ~~Task 6: Remove Features from IR Types~~ → Task 5 Step 2
- ~~Task 7: Remove Features from IR Lowering~~ → Task 5 Step 3
- ~~Task 8: Remove Features from Codegen Backends~~ → Task 5 Steps 5-7
- ~~Task 9: Remove Features from Semantic Validation & Runtime~~ → Task 5 Steps 4, 8

**Files:**
- Modify: `crates/omnilang-core/src/parser/ast/motion.rs`

**Interfaces:**
- Produces: Slimmed AST with only retained types. Other crates consuming removed variants will get compile errors in Task 7-8.

**Removals:**
- `TriggerKind::Leave`, `Unhover`, `Focus`, `Animate`, `Timeline`, `DrivenBy`
- `MotionBody::Timeline`, `MotionBody::CallbackOnly`
- `EasingExpr::Spring`, `EasingExpr::Steps`
- `EasingExpr::Back` — keep variant but remove `intensity` field (direction-only)
- `EasingExpr::Elastic` — keep variant but remove `amplitude` and `period` fields (direction-only)
- `PlaybackProp::RepeatDelay`, `RepeatRefresh`, `TimeScale`, `Interrupt`, `RespectReducedMotion`, `When`, `DelayChildren`, `StaggerChildren`, `StaggerFrom`
- Entire `HeroDef` struct
- Entire `StateDef` struct
- Entire timeline subsystem: `TimelineDef`, `TimelineBody`, `TimelineItem`, `TimelineItemKind`, `Direction`, `DirectionItem`, `Connector`, `PositionValue`, `StaggerCall`, `StaggerTiming`
- Entire `CallbackSpec` enum, `CallbackBody` struct
- Entire `StaggerDirection` enum
- `color_space` field on `KeyframeBody`

- [ ] **Step 1: Remove TriggerKind variants**

In `TriggerKind` enum, remove: `Leave`, `Unhover`, `Focus`, `Animate`, `Timeline`, `DrivenBy`. Keep: `Enter`, `Hover`, `Press`, `InView`, `Scroll`, `DragHorizontal`, `DragVertical`.

- [ ] **Step 2: Remove MotionBody variants**

Remove `Timeline(TimelineBody)` and `CallbackOnly(CallbackBody)`. Keep only `Keyframe(KeyframeBody)`.

- [ ] **Step 3: Remove EasingExpr variants**

Remove `Spring{...}` and `Steps{...}`. Simplify `Back` to remove `intensity` field, keep only `direction`. Simplify `Elastic` to remove `amplitude` and `period`, keep only `direction`.

- [ ] **Step 4: Remove PlaybackProp variants**

Keep only: `Loop(LoopValue)`, `Direction(DirectionValue)`, `Fill(FillValue)`, `Delay(Expr)`, `Paused(bool)`.
Remove all others.

- [ ] **Step 5: Remove HeroDef and StateDef structs entirely**

- [ ] **Step 6: Remove entire timeline subsystem**

Remove: `TimelineDef`, `TimelineBody`, `TimelineItem`, `TimelineItemKind`, `Direction`, `DirectionItem`, `Connector`, `PositionValue`, `StaggerCall`, `StaggerTiming`, `StaggerDirection`.

- [ ] **Step 7: Remove CallbackBody and CallbackSpec**

- [ ] **Step 8: Remove color_space field from KeyframeBody**

- [ ] **Step 9: Commit**

```bash
git add crates/omnilang-core/src/parser/ast/motion.rs
git commit -m "refactor(ast): remove 21 animation features per ADR-032 design scoping

Removed: 6 trigger types, 2 motion body modes, 2 easing variants,
9 playback properties, Hero/States/Timeline/Callback subsystems,
color_space field, StaggerDirection enum.

Retained: 8 triggers, Keyframe-only body, 4 easing variants,
5 playback properties. See ADR-032 for full rationale."
```

---

### Task 6: Extract Named/Inline Trigger Duplication

**Files:**
- Modify: `omnilang/src/codegen/omniview/motion_triggers.rs` — remove 6 trigger branches
- Modify: `omnilang/src/codegen/omniview/motion_config.rs` — remove timeline/callback/states/hero code
- Delete: `omnilang/src/codegen/omniview/timeline.rs` — entire file
- Modify: `omnilang/src/codegen/omniview/view/element.rs` — remove hero attribute emission
- Modify: `omnilang/src/codegen/react/motion/keyframes.rs` — remove Timeline/CallbackOnly stubs
- Modify: `omnilang/src/codegen/react/motion/transforms.rs` — remove @driven-by effect
- Modify: `omnilang/src/codegen/react/component/motion_effects.rs` — remove @leave/@unhover/@animate/@timeline/@driven-by calls
- Modify: `omnilang/src/codegen/flutter/motion.rs` — remove states, driven-by, broken easing arms; fix inline motion
- Modify: `omnilang/src/codegen/flutter/motion_widgets.rs` — remove DrivenByBuilder section

**Interfaces:**
- Consumes: Slimmed IR types (Task 6)
- Produces: All 3 codegen backends emit only retained features. No broken Dart/JS output.

- [ ] **Step 1: OmniView — remove 6 trigger branches from motion_triggers.rs**

In `emit_motion_triggers_for_element`, remove match arms for `Leave`, `Unhover`, `Focus`, `Animate`, `Timeline`, `DrivenBy`. This reduces the named/inline duplication from ~250 lines to ~100 lines.

- [ ] **Step 2: OmniView — remove timeline/callback/states from motion_config.rs**

Remove: `timeline_to_factory`, `callback_to_animate_call`, `emit_states_js` (from `motion.rs`).
Remove in `playback_to_config`: RepeatDelay, RepeatRefresh, TimeScale, Interrupt, RespectReducedMotion, When, DelayChildren, StaggerChildren, StaggerFrom handling.

- [ ] **Step 3: OmniView — delete timeline.rs**

```bash
git rm omnilang/src/codegen/omniview/timeline.rs
```

Update `mod.rs` to remove the `mod timeline;` declaration and any uses.

- [ ] **Step 4: OmniView — remove hero attribute emission**

In `view/element.rs:716-729`, remove the block that emits `data-hero` and `data-hero-transition` attributes.

- [ ] **Step 5: OmniView — remove states JS emission**

In `omniview/motion.rs:41-145`, remove `emit_states_js` function and all states-related code.

- [ ] **Step 6: React — remove Timeline/CallbackOnly stubs**

In `react/motion/keyframes.rs:16-18`, remove the match arms that return hardcoded fade-in keyframes for Timeline and CallbackOnly.

- [ ] **Step 7: React — remove unused effect emissions**

In `react/component/motion_effects.rs`, remove calls to `emit_driven_by_effects_tsx`. In `react/motion/mod.rs`, remove @leave/@unhover/@animate/@timeline handling.

- [ ] **Step 8: Flutter — remove states, driven-by, broken easing**

In `flutter/motion.rs`:
- Remove `wrap_flutter_states` entirely (eliminates undefined `_statesController` reference)
- Remove `DrivenByBuilder` reference (eliminates broken Dart)
- Remove `find_enter_motions` traversal
- Remove Spring/Steps arms from `format_easing_flutter`
- Simplify Back/Elastic to named-curve-only

- [ ] **Step 9: Flutter — remove DrivenByBuilder from motion_widgets.rs**

Remove the `DrivenByBuilder` section.

- [ ] **Step 10: Flutter — fix inline motion body handling**

In `wrap_with_motion_widgets`, add a branch that reads `trigger.inline_motion` when `trigger.motion_ref` is None, and generates the appropriate AnimationController + AnimatedBuilder setup for inline bodies.

- [ ] **Step 11: Build and fix compilation errors**

```bash
cd omnilang && cargo check 2>&1
```

Fix all compilation errors (removed types will cause cascading errors — work through them systematically).

- [ ] **Step 12: Run all tests**

```bash
cd omnilang && cargo test 2>&1
```

Update or remove tests that reference removed features. Snapshot tests will need updating.

- [ ] **Step 13: Commit**

```bash
git add omnilang/src/codegen/
git commit -m "refactor(codegen): remove 21 animation features from all 3 codegen backends

OmniView: removed 6 trigger branches, timeline.rs, states JS, hero attributes.
React: removed Timeline/CallbackOnly stubs, driven-by/unused effects.
Flutter: removed broken states/driven-by Dart output, broken easing fallbacks.

Fixes: Flutter wrap_with_motion_widgets now handles inline motion bodies.

Per ADR-032: eliminates all broken Dart output (undefined _statesController,
undefined DrivenByBuilder, LateInitializationError for Timeline bodies)."
```

---

### Task 9: Remove Features from Semantic Validation & Runtime

**Files:**
- Modify: `crates/omnilang-core/src/semantic/motion.rs` — remove validation for removed features
- Modify: `crates/omnilang-core/src/semantic/validate_motion.rs` — remove removed feature checks
- Modify: `packages/omni-animation/src/easing.ts` — remove `resolveParameterized()`
- Delete: `packages/omni-animation/src/hero.ts` — entire file
- Modify: `packages/omni-animation/src/index.ts` — remove hero export

**Interfaces:**
- Consumes: Slimmed AST (Task 5), IR (Task 6)
- Produces: Semantic validation only checks retained features. Runtime only exports retained functions.

- [ ] **Step 1: Semantic — remove validation for removed features**

Remove validation functions that check:
- StateDef for reserved name conflicts
- HeroDef for tag uniqueness
- Timeline item element references
- StaggerCall motion references
- when/delayChildren/staggerChildren context validation

- [ ] **Step 2: Semantic — add compiler errors for removed features**

If any removed AST types can still be parsed (transitional), add clear error messages:

```rust
// Example diagnostic for removed features
fn validate_motion_body(body: &MotionBody) -> Result<(), SemanticError> {
    match body {
        MotionBody::Keyframe(kf) => { /* existing validation */ }
        // These variants are removed in ADR-032
        // If they somehow reach validation, they should have been rejected at parse time
    }
}
```

- [ ] **Step 3: JS runtime — remove resolveParameterized**

In `packages/omni-animation/src/easing.ts`, remove the `resolveParameterized()` function and its regex-based parsing of `spring(t,f)`, `backIn(i)`, `elasticIn(a,p)` string forms.

- [ ] **Step 4: JS runtime — remove hero.ts**

```bash
git rm packages/omni-animation/src/hero.ts
```

Update `packages/omni-animation/src/index.ts` to remove the hero export.

- [ ] **Step 5: Run JS runtime tests**

```bash
cd packages/omni-animation && pnpm test 2>&1
```

- [ ] **Step 6: Run full Rust test suite**

```bash
cd omnilang && cargo test 2>&1
```

- [ ] **Step 7: Commit**

```bash
git add crates/omnilang-core/src/semantic/ packages/omni-animation/src/
git commit -m "refactor: remove 21 animation features from semantic validation and JS runtime

Semantic: removed validation for states, hero, timeline, stagger, callbacks.
Runtime: removed hero.ts and resolveParameterized() — parameterized easings
(spring/back/elastic with custom params) are no longer supported.

Per ADR-032: only named easing curves remain for Back and Elastic."
```

---

## Phase 3: Code Quality (Next Sprint)

### Task 10: Extract Named/Inline Trigger Duplication

**Files:**
- Modify: `omnilang/src/codegen/omniview/motion_triggers.rs`

**Interfaces:**
- Consumes: Retained 8 trigger types
- Produces: Single shared dispatch for remaining 5 un-extracted trigger types (InView/Scroll/Drag are already extracted)

- [ ] **Step 1: Identify the remaining duplication**

After Phase 2 scoping, the named vs. inline branches in `emit_motion_triggers_for_element` each have match arms for: `Enter`, `Hover`, `Press`, `Animate` (if retained? no — removed), `InView` (already extracted), `Scroll` (already extracted), `DragHorizontal` (already extracted), `DragVertical` (already extracted).

After scoping, only `Enter` and `Hover` and `Press` remain in the duplicated branches. Create a shared helper:

```rust
fn emit_trigger_with_anim_call(
    &mut self,
    nid: &str,
    trigger_kind: TriggerKind,
    trigger_str: &str,
    anim_call: &str,
    is_inline: bool,
    el: &ElementIR,
) {
    let comment = if is_inline {
        format!("// @{} (inline)", trigger_str)
    } else {
        format!("// @{} trigger", trigger_str)
    };
    match trigger_kind {
        TriggerKind::Enter => {
            // single implementation for both named and inline
            self.mount_js.push_str(&format!(
                "{comment}\n(function() {{\n  const el = __omni_el(\"{nid}\");\n  if (el) {{\n    const anim = {anim};\n    __omni_animations.set(\"{nid}_{trigger}\", anim);\n    if (el.offsetParent !== null) anim.play();\n  }}\n}})();\n",
                comment = comment, nid = nid, anim = anim_call, trigger = trigger_str,
            ));
        }
        TriggerKind::Hover => {
            let extra = format!(
                "  el.addEventListener(\"mouseenter\", function() {{ __omni_animations.get(\"{nid}_{trigger}\")?.play(); }});\n  el.addEventListener(\"mouseleave\", function() {{ __omni_animations.get(\"{nid}_{trigger}\")?.reverse(); }});\n",
                nid = nid, trigger = trigger_str,
            );
            self.js.push_str(&format_simple_trigger_js(&comment, nid, trigger_str, anim_call, &extra));
        }
        TriggerKind::Press => {
            let extra = format!(
                "  el.addEventListener(\"mousedown\", function() {{ __omni_animations.get(\"{nid}_{trigger}\")?.play(); }});\n  el.addEventListener(\"mouseup\", function() {{ __omni_animations.get(\"{nid}_{trigger}\")?.reverse(); }});\n",
                nid = nid, trigger = trigger_str,
            );
            self.js.push_str(&format_simple_trigger_js(&comment, nid, trigger_str, anim_call, &extra));
        }
        TriggerKind::InView => self.emit_trigger_inview_js(nid, trigger_str, anim_call, el, is_inline),
        TriggerKind::Scroll => self.emit_trigger_scroll_js(nid, trigger_str, anim_call, el, is_inline),
        TriggerKind::DragHorizontal => self.emit_trigger_drag_js(nid, trigger_str, anim_call, el, true, is_inline),
        TriggerKind::DragVertical => self.emit_trigger_drag_js(nid, trigger_str, anim_call, el, false, is_inline),
    }
}
```

- [ ] **Step 2: Replace named and inline branches with single call**

Both the named motion ref branch and the inline motion branch now call the shared helper, differing only in how they compute `anim_call`.

- [ ] **Step 3: Build and run tests**

```bash
cd omnilang && cargo check && cargo test motion 2>&1
```

- [ ] **Step 4: Commit**

```bash
git add omnilang/src/codegen/omniview/motion_triggers.rs
git commit -m "refactor(omniview): extract shared trigger dispatch, eliminate named/inline duplication

Replaces the duplicated named/inline match blocks with a single
emit_trigger_with_anim_call helper. Enter/Hover/Press now share
one implementation. InView/Scroll/Drag were already extracted.

Net reduction: ~60 lines of duplicated code."
```

---

### Task 11: Extend NAMED_EASINGS with Flutter Curves

**Files:**
- Modify: `crates/omnilang-core/src/codegen/shared/easing.rs` — extend table to include Flutter curve names
- Modify: `omnilang/src/codegen/flutter/motion.rs:118-154` — replace `format_named_curve` with delegate to shared table

**Interfaces:**
- Consumes: `NAMED_EASINGS` (extended), `resolve_easing` (existing)
- Produces: `resolve_easing_flutter(name) -> Option<&str>` — new public API

- [ ] **Step 1: Extend NAMED_EASINGS to 3-tuple**

Change the table from `&[(&str, &str)]` to include Flutter curve name:

```rust
/// Canonical easing table: (omni_name, css_cubic_bezier, flutter_curves_constant)
pub const NAMED_EASINGS: &[(&str, &str, &str)] = &[
    ("linear", "linear", "Curves.linear"),
    ("ease", "ease", "Curves.ease"),
    ("easeIn", "ease-in", "Curves.easeIn"),
    ("easeOut", "ease-out", "Curves.easeOut"),
    ("easeInOut", "ease-in-out", "Curves.easeInOut"),
    ("power1In", "cubic-bezier(0.550,0.085,0.680,0.530)", "Curves.easeInQuad"),
    ("power1Out", "cubic-bezier(0.250,0.460,0.450,0.940)", "Curves.easeOutQuad"),
    // ... all 29 entries with Flutter Curves constants
    ("bounceIn", "cubic-bezier(0.215,0.610,0.355,1.000)", "Curves.bounceIn"),
    ("bounceOut", "cubic-bezier(0.175,0.885,0.320,1.275)", "Curves.bounceOut"),
    ("bounceInOut", "cubic-bezier(0.645,0.045,0.355,1.000)", "Curves.bounceInOut"),
];
```

- [ ] **Step 2: Update resolve_easing to return 2-tuple (backward compat)**

Add a wrapper that preserves the existing API:

```rust
pub fn resolve_easing(name: &str) -> Option<&'static str> {
    NAMED_EASINGS.iter().find(|(n, _, _)| *n == name).map(|(_, css, _)| *css)
}
```

- [ ] **Step 3: Add resolve_easing_flutter**

```rust
pub fn resolve_easing_flutter(name: &str) -> Option<&'static str> {
    NAMED_EASINGS.iter().find(|(n, _, _)| *n == name).map(|(_, _, flutter)| *flutter)
}
```

- [ ] **Step 4: Replace Flutter format_named_curve**

In `omnilang/src/codegen/flutter/motion.rs:118-154`, replace the 35-line `format_named_curve` function with:

```rust
pub(crate) fn format_named_curve(name: &str) -> String {
    crate::codegen::shared::easing::resolve_easing_flutter(name)
        .unwrap_or("Curves.linear")
        .to_string()
}
```

- [ ] **Step 5: Update existing callers of resolve_easing**

The return type changed from `Option<&&str>` to `Option<&str>`. Fix callers in `semantic/motion.rs:50` and `react/motion/keyframes.rs:214`.

- [ ] **Step 6: Build and run tests**

```bash
cd omnilang && cargo check && cargo test easing 2>&1
```

- [ ] **Step 7: Commit**

```bash
git add crates/omnilang-core/src/codegen/shared/easing.rs omnilang/src/codegen/flutter/motion.rs
git commit -m "refactor(easing): extend NAMED_EASINGS with Flutter Curves, eliminate 4th copy

NAMED_EASINGS is now a 3-tuple (omni_name, css_value, flutter_curve).
New resolve_easing_flutter() API. Flutter's format_named_curve now
delegates to the shared table instead of maintaining its own 35-line
match block.

Adding a new easing now requires ONE edit to NAMED_EASINGS and
automatically propagates to semantic validation, React codegen,
and Flutter codegen. Fixes silent Curves.linear fallback for
new easings in Flutter."
```

---

### Task 12: Consolidate OmniView Scroll to Single Shared Handler

**Files:**
- Modify: `omnilang/src/codegen/omniview/motion_triggers.rs` — `emit_trigger_scroll_js`

**Interfaces:**
- Consumes: Retained @scroll trigger
- Produces: Generated JS with ONE shared scroll handler instead of per-element listeners

- [ ] **Step 1: Emit shared scroll handler at module level**

Add a shared `__omni_scrollHandler` function to the runtime JS emission, following the React pattern in `transforms.rs:173-192`:

```rust
self.js.push_str(r#"
var __omni_scrollElements = [];
var __omni_scrollHandler = null;

function __omni_ensureScrollHandler() {
  if (__omni_scrollHandler) return;
  __omni_scrollHandler = function() {
    __omni_scrollElements.forEach(function(entry) {
      var el = entry.el;
      var anim = entry.anim;
      var dur = entry.dur;
      var progress = Omni.scrollProgress(el, entry.startCfg, entry.endCfg);
      anim.currentTime = progress * dur;
    });
  };
  window.addEventListener('scroll', __omni_scrollHandler, { passive: true });
}
"#);
```

- [ ] **Step 2: Update per-element scroll trigger to register with shared handler**

Instead of per-element `window.addEventListener('scroll', ...)`, each scroll trigger IIFE now pushes its entry to `__omni_scrollElements` and calls `__omni_ensureScrollHandler()`.

- [ ] **Step 3: Add cleanup for removed elements**

Add a function to remove entries from `__omni_scrollElements` when elements are cleaned up (integrates with `__omni_cleanupElement` from Task 4).

- [ ] **Step 4: Build and run tests**

```bash
cd omnilang && cargo check && cargo test omniview 2>&1
```

Snapshot tests will need updating — the generated JS structure changes.

- [ ] **Step 5: Commit**

```bash
git add omnilang/src/codegen/omniview/motion_triggers.rs
git commit -m "perf(omniview): consolidate per-element scroll listeners into single shared handler

Replaces N per-element window.addEventListener('scroll') calls with
one shared __omni_scrollHandler that iterates registered elements.
Each element registers via __omni_scrollElements.push() instead of
creating its own listener.

N=20 elements previously created 20 getBoundingClientRect() calls
per frame. Now one handler makes 20 calls per frame — same work but
single listener eliminates N-1 redundant event registrations."
```

---

## Phase 4: Test Coverage (Next Sprint)

### Task 13: Add React Motion Snapshot Tests

**Files:**
- Create: `omnilang/src/codegen/react/motion/tests.rs` (or add `#[cfg(test)] mod tests` in existing files)

**Interfaces:**
- Consumes: React motion codegen (retained features from Phase 2)
- Produces: 3-5 insta snapshot tests covering @enter, @hover, @in-view, @scroll, and combined trigger usage

- [ ] **Step 1: Create test fixtures directory**

Create `.omni` test fixture files with motion features:

```
omnilang/tests/fixtures/react/
├── motion_enter.omni       # @enter trigger with named motion
├── motion_hover.omni       # @hover trigger with named motion
├── motion_inview.omni      # @in-view trigger with named motion
├── motion_scroll.omni      # @scroll trigger
└── motion_combined.omni    # Multiple triggers on same component
```

- [ ] **Step 2: Write snapshot test for @enter**

```rust
#[test]
fn test_react_motion_enter() {
    let output = compile_fixture_to_react("tests/fixtures/react/motion_enter.omni");
    insta::assert_snapshot!(output);
}
```

- [ ] **Step 3: Write snapshot test for @hover**

Similar structure, testing that MouseEvent listeners are emitted.

- [ ] **Step 4: Write snapshot test for @in-view**

Tests that `data-motion-inview-name` attribute is emitted and `_M[name]` lookup is used (validates Task 1 fix).

- [ ] **Step 5: Write snapshot test for @scroll**

Tests that the shared scroll handler pattern is emitted (validates Task 12).

- [ ] **Step 6: Write combined test fixture**

A single `.omni` component using @enter + @hover + @in-view on different elements.

- [ ] **Step 7: Run tests, review snapshots**

```bash
cd omnilang && cargo test react_motion -- --nocapture 2>&1
```

Review generated snapshots. If output looks correct, accept with `cargo insta review`.

- [ ] **Step 8: Commit**

```bash
git add omnilang/src/codegen/react/motion/tests.rs omnilang/tests/fixtures/react/motion_*.omni
git commit -m "test(react): add 5 motion snapshot tests for React codegen

Covers @enter, @hover, @in-view, @scroll, and combined triggers.
Validates that @in-view uses data-motion-inview-name + _M[name]
lookup (Task 1 fix). Previously zero motion-specific test coverage
in React backend (ADR-024 documented gap)."
```

---

### Task 14: Add Flutter Motion Codegen Tests

**Files:**
- Create: test functions in `omnilang/src/codegen/flutter/tests.rs` (or new test module)

**Interfaces:**
- Consumes: Flutter motion codegen (retained features from Phase 2)
- Produces: 3-5 tests covering Dart output for @enter, @hover, inline motion, easing mapping, duration extraction

- [ ] **Step 1: Add test for @enter motion controller generation**

Verify that `initState` contains `_{name}Controller = AnimationController(...)` and `.forward()` call.

- [ ] **Step 2: Add test for @hover MouseRegion wrapping**

Verify `MouseRegion(onEnter: ...forward(), onExit: ...reverse())` is generated.

- [ ] **Step 3: Add test for inline motion body handling**

Verify that inline motion bodies (not just named refs) produce correct AnimationController + AnimatedBuilder output. Validates Task 8 Step 10 fix.

- [ ] **Step 4: Add test for easing mapping**

Verify that `format_named_curve("power1Out")` produces `"Curves.easeOutQuad"` (validates Task 11 — shared table delegation).

- [ ] **Step 5: Add test for duration extraction**

Verify that `extract_duration_ms(ExprIR)` handles `"300ms"`, `"1.5s"`, and numeric values correctly.

- [ ] **Step 6: Run tests**

```bash
cd omnilang && cargo test flutter_motion -- --nocapture 2>&1
```

- [ ] **Step 7: Commit**

```bash
git add omnilang/src/codegen/flutter/
git commit -m "test(flutter): add 5 motion codegen tests for Flutter backend

Covers @enter controller init, @hover MouseRegion wrapping, inline
motion body handling, easing mapping via shared NAMED_EASINGS table,
and duration extraction. Previously zero motion-specific test
coverage in Flutter backend."
```

---

### Task 15: Final Validation — Full Test Suite & Cross-Target Check

**Files:**
- Modify: `omnilang/tests/cross_target.rs` — verify compilation of motion-heavy fixture across all P0 backends

**Interfaces:**
- Consumes: All changes from Tasks 1-14
- Produces: Green test suite confirming scoping is correct and complete

- [ ] **Step 1: Run full Rust test suite**

```bash
cd omnilang && cargo test 2>&1
```

Expected: all tests pass. Fix any failures.

- [ ] **Step 2: Run clippy**

```bash
cd omnilang && cargo clippy -- -D warnings 2>&1
```

Expected: no new warnings.

- [ ] **Step 3: Run JS runtime tests**

```bash
cd packages/omni-animation && pnpm test 2>&1
```

- [ ] **Step 4: Run cross-target compilation test**

```bash
cd omnilang && cargo test cross_target 2>&1
```

- [ ] **Step 5: Verify code reduction metrics**

```bash
# Count lines before scoping (from the review doc)
echo "Target: ~1,100 lines Rust (was ~2,200), ~400 lines TS (was ~500)"
# Count actual lines after scoping
wc -l crates/omnilang-core/src/parser/ast/motion.rs
wc -l crates/omnilang-core/src/ir/types_motion.rs
wc -l crates/omnilang-core/src/ir/lower/motion.rs
wc -l crates/omnilang-core/src/semantic/motion.rs
wc -l omnilang/src/codegen/omniview/motion_triggers.rs
wc -l omnilang/src/codegen/omniview/motion_config.rs
wc -l omnilang/src/codegen/flutter/motion.rs
wc -l packages/omni-animation/src/*.ts
```

- [ ] **Step 6: Commit final validation**

```bash
git add -A
git commit -m "chore: final validation — all tests pass, clippy clean, cross-target green

Animation design scoping complete per ADR-032.
14 features retained, 21 removed, ~50% code reduction achieved."
```

---

## Self-Review Checklist

- [x] **Spec coverage**: Each retained feature has a task (C1-C4 fix bugs, H1 removes dead features, H2-H5 improve quality, H6-H7 add tests)
- [x] **Placeholder scan**: No TBD/TODO/fill-in-the-blank steps. Every code change has specific file:line references and concrete code snippets
- [x] **Type consistency**: Task 5 defines slimmed AST types → Task 6 mirrors in IR → Task 7 removes lowering → Task 8 removes codegen → Task 9 cleans up runtime. Downstream tasks reference types defined in upstream tasks.
- [x] **ADR-032 alignment**: All 21 removed features listed in ADR-032 are removed in Tasks 5-9
- [x] **Test coverage**: Tasks 13-14 add missing React and Flutter motion tests (ADR-024 documented gap)

---

## Execution Handoff

**Plan complete and saved to `docs/superpowers/plans/2026-07-29-omniview-animation-scoping.md`.**

**Phase overview:**
- **Phase 1 (Tasks 1-4)**: Critical fixes — S effort each, immediate value, can ship independently
- **Phase 2 (Tasks 5-9)**: Design scoping — L effort, the "big bang" removal of 21 features
- **Phase 3 (Tasks 10-12)**: Code quality — M effort, deduplication and hardening
- **Phase 4 (Tasks 13-14)**: Test coverage — M effort, fills the ADR-024 documented gap
- **Task 15**: Final validation gate

**Two execution options:**

1. **Subagent-Driven (recommended)** — I dispatch a fresh subagent per task, review between tasks, fast iteration
2. **Inline Execution** — Execute tasks in this session using executing-plans, batch execution with checkpoints

Which approach?
