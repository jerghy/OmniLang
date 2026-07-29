# Architecture Review: OmniView Animation Subsystem v2 (Post-ADR-032 Verification)

## Overview

Follow-up architecture review of the OmniView animation subsystem after the ADR-032 design scoping implementation (completed 2026-07-29). **10 specialists** independently reviewed the post-scoping architecture to verify soundness, identify new issues introduced during the scoping implementation, and assess the completeness of the ADR-032 cleanup.

## Review Details

- **Component Reviewed**: OmniView Animation Subsystem (post-ADR-032)
- **Review Date**: 2026-07-29
- **Team**: Compiler & Language Architect, Rust Expert, TypeScript & React Specialist, Performance Specialist, Maintainability Expert, Security Specialist, Systems Architect, Pragmatic Enforcer, Domain Expert, Implementation Strategist
- **Prior Review**: `.architecture/reviews/component-omniview-animation.md` (2026-07-29, 8 experts)
- **ADR**: ADR-032 (Accepted)
- **Test Baseline**: 837 Rust tests pass, 18 JS tests pass, clippy clean, cross-target green

---

## Executive Summary

The post-ADR-032 animation architecture is **sound at its core** — the pipeline layering is clean, the remaining feature set (7 triggers × 1 body mode × 4 easing variants) is appropriately scoped, and the shared `NAMED_EASINGS` canonical table is a well-designed cross-backend contract. Zero cross-backend feature gaps have been achieved, and all 4 critical bugs (C1-C4) from the prior review have been fixed.

However, the review uncovered **one P0 correctness bug** and **significant incomplete cleanup** from the ADR-032 scoping. The easing direction normalization bug silently causes `backIn`/`backInOut`/`elasticIn`/`elasticInOut` to produce the wrong curves on all backends. Additionally, ~200+ lines of dead code, dead lexer tokens, dead pipeline fields, and stale documentation survived the scoping — collectively representing about 15% of the post-scoping code surface that exists as vestigial carry-over.

**Overall Assessment**: **Adequate → Good after P0 fix**. The core architecture is correct; the issues are surgical and well-understood.

### Top 3 Priorities

1. **P0: Fix Back/Elastic easing direction normalization** — `backIn`/`elasticIn`/etc. produce `*Out` curves on all backends (S effort, 15 min)
2. **HIGH: Remove dead `path`/`pathRotate` fields from entire pipeline** — silently parsed/lowered/discarded, violates ADR-032's "no silent feature loss" principle (S effort)
3. **HIGH: Fix non-deterministic Flutter `extract_tween_begin`/`end`** — FxHashMap iteration order affects multi-property keyframe output (S effort)

### Immediate Actions

- Fix `parser/expr/motion.rs:55-63` — normalize easing direction to strip prefix
- Remove `path`/`pathRotate` from AST/IR/lowering (emit parse error if written)
- Add property name parameter to `extract_tween_begin`/`extract_tween_end`

---

## Cross-Cutting Themes

### Theme 1: Easing Direction Normalization Bug (P0) — 6/10 reviewers

**Identified by**: Compiler Architect, TS/React Specialist, Maintainability Expert, Pragmatic Enforcer, Domain Expert, Systems Architect

The parser stores the full easing name as the direction string:
```
"backIn" → direction: Some("backIn")
"elasticInOut" → direction: Some("elasticInOut")
```

But all codegen backends and the JS runtime check for the suffix form:
- OmniView `motion_config.rs:25-38`: checks `"in"` / `"out"` / `"inOut"`
- React `keyframes.rs:127-143`: checks `"In"` / `"Out"` / `"InOut"`
- Flutter `motion.rs:95`: ignores direction entirely (always `easeOut*`)
- JS `easing.ts:62-74`: expects `'in'` / `'out'` / `'inOut'`

**Result**: `ease: backIn` → `backOut` curve on all targets. `backInOut`, `elasticIn`, `elasticInOut` also affected. This is the single correctness bug found.

**Fix**: Strip the prefix in the parser (`motion.rs:55-63`). Normalize `"backIn"` → `direction: Some("In")`, `"backOut"` → `direction: None`, etc. This fixes all backends simultaneously.

