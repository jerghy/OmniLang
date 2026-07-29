# Architecture Review: OmniView High-Frequency Element Manipulation

**Date**: 2026-07-29
**Review target**: OmniView's architecture for high-frequency DOM element manipulation — the equivalent of React's `useRef` vs `setState` distinction for performance-critical element access
**Reviewers**: Compiler Architect (Dr. Lin Wei), TypeScript & React Specialist (Emma Rodriguez), Performance Specialist, Maintainability Expert, Pragmatic Enforcer, Rust Expert (Katya Volkov)
**Status**: Complete

---

## Executive Summary

OmniView's high-frequency element manipulation architecture is **fundamentally sound but has 3 confirmed correctness bugs that need fixing**. The debug runtime (OmniView) already has a well-designed element access system (`__omni_el()` with caching + direct DOM manipulation in reactive effects), but this system breaks in two specific scenarios: (1) `@for` list items with motion triggers — only the first item gets animated, and (2) FLIP animations silently do nothing because the DOM clear loop runs before position capture. The React production codegen has a related but distinct gap: conditionally rendered elements with motion triggers are never discovered by the mount-only `useEffect` + `querySelectorAll` pattern.

**The key architectural question — whether to add `useRef` to the React codegen — has a clear resolution from the multi-perspective review**: do NOT add `useRef` now. The `querySelectorAll` + `useEffect([])` pattern is correct for all current static-element use cases. Adding `useRef` would create IR-level changes, ref-forwarding complexity, and cross-backend asymmetry with zero measurable benefit. If a concrete performance issue emerges, a `MutationObserver` approach (as recommended by the Compiler Architect) is more minimal and targeted than a full `useRef` migration.

**Overall Assessment**: Adequate — 3 real bugs to fix, clean architecture for the debug runtime role, pragmatic deferral of `useRef` until needed.

---

## Cross-Cutting Themes

### Theme 1: Cross-backend asymmetry in element access mechanisms (5/6 reviewers)

Every reviewer identified this, though with different severity assessments:

- **OmniView**: `data-nid` + `__omni_el()` with caching + direct DOM manipulation in `__omni_effect`
- **React**: `querySelectorAll('[data-motion-*]')` in `useEffect([])` — no caching, mount-only
- **Flutter**: Widget tree identity — no DOM concept at all

The asymmetry is intentional (ADR-028: OmniView is a debug runtime, ADR-021: cross-target parity matters for features, not mechanisms). However, the Maintainability Expert notes that **no cross-backend translation guide exists** — when a new element-level feature is added, there's no documented mapping from OmniView's `__omni_el()` approach to React's declarative approach, creating the same class of bugs ADR-021 caught reactively.

**Recommendation**: Create a pattern mapping table in `codegen/mod.rs` documenting how each element-manipulation pattern maps across backends (Maintainability Expert, MEDIUM, M effort).

### Theme 2: @for list performance and correctness (3/6 reviewers)

The `@for` list rendering path is the single most performance-critical code path and contains the most significant bugs:

- **FLIP is dead code**: `Omni.flipBefore(parent)` runs AFTER the DOM clear loop, so it always captures zero elements (Performance Specialist, HIGH)
- **Motion triggers only affect the first item**: `__omni_el(nid)` uses `querySelector` which returns the first match — all cloned items share the same `data-nid`, so only item 0 gets animated (Compiler Architect, HIGH-2)
- **_elCache rebuilt per item per render**: Even for key-matched (recycled) items, the entire `Map` is rebuilt with `querySelectorAll` scans on every re-render (Performance Specialist, MEDIUM)
- **Cache strategy proliferation**: 3 distinct caching strategies without documented rationale (Maintainability Expert, LOW)

### Theme 3: Code duplication in the codegen pipeline (2/6 reviewers)

