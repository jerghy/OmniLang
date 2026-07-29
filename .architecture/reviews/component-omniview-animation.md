# Architecture Review: OmniView Animation Subsystem

## Overview

Two-phase comprehensive architecture review of the OmniView animation subsystem. **Phase 1** (2026-07-29): 8-expert engineering review covering implementation quality, performance, security, and maintainability. **Phase 2** (2026-07-29): Design architecture review focused on feature-set scoping — driven by the principle that only features both React and Flutter can implement well should be retained. Flutter is the capability constraint.

## Review Details

- **Component Reviewed**: OmniView Animation Subsystem
- **Review Dates**: 2026-07-29 (Phase 1: Engineering) + 2026-07-29 (Phase 2: Design)
- **Phase 1 Team**: Compiler & Language Architect, Rust Expert, TypeScript & React Specialist, Performance Specialist, Maintainability Expert, Security Specialist, Systems Architect, Pragmatic Enforcer
- **Phase 2**: Domain Expert (feature taxonomy), Compiler Architect (Flutter capability matrix) → Design-level cross-target fit analysis
- **Design Principle**: **"克制" (Restraint)** — ship only what React and Flutter can both do well. Stability over quantity. Flutter is the capability floor — if Flutter can't implement it with full fidelity, it's out.
- **Scale**: Current design: 14 trigger types, 3 motion body modes, 6 easing expression variants, 14 PlaybackProp variants, 5 advanced systems, 5 lifecycle callbacks → **Recommended: 8 trigger types, 1 motion body mode, 4 easing variants, 5 PlaybackProp variants, 1 advanced system**

---

## Phase 2: Design Architecture Review — Feature-Set Scoping

### Design Principle

动画设计应该**克制**：**只保留 React 和 Flutter 两个目标都能高质量实现的功能**。OmniView 调试运行时是第一优先级，但它跟随 React/Flutter 的能力边界走，不强求超出 Web/Flutter 平台能力的特性。

**Flutter 是约束条件** — 它的动画能力天然不如 Web（无 WAAPI、无原生 Spring 曲线、无 CSS 滚动时间线、无跨 widget 时间线编排）。因此，设计必须以 Flutter 能实现的功能为上限。

### Feature Cross-Reference Matrix: React ↔ Flutter

#### ✅ KEEP — Both React and Flutter Can Implement Well (12 features)

| # | Feature | React | Flutter | Fidelity | Notes |
|---|---------|-------|---------|----------|-------|
| 1 | `@enter` trigger | ✅ | ✅ | **Full** | React: useEffect([]). Flutter: initState().forward() |
| 2 | `@hover` trigger | ✅ | ✅ | **Full** | React: mouseenter/mouseleave. Flutter: MouseRegion |
| 3 | `@press` trigger | ✅ | ✅ | **Full** | React: mousedown/mouseup. Flutter: GestureDetector |
| 4 | `@in-view` trigger | ⚠️ Bug | ✅ | **Full** (after fix) | React hardcodes opacity fade-in, ignores user motion — S-effort fix: emit `data-motion-inview-name`, lookup `_M[name]` |
| 5 | `@scroll` trigger | ✅ | ✅ | **Full** | React: shared scroll handler. Flutter: _OmniScrollDrivenWidget |
| 6 | `@drag-horizontal` trigger | ✅ | ✅ | **Full** | React: pointer events. Flutter: _OmniDragWidget |
| 7 | `@drag-vertical` trigger | ✅ | ✅ | **Full** | Same, axis difference only |
| 8 | Keyframe motion body | ✅ | ✅ | **Full** | React: WAAPI el.animate(). Flutter: AnimationController + Tween |
| 9 | Named easings (29 curves) | ✅ | ✅ | **Full** | Both have complete mapping tables |
| 10 | CubicBezier easing | ✅ | ✅ | **Full** | React: CSS cubic-bezier(). Flutter: Cubic() |
| 11 | Inline motion bodies | ✅ | ❌ Gap | **Full** (after Flutter fix) | React handles inline. Flutter `wrap_with_motion_widgets` ignores `inline_motion` — M-effort fix |
| 12 | FLIP for @for lists | ✅ | N/A | **Full** | Runtime-only via `flipBefore`/`flipAfter`. Flutter side: list key tracking is framework-native. |