### Theme 2: Incomplete ADR-032 Cleanup — 8/10 reviewers

**Identified by**: Compiler Architect, Rust Expert, TS/React Specialist, Maintainability Expert, Pragmatic Enforcer, Domain Expert, Systems Architect, Implementation Strategist

The ADR-032 scoping removed 21 features from the middle of the pipeline (IR, lowering, codegen) but left structural remnants at the pipeline edges. Collectively these represent ~200+ lines of dead/vestigial code:

| Remnant | Lines | Location | Status |
|---------|-------|----------|--------|
| Dead @focus JS codegen in React | ~20 | `react/motion/mod.rs:127-165` | M1 (parked) |
| 7 dead lexer tokens | ~25 | `lexer/token.rs`, `keywords.rs` | M2 (parked) |
| Stale doc comments | ~15 | 3+ files | M3 (parked) |
| Dead `playback_to_config` fn | 44 | `motion_config.rs:110-154` | New finding |
| Dead `is_transform_prop` fn | 13 | `motion_config.rs:157-170` | New finding |
| Dead `format_transform_value` fn | 10 | `motion.rs:25-34` | New finding |
| Dead `path`/`pathRotate` fields | ~20 | AST → IR → lowering | New finding |
| Dead HeroDef parsing chain | ~40 | `component.rs` + 2 types + 11 lint stubs | New finding |
| Dead `hover_reverse` field | ~10 | IR + lowering | New finding |
| Dead `MotionExprIR` enum | 5 | `ir/types_motion.rs:100-104` | New finding |
| 6 dead golden snapshot files | 90 | `tests_regression/snapshots/` | New finding |
| 4 dead golden .omni fixture files | ~100 | `tests/fixtures/golden/` | New finding |

### Theme 3: Flutter Codegen Non-Determinism — 4/10 reviewers

**Identified by**: Rust Expert, Systems Architect, Maintainability Expert, Performance Specialist

`extract_tween_begin`/`extract_tween_end` in `flutter/motion.rs:39-71` iterate `FxHashMap::values()` without a property name filter. When a keyframe stop has multiple properties, the returned value depends on HashMap iteration order (non-deterministic due to hash randomization). This means the same `.omni` source can produce different Flutter Dart output across compilations.

**Fix**: Add a `prop: &str` parameter to both functions and look up by key.

### Theme 4: Easing Table Fragmentation — 5/10 reviewers

**Identified by**: TS/React Specialist, Maintainability Expert, Systems Architect, Security Specialist, Pragmatic Enforcer

The same easing name-to-value mapping exists in 3 independent sites:
1. Canonical `NAMED_EASINGS` in Rust (`shared/easing.rs`) — used by semantic validation and Flutter
2. JS `NAMED_EASINGS` in `easing.ts` — used by OmniView runtime
3. Hardcoded values in React `translate_easing` (`keyframes.rs`) — bypasses the canonical table for Back/Elastic structural variants

The React codegen's `translate_easing` is the only path that hardcodes cubic-bezier values instead of delegating to the canonical table. This caused the BackIn value bug where `0.165` was used instead of `-0.280` (a separate bug from Theme 1, specific to the React hardcoded value).

**Fix**: Refactor `translate_easing` to construct canonical easing names (e.g., `"backIn"`) and delegate to `resolve_easing()`.

### Theme 5: Silent Feature Loss Without Migration Errors — 4/10 reviewers

**Identified by**: Domain Expert, Implementation Strategist, Systems Architect, Compiler Architect

ADR-032's validation criteria requires "clear compiler errors with migration guidance" for removed features. Several removed features are silently accepted:
- `@drag-scrub`/`@drag-momentum`: parsed, lowered, silently dropped at `lower/view.rs:522-523`
- `@hover-reverse`: parsed, lowered into IR, no codegen backend reads it
- Component-level `@hero`: parsed into `HeroDef`, lowerer discards with `hero_defs: vec![]`
- `path`/`pathRotate`: parsed, lowered into IR, no backend emits them