- **Triplicated `_elCache` lookup** in `list.rs` lines 256/297/327 — identical element-lookup JS copy-pasted for text, class, and attribute bindings (Maintainability Expert, MEDIUM)
- **Triplicated `__omni_effect` wrapper** in `view/style.rs` — 3 match arms emit the same effect preamble (Maintainability Expert, MEDIUM)
- **Duplicated `__omni_buildNidMap()`** in `editor.rs` and `debug.rs` (Maintainability Expert, MEDIUM)
- **78 instances of `push_str(&format!(...))`** — temporary String allocations (Performance Specialist + Rust Expert, LOW)
- **Wasteful clone in `try_inline_component`** — `el.children.clone()` before checking if slots exist (Rust Expert, HIGH on efficiency)

---

## Trade-offs and Disagreements

### Disagreement: Should `useRef` be added to React codegen?

| Reviewer | Position | Rationale |
|----------|----------|-----------|
| **React Specialist** (Emma) | YES (MEDIUM) | querySelectorAll is fragile for dynamic elements; useRef is the React-idiomatic way to get stable element references |
| **Compiler Architect** (Lin Wei) | PARTIAL YES (HIGH-1 on the underlying problem) | Dynamic elements not discovered is a real bug, but `MutationObserver` is simpler than full useRef migration |
| **Pragmatic Enforcer** | STRONG NO | No consumer needs it; adds IR types, ref-forwarding, import changes with zero measurable benefit; classic YAGNI |
| **Performance Specialist** | Silent | Focused on FLIP, _elCache, allocations — not on useRef |
| **Maintainability Expert** | Silent | Focused on duplication, not on useRef |
| **Rust Expert** | Silent | Focused on Rust-side allocation patterns |

**Resolution**: Do NOT add `useRef` to React codegen now.

The Compiler Architect's `MutationObserver` approach is the pragmatic middle ground: it fixes the real bug (dynamic elements with motion triggers not discovered) without adding the architectural complexity of a full `useRef` system. If the `MutationObserver` approach proves insufficient for future use cases, `useRef` can be revisited — but only when a concrete consumer demonstrates the need.

**Key insight**: The React Specialist's concern about `querySelectorAll` being "fragile" applies primarily to dynamically rendered content (conditional blocks, @for items). For static elements (the current common case), `querySelectorAll` in `useEffect([])` is correct and sufficient. The Compiler Architect correctly identifies that the issue is about element *discovery*, not element *reference stability* — and `MutationObserver` addresses discovery without requiring refs.

### Trade-off: OmniView element caching complexity vs debug runtime simplicity

The Pragmatic Enforcer and Rust Expert both concluded that the OmniView debug runtime is **not over-engineered** — the ~220-line inline runtime (signals, computed, effects, element lookup, error reporting, sanitizers) is appropriately scoped for a debug tool. The 14 intentional trade-offs in ADR-028 (no signal batching, no minification, string-based codegen, etc.) are well-documented and should not be revisited.

---

## Prioritized Recommendations

### HIGH — Fix this sprint (correctness bugs)

| # | Finding | Location | Fix | Effort | Source |
|---|---------|----------|-----|--------|--------|
| H1 | @for + motion triggers broken — only first list item gets animated | `list.rs`, `motion_triggers.rs`, `view/element.rs:716-718` | Collect motion bindings per-clone like text/class/attr bindings; add `for_motion_bindings` to ComponentScope; use `_elCache.get(nid)` instead of `__omni_el(nid)` in clone context | S-M | Compiler Architect |
| H2 | FLIP animations non-functional — flipBefore runs after DOM clear | `list.rs:117-122` (clear loop) vs `list.rs:187-189` (flipBefore) | Reorder JS emission: emit `Omni.flipBefore(parent)` BEFORE the while-loop that removes old items | S | Performance Specialist |
| H3 | Wasteful `el.children.clone()` in `try_inline_component` before slot check | `component_html.rs:122` | Check `has_any_slot_attr(el)` before cloning; only clone when slots are present | S | Rust Expert |