#### ❌ REMOVE — Flutter Cannot Implement with Acceptable Fidelity (21 features)

| # | Feature | React | Flutter | Reason for Removal |
|---|---------|-------|---------|-------------------|
| 1 | `@leave` trigger | ❌ | ❌ | **Both absent.** Flutter has no exit-animation lifecycle — widgets are removed synchronously. Architecturally impossible without AnimatedSwitcher parent-level refactoring. |
| 2 | `@unhover` trigger | ❌ | ⚠️ Broken | React absent. Flutter merged with @hover — references wrong controller when unhover motion differs from hover motion. |
| 3 | `@focus` trigger | ✅ | ❌ | Flutter absent — no Focus widget wrapping generated. Adding would require per-element FocusNode management. |
| 4 | `@animate` trigger | ❌ | ❌ | **Both absent.** Auto-play distinct from @enter. Neither backend wires it. |
| 5 | `@timeline` trigger | ❌ | ❌ | **Both stub/no-op.** Flutter architecturally impossible — cross-widget timeline orchestration has no natural host in Flutter's widget tree. |
| 6 | `@driven-by` trigger | ⚠️ Static | ❌ Broken | React runs once on mount (never re-reads signal). Flutter generates broken Dart (undefined `DrivenByBuilder` class). |
| 7 | Timeline motion body | ❌ Stub | ❌ Broken | Both non-functional. Flutter generates `LateInitializationError` at runtime (controllers declared `late final`, never initialized). |
| 8 | CallbackOnly motion body | ❌ Stub | ❌ Broken | Same as Timeline. All 5 lifecycle callbacks discarded. |
| 9 | States: reserved (idle/hover/pressed) | ❌ | ⚠️ Broken | React completely absent. Flutter generates broken Dart — `_statesController` referenced but never declared. Compile error. |
| 10 | States: custom signal-driven | ❌ | ❌ No-op | React absent. Flutter `return child!;` — no-op placeholder. |
| 11 | Hero transitions | ❌ | ⚠️ Partial | React absent. Flutter basic Hero works but flightShuttleBuilder is stub. No JS runtime in OmniView. |
| 12 | Spring easing (parameterized) | ⚠️ Partial | ⚠️ @enter-only | Flutter has no native Spring curve. @enter has a workaround via `SpringSimulation`, but all other triggers fall back to `Curves.linear`. Full fidelity loss for non-enter triggers. |
| 13 | Elastic easing (parameterized) | ⚠️ Partial | ⚠️ Partial | Amplitude and period parameters lost in both backends. Collapses to named-curve approximation. |
| 14 | Back easing (parameterized) | ⚠️ Partial | ⚠️ Partial | Intensity parameter lost in both backends. Collapses to named-curve approximation. |
| 15 | Steps easing | ⚠️ Needs custom | ❌ Fallback | Flutter has no Steps curve class. Complete fidelity loss — falls back to `Curves.linear`. |
| 16 | Stagger (timeline stagger) | ❌ | ❌ | **Both unimplemented.** Flutter architecturally complex — multi-child stagger requires per-child AnimationController or shared controller with delayed starts. |
| 17 | 9 PlaybackProp variants | ⚠️ Partial | ❌ | Flutter only consumes `over` (duration). These 9 are ignored by Flutter: RepeatDelay, RepeatRefresh, TimeScale, Interrupt(4 values), RespectReducedMotion, When(3 values), DelayChildren, StaggerChildren, StaggerFrom |
| 18 | 5 CallbackSpec hooks | ❌ Stub | ❌ | OnStart/OnUpdate/OnComplete/OnRepeat/OnInterrupt — none implemented in any non-OmniView backend |
| 19 | when/delayChildren/staggerChildren | ❌ Dropped | ❌ Dropped | Silently dropped during IR lowering (`ir/lower/view.rs:545`). Zero consumers. |
| 20 | color_space field | ❌ | ❌ | Zero consumers anywhere in the entire pipeline — AST, IR, codegen, runtime |
| 21 | StaggerDirection::Random | ⚠️ Fallback | ❌ | Always falls back to `Edges` with a warning. Never implemented. |

