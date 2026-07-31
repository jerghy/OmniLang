# Architecture Review: Cross-Target Feature Gap Analysis — React & Flutter vs OmniView

**Date**: 2026-07-31
**Scope**: All P0 codegen targets — OmniView (debug ref), React (production web), Flutter (production mobile)
**Review type**: Cross-target feature parity (builds on ADR-021)
**Specialists**: 5 (Compiler & Language Architect, TypeScript & React Specialist, Systems Architect, Implementation Strategist, Pragmatic Enforcer)

---

## Executive Summary

The React and Flutter production codegen backends have **significant feature gaps** relative to the OmniView debug runtime. The gap is worse than "missing features" — it is a **hard compile gate**: any `.omni` program containing a `design` block (including the flagship `learn-omniview` project) compiles to **zero output files** in both React and Flutter (`omnilang/src/codegen/mod.rs:152-167, 192-203`). Additionally, the Flutter backend generates **Dart code that does not compile** for any non-trivial component — 6 CRITICAL bugs verified in committed golden snapshots. The React store codegen has **4 HIGH-severity silent-wrong-value bugs** (stale getters, broken Zustand reads, string corruption, class-map signal drops).

**Overall Assessment**: **React is Internal Tool quality (close to Production-Ready)** — JSX/CSS emitters are strong, but store correctness bugs and design-token absence block production claims. **Flutter is Prototype quality** — the widget skeleton is idiomatic, but generated Dart fails to compile for common patterns, and no validation gate (`dart analyze`) exists.

**Top finding**: The cross-target parity promise of ADR-021 is not being met. Design blocks cause full-program rejection; slots silently drop children; optional chaining loses null-safety; and Flutter effects, store reads, and composition all emit uncompilable code. All five specialists independently converged on this assessment.

---

## Cross-Cutting Themes (Appearing in 3+ Specialist Reviews)

### Theme 1: Design Blocks Are a Hard Compile Gate (5/5 specialists)

Every `.omni` program with a `design` block (tokens, classes, variants, themes) returns `CompileOutput { files: vec![], warnings: [DESIGN_BLOCK_WARNING] }` from React/Flutter. The `learn-omniview` reference project — the language's own flagship demo — produces zero files for either production target. This blocks all downstream design-dependent features (dynamic tokens, theme switching, class resolution, variant binding).

**Location**: `omnilang/src/codegen/mod.rs:152-167` (React), `192-203` (Flutter)
**Existing code to reuse**: `omnilang/src/codegen/omniview/design.rs` (306 lines, mostly target-agnostic)

### Theme 2: Flutter Generates Uncompilable Dart (4/5 specialists)

The committed golden snapshot (`parameterized__component__regression_flutter_fixture_component.snap:11,32`) encodes broken Dart:
- `const {` with no class name (syntax error)
- `_runEffect_N()` calls with no method definition (undefined identifier)
- Components referencing props/stores emit undefined Dart identifiers

No `dart analyze` or `flutter analyze` step validates any generated output. The test suite asserts substrings (`content.contains(...)`) that cannot catch these failure modes.

### Theme 3: Slots Are Broken in Both Production Targets (4/5 specialists)

React: `{/* WARNING: <slot> is not supported */}` — parent children pass through JSX but the child never renders `props.children` → **children silently dropped**.
Flutter: `SizedBox.shrink()` — parent emits `children:` param but child widget has no `children` field → **Dart compile error**.

**Locations**: `react/view.rs:827-830`, `flutter/view.rs:57-63, 141-146`
**Reference project impact**: `Modal.omni`, `NotificationsPanel.omni`, `SlotDefaultDemo.omni`, `Overview.omni`, `SignalPatternsDemo.omni` — 5 components use slots.

### Theme 4: No Output Validation Exists (4/5 specialists)

