# Architecture Review v2: OmniView High-Frequency Element Manipulation

**Date**: 2026-07-30
**Review Type**: Component — @ref Element Reference System + High-Frequency DOM Architecture (Follow-up)
**Expert Panel**: 6 specialists (Compiler Architect, TS/React Specialist, Performance Specialist, Maintainability Expert, Pragmatic Enforcer, Rust Expert)
**Previous Review**: `.architecture/reviews/component-omniview-high-freq-dom.md` (2026-07-30) — all P0/P1 fixes applied

---

## Executive Summary

**Overall Assessment: Adequate → Strong (Improved from "Needs Improvement")**

The @ref element reference system has advanced significantly since the v1 review. All P0 bugs from the prior review are fixed: `ref_names` is threaded through all React expression paths (was dead code producing `ReferenceError`), FLIP animations work correctly, @for clones have unique nids, `ref_nid_map` has uniqueness guards, and unnecessary clones are eliminated. The system compiles correctly end-to-end on both OmniView and React backends.

**However, this review uncovered one new correctness bug (stale `__omni_el` cache) and one persistent critical gap (@ref inside @for with no diagnostic).** The stale cache bug is significant: `__omni_el` returns detached DOM elements indefinitely and never evicts stale entries, causing both correctness issues (manipulating dead elements) and memory leaks in long-running SPAs. Additionally, 5 of 6 specialists flagged the @for+@ref gap — it will be the first thing real developers encounter, and the compiler silently produces incorrect output.

**Pragmatic Enforcer Assessment**: Necessity 6/10, Complexity 5/10, Ratio 0.83 — well below 1.5 threshold. The feature is worth its complexity cost. Phase 1 scope containment remains excellent.

---

## Cross-Cutting Themes

### Theme 1: @ref Inside @for — The #1 Undefined Behavior (5/6 Specialists)

Every specialist who examined this independently reached the same conclusion: `@ref` inside `@for` is the most impactful gap. The compiler silently accepts it and produces incorrect output — the ref binds to the template element (OmniView) or the last clone (React), not the per-item element the developer expects.

| Specialist | Severity | Key Insight |
|-----------|----------|-------------|
| Compiler Architect | **HIGH** | Silent wrong-element manipulation at 60fps; test only verifies "doesn't crash" |
| Pragmatic Enforcer | **HIGH** | "This will be the FIRST thing real developers try. The compiler accepts it silently." |
| TS/React Specialist | **MEDIUM** | In React, `ref={boxRef}` inside `.map()` gives last element — known anti-pattern |
| Rust Expert | **LOW** | No semantic guard; `debug_assert!` would crash in debug mode for duplicate ref names |
| Maintainability Expert | **LOW** | Missing negative test for "ref not in @for" invariant |

**Resolution**: Add a compiler diagnostic (Warning) when `ref_binding` is detected inside a `ListRendering` subtree. Promote from deferred Phase 2 to immediate P0.

### Theme 2: scrollTo/scrollBy — Cross-Backend API Inconsistency (3/6 Specialists)

ADR-036 and the design document specify `scrollTo(x, y)` and `scrollBy(dx, dy)` as part of the cross-backend ref API. They work on React (raw DOM element via `useRef.current`) but are not emitted in the OmniView IIFE wrapper — producing `TypeError: box.scrollTo is not a function` on the debug runtime.

| Specialist | Severity | Key Insight |
|-----------|----------|-------------|
| Compiler Architect | **HIGH** | "Works on React but crashes on OmniView" — cross-backend inconsistency |
| Pragmatic Enforcer | **MEDIUM** | "4-line fix. Documentation gap erodes trust." |
| Performance Specialist | — | Confirmed as documentation debt, not performance concern |

**Resolution**: Add 4 lines to `emit_ref_wrappers()`. Trivial fix, disproportionate impact on developer trust.

### Theme 3: Stale `__omni_el` Cache — New Correctness Bug (Performance Specialist)

**This is a NEW finding not identified in any prior review.** The `__omni_el` runtime function (`runtime.rs:238-246`) uses a plain object cache that never evicts entries. When an element is removed from the DOM:

1. The cache still holds a reference to the detached DOM node
2. `querySelector` fails to find a replacement → cache entry is NOT updated
3. All subsequent calls return the *detached* (dead) element
4. The detached DOM node is never garbage collected