### Design Audit: What to Remove from Each Layer

After applying the KEEP/REMOVE decisions above, the following should be **removed** from each pipeline layer:

#### Parser AST (`crates/omnilang-core/src/parser/ast/motion.rs`)

Remove these enum variants and types entirely:
- `TriggerKind::Leave`, `TriggerKind::Unhover`, `TriggerKind::Focus`, `TriggerKind::Animate`, `TriggerKind::Timeline`, `TriggerKind::DrivenBy` → 14 → **8**
- `MotionBody::Timeline`, `MotionBody::CallbackOnly` → 3 → **1** (Keyframe only)
- `EasingExpr::Spring`, `EasingExpr::Steps` → 6 → **4**
- `EasingExpr::Back` — remove `intensity` field, keep direction-only (or collapse to Named)
- `EasingExpr::Elastic` — remove `amplitude`/`period`, keep direction-only (or collapse to Named)
- `PlaybackProp::RepeatDelay`, `RepeatRefresh`, `TimeScale`, `Interrupt`, `RespectReducedMotion`, `When`, `DelayChildren`, `StaggerChildren`, `StaggerFrom` → 14 → **5**
- `HeroDef` — entire struct
- `StateDef` — entire struct
- `TimelineDef`, `TimelineBody`, `TimelineItem`, `TimelineItemKind`, `Direction`, `DirectionItem`, `Connector`, `PositionValue`, `StaggerCall`, `StaggerTiming` — entire timeline subsystem
- `CallbackSpec`, `CallbackBody` — entire callback subsystem
- `StaggerDirection` — entire enum (stagger is timeline-only)
- `color_space` field on `KeyframeBody`
- `LoopValue`, `FillValue`, `DirectionValue` — keep (in 5 retained PlaybackProps)

#### IR Types (`crates/omnilang-core/src/ir/types_motion.rs`)

Mirror all AST removals in the IR types:
- `TriggerKind::Leave/Unhover/Focus/Animate/Timeline/DrivenBy` → 13 → **7**
- `MotionBodyIR::Timeline/CallbackOnly` → 3 → **1**
- `EasingIR::Spring/Steps` → 6 → **4**
- `TimelineIR`, `TimelineItemKindIR`, `DirectionItemIR`, `ConnectorIR`, `PositionIR`, `StaggerIR`, `StaggerTimingIR` — entire timeline IR
- `CallbackBodyIR`, `CallbackKindIR` — entire callback IR
- `HeroIntent` — entire struct
- `StatesIR`, `StateTweenIR` — entire states IR
- `PlaybackIR` — remove 9 unused fields, keep 5 + 2 booleans
- `WhenValue` — entire enum
- `InterruptValue` — entire enum
- `DragBehavior.scrub`, `DragBehavior.momentum` — fields defined but never emitted

#### Lowering (`crates/omnilang-core/src/ir/lower/motion.rs`, `view.rs`)

- Remove all lowering functions for removed AST variants
- Remove `when`/`delayChildren`/`staggerChildren` drop at `view.rs:545-550`
- Remove Hero definition lowering

#### Codegen — OmniView (`omnilang/src/codegen/omniview/motion_triggers.rs`, `motion_config.rs`, `timeline.rs`)

- Remove 6 trigger kind branches (Leave, Unhover, Focus, Animate, Timeline, DrivenBy) → trigger duplication reduces from 250 lines to ~100 lines
- Remove `timeline_to_factory`, `callback_to_animate_call`
- Remove `emit_states_js`
- Remove Hero attribute emission (`view/element.rs:716-729`)
- Remove `InterruptValue`, `WhenValue`, `RepeatDelay`, `RepeatRefresh`, `TimeScale` handling in `playback_to_config`