**Fix**: Add explicit parse errors or semantic warnings with migration guidance.

### Theme 6: Observer/Memory Leak Gaps — 3/10 reviewers

**Identified by**: Performance Specialist, Security Specialist, TS/React Specialist

The `__omni_cleanupElement` fix (C4) addressed the `__omni_animations` Map leak but sibling data structures have incomplete cleanup:
- `__omni_scrollRemoveEntry` called after `el.remove()` — can never find the element
- `__omni_inview_observers` array never evicts disconnected observers
- `__omni_el._cache` retains stale entries after element removal

---

## Individual Specialist Reviews

### Compiler & Language Architect (Dr. Lin Wei)

**Perspective**: Pipeline soundness, cross-backend codegen consistency, IR design, DSL semantics.

**Key Finding**: The easing direction parsing bug is a pipeline-wide semantic gap — the parser stores data in one format, codegen expects another, and no intermediate pass normalizes. This is exactly the kind of cross-layer contract violation that ADR-032's pipeline simplification was supposed to prevent.

**Strengths**: Clean shared `NAMED_EASINGS` table; thorough semantic validation; shared scroll handler pattern; React hover/press effect cleanup.

**Concerns**:
- **HIGH**: Back/Elastic direction normalization bug — all 4 directional variants affected on all backends
- **MEDIUM**: Bounce/elastic cubic-bezier approximations identical to power curves on Web — different behavior between Web and Flutter for same easing name
- **MEDIUM**: Dead @focus JS in React codegen
- **MEDIUM**: FLIP only implemented in OmniView (not React/Flutter)
- **MEDIUM**: Path animation fields flow through pipeline silently undelivered
- **LOW**: Flutter InView `Scrollable.maybeOf` returns null for non-Scrollable ancestors

**Recommendations**: Fix direction normalization (P0, S), remove @focus dead code (P1, S), add path animation warnings (P1, S).

---

### Rust Expert (Katya Volkov)

**Perspective**: Type safety, ownership patterns, error handling, unsafe usage, idiomatic Rust.

**Key Finding**: Zero `unsafe` blocks, zero production `unwrap()`/`expect()` — the animation subsystem is exemplar Rust. The dead code items (`#[allow(dead_code)]` functions, `MotionExprIR` enum) are the main quality concern.

**Strengths**: Exhaustive pattern matching; type-driven AST/IR design; shared `NAMED_EASINGS`; `unreachable!()` with clear messages.

**Concerns**:
- **MEDIUM**: Flutter `extract_tween_begin`/`end` FxHashMap non-determinism
- **LOW**: Dead `format_transform_value`, `playback_to_config`, `is_transform_prop` functions with `#[allow(dead_code)]`
- **LOW**: `map_or(&[] as &[TriggerIntent], ...)` anti-pattern repeated 4 times
- **LOW**: Silent `unwrap_or("ease-out")` fallback without diagnostic
- **LOW**: Inconsistent `Emitter` parameter style across backends

**Recommendations**: Add `debug_assert!` to easing fallback (S), remove dead functions (M), add `ElementIR::triggers()` helper (S), fix Flutter non-determinism (S→L depending on approach).

---

### TypeScript & React Specialist (Emma Rodriguez)

**Perspective**: JS runtime architecture, React codegen output, WAAPI usage, TS type safety.

**Key Finding**: The React backend hardcodes easing cubic-bezier values at compile time while OmniView defers to JS runtime — this architectural divergence means the React backend is the only site where easing drift can occur at compile time.

**Strengths**: Clean WAAPI abstraction in `animate.ts`; scroll handler consolidation; stagger algorithm with good test coverage; FLIP edge-case handling; canonical easing table design.

**Concerns**:
- **HIGH**: React `translate_easing` has wrong BackIn value (`0.165` vs `-0.280`) — distinct from the parser direction bug
- **MEDIUM**: @focus dead code emitted in every React motion component
- **MEDIUM**: Easing values duplicated across 3 independent sites
- **LOW**: Drag velocity computed but never used (both backends)
- **LOW**: Module-level FLIP state leaks across component lifetimes
- **LOW**: React codegen discards Animation return value (no lifecycle management)
- **LOW**: Stale comments referencing removed triggers