```javascript
// Current (buggy):
var el = cache[nid];
if (!el || !el.isConnected) {
  el = document.querySelector('[data-nid="' + nid + '"]');
  if (el) cache[nid] = el;   // BUG: no else branch to delete stale entry
}
return el;  // Returns detached element when querySelector fails
```

This affects ALL code paths using `__omni_el`: the @ref IIFE wrapper, style effects, text bindings, attribute bindings.

**Resolution**: Add `else { delete cache[nid]; return null; }` — 2 lines, zero runtime overhead.

### Theme 4: Dead Code and DRY Violations (3/6 Specialists)

Multiple specialists identified code quality issues that, while individually minor, create maintenance drag:

| Finding | Specialists | Files |
|---------|------------|-------|
| `ReactStmtFormatter.ref_names` dead field | TS/React, Rust, Maintainability | `expr.rs:141` |
| Two-path expression formatting divergence | Pragmatic, Maintainability | `expr.rs:434 vs :33` |
| Triple `_elCache` template in list.rs | Rust, Maintainability | `list.rs:281,322,352` |
| Triple `__omni_effect` wrapper in style.rs | Maintainability | `style.rs:47,80,124` |
| Duplicate `__omni_buildNidMap` in editor.rs | Maintainability | `editor.rs:27,47` |
| `///` in generated JS (should be `//`) | Rust | `store.rs:595`, `mod.rs:265` |

---

## Trade-offs and Disagreements

### Trade-off 1: Hardcoded HTMLDivElement vs YAGNI

**TS/React Specialist**: HIGH — Every non-`<div>` @ref produces TypeScript type errors. `useRef<HTMLDivElement>(null)` is wrong for `<input>`, `<video>`, `<canvas>`, `<svg>`.

**Pragmatic Enforcer**: This was rated P2-1 (deferred). The fix requires adding element tag tracking to `RefDeclIR` + a type mapping table. Necessity is real but the fix touches IR types.

**Resolution**: Promote to P1. The fix is bounded (~2 hours) and the blast radius is every non-div element. Users cannot use `@ref` with `<input @ref={inputRef}>` without manually type-casting the generated code.

### Trade-off 2: debug_assert! vs assert! in push_ref_nid

**Rust Expert**: `debug_assert!` in `push_ref_nid` means duplicate ref bindings would silently produce broken JS in release mode. The semantic validator emits a Warning, not an Error — so this is a real path in release builds.

**Pragmatic Enforcer**: If the semantic validator catches it as a Warning, the codegen should be robust to receiving duplicates.

**Resolution**: Change to `assert!`. The 1-character change eliminates a silent-corruption path. Alternatively, make it a graceful skip with a logged warning — but `assert!` is simpler and more consistent with the codebase's "semantic validator must catch this" invariant.

### Trade-off 3: Signal Batching — Still Deferred

**Performance Specialist**: Per-property style effects (4 properties = 4 independent `__omni_effect` closures) is a higher-value optimization than signal batching. Consolidate effects per-element first.

**Pragmatic Enforcer**: ADR-037 gate conditions are correct. No real app profiling data exists.

**Resolution**: Maintain ADR-037 deferral. Prioritize per-element style effect consolidation (P1-3 from v1 review) before signal batching.

---

## Prioritized Recommendations

### Critical (this sprint)

| # | Finding | Severity | Action | Effort |
|---|---------|----------|--------|--------|
| **P0-1** | `__omni_el` cache returns stale detached elements — correctness bug + memory leak | **CRITICAL** | Add `else { delete cache[nid]; return null; }` at `runtime.rs:243`. Also add `new Map()` with size limit for cache eviction. | **XS** (2 lines + 5 lines) |
| **P0-2** | `@ref` inside `@for` silently produces incorrect output — no compiler diagnostic | **HIGH** | Add Warning diagnostic in `validate_refs.rs` when ref binding found inside `ListRendering` subtree. | **XS** (~20 lines) |
| **P0-3** | `scrollTo`/`scrollBy` missing from OmniView wrapper — cross-backend crash | **HIGH** | Add 4 template lines to `emit_ref_wrappers()` in `store.rs`. | **XS** (4 lines, 5 min) |