#### Codegen — React (`omnilang/src/codegen/react/motion/`)

- Remove `MotionBodyIR::Timeline/CallbackOnly` stub branches in `keyframes.rs:16-18`
- Remove @leave/@unhover/@animate/@timeline branches
- Remove `@driven-by` effect emission
- Remove all states codegen (currently absent — no removal needed)

#### Codegen — Flutter (`omnilang/src/codegen/flutter/motion.rs`, `motion_widgets.rs`)

- Remove `wrap_flutter_states` entirely — eliminates broken `_statesController` reference
- Remove `DrivenByBuilder` reference — eliminates broken Dart output
- Remove `find_enter_motions` (replaced by inline controller creation per element)
- Remove `format_named_curve` — replace with delegate to shared `NAMED_EASINGS` (eliminate 4th copy)
- Remove `format_easing_flutter` Spring/Steps arms — keep Named/CubicBezier/Back/Elastic only
- Fix inline motion body handling in `wrap_with_motion_widgets`

#### Runtime (`packages/omni-animation/src/`)

- Keep: `animate.ts`, `timeline.ts`, `stagger.ts`, `scroll.ts`, `flip.ts`, `easing.ts`, `keyframes.ts`
- Remove: `hero.ts` — no consumer (no JS runtime reads `data-hero` attributes)
- Remove `resolveParameterized()` in `easing.ts` (Spring/Steps parameterized forms) — keep Named/CubicBezier/Back/Elastic only

---

### Scope Reduction Impact

| Metric | Before | After | Reduction |
|--------|--------|-------|-----------|
| Trigger types | 14 | **8** | -43% |
| Motion body modes | 3 | **1** | -67% |
| Easing expression variants | 6 | **4** | -33% |
| Named easings | 29 | **29** | 0% (all standard, no removal needed) |
| PlaybackProp variants | 14 | **5** | -64% |
| Advanced animation systems | 5 | **1** (FLIP) | -80% |
| State engine modes | 2 | **0** | -100% |
| Callback lifecycle hooks | 5 | **0** | -100% |
| Timeline item kinds | 5 | **0** | -100% |
| Position value variants | 6 | **0** | -100% |
| ~Rust code (motion-related) | ~2,200 lines | ~1,100 lines | -50% |
| ~TS runtime | ~500 lines | ~400 lines | -20% |
| File count (motion-related Rust) | 19 files | ~12 files | -37% |
| Cross-backend feature gap | 60% gap | **0% gap** | Eliminated |

---

## Phase 1: Engineering Architecture Review

### Executive Summary

The OmniView animation subsystem has a **solid architectural foundation** — clean pipeline separation (AST → IR → codegen → runtime), WAAPI-first compositor-thread animations. Phase 2 design scoping eliminates the primary architectural problem (cross-backend feature gaps) by removing features that cannot achieve full fidelity in both React and Flutter targets. Remaining engineering concerns are focused: duplication in `motion_triggers.rs`, easing table fragmentation, memory leaks in the runtime, and test coverage gaps.

**Overall Architecture Health**: **Post-scoping: Good** — strong core with manageable technical debt. The design simplification eliminates 50% of code surface and 100% of cross-backend feature gaps.

**Key Strengths** (Post-Scoping):
- Clean pipeline layering: Parser/AST → Semantic validation → IR lowering → Multi-target codegen → Runtime
- WAAPI-first design delegates animation to compositor thread
- **Design restraint**: 8 triggers × 1 body mode × 4 easing variants = simple, implementable, testable
- ADR-022 canonical easing table (extend to cover Flutter)
- ADR-024 React useEffect lifecycle split
- **Zero cross-backend feature gap** — every shipped feature works in both React and Flutter