Neither `tsc --noEmit`, `dart analyze`, nor `flutter analyze` runs on generated code anywhere. The parameterized regression matrix (`tests_regression/parameterized.rs`) snapshots text only. Multiple CRITICAL bugs survive in committed snapshots because substring assertions can't catch missing class names, undefined identifiers, or invalid constructor parameters.

### Theme 5: ADR-021 Parity Policy Needs Refinement (3/5 specialists)

The blanket "semantically equivalent output for all backends" formulation is in tension with ADR-028's 14 intentional OmniView differences. Recommendation: reframe as "semantically equivalent behavior for a documented feature floor, with hard diagnostics outside it."

### Theme 6: ADR-020 Status Is Stale (3/5 specialists)

Riverpod is **already implemented** in the Flutter backend (`StateProvider`, `ConsumerStatefulWidget`, `WidgetRef`), contradicting the ADR's "⚠️ not yet implemented" status. But the implementation targets Riverpod 2.x APIs; ADR-020 selected 3.0. The ADR's acceptance criteria remain unchecked despite the code existing.

---

## Trade-offs and Disagreements

| Issue | Position A | Position B | Resolution |
|-------|-----------|-----------|------------|
| **Dynamic tokens in React** | Pragmatic Enforcer: Defer (CSS custom properties suffice; 1.67 ratio) | Implementation Strategist: Include in Phase 1 (4 tasks, M effort) | Defer to Phase 1.4 but gate on user demand. CSS `data-theme` covers ~95%; dynamic tokens are the debugging convenience case. |
| **Flutter Riverpod migration** | Pragmatic Enforcer: Defer (works with ChangeNotifier; 1.0 ratio) | Implementation Strategist: Code already migrated — fix ADR, don't rewrite code | Code is migrated to 2.x APIs. Fix: pin 2.x and update ADR-020, OR complete 3.0 migration. Not a "do nothing" decision. |
| **Flutter CSS property mapping** | Systems Architect: Backend is under-invested (6 of hundreds of CSS properties) | Pragmatic Enforcer: Extend reactively per user demand (0.43 ratio) | Both agree: semantic table for ~20 common properties + diagnostics for the rest. Not "all CSS." |
| **Animation parity claim** | ADR-032: Retained features "work identically in React and Flutter" | Systems Architect: Flutter drops playback, multi-stop keyframes, inline motions, and has wrong values for multi-property. | ADR-032 claim is violated. Flutter needs: per-property tweens, TweenSequence, playback honor, duration parsing (see C5 below). |

---

## Prioritized Recommendations

### CRITICAL (0-2 weeks — blocks production use entirely)

| # | Finding | Backend | Location | Fix |
|---|---------|---------|----------|-----|
| **C1** | Design blocks → zero output files | React + Flutter | `codegen/mod.rs:152-203` | Implement React CSS-variable emission reusing `omniview/design.rs`; Flutter: minimal ThemeData bridge. Until then, change warning→error (at least be loud about the failure). |
| **C2** | `@effect` calls undefined `_runEffect_N()` | Flutter | `flutter/mod.rs:243-249` | Generate real method; call from initState + post-setState (deps). Delete the `// @effect: re-executes` false comment. |
| **C3** | `const {` constructor missing class name | Flutter | `flutter/mod.rs:105-112` | Emit `const CompName({` instead of `const {`. |
| **C4** | Store reads in views → undefined Dart | Flutter | `flutter/view.rs:46-49` + shared `expr.rs:240-253` | Rewrite `Store.field` → `ref.watch(storeFieldProvider)` in view context. Thread store-alias→provider-name map. |
| **C5** | `color: #ff0000` → invalid Dart literal | Flutter | `flutter/view.rs:191-193` | Convert hex/named CSS colors → `Color(0xFFFFFFFF)`. |
| **C6** | Slug/prop/event blind mapping → invalid constructor params | Flutter | `flutter/utils.rs:41-102` + `view.rs:150-160, 270-280` | Per-widget parameter whitelist; unmappable pairs → compiler diagnostic. |
| **C7** | Store-signal mutations → undefined setters in components | React | `react/expr.rs:211-232` + `component/mod.rs:157-164` | Restore store ownership on signal refs or route through store setState/action calls. |
| **C8** | Zustand action conditions emit `ReferenceError` | React | `react/mod.rs:508-546` | Apply `rewrite_signal_refs_for_zustand` to all expressions in action body; always emit `const state = get();` when any signal referenced. |