### IMPORTANT — Fix next sprint (functional gaps + cleanups)

| # | Finding | Location | Fix | Effort | Source |
|---|---------|----------|-----|--------|--------|
| I1 | React motion effects don't discover dynamically added elements | `react/motion/mod.rs:68-83`, `react/motion/transforms.rs:18-56,75-161,194-333` | Add `MutationObserver` in the mount effect to detect new elements with `data-motion-*` attributes — minimal, targeted fix | S-M | Compiler Architect |
| I2 | `_elCache` rebuilt per item per render even for key-matched items | `list.rs:237-247` | Persist `_elCache` on the element (`clone.__cache = _elCache`) for key-matched items; reuse on recycle | S | Performance Specialist |
| I3 | Triplicated `_elCache` element-lookup code | `list.rs:256,297,327` | Extract `gen_el_cache_lookup(nid)` helper method — 3× copy-paste → 1 method | S | Maintainability Expert |
| I4 | Create cross-backend element-manipulation translation guide | `codegen/mod.rs` (new doc comment) | Table mapping OmniView patterns → React → Flutter equivalents to prevent future ADR-021 bugs | M | Maintainability Expert |
| I5 | Triplicated `__omni_effect` wrapper in dynamic style emission | `view/style.rs:47-54,80-88,125-136` | Extract `emit_dynamic_style_effect(nid, prop, value_expr)` helper | S | Maintainability Expert |
| I6 | Duplicated `__omni_buildNidMap()` in editor.rs and debug.rs | `editor.rs:48-57`, `debug.rs:27-37` | Emit from single location; gate extra registrations on `!debug_mode` | S | Maintainability Expert |

### NICE-TO-HAVE — Defer (optimizations + documentation)

| # | Finding | Location | Fix | Effort | Source |
|---|---------|----------|-----|--------|--------|
| N1 | 78 instances of `push_str(&format!(...))` | 10 files in `codegen/omniview/` | Mechanical conversion to `write!(self.js, ...)` | S (mechanical) | Performance + Rust |
| N2 | innerHTML 3-pass string escaping | `view/mod.rs:93-96` | Single-pass char iterator | S | Performance |
| N3 | `inject_slot_children` clones view tree even without slot replacement | `component_html.rs:128-131` | Operate on `Cow<ViewTree>`; only clone when slot found | M | Rust |
| N4 | `view_to_html` returns owned Strings → intermediate allocations | `view/mod.rs:142` | Accept `&mut String` out parameter | L (touches 8+ methods) | Rust |
| N5 | Global registries (`__omni_scrollElements`, `__omni_inview_observers`) lack GC sweep | `runtime.rs:133-134,276-284` | Extend `__omni_cleanupElement` to component teardown path | S | Pragmatic |
| N6 | Document the mount-time DOM coupling constraint for React motion | `react/motion/mod.rs:68` | Add comment: "Elements must exist in initial render; use key-based re-mount for dynamic content" | S | Pragmatic |
| N7 | Document rationale for 3 element caching strategies | `runtime.rs` near __omni_el | Comments explaining global cache vs scoped Map vs direct querySelector | S | Maintainability |
| N8 | Add `useRef` to React codegen | Multiple files in `react/` | **DEFERRED** — evaluate only when `MutationObserver` proves insufficient or a concrete consumer demonstrates need | L | React Specialist (deferred per Pragmatic resolution) |

---

## Individual Member Reviews

### Compiler & Language Architect (Dr. Lin Wei)

> Full text in agent output at `ae2a0f6e6e853ae58` (Rust Expert) and `a898861e8869f5fe2` (Compiler Architect).

**Perspective**: Evaluated the compilation pipeline from `.omni` source through IR to multi-target codegen, assessing how element identity and access patterns are handled across targets.