**Top Engineering Concerns** (post-scoping, ranked):
1. `__omni_animations` Map has unbounded growth — no eviction, no lifecycle cleanup
2. Named/inline trigger duplication in `motion_triggers.rs` (~100 lines, reduced from ~250)
3. Flutter `wrap_with_motion_widgets` ignores inline motion bodies
4. Scroll/drag config strings interpolated into generated JS without escaping
5. React `@in-view` hardcodes opacity fade-in instead of using user motion
6. `bounceIn`/`bounceOut` have identical cubic-bezier values
7. No motion test coverage in React or Flutter backends

### Cross-Cutting Themes (Engineering)

Themes that remain relevant after design scoping:

#### Theme 1: Trigger Duplication in motion_triggers.rs (3/8 reviewers)

The named motion ref branch and inline motion branch each contain a `match trigger.kind` over 8 trigger types (down from 11 after scoping). Three types (InView, Scroll, Drag) are already extracted into shared helpers. **Fix**: Extract remaining 5 types into shared dispatch.

#### Theme 2: Easing Table Fragmentation (4/8 reviewers)

Flutter's `format_named_curve` is a 4th copy of the easing data. ADR-022 fixed the named easing table for semantic/React but missed Flutter. **Fix**: Extend `NAMED_EASINGS` to carry Flutter curve constants.

#### Theme 3: No Motion Test Coverage Beyond OmniView (3/8 reviewers)

React and Flutter backends have zero motion-specific tests. **Fix**: Add 3-5 snapshot tests per backend.

#### Theme 4: Lifecycle Management Gap in OmniView Native (3/8 reviewers)

No component unmount lifecycle — animations, observers, event listeners accumulate. React has proper `useEffect` cleanup. **Fix**: Add `__omni_unmount()` hook.

#### Theme 5: Easing Approximation Quality (2/8 reviewers)

`bounceIn`/`bounceOut` map to identical cubic-bezier values. `elasticOut`/`spring`/`power1Out` all alias. **Fix**: Compute proper asymmetric approximations for bounce pair.

---

### Individual Engineering Perspectives (Phase 1)

#### Compiler & Language Architect (Dr. Lin Wei)

**Key Finding**: Rich AST, progressive semantic loss through pipeline. Features are parsed → validated → lowered → then silently dropped at codegen boundary. Post-scoping, this problem is eliminated — only features that both React and Flutter can implement survive to codegen.

**Post-Scoping Concerns**:
- Inline motion bodies: React handles them, Flutter ignores them — M-effort Flutter fix needed
- React `@in-view` hardcodes opacity — S-effort fix
- `bounceIn`/`bounceOut` identical values — S-effort fix

#### Rust Expert (Katya Volkov)

**Key Finding**: Zero unsafe blocks, disciplined error handling. Post-scoping removes many duplicated match arms (callback parsing, keyframe body dual-pass, timeline lowering). Remaining concerns: `extract_tween_begin/end` FxHashMap non-determinism, `parse_duration_ms` returns `Result<f64, String>`.

**Post-Scoping Concerns**:
- `extract_tween_begin/end` iterates FxHashMap non-deterministically — MEDIUM
- `parse_duration_ms` returns `Result<f64, String>` — LOW (add typed error)

#### TypeScript & React Specialist (Emma Rodriguez)

**Key Finding**: Clean TS module boundaries, strict mode, immutable patterns. Post-scoping removes `hero.ts`, `resolveParameterized()`. Remaining concerns: `as unknown as PropertyIndexedKeyframes` type escape, no tests for `flip.ts`/`scroll.ts`.

**Post-Scoping Concerns**:
- `as unknown as PropertyIndexedKeyframes` cast in `animate.ts:26` — MEDIUM
- No test coverage for `flip.ts`, `scroll.ts` — MEDIUM
- `staggerDelays` `edges` direction bug — LOW

#### Performance Specialist

**Key Finding**: WAAPI-first is the right perf decision. Post-scoping removes many accumulation vectors (timeline items, hero state, callback objects). Remaining concerns: Map leak, per-element scroll listeners, FLIP reflow.