### High Priority (next session)

| # | Finding | Severity | Action | Effort |
|---|---------|----------|--------|--------|
| **P1-1** | `HTMLDivElement` hardcoded for all `useRef` types | **HIGH** | Add `element_tag: Option<String>` to `RefDeclIR`, populate during lowering, emit correct TypeScript type in `gen_ref_hooks()`. | **M** (2h) |
| **P1-2** | `ReactStmtFormatter.ref_names` dead field + `#[allow(dead_code)]` | **HIGH** | Remove field and its clone construction. | **XS** (5 min) |
| **P1-3** | Dual formatting path divergence (`format_expr_ir_tsx` vs `ReactFormatExpr.format_identifier`) | **MEDIUM** | Delegate `IdentifierExpr` branch to `ReactFormatExpr.format_identifier()` — eliminate special case. | **XS** (15 min) |
| **P1-4** | Per-property style effects — N independent `__omni_effect` per element | **MEDIUM** | Consolidate per-element style effects into single `__omni_effect` call in `style.rs`. | **M** (~50 lines) |
| **P1-5** | `debug_assert!` → `assert!` in `push_ref_nid` | **MEDIUM** | Change to hard assert — prevent silent JS corruption in release builds. | **XS** (1 character) |
| **P1-6** | `///` → `//` in generated JS comments | **MEDIUM** | Fix 3 occurrences in `store.rs:595,600` and `mod.rs:265`. | **XS** (5 min) |
| **P1-7** | No `css_only` guard on `emit_ref_wrappers` | **MEDIUM** | Add `if self.css_only { return; }` at top. | **XS** (1 line) |

### Medium Priority (this week)

| # | Finding | Severity | Action | Effort |
|---|---------|----------|--------|--------|
| **P2-1** | Missing test coverage: ref in effects, computeds, lifecycles, inline handlers | **MEDIUM** | Add 5-7 test cases across React and OmniView backends. | **M** (2-4h) |
| **P2-2** | Component ref forwarding silently drops binding — no diagnostic | **MEDIUM** | Add Warning when `@ref` used on custom component tag. | **XS** (~10 lines) |
| **P2-3** | Triple `_elCache` template in `list.rs` | **MEDIUM** | Extract to shared `_elCacheGet` helper. | **S** (1-2h) |
| **P2-4** | Triple `__omni_effect` wrapper in `style.rs` | **MEDIUM** | Extract `open_style_effect`/`close_style_effect` helpers. | **S** (30 min) |
| **P2-5** | Duplicate `__omni_buildNidMap` in `editor.rs` | **MEDIUM** | Extract to single definition before debug_mode guard. | **S** (30 min) |

### Nice-to-Have (next 2 weeks)

| # | Finding | Severity | Action | Effort |
|---|---------|----------|--------|--------|
| **P3-1** | `gen_view_node_tsx` 8-parameter function (800 lines) | **LOW** | Extract `ViewContext` struct to collapse parameters. | **M** (2-3h) |
| **P3-2** | Tree traversal duplication in `state.rs` (6 recursive functions) | **LOW** | Extract `walk_view_tree` helper or visitor trait. | **L** (1-2 days) |
| **P3-3** | `FxHashSet<String>` over-cloning in statement formatting | **LOW** | Share via `Rc` or restructure `ReactStmtFormatter` construction. | **S** (30 min) |
| **P3-4** | Document signal-vs-@ref performance boundary for developers | **LOW** | Add section to @ref language documentation. | **S** (docs only) |

### Deferred (Phase 2, gated)

| # | Finding | Trigger | Action | Effort |
|---|---------|---------|--------|--------|
| **D-1** | `@for` + `@ref` semantics (per-item ref access) | After P0-2 diagnostic is in place | Design per-item ref pattern (ref callback, Map<key, Element>, or @ref:name on @for) | **L** (1-2 days) |
| **D-2** | Component ref forwarding (`forwardRef`) | When library components need to expose DOM nodes | Implement across React and OmniView backends | **M** (4-8h) |
| **D-3** | Signal batching (microtask-based) | When profiling data shows 60fps frame drops (ADR-037 gate) | Implement microtask queue + codegen call sites | **M** (1-2 days) |
| **D-4** | @for keyed DOM reuse (not wipe-and-rebuild) | When 500+ item lists cause visible jank in OmniView debug | Implement keyed reconciliation in @for runtime | **XL** (runtime + codegen) |
| **D-5** | Flutter GlobalKey codegen | When Flutter backend is activated | Implement Flutter ref codegen | **M** (1 day) |