**Recommendations**: Fix BackIn value (S, 1 line), remove @focus block (S, 10 lines), refactor `translate_easing` to delegate to canonical table (S, 30 min).

---

### Performance Specialist

**Perspective**: WAAPI compositor-thread usage, forced reflows, memory leaks, GC pressure, scroll efficiency.

**Key Finding**: The OmniView backend's shared scroll handler and direct animation reference storage is significantly more performant than the React backend's per-element `getAnimations()` calls — a gap of 180+ allocations/sec for 3 scroll-driven elements.

**Strengths**: Shared scroll handler with passive listener; `__omni_cleanupElement` Map eviction; animation reference storage avoiding `getAnimations()`; FLIP self-cleaning listeners; small dependency-free runtime.

**Concerns**:
- **MEDIUM**: `__omni_scrollRemoveEntry` called after `el.remove()` — element already disconnected, cleanup short-circuits
- **MEDIUM**: React scroll/drag handlers call `getAnimations()` per-frame (180+ allocations/sec at 60fps)
- **MEDIUM**: `scrollProgress` calls `getBoundingClientRect()` twice per frame per element
- **MEDIUM**: `__omni_inview_observers` array leaks disconnected IntersectionObservers
- **LOW**: Non-compositor property animations trigger main-thread layout (guidance issue)
- **LOW**: `setInterval` polling fallback in wait-mode transitions
- **LOW**: Drag velocity computed but never consumed
- **LOW**: `__omni_el._cache` retains stale entries

**Recommendations**: Fix cleanup order in @for leave path (S), remove dead velocity computation (S), cache `getBoundingClientRect()` in `scrollProgress` (S), store animation refs in React backend (M).

---

### Maintainability Expert

**Perspective**: Code organization, duplication, comment accuracy, file cohesion, technical debt, test coverage gaps.

**Key Finding**: The React backend's `translate_easing` bypasses the canonical `NAMED_EASINGS` table — this architectural duplication is the root cause of both the BackIn value bug and the easing drift risk. The Flutter backend correctly delegates; React should too.

**Strengths**: Canonical `NAMED_EASINGS` table design; clean trigger dispatch with `unreachable!()` guard; OmniView runtime delegation pattern (immune to easing drift); Flutter proper delegation to canonical table; inline/named trigger unification.

**Concerns**:
- **HIGH**: React BackIn cubic-bezier value wrong (`0.165` → should be `-0.280`)
- **MEDIUM**: Dead @focus JS codegen in React
- **MEDIUM**: Dead `playback_to_config` function (`#[allow(dead_code)]`)
- **MEDIUM**: Dead `is_transform_prop` function (`#[allow(dead_code)]`)
- **MEDIUM**: Easing drift risk across 3 independent sites
- **MEDIUM**: `extract_tween_begin`/`end` FxHashMap non-determinism
- **LOW**: Stale doc comment in `motion_triggers.rs`
- **LOW**: Stale comment in `view/element.rs`
- **LOW**: Dead lexer tokens (M2)

**Recommendations**: Fix BackIn value (XS, 1 line), remove @focus block (XS), remove dead functions (XS), refactor `translate_easing` (S, 30 min), fix Flutter non-determinism (S).

---

### Security Specialist

**Perspective**: Codegen injection vectors, XSS risks, CSP compatibility, observer/event-listener lifecycle safety.

**Key Finding**: The injection surface is small and well-understood — all three classes of user-controlled strings (nid values, expression values, scroll/drag config strings) are properly escaped. The CSP `style-src 'unsafe-inline'` is a known architectural tradeoff.

**Strengths**: Consistent escaping strategy (`escape_literal_str` + `escape_html`); defense-in-depth URL injection blocking (`is_dangerous_url` + `__omni_sanitizeUrl`); no `eval()`/`new Function()` in generated code; CSS injection protection at runtime; compiler-level sandboxing with external runtime script.