**Post-Scoping Concerns**:
- `__omni_animations` Map unbounded growth — HIGH (add eviction + cleanup hook)
- Per-element scroll listeners in OmniView — HIGH (consolidate to shared handler)
- FLIP `offsetHeight` forced reflow — MEDIUM (replace with WAAPI)
- `getAnimations()` per-frame allocation in React — MEDIUM

#### Maintainability Expert

**Key Finding**: Pipeline structure is clean but codegen duplication is high. Post-scoping eliminates timeline stubs, callback stubs, states codegen, hero codegen — reducing code surface by ~50%.

**Post-Scoping Concerns**:
- Named vs. inline trigger duplication (reduced from 132 to ~60 lines after scoping) — HIGH
- Flutter easing table is 4th copy — MEDIUM (extend NAMED_EASINGS)
- No React/Flutter motion tests — MEDIUM

#### Security Specialist

**Key Finding**: String-concatenated JS codegen is the primary injection surface. Post-scoping removes several interpolation sites (hero attributes, callback bodies). Critical finding remains: scroll/drag config strings unescaped.

**Post-Scoping Concerns**:
- Scroll/drag config string interpolation without escaping — HIGH (S-effort fix)
- Event listener leak — OmniView IIFE pattern — MEDIUM
- InView observer leak — MEDIUM
- States non-transform double-quoting — removed with states system

#### Systems Architect

**Key Finding**: Clean pipeline layering but implicit dependency contract between codegen and runtime. Post-scoping removes the biggest cross-backend gaps (States, Timeline, CallbackOnly, Hero). Remaining concern: parameterized easing math was the biggest design inconsistency — post-scoping removes Spring/Steps, keeps only Named/CubicBezier/Back/Elastic.

#### Pragmatic Enforcer

**Key Finding**: Strong core (keyframes, easing, triggers) but over-engineering at the periphery. Post-scoping is effectively a full YAGNI pass: 21 of 33 features removed. Remaining feature set has high necessity-to-complexity ratios.

**Post-Scoping Feature Scores**:

| Feature | Necessity | Complexity | Ratio | Verdict |
|---------|-----------|------------|-------|---------|
| @enter trigger | 10 | 2 | 5.00 | ✅ Keep |
| @hover trigger | 9 | 2 | 4.50 | ✅ Keep |
| @press trigger | 8 | 2 | 4.00 | ✅ Keep |
| @in-view trigger | 7 | 3 | 2.33 | ✅ Keep (fix React bug) |
| @scroll trigger | 8 | 4 | 2.00 | ✅ Keep |
| @drag-horizontal trigger | 7 | 5 | 1.40 | ✅ Keep |
| @drag-vertical trigger | 7 | 5 | 1.40 | ✅ Keep |
| Keyframe body | 10 | 3 | 3.33 | ✅ Keep |
| Named easings (29) | 9 | 1 | 9.00 | ✅ Keep |
| CubicBezier easing | 8 | 1 | 8.00 | ✅ Keep |
| Back easing (named only) | 7 | 1 | 7.00 | ✅ Keep |
| Elastic easing (named only) | 6 | 1 | 6.00 | ✅ Keep |
| Inline motion bodies | 8 | 2 | 4.00 | ✅ Keep (fix Flutter) |
| FLIP for @for | 9 | 3 | 3.00 | ✅ Keep |
| PlaybackProp (5 retained) | 7 | 2 | 3.50 | ✅ Keep |

---

### Prioritized Recommendations (Post-Scoping)

#### Critical — Implement Immediately (S effort each)

| # | Action | Effort | Risk Addressed |
|---|--------|--------|---------------|
| C1 | Fix React `@in-view` to use user motion — emit `data-motion-inview-name`, lookup `_M[name]` | S | Broken React feature |
| C2 | Escape scroll/drag config strings in codegen — apply `escape_literal_str` before JS interpolation | S | Supply-chain injection |
| C3 | Fix `bounceIn`/`bounceOut` identical values — compute asymmetric cubic-bezier approximations | S | Silent easing inversion |
| C4 | Add `__omni_animations.delete()` after @leave + `__omni_cleanupElement()` for @for key removal | S | Memory leak |