### HIGH (2-8 weeks — correctness gaps with silent failure modes)

| # | Finding | Backend | Location | Fix |
|---|---------|---------|----------|-----|
| **H1** | `<slot>` silently drops children (React) / compile error (Flutter) | Both | `react/view.rs:827-830`, `flutter/view.rs:57-63, 141-146` | React: `{props.children}` for default slot. Flutter: `final List<Widget> children;` on widget + project into slot position. |
| **H2** | `OptionalChainExpr` (`?.`) silently dropped → `user.name` | Both | shared `expr.rs:137-139` | Override `format_optional_chain` in ReactFormatExpr and DartFormatExpr. |
| **H3** | `BlockExpr` statements silently dropped in expression context | Both | `react/expr.rs:28-31`, shared `expr.rs:410-413` | Emit IIFE wrapper preserving statements + tail return. |
| **H4** | Hooks-mode chained getters permanently stale | React | `react/mod.rs:244-258` | Include getter names in dependency scan (transitive closure). |
| **H5** | Zustand string-literal corruption from text rewriting | React | `react/mod.rs:557-591` | Rewrite identifiers at IR level, not on formatted text. |
| **H6** | Class-map-only signals dropped → undefined variable on render | React | `react/component/state.rs:465-516` | Collect signals from `class_map_bindings` values + inline style `$signal` refs. |
| **H7** | Flutter component composition: prop defaults ignored; events not declared; children param missing | Flutter | `flutter/mod.rs:98-120` | Non-required fields with defaults; `final void Function(T)? onX` event fields; `final List<Widget> children` slot param. |
| **H8** | Flutter animation: single-property tween only; playback dropped; multi-stop keyframes collapsed | Flutter | `flutter/motion.rs:24-53, 104-156`, `mod.rs:210-226` | Per-property tweens; TweenSequence for multi-stop; honor loop/delay/direction/fill; fix `1.5s` duration parsing. |
| **H9** | Flutter `@for` renders `ListView.builder` (scroll view) — crash inside non-scrollable parent | Flutter | `flutter/view.rs:558-561` | Use `Column(children: [...for...])` when parent isn't scrollable; thread `key_expr` → `ValueKey`. |
| **H10** | Flutter Riverpod version mismatch: 2.x APIs in code, 3.0 in ADR | Flutter | `flutter/mod.rs:371-389` | Decide version (pin 2.x or complete 3.0 migration); update ADR-020 status. |

### MEDIUM (2-6 months — quality and robustness)