**Concerns**:
- **MEDIUM**: InView IntersectionObserver leak — `__omni_cleanupElement` doesn't disconnect observers
- **MEDIUM**: CSP `style-src 'unsafe-inline'` across all generated pages (architectural tradeoff)
- **LOW**: `easing_to_js` trusts parser output without escaping (defense-in-depth gap)
- **LOW**: `__omni_el` uses `querySelector` with string interpolation
- **LOW**: Theme runtime `localStorage` without integrity check (mitigated)

**Recommendations**: Add `escape_literal_str` to `easing_to_js` (S, 3 lines), add InView observer tracking to `__omni_cleanupElement` (S→M), switch `__omni_el` to `getElementById` (S).

---

### Systems Architect

**Perspective**: Pipeline coherence, component coupling, architectural principles, dependency contracts.

**Key Finding**: Pipeline topology is clean with one layering exception — semantic validation imports from codegen module to validate easing names, inverting the natural dependency direction. `NAMED_EASINGS` should live in a neutral location (e.g., `ir/easing.rs`).

**Strengths**: Shared `NAMED_EASINGS` as cross-backend contract; conditional runtime embedding; shared scroll handler; generator state isolation (InliningState save/restore); thorough semantic validation.

**Concerns**:
- **HIGH**: Dead `path`/`pathRotate` fields flow through entire pipeline without backend emission
- **HIGH**: Non-deterministic Flutter keyframe property extraction via HashMap iteration
- **MEDIUM**: Unverified duplicated easing tables across Rust and JS
- **MEDIUM**: React @in-view silent fade-in fallback hides motion name errors
- **MEDIUM**: Dead `playback_to_config` method
- **MEDIUM**: Dead @focus code in React backend
- **MEDIUM**: Inconsistent easing resolution strategy across backends
- **LOW**: React `useEffect` with empty deps creates duplicate animations under StrictMode

**Recommendations**: Remove `path`/`pathRotate` from pipeline (S), remove Flutter `motion_has_path` logic (S), fix Flutter non-determinism (S), add cross-verification test (M), standardize easing resolution (M).

---

### Pragmatic Enforcer

**Perspective**: YAGNI, necessity/complexity ratios, dead code, over-engineering.

**Key Finding**: The remaining architecture is appropriately scoped — no feature stands out as YAGNI-violating. The issues are **incomplete cleanup** from ADR-032, not over-engineering. Collectively ~200 lines of dead/vestigial code survived the scoping cut.

**Necessity/Complexity Analysis**: All 7 retained triggers have necessity-to-complexity ratios ≥ 1.40, with most above 2.0. The `NAMED_EASINGS` table has 4 duplicate entries (same cubic-bezier values under different names) inflating the table by ~15%.

**Concerns**:
- **HIGH**: Easing direction normalization bug — affects all backends
- **MEDIUM**: 7 dead lexer tokens from pre-ADR-032
- **MEDIUM**: HeroDef parsing chain produces data immediately discarded
- **MEDIUM**: 3 dead functions exist only for their tests
- **MEDIUM**: React @focus dead code emission
- **LOW**: 4 duplicate easing approximations
- **LOW**: `path_rotate` field parsed but never used
- **LOW**: Stale doc comments

**Recommendations**: Fix easing direction (S, 15 min), remove all dead code (~2 hours across 8 items), zero architectural surgery required.

---

### Domain Expert

**Perspective**: Animation domain modeling, feature coverage for real user needs, semantic accuracy.

**Key Finding**: The ADR-032 scoping was thorough at the IR/codegen level but left domain concepts alive at pipeline edges. The lexer still tokenizes removed triggers, the parser still parses `@hero`, and the JS runtime still exports `timeline()`. These create false domain promises — the model says features exist that the compiler cannot deliver.