#### High — Implement Within 2 Weeks (S-M effort each)

| # | Action | Effort | Risk Addressed |
|---|--------|--------|---------------|
| H1 | **Execute design scoping** — remove 21 features from AST/IR/codegen/runtime per Phase 2 analysis | L | Architectural clarity, maintenance burden, cross-backend gaps |
| H2 | Extract named/inline trigger duplication — reduce `motion_triggers.rs` by ~60 lines | M | Code drift, bug surface |
| H3 | Fix Flutter inline motion body support in `wrap_with_motion_widgets` | M | Flutter feature gap |
| H4 | Refactor OmniView scroll to single shared handler — follow React pattern | M | Scroll jank at scale |
| H5 | Extend `NAMED_EASINGS` to carry Flutter Curves — eliminate 4th easing copy | S | Silent Flutter fallback to Curves.linear |
| H6 | Add 3-5 React motion snapshot tests | M | Regression detection |
| H7 | Add 3-5 Flutter motion codegen tests | M | Regression detection |

#### Medium — Implement Within 2 Months

| # | Action | Effort |
|---|--------|--------|
| M1 | Add `__omni_unmount()` lifecycle hook — root fix for Map leak, observer leak, listener leak | L |
| M2 | Replace FLIP `void el.offsetHeight` with WAAPI `el.animate()` — follow `hero.ts` pattern | S |
| M3 | Fix `staggerDelays` `edges` direction — per-wave delay assignment | S |
| M4 | Align `Keyframe` type with DOM `Keyframe` — remove `as unknown as` cast | S |
| M5 | Add vitest+jsdom tests for `flip.ts`, `scroll.ts` | M |
| M6 | Fix `extract_tween_begin/end` FxHashMap non-determinism — add assertion or accept property name param | S |
| M7 | Replace `setInterval` polling with `finish` event in editor wait mode | S |
| M8 | Add cross-target motion golden snapshot test | M |

---

### Technical Debt Assessment (Post-Scoping)

| Area | Before | After Scoping | Impact |
|------|--------|---------------|--------|
| Cross-backend feature parity | **High (60% gap)** | **Zero (eliminated)** | Users get identical animation behavior across targets |
| Codegen duplication | High | Medium (~100 residual lines) | Manageable, with clear extraction path |
| Test coverage (React/Flutter) | High | High (to be addressed in H6/H7) | Plan: add snapshot tests |
| Easing table synchronization | Medium | Low (after H5) | Single source of truth for all backends |
| Lifecycle management (OmniView) | Medium | Medium (M1 deferred) | Long-term fix: __omni_unmount() |
| JS codegen string safety | Medium | Low (after C2 fix) | Only 1 known injection vector, S-effort fix |

---

### Risks If Unaddressed

1. **Memory exhaustion in SPA pages**: The `__omni_animations` Map leak causes measurable growth under repeated mount/unmount cycles. Fix: C4 (immediate) + M1 (long-term).

2. **Security incident via supply chain**: Unescaped scroll/drag config strings in codegen allow JS injection from compromised `.omni` files. Fix: C2 (S-effort, immediate).

3. **Flutter inline motion ignored**: Users writing inline motion triggers on Flutter get silent no-ops. Fix: H3 (M-effort).

4. **Silent easing inversion**: `bounceOut` produces bounce-in behavior. Users never know they got the wrong curve. Fix: C3 (S-effort, immediate).

5. **React @in-view ignores user motion**: All React @in-view triggers produce generic fade-in regardless of user definition. Fix: C1 (S-effort, immediate).

6. **Design scope creep**: Without removing the 21 identified features, cross-backend gaps will continue widening and maintenance burden will grow. Fix: H1 (L-effort design scoping execution).

---

### Success Metrics (Post-Scoping)