**Key findings**:
- **HIGH-1**: React motion effects (`useEffect([], querySelectorAll)`) never discover dynamically rendered elements — conditionally rendered `@enter`/`@hover`/`@press` elements silently lose animation in React production builds
- **HIGH-2**: `@for` list items with motion triggers only animate the first item — `__omni_el(nid)` uses `querySelector` which returns DOM-first-match; cloned items share `data-nid` but `_elCache` is not used for motion setup
- **MEDIUM-1**: React `@for` lists lack enter/leave/FLIP animations — OmniView has them but IR doesn't encode list-level animation intent

**Strengths noted**: 6-phase JS execution order correctly enforced via buffer swap, `_elCache: Map` design is correct for per-clone access, key-based reconciliation in @for is well-structured.

### TypeScript & React Specialist (Emma Rodriguez)

> Full text in agent output at `a910a197d594d30b6`.

**Perspective**: Evaluated React codegen as a production frontend framework, analyzing the gap between OmniView's `__omni_el()` pattern and React's declarative model.

**Key findings**:
- Confirmed zero `useRef` instances in entire React codegen
- All motion effects use `document.querySelectorAll('[data-motion-*]')` in `useEffect([], ...)` — mount-only, no cache
- Every signal mutation triggers full React reconciliation via `useState`
- No language-level mechanism for developers to express "this element needs a stable reference"

**Recommendation**: Add `useRef` path for elements with motion triggers (MEDIUM), with step-by-step plan from XS import change to L full optimization. *[Note: recommendation deferred per cross-review resolution]*

**Strengths noted**: Correct hook dependency tracking, clean two-effect split per ADR-024, proper event cleanup for scroll/drag, Zustand alternative for global state.

### Performance Specialist

> Full text in agent output at `a75b943d782b2abec`.

**Perspective**: Analyzed runtime performance characteristics of innerHTML-based rendering, signal dispatch, @for re-render cycles, and FLIP animation pipeline.

**Key findings**:
- **HIGH**: FLIP `before`/`after` captures empty DOM — clear loop runs before `Omni.flipBefore(parent)`, so FLIP always sees zero elements
- **MEDIUM**: `_elCache` Map rebuilt from scratch per item per render — O(S×N) DOM queries wasted on recycled items
- **LOW**: innerHTML 3-pass escaping (3× allocation), 78× `push_str(&format!(...))` temporary allocations

**Strengths noted**: Pre-compiled bindings (zero `new Function`), `_elCache` for O(1) lookup, delegated events, shared scroll handler, template-based cloning, conditional motion runtime inclusion.

### Maintainability Expert

> Full text in agent output at `a83ea8d15fad0b0d7`.

**Perspective**: Evaluated code duplication, pattern consistency, architectural drift, and cross-backend maintainability of element access patterns.

**Key findings**:
- **MEDIUM ×3**: Triplicated `_elCache` lookup code in list.rs (256/297/327), triplicated `__omni_effect` wrapper in style.rs, duplicated `__omni_buildNidMap()` in editor.rs + debug.rs
- **MEDIUM**: No cross-backend element-manipulation translation guide — new IR features risk OmniView-only implementation
- **LOW**: Inconsistent element access in highlight functions (bypass cache), 3 caching strategies without documented rationale

**Strengths noted**: Single clean `__omni_el()` definition, scoped `_elCache` for @for, well-documented 6-phase JS execution, proactive ADR-028 trade-off documentation, consistent error handling around element access.

### Pragmatic Enforcer

> Full text in agent output at `ae3b2ec41d59346b3`.

**Perspective**: Applied YAGNI analysis — is the architecture appropriately scoped for its debug-runtime role? Is adding `useRef` warranted?

**Key findings**:
- **Necessity of useRef**: 0/10 — no consumer needs it, all tests pass, querySelectorAll is simpler
- **Complexity of useRef**: 6/10 — would require IR changes, ref-forwarding, import changes
- **Cost-benefit ratio**: ~infinity (zero benefit, positive cost)
- OmniView runtime is NOT over-engineered — ~220 lines of essential JS
- React motion two-effect split is correct per ADR-024 — do not change
- @for `_elCache` Map is correct and necessary — do not remove