---

## Risks if Unaddressed

1. **Stale `__omni_el` cache (P0-1)**: In a long-running OmniView SPA with dynamic content (tabs, modals, @for lists), detached DOM nodes accumulate in `__omni_el._cache`. The cache never shrinks, holding references to dead elements that can't be garbage collected. Every ref lookup that hits a stale cache entry manipulates a detached element — mutations silently have no visual effect, reads return stale values. **This is the highest-risk finding of this review.**

2. **@ref in @for without diagnostic (P0-2)**: The most natural thing a developer writes. The compiler accepts it. The generated code looks correct. At runtime, the ref manipulates the wrong element (template or last clone). At 60fps in a drag handler, this produces confusing visual glitches. **This will be the #1 bug report when developers adopt @ref.**

3. **scrollTo/scrollBy crash (P0-3)**: A feature documented in ADR-036 fails at runtime on the debug target. Developer writes `box.scrollTo(0, 100)`, gets `TypeError`, loses trust in the framework. **4 lines of code to fix, disproportionate trust damage.**

4. **HTMLDivElement type errors (P1-1)**: Every non-`<div>` @ref produces a TypeScript compilation error. `<input @ref={inputRef}>` → `Property 'value' does not exist on type 'HTMLDivElement'`. **Users cannot use @ref with form elements, media elements, or canvas without type-casting.**

5. **Dead code inertia**: `ReactStmtFormatter.ref_names` with `#[allow(dead_code)]` normalizes ignoring compiler warnings. Future developers will follow the pattern and add more dead fields.

6. **Template drift**: Triple `_elCache`, triple `__omni_effect`, and duplicate `__omni_buildNidMap` templates will silently diverge when fixes are applied to only one copy.

---

## Individual Member Reviews

### Compiler & Language Architect (Dr. Lin Wei)

**Summary**: The IR design (`ref_binding: Option<String>` + `ComponentIR.refs`) is minimal and sufficient for Phase 1. The expression translation pipeline is thorough — `ref_names` propagates through all 8 code paths in React codegen. However, two HIGH concerns: (1) `@ref` inside `@for` produces incorrect output silently — this is the most significant correctness gap, and (2) `scrollTo`/`scrollBy` specified in ADR-036 but not emitted in OmniView wrapper — cross-backend crash. MEDIUM: component ref silently dropped on custom components, no `css_only` guard.

**Severity findings**: HIGH×2 (for+ref silent failure, scrollTo/scrollBy cross-backend gap), MEDIUM×2 (component ref silent drop, css_only guard missing), LOW×2 (ref_names vs bindings, null return semantics).

---

### TypeScript & React Specialist (Emma Rodriguez)

**Summary**: The `useRef` codegen is architecturally sound: reconciliation bypass works correctly, hook ordering is deterministic, no stale closures for mixed ref+signal access, and ref_names threading is confirmed complete in all 9 call sites. However, the hardcoded `HTMLDivElement` is the single blocking issue — users cannot use `@ref` with non-div elements in TypeScript strict mode. The dead `ReactStmtFormatter.ref_names` field and missing test coverage for effects/computeds/lifecycles are the next priorities.