- **Feature parity**: Every animation feature works identically in React and Flutter (0 exceptions)
- **Warning coverage**: Every removed feature's AST variant produces a clear compiler error during migration
- **Code reduction**: ~50% fewer lines in motion-related code
- **Test coverage**: React and Flutter backends have motion tests covering @enter, @hover, @in-view, @scroll
- **Lifecycle correctness**: `__omni_animations` Map stays bounded (≤ entries = currently-mounted animated elements)
- **Easing consistency**: All backends use the same `NAMED_EASINGS` table

### Follow-up Plan

1. **ADR**: Create ADR-032 documenting the design scoping decision — "OmniView Animation Design Scoping: React-Flutter Parity Floor"
2. **Phase A** (this sprint): Execute Critical items C1-C4 — all S-effort, high-impact
3. **Phase B** (next sprint): Execute High items H1-H7 — design scoping + codegen hardening
4. **Phase C** (backlog): Medium items M1-M8 — long-term architectural investments
5. **Migration**: For users with `.omni` files using removed features, emit clear compiler errors with migration guidance (e.g., "Timeline animations are not supported. Use individual @animate triggers with delays instead.")

---

## Appendices

### A. Review Methodology

**Phase 1 (Engineering)**: 8 architecture team members independently reviewed the animation subsystem from their specialist perspectives. Each expert read code, ADRs, and produced structured findings. Aggregated with cross-cut analysis and conflict resolution.

**Phase 2 (Design)**: Domain Expert produced complete feature taxonomy from AST/IR/codegen/runtime. Compiler Architect analyzed Flutter codegen capability per feature. Cross-referenced both analyses to produce React ↔ Flutter parity matrix. Applied design principle: keep only features both targets can implement with full fidelity.

### B. Referenced Documents

- `ADR-022-canonical-easing-table.md` — Easing table deduplication (partial — Flutter missed)
- `ADR-024-react-motion-useeffect-lifecycle.md` — React motion useEffect lifecycle split
- `ADR-021-cross-target-feature-parity.md` — Cross-target feature parity (DOM only, needs extension for animation)
- `ADR-025-omniview-review-remediation-2026-07-29.md` — Prior review remediation
- `ADR-028-omniview-debug-runtime-design-tradeoffs.md` — Debug runtime trade-offs
- `memory/omnilang-animation.md` — @omni/animation package architecture

### C. Key Files Inventory (Post-Scoping)

Files that remain after design scoping:

**Parser/AST**: `crates/omnilang-core/src/parser/ast/motion.rs` (~180 lines, down from 311)
**Parser logic**: `crates/omnilang-core/src/parser/expr/motion.rs`, `crates/omnilang-core/src/parser/top_level/motion.rs`
**IR types**: `crates/omnilang-core/src/ir/types_motion.rs` (~200 lines, down from 350)
**IR lowering**: `crates/omnilang-core/src/ir/lower/motion.rs` (~250 lines, down from 506)
**Semantic validation**: `crates/omnilang-core/src/semantic/motion.rs` (~150 lines, down from 244)
**Shared easing**: `crates/omnilang-core/src/codegen/shared/easing.rs` (extended with Flutter curves)
**OmniView codegen**: `motion.rs` (~100 lines), `motion_config.rs` (~200 lines), `motion_triggers.rs` (~350 lines, down from 651)
**React codegen**: `motion_effects.rs`, `motion/mod.rs`, `motion/transforms.rs`, `motion/keyframes.rs`
**Flutter codegen**: `motion.rs` (~250 lines, down from 421), `motion_widgets.rs` (kept)
**JS runtime**: `animate.ts`, `timeline.ts`, `stagger.ts`, `scroll.ts`, `flip.ts`, `easing.ts`, `keyframes.ts` (7 files, down from 8)

Files removed entirely:
- `packages/omni-animation/src/hero.ts`
- `omnilang/src/codegen/omniview/timeline.rs` (merge remaining into motion_config.rs or remove)
- Hero-related code in `view/element.rs:716-729`
- States-related code in `omniview/motion.rs:41-145`
- `omnilang/src/codegen/flutter/motion_widgets.rs` DrivenByBuilder section