| # | Finding | Backend | Fix |
|---|---------|---------|-----|
| **M1** | Default list key `String(item)` → `"[object Object]"` duplicate keys for object lists | React | Scalar check: keep `String(item)` for primitives; fall back to index + diagnostic for objects. |
| **M2** | Motion effects use global `document.querySelectorAll` + per-component MutationObservers | React | Scope to component root ref; remove body observers. |
| **M3** | `match` as expression statement discards arm values (functions return `undefined`) | React | Emit `return` in expression-statement context. |
| **M4** | Whole-store subscriptions trigger unnecessary re-renders | React | Emit selectors (`useStore(s => s.items)`) in Zustand mode. |
| **M5** | Cross-action stale closures in Hooks-mode store | React | Include action names in dependency computation. |
| **M6** | Event emits call optional props without `?.` guard → TypeError | React | `onClose?.(args)`. |
| **M7** | Motion controllers created for all module motions, not just this component's | Flutter | Filter to component-referenced motions only (cost: 500 unnecessary controllers in 50-comp app). |
| **M8** | `class_map_bindings` → empty `BoxDecoration()` visual no-op | Flutter | Resolve `style_classes` → per-class decoration entries. |
| **M9** | Match literal arms silently dropped; bindings never destructured | Flutter | Mirror React implementation: Dart IIFE with `switch`, binding extraction, default throw. |
| **M10** | Inline motion bodies and `paused_override` silently dropped | Both | React: inline keyframes into `_M` map. Flutter: synthetic motion for wrapping. |
| **M11** | `@ref` forwarding for custom components silently skipped | React | Emit compile-time diagnostic (Phase 2 feature). |
| **M12** | Type soundness holes: `any` fallbacks, `is_nullable` ignored | React | `unknown` over `any`; propagate `is_nullable` → `?`. |
| **M13** | `@ref` entirely unimplemented in Flutter (GlobalKey Phase 2) | Flutter | Reduced-surface GlobalKey (focus, scrollTo, size query only — not DOM-shaped API). |
| **M14** | No `main.dart`/`MaterialApp`/pubspec — output isn't runnable | Flutter | Minimal app shell generation. |
| **M15** | Helper classes duplicated per-file (`_OmniInViewWidget` etc.) | Flutter | Shared helpers file. |

---

## Implementation Roadmap (from Implementation Strategist)