**Recommendation**: Do NOT add `useRef`. Document the mount-time DOM constraint. Defer global registry lifecycle sweep to S-sized maintenance.

### Rust Expert (Katya Volkov)

> Full text in agent output at `ae2a0f6e6e853ae58`.

**Perspective**: Evaluated Rust-side code generation quality, nid identifier system soundness, allocation patterns, and zero-cost abstraction opportunities.

**Key findings**:
- **HIGH**: `try_inline_component` clones `el.children` before checking `has_any_slot_attr` — O(n) wasted allocation for every component inlining without slot children
- **MEDIUM**: `inject_slot_children` clones full view tree even without slot replacement — use `Cow<ViewTree>` instead
- **MEDIUM**: 156 instances of `let _ = writeln!()` discard format errors — add `write_js!` helper macro
- **LOW**: `view_to_html` returns owned Strings → intermediate allocations in tree traversal
- **INFO**: `FxHashMap::default()` over `with_capacity()` for known-size maps

**Strengths noted**: ComponentScope take/restore pattern (ADR-034) is a textbook improvement, 6-phase execution via buffer swap, nid system is simple and correct, conditional motion runtime inclusion, shared scroll handler, binding infinite-loop guard, phase guard assertions with `#[track_caller]`.

---

## Risks If Unaddressed

### Immediate risks (HIGH findings)

1. **Broken @for + motion combinations** (H1): Any OmniView user combining `@for` with `@enter`/`@hover`/`@press`/`@scroll`/`@drag` sees only the first list item animated. This is a visible, user-facing bug with no workaround.

2. **Silent FLIP non-functionality** (H2): Developers adding keyed `@for` lists expecting reorder animations see nothing. The `Omni.flipBefore`/`flipAfter` calls execute but produce no visual effect. Debugging time wasted on "why isn't FLIP working?"

3. **Wasteful compilation for large components** (H3): Components with 20+ nested elements and many custom components pay O(n) cloning cost per inlining. Acceptable now; grows with component complexity.

### Medium-term risks (IMPORTANT findings)

4. **React silently drops animations for dynamic content** (I1): `.omni` source with conditionally rendered `@enter` elements works in OmniView (debug) but silently fails in React (production). Developer experience gap.

5. **@for list performance cliff at N > 500** (I2): Without `_elCache` reuse for recycled items, list re-renders degrade linearly. Acceptable for debug runtime; becomes visible lag in editor preview.

6. **Future ADR-021-class bugs** (I4): Without a cross-backend pattern mapping guide, new element-level IR features will be implemented OmniView-only, requiring reactive parity fixes.

---

## Success Metrics

| Metric | Current | Target | Timeline |
|--------|---------|--------|----------|
| @for + motion correctness | BROKEN (only 1st item) | All items get motion | This sprint |
| FLIP animation | Non-functional | Works for keyed lists | This sprint |
| try_inline_component clone waste | Always clones | Conditional (slot-aware) | This sprint |
| React dynamic motion discovery | Not supported | MutationObserver fallback | Next sprint |
| _elCache reuse for recycled items | Rebuilt every render | Persisted on element | Next sprint |
| Codegen duplication (triplicated patterns) | 3 sites × 2 patterns | 1 shared helper each | Next sprint |
| Cross-backend pattern docs | None | Pattern mapping table | Next sprint |

---

## Follow-Up Plan

1. **Create ADR** for the @for + motion binding fix (H1) — documents the architectural decision to add `for_motion_bindings` to ComponentScope
2. **Create ADR** for the React motion discovery approach (I1) — documents the decision to use `MutationObserver` over `useRef`, with criteria for future re-evaluation
3. **Sprint plan**: Fix H1-H3 immediately (estimated 3-4 hours total), then I1-I6 next sprint
4. **Defer N1-N8** to maintenance backlog — document in `.architecture/deferrals.md`