**Severity findings**: HIGH×2 (HTMLDivElement hardcoded, dead ref_names field), MEDIUM×3 (test coverage gaps, @for+@ref undefined, component forwarding silent drop), LOW×2 (store actions can't access refs, stale closure with debounce).

---

### Performance Specialist

**Summary**: The @ref path is clean and meets 60fps requirements — a drag frame costs ~3 function calls and 1 string allocation with zero framework overhead. However, the `__omni_el` cache has a significant bug: when an element is removed from the DOM, the cache returns a stale detached reference forever, never evicting the entry. This affects ALL code paths using `__omni_el`. Additionally, per-property style effects create N independent closures per element (4 animated properties = 4 effect closures = 4 DOM writes per frame instead of 1), and @for uses full wipe-and-rebuild even with keys.

**Severity findings**: MEDIUM/P0 (stale element cache — correctness + memory), MEDIUM×2 (per-property effects, @for wipe-and-rebuild), LOW×3 (documentation gap, IIFE method overhead, signal batching deferral correct).

---

### Maintainability Expert

**Summary**: The `ref_names` threading is surprisingly complete across 10+ function call sites. However, multiple DRY violations persist from the v1 review: triple `_elCache` template in list.rs, triple `__omni_effect` wrapper in style.rs, and duplicate `__omni_buildNidMap` in editor.rs. The dead `ReactStmtFormatter.ref_names` field normalizes ignoring compiler warnings. The two-path expression formatting (free function + trait method) is the highest latent risk — they handle `$` prefix differently and could silently diverge.

**Severity findings**: MEDIUM×3 (_elCache triplication, __omni_effect triplication, __omni_buildNidMap duplication), LOW×4 (dead field, two-path divergence, test gaps, 8-param function).

---

### Pragmatic Enforcer (YAGNI Guardian)

**Summary**: The @ref system's 0.83 Complexity/Necessity ratio is well below the 1.5 threshold — the feature is worth doing. Phase 1 scope containment is excellent (Flutter and forwardRef correctly deferred). However, three immediate actions are needed: (1) add @for+@ref diagnostic (promote from deferred to P0 — this will be the #1 thing developers encounter), (2) add scrollTo/scrollBy to close doc-implementation gap, (3) unify dual formatting paths. The IIFE wrapper API surface with 6 zero-consumer methods is fine — 12 lines of code is negligible insurance.

**Necessity**: 6/10 (genuinely needed for React, zero application validation). **Complexity**: 5/10 (250 lines production, clean IR). **Ratio**: 0.83. **Verdict**: Approve with conditions.

---

### Rust Expert (Katya Volkov)

**Summary**: The semantic validation architecture is clean — `validate_refs()` runs before codegen and catches all three error conditions. Component-level isolation is correct (ref_nid_map cleared per-component). However, several Rust-specific concerns: (1) `debug_assert!` in `push_ref_nid` should be `assert!` — in release mode, a suppressed semantic warning leads to silently invalid JS, (2) `ReactStmtFormatter.ref_names` is dead code masked by `#[allow(dead_code)]` — wastes a `FxHashSet<String>` clone per statement, (3) `///` in generated JS should be `//`, (4) `FxHashSet<String>` is over-cloned in statement formatting (2 clones where 1 would suffice). The six recursive view tree traversal functions in `state.rs` are a long-term DRY concern.

**Severity findings**: MEDIUM×4 (/// in JS, debug_assert!→assert!, dead ref_names field, FxHashSet over-cloning), LOW×6 (ref name escaping, unguarded ref mismatch, tree traversal duplication, _elCache duplication, ref-in-@for test gap, source_file.clone()).

---

## Success Metrics and Follow-up

**Pre-merge checks for P0 fixes**:
- All workspace tests pass with zero regressions
- New test: `__omni_el` returns `null` for removed elements (not stale reference)
- New test: compiler emits Warning for `@ref` inside `@for`
- New test: `box.scrollTo(x, y)` works in OmniView debug runtime
- Clippy clean, rustfmt clean

**Follow-up plan**:
1. Execute P0-1 through P0-3 (estimated 30 minutes total)
2. Execute P1-1 through P1-7 (estimated 4-6 hours total)
3. Execute P2-1 through P2-5 (estimated 4-8 hours total)
4. Update ADR-036 API table to confirm scrollTo/scrollBy implementation
5. Schedule specialist-review for deferred items (D-1 through D-5) when their gate conditions are met

---

## References

- ADR-036: Element Reference System
- ADR-037: Signal Batching Deferral
- ADR-028: OmniView Debug Runtime Trade-offs (D3: synchronous subscriber notification)
- ADR-034: ComponentScope Inlining Isolation
- Prior review: `.architecture/reviews/component-omniview-high-freq-dom.md` (2026-07-30)
- Memory: `omniview-review-high-freq-dom-2026-07-29`
- Memory: `session-2026-07-30-high-freq-dom-fixes`
- Memory: `session-2026-07-29-element-ref-system`
- Memory: `adr-037-signal-batching`