**Concerns**:
- **HIGH**: `@drag-scrub`/`@drag-momentum` silently dropped by lowerer with zero diagnostic — violates ADR-032's "clear compiler errors" requirement
- **MEDIUM**: Dead @focus codegen in React shipped in every component
- **MEDIUM**: `@hover-reverse` is dead concept spanning full pipeline
- **MEDIUM**: Removed trigger tokens produce cryptic errors ("Unexpected token: AtFocus") instead of migration guidance
- **MEDIUM**: Runtime library ships removed-feature code (`timeline.ts` exported, spring/stepStart/stepEnd in easing map)
- **LOW**: `@paused` per-element override is OmniView-only
- **LOW**: Path animation concept exists in IR but no backend implements it
- **LOW**: `@drag-scrub`/`@drag-momentum` lexer/parser infrastructure remains

**Recommendations**: Add compile error for @drag-scrub/momentum (S), add migration error messages for removed triggers (S), guard @focus in React (S), remove @hover-reverse (M), remove timeline export (S).

---

### Implementation Strategist

**Perspective**: Blast radius, reversibility, migration impact, sequencing of parked findings.

**Key Finding**: The "big bang" atomic commit strategy for ADR-032 was correct — it avoided broken intermediate states. Reversibility is strong for individual features (each is an independent AST/IR/codegen unit). The parked findings (M1-M3) are real but low-risk; the scope of dead code is larger than initially reported (6 additional dead snapshot files, 4 dead golden fixtures, active Hero parser path).

**Concerns**:
- **MEDIUM**: Golden fixture files reference removed features (HeroTransitionDemo, TimelineMotionDemo, PathTimelineDemo, MotionTriggersDemo)
- **MEDIUM**: Hero definitions silently parsed then discarded (no compiler error)
- **MEDIUM**: React @focus dead codegen (M1)
- **LOW**: 6 unreferenced dead snapshot files
- **LOW**: Dead lexer tokens (M2 — specific locations mapped)
- **LOW**: Stale documentation across `docs/omniview/` (M3 — specific files and line numbers mapped)