### Phase 0 — Foundations (~3-4 days)
- T0.1: Expand parameterized cross-target matrix with design/slots/refs/key_expr fixtures
- T0.2: Fix ADR-020 governance (mark Riverpod implemented; write ADR-052 for parity plan)
- T0.3: Extract design CSS emission to `codegen/shared/design.rs` (reuse OmniView's `emit_design_css`)

### Phase 1 — React Design Parity (~8-10 days)
- T1.1: React design tokens + themes → global `.design.css` + `useTheme()` hook
- T1.2: Decouple design-block gate for React
- T1.3: React design classes/variants → resolve `class: Design.Card`
- T1.4: React dynamic tokens (deferred behind user demand gate per Pragmatic Enforcer)
- T1.5: Verify: `omnic build --target react` on learn-omniview + Vite smoke test

### Phase 2 — Flutter Design Parity (~12-16 days)
- T2.1: Expand CSS property map (6→~20) + fix C5 color emission
- T2.2: Flutter static tokens → typed Dart constants + ThemeData
- T2.3: Flutter classes/variants → factory/decoration helpers
- T2.4: Flutter dynamic tokens (recommend: diagnostic for v1)

### Phase 3 — Composition & Correctness (~8-10 days)
- T3.1-T3.2: Slots in both targets
- T3.3-T3.4: Ref forwarding (React forwardRef; Flutter GlobalKey reduced surface)
- T3.5: `@ref` inside `@for` (both targets)
- T3.6: Flutter `key_expr` → `ValueKey`

**Total: ~33-43 person-days (~6-8 person-weeks)**

**Quick wins (high impact, low effort — can ship this week):**
1. Flutter: Fix C2+C3+C4+C5 (effect + constructor + store-read + color) — 4 small changes, unblocks compilation
2. React: Fix H4+H5+H6 (getter deps + Zustand rewrite + class-map signals)
3. Both: Fix H2+H3 (OptionalChain + BlockExpr) — 2 shared overrides

---

## Risks if Unaddressed

1. **Production-target claims become indefensible.** ADR-003 declares React and Flutter as P0 production targets. With design blocks refusing compilation, slots silently dropping, and Flutter emitting broken Dart, neither target can compile the project's own flagship demo. Every subsequent review will re-discover the same gaps.

2. **Silent data corruption in generated React stores.** H1 (stale chained getters), H3 (string-literal corruption), and H5 (Zustand ReferenceError reads) produce wrong data with zero warnings. This is the worst failure mode for a codegen product — bugs appear in user applications far from the compiler.

3. **Flutter backend is aging without feedback.** Without a `dart analyze` CI gate, every change risks silently breaking correctness. The downstream user (who runs `omnic build --target flutter` and gets broken Dart) is the only validator.

4. **Trust erosion.** Each ADR-021 "silent feature loss" pattern (slots, optional chaining, block expressions, inline motions, match arms, paused_override) that persists in the codebase makes the parity promise less credible.

5. **Governance drift.** ADR-020's stale status (code says Riverpod, ADR says ChangeNotifier) and the absence of any design-token deferral item in `.architecture/deferrals.md` mean the planning documents don't reflect reality — the project is operating on wrong information.

---

## Success Metrics and Follow-Up

### Acceptance Criteria for This Review
- [ ] C1-C8 CRITICAL items resolved (design gate + Flutter compilation)
- [ ] H1-H10 HIGH items resolved or explicitly deferred with diagnostics
- [ ] CI gate: `dart analyze` on generated Flutter fixture output
- [ ] CI gate: `tsc --noEmit` on generated React fixture output
- [ ] ADR-020 status corrected; ADR-052 filed (parity plan)
- [ ] `.architecture/deferrals.md` updated with design-token items

### Follow-Up
1. **Immediate**: Write ADR-052 — Cross-Target Feature Floor & Parity Redefinition
2. **Phase 0**: Extract shared design-CSS emitter + expand test matrix
3. **After Phase 1**: `omnic build --target react` on learn-omniview must produce a working Vite app
4. **After Phase 2**: `omnic build --target flutter` on learn-omniview must produce `dart analyze`-clean output
5. **Ongoing**: Add `dart analyze`/`tsc --noEmit` to CI for generated regression fixtures

---

## Per-Specialist Review Summaries

### 1. Compiler & Language Architect — Dr. Lin Wei

**Assessment**: Conducted a 45-feature matrix mapping every IR construct to its React/Flutter implementation status. Found 7 ❌ (completely missing) and 20 ⚠️ (partially broken) feature entries across the two production targets.

**Top findings**:
- 6 CRITICAL: design-block gate (both), Flutter store-read broken, Flutter @effect undefined, Flutter slots unmatched params, React store-signal mutation broken, Flutter design-variant leak
- 7 HIGH: slots in both, OptionalChain dropped, BlockExpr dropped, React Zustand broken, Flutter component props/events/children, Flutter @for scroll semantics, Flutter animation fidelity
- Verified shared `FormatExpr` defaults are the root cause for H2/H3 — neither React nor Flutter overrides the shared default behavior

### 2. TypeScript & React Specialist — Sarah Chen

**Assessment**: React backend rated **Internal Tool, close to Production-Ready**. JSX emission, CSS modules, and security posture are strong. Four verified correctness bugs and the design-token absence block the final step.

**Top findings** (empirically verified by compiling tests against current compiler):
- H1: Chained getters (`quadCount = doubleCount * 2`) produce permanently stale values — deps `[]` instead of `[doubleCount]`
- H2: Zustand action `if (count > 5)` emits `ReferenceError` — no `count` in scope, rewrite not applied
- H3: Zustand `set({ count: "current count is 0" })` becomes `"current state.count is 0"` — string-literal corruption
- H4: Class-map-only signals generate `ReferenceError` — signal not collected as read, no `useState` emitted
- H5: `String(item)` for object lists → duplicate `"[object Object]"` keys

### 3. Systems Architect — Dr. Marcus Webb

**Assessment**: Flutter backend rated **Prototype**. Widget skeleton is idiomatic, but generated Dart does not compile for any non-trivial component. No validation gate exists.

**Top findings**:
- C1-C6: 6 CRITICAL compile-error generators (see CRITICAL table above)
- Committed golden snapshot encodes `const {` (missing class name) and undefined `_runEffect_0()` — bugs are literally in the repo
- Riverpod is partially implemented targeting the wrong version (2.x vs ADR's 3.0)
- Animation ADR-032 parity claim is violated: playback dropped, multi-stop keyframes collapsed, inline motions produce phantom controllers
- `class_map_bindings` → no-op `BoxDecoration()` — dynamic classes functionally dead

### 4. Implementation Strategist — Dr. Aisha Okafor

**Assessment**: Delivered a phased roadmap with dependency graph, blast radius analysis, and reversibility assessments. Every task is additive or a one-line gate change — no data migrations, no IR changes, no irreversible steps.

**Key insights**:
- Flutter Riverpod is **already implemented** (contradicting pre-analysis assumption and stale ADR-020)
- IR is fully backend-agnostic — no IR changes needed for design work
- Single highest-leverage item: gate decoupling after design CSS emission (T1.2)
- Hard problems identified: Flutter CSS box model→widget tree (H1), variant system (H2), dynamic tokens (H3), multiple design blocks in React (H4), pseudo-states in Flutter (H5), @ref in @for (H6)
- Overall reversal risk: Low — all changes are additive file emission or one-line gate changes

### 5. Pragmatic Enforcer

**Assessment**: Applied YAGNI analysis to every feature category. Recommends implementing ~70% of OmniView features for production targets, with hard diagnostics for the remaining 30%.

**Key decisions**:
- Implement Now: React static design tokens (0.44 ratio), slots (0.375/0.5 ratio)
- Defer/Drop: Dynamic tokens in both targets (1.67-2.67 ratio), Riverpod migration (1.0 ratio)
- Redesign Simpler: ADR-021 parity policy (behavior floor + diagnostics, not identical output), Flutter @ref surface (GlobalKey subset, not DOM-shaped API)
- Explicitly Exclude from Parity: All 14 ADR-028 debug trade-offs + dynamic token bridge + source maps for React/Flutter
- Top over-engineering risk: Building `__omni_effect` bridge for React/Flutter (debug-runtime solution for problems production targets solve idiomatically)

---

## References

- [ADR-003](./adrs/ADR-003-codegen-target-prioritization.md) — P0 target prioritization (OmniView, React, Flutter)
- [ADR-020](./adrs/ADR-020-flutter-state-management-riverpod.md) — Flutter Riverpod migration (stale — code already migrated)
- [ADR-021](./adrs/ADR-021-cross-target-feature-parity.md) — Cross-target feature parity enforcement
- [ADR-028](./adrs/ADR-028-omniview-debug-runtime-design-tradeoffs.md) — 14 intentional OmniView design trade-offs
- [ADR-032](./adrs/ADR-032-omniview-animation-design-scoping.md) — Animation scoping 33→14 features
- [ADR-036](./adrs/ADR-036-element-reference-system.md) — Element Reference System
- [ADR-047](./adrs/ADR-047-dynamic-variable-design-tokens.md) — Dynamic design tokens Phase 1
- [ADR-049](./adrs/ADR-049-dynamic-token-variables-phase2-runtime-signals.md) — Dynamic tokens Phase 2 (runtime signals)
- [`.architecture/deferrals.md`](../deferrals.md) — Current deferred items

**Key source files**:
- `omnilang/src/codegen/mod.rs:57-65, 152-203` — Design block gate
- `omnilang/src/codegen/omniview/design.rs` — Design CSS emission (to be extracted to shared)
- `omnilang/src/codegen/react/mod.rs:244-258, 480-552, 557-591` — Store bugs
- `omnilang/src/codegen/react/component/state.rs:465-516` — Class-map signal gap
- `omnilang/src/codegen/flutter/mod.rs:98-120, 243-249, 371-389` — Constructor, effect, Riverpod
- `omnilang/src/codegen/flutter/view.rs:57-63, 88-98, 141-146, 191-193, 558-561` — Slots, CSS map, composition, colors, @for
- `crates/omnilang-core/src/codegen/shared/expr.rs:137-139, 383-403, 410-413` — OptionalChain, Match, BlockExpr defaults