**Recommendations**: Remove @focus dead code (S, #1 priority), add compiler error for component-level @hero (S), clean golden fixtures (S), remove dead snapshots (S), update source comments (S), update docs (L, deferred).

---

## Prioritized Recommendations

### CRITICAL — Fix Immediately (P0)

| # | Action | Effort | Reviews | Files |
|---|--------|--------|---------|-------|
| **P0** | Fix Back/Elastic easing direction normalization — strip prefix in parser so `"backIn"` → `direction: Some("In")` | **S** (15 min) | 6/10 | `parser/expr/motion.rs:55-63` |

This is the only correctness bug found. It silently causes 4 named easings to produce the wrong curves on all backends. The fix is 5 lines in the parser and fixes all backends simultaneously.

### HIGH — Implement Within 1 Week (S effort each)

| # | Action | Effort | Reviews | Files |
|---|--------|--------|---------|-------|
| **H1** | Remove dead `path`/`pathRotate` from AST, IR, lowering, parser — emit parse error if written | S | 4/10 | `parser/ast/motion.rs:35-37`, `ir/types_motion.rs:33-37`, `ir/lower/motion.rs:72-73`, `parser/top_level/motion.rs` |
| **H2** | Fix Flutter `extract_tween_begin`/`end` non-determinism — add property name parameter | S | 4/10 | `flutter/motion.rs:39-71`, callers in `flutter/mod.rs` |
| **H3** | Add migration errors for silently-dropped features (@drag-scrub, @drag-momentum, @hover-reverse, @hero in components) | S | 4/10 | `parser/top_level/component.rs`, `parser/view/mod.rs`, `ir/lower/view.rs` |
| **H4** | Fix React BackIn hardcoded cubic-bezier value (`0.165` → `-0.280`) in `translate_easing` | **XS** (1 line) | 4/10 | `react/motion/keyframes.rs:130` |
| **H5** | Add `escape_literal_str` to `easing_to_js` in OmniView codegen | S (3 lines) | 2/10 | `omniview/motion_config.rs:18,28,33` |

### MEDIUM — Implement Within 2 Weeks

| # | Action | Effort | Reviews |
|---|--------|--------|---------|
| **M1** | Remove dead @focus JS codegen from React backend (M1 parked finding) | S | 8/10 |
| **M2** | Remove dead `playback_to_config`, `is_transform_prop`, `format_transform_value` functions + tests | S | 5/10 |
| **M3** | Remove dead `MotionExprIR` enum | S | 3/10 |
| **M4** | Remove 7 dead lexer tokens (M2 parked finding) | S | 5/10 |
| **M5** | Remove HeroDef dead parsing chain + 11 lint file stubs | S | 4/10 |
| **M6** | Remove dead `hover_reverse` field from IR + lowering | S | 2/10 |
| **M7** | Fix `__omni_scrollRemoveEntry` cleanup order (call before `el.remove()`) | S | 2/10 |
| **M8** | Refactor React `translate_easing` to delegate to canonical `NAMED_EASINGS` | S | 5/10 |
| **M9** | Fix `__omni_inview_observers` array leak — evict or use WeakSet | S→M | 3/10 |
| **M10** | Update stale doc comments across 5+ files (M3 parked finding) | S | 6/10 |
| **M11** | Clean up 6 dead golden snapshot files + 4 dead golden fixture files | S | 2/10 |
| **M12** | Add `ElementIR::triggers()` helper to eliminate `map_or(&[] as &[TriggerIntent], ...)` | S | 2/10 |

### LOW — Deferred (Backlog)

| # | Action | Effort |
|---|--------|--------|
| L1 | Deduplicate 4 identical easing approximations in `NAMED_EASINGS` table | S |
| L2 | Remove drag velocity dead computation from both backends | S |
| L3 | Cache `getBoundingClientRect()` in `scrollProgress` to avoid double layout | S |
| L4 | Store animation references in React backend (eliminate per-frame `getAnimations()`) | M |
| L5 | Add React StrictMode animation cleanup (cancel on unmount) | S |
| L6 | Update `docs/omniview/` animation documentation for post-ADR-032 feature set | M |
| L7 | Add cross-verification test for Rust vs JS easing tables | M |
| L8 | Standardize easing resolution strategy across backends (all compile-time) | M |

---

## Trade-offs and Disagreements

### Trade-off 1: Compile-time vs Runtime Easing Resolution

**Positions**:
- Systems Architect, TS/React Specialist: Standardize on compile-time resolution (removes JS runtime as divergence point)
- OmniView backend (current design): Delegate to JS runtime (flexibility for parameterized easing)

**Resolution**: Accept the divergence for now. The canonical `NAMED_EASINGS` table is the shared source of truth; the JS runtime is a downstream consumer. The real fix is M8 (React delegation to canonical table) + L8 (long-term standardization). The BackIn value bug proves the current React hardcoding approach is the risk, not the OmniView runtime approach.

### Trade-off 2: Remove vs Error for Dead Pipeline Fields

**Positions**:
- Systems Architect, Pragmatic Enforcer: Remove dead fields entirely (clean break)
- Domain Expert: Add migration errors first, remove later (user-friendly)

**Resolution**: H1 + H3 specifies both: remove fields AND emit clear compiler errors. This gives users migration guidance while cleaning the codebase. The `path`/`pathRotate` fields had no consumers even pre-ADR-032, so removing them is safe.

### Trade-off 3: Flutter Multi-Property Keyframe Fix Scope

**Positions**:
- Rust Expert: Conservative fix — always return "0.0"/"1.0" for multi-property stops
- Systems Architect: Full fix — per-property `Tween<double>` instances

**Resolution**: Conservative fix (H2 with property name parameter) is recommended for immediate. The full per-property tween approach is deferred (L-effort, tracked as Rust Expert long-term item #4).

---

## Risks If Unaddressed

1. **Easing correctness (P0)**: Users writing `ease: backIn` or `ease: elasticInOut` get incorrect animation curves on all backends. This is a silent behavioral error with no warning.
2. **Build reproducibility (H2)**: Flutter codegen output varies non-deterministically between compilation runs for multi-property keyframes. Makes CI caching unreliable and debugging Flutter codegen issues difficult.
3. **User trust erosion (H1, H3)**: Silent data loss for `@drag-scrub`, `@drag-momentum`, `path:`, `@hover-reverse` — users write valid-looking syntax with zero visual effect and zero diagnostic. Contradicts ADR-032's "clear compiler errors" validation criterion.
4. **Codebase entropy (M1-M12)**: Each dead code remnant increases cognitive load for new contributors and creates false impressions of supported features. The ~200 lines of dead code represent ~15% of the post-scoping code surface.
5. **Memory growth (M7, M9)**: Long-lived pages with dynamic content accumulate entries in `__omni_scrollElements` and `__omni_inview_observers`. Realistic maximum is bounded (~100KB for typical pages), but the design should be clean.

---

## Success Metrics

| Metric | Pre-ADR-032 | Post-Scoping (Current) | Target (After This Review) |
|--------|-------------|----------------------|---------------------------|
| Cross-backend feature gaps | 60% | 0% | 0% ✅ |
| Dead code lines in motion subsystem | ~1,100 (removed features) | ~200 (vestigial) | ~0 |
| Easing correctness | Bounce values identical | 4 directional variants wrong | All correct |
| Flutter codegen determinism | Non-deterministic for multi-prop | Non-deterministic for multi-prop | Deterministic |
| Silent feature loss paths | 21 features | ~6 paths (drag-scrub, hover-reverse, hero, path, etc.) | 0 (all produce errors) |
| Migration guidance for removed features | None | "Unexpected token" for some | Clear error messages for all |
| Rust tests | 837 pass | 837 pass | 837 pass (no regression) |
| JS tests | 18 pass | 18 pass | 18 pass (no regression) |

---

## Follow-up Plan

1. **Immediate** (today): Fix P0 (easing direction normalization) — 15 min, 1 file
2. **This session**: Execute H1-H5 (5 items, all S effort, ~2 hours total)
3. **Next session**: Execute M1-M12 (12 items, all S effort, ~3 hours total)
4. **Backlog**: L1-L8 (deferred improvements, mix of S/M effort)
5. **ADR**: ADR-032 remains Accepted — no architectural decision change needed, only cleanup of implementation artifacts

---

## Appendices

### A. Review Methodology

10 architecture team members independently reviewed the post-ADR-032 animation subsystem from their specialist perspectives. Each expert read current code, ADRs, and memory files. Aggregated with cross-cut analysis (themes appearing in 3+ reviews) and conflict resolution (explicit trade-off documentation).

### B. Referenced Documents

- `ADR-032-omniview-animation-design-scoping.md` — Design scoping decision (Accepted)
- `ADR-022-canonical-easing-table.md` — Canonical easing table
- `ADR-024-react-motion-useeffect-lifecycle.md` — React motion useEffect lifecycle
- `ADR-021-cross-target-feature-parity.md` — Cross-target parity
- `.architecture/reviews/component-omniview-animation.md` — Prior review (2026-07-29)
- `memory/omnilang-animation.md` — @omni/animation architecture
- `memory/omniview-animation-review-2026-07-29.md` — Prior review summary
- `memory/session-2026-07-29-animation-scoping.md` — ADR-032 implementation session

### C. Key Files Inventory (Current)

| File | Lines | Role |
|------|-------|------|
| `crates/omnilang-core/src/parser/ast/motion.rs` | 147 | AST types |
| `crates/omnilang-core/src/ir/types_motion.rs` | 274 | IR types |
| `crates/omnilang-core/src/codegen/shared/easing.rs` | 73 | Canonical easing table |
| `crates/omnilang-core/src/ir/lower/motion.rs` | ~250 | AST → IR lowering |
| `crates/omnilang-core/src/semantic/motion.rs` | ~150 | Semantic validation |
| `omnilang/src/codegen/omniview/motion_triggers.rs` | 529 | OmniView trigger JS emission |
| `omnilang/src/codegen/omniview/motion_config.rs` | ~200 | Keyframe → JS config |
| `omnilang/src/codegen/react/motion/` | 4 files | React codegen |
| `omnilang/src/codegen/flutter/motion.rs` | ~250 | Flutter codegen |
| `packages/omni-animation/src/` | 7 files | JS runtime |
