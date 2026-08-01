# ADR-038: Refined Element Reference System Architecture (v2 Review)

## Status

Superseded by [ADR-052](ADR-052-semantic-level-dsl-cross-target-codegen.md) / [ADR-053](ADR-053-semantic-level-dsl-syntax-formalization-and-implementation.md)（2026-08-01，见 [ADR-054](ADR-054-omniview-semantic-dsl-era-design-philosophy.md)）

> v2 细化的缓存修正/@for 诊断等精神保留，但 ref 的语义形态被 ADR-052 全面重定义为语义 ops；Phase 2 内容不再适用。原文保留作历史参考。

## Context

The @ref element reference system (ADR-036) was introduced as the primary mechanism for high-frequency DOM manipulation in OmniView — analogous to React's `useRef` for bypassing `setState` reconciliation at 60fps. Phase 1 implemented the core IR, semantic validation, and codegen for both OmniView and React backends.

The v2 architecture review (`.architecture/reviews/component-omniview-high-freq-dom-v2.md`, 2026-07-30) — a follow-up after all P0/P1 fixes from the v1 review — identified 1 new CRITICAL bug and several architectural refinement decisions. This ADR documents those refined decisions, superseding the Phase 1 design decisions from ADR-036 where they evolved.

## Decision Drivers

* **Correctness**: The `__omni_el` cache returned stale detached DOM elements indefinitely — both a correctness bug (manipulating dead elements) and a memory leak
* **Developer experience**: @ref inside @for was the #1 gap identified by 5/6 specialists — the compiler silently produced incorrect output for the most natural use case
* **Cross-backend consistency**: `scrollTo`/`scrollBy` worked on React (raw DOM via `useRef.current`) but crashed on OmniView debug runtime
* **Type safety**: Hardcoded `HTMLDivElement` for all `useRef` types produced TypeScript errors for non-div elements
* **Code hygiene**: Dead code ($\lnot$DRY violations, dual formatting paths, `debug_assert!` vs `assert!`) created maintenance drag

## Decision

### D1: @ref Restricted API Surface — scrollTo/scrollBy Added (refines ADR-036 D3)

The OmniView ref IIFE wrapper now exposes `scrollTo(x, y)` and `scrollBy(dx, dy)` in addition to the original API surface (`.style`, `.getBoundingClientRect()`, `.capturePointer()`, `.releasePointer()`, `.focus()`, `.blur()`, `.offsetWidth`, `.offsetHeight`).

**Why**: ADR-036 documented these methods but the implementation was missing — a cross-backend inconsistency where React worked but OmniView threw `TypeError`. Adding 4 lines of template code closes the gap.

**Still excluded**: `innerHTML`, `addEventListener`, `querySelector`, `parentElement` — XSS and event-leak prevention remains intact.

### D2: @ref Inside @for — Phase 2 Deferral with Compiler Warning (new)

`@ref` bindings inside `@for` loop bodies are explicitly unsupported in Phase 1. The compiler now emits a Warning diagnostic when this pattern is detected:

```
ref 'name' is used inside @for — each @ref inside a loop binds to the
template element, not individual items. @ref inside @for is not yet
supported (planned for Phase 2).
```

**Why**: The compiler previously accepted this silently and produced incorrect output — the ref bound to the template element (OmniView) or the last clone (React). Five of six architecture review specialists independently identified this as the highest-impact gap. The warning prevents users from encountering undefined behavior without blocking compilation.

**Phase 2 options** (not yet decided): ref callbacks per item, `Map<key, Element>` keyed by index, or `@ref` on the `@for` container itself.

### D3: `useRef` Type Strategy — HTMLElement Fallback (new)

React codegen now uses a tag→TypeScript-type mapping for `useRef` declarations:

| OmniView tag | Generated TypeScript type |
|-------------|--------------------------|
| `input`, `textarea` | `HTMLInputElement` |
| `select` | `HTMLSelectElement` |
| `video` | `HTMLVideoElement` |
| `canvas` | `HTMLCanvasElement` |
| `img` | `HTMLImageElement` |
| `svg` | `SVGSVGElement` |
| `div`, `section`, `article`, `nav`, etc. | `HTMLDivElement` |
| Unknown / missing | `HTMLElement` |

**Why**: The previous hardcoded `HTMLDivElement` caused TypeScript compilation errors for non-div elements (`Property 'value' does not exist on type 'HTMLDivElement'`). `HTMLElement` is the correct generic fallback — it's the base type for all HTML elements and avoids false type errors. Full per-element tag tracking (populating `element_tag` from the IR element's tag) is deferred to a future refinement.

### D4: `__omni_el` Cache Correctness — Evict on Miss (new)

The `__omni_el` runtime function now evicts stale cache entries and returns `null` when the element is not found:

```javascript
function __omni_el(nid) {
  var cache = __omni_el._cache || (__omni_el._cache = {});
  var el = cache[nid];
  if (!el || !el.isConnected) {
    el = document.querySelector('[data-nid="' + nid + '"]');
    if (el) {
      cache[nid] = el;
    } else {
      delete cache[nid];  // ← NEW: evict stale entries
    }
  }
  return el || null;  // ← NEW: consistent null return
}
```

**Why**: Without eviction, the cache held references to detached DOM nodes indefinitely — both a correctness issue (all subsequent calls returned dead elements) and a memory leak (detached nodes could not be garbage collected). This affected ALL code paths using `__omni_el`: @ref wrappers, style effects, text bindings, attribute bindings.

### D5: Dual Formatting Path Unification (new)

The React expression formatting had two independent paths for ref name translation:
- `format_expr_ir_tsx` (free function) — `IdentifierExpr` branch with special-case ref checking
- `ReactFormatExpr.format_identifier` (trait method) — general identifier formatting with `$` prefix stripping

**Decision**: `format_expr_ir_tsx` now delegates `IdentifierExpr` entirely to `ReactFormatExpr.format_identifier()`, eliminating the special case. This fixes a latent inconsistency where the free function did NOT strip `$` prefix before checking `ref_names`, while the trait method always did.

### D6: Defense-in-Depth Hardening (new)

Two defensive improvements:
- `push_ref_nid`: `debug_assert!` → `assert!` — the semantic validator emits a Warning (not Error) for duplicate bindings; release builds need the same protection to prevent silently broken JS
- `emit_ref_wrappers`: Added `css_only` guard — other JS-emitting methods check this; the omission was a latent leak path if a component with `@ref` goes through the style-only pipeline

**Architectural Components Affected:**
* `crates/omnilang-core/src/ir/types.rs` — `RefDeclIR.element_tag: Option<String>` added
* `crates/omnilang-core/src/semantic/validate_refs.rs` — `warn_refs_in_for_bodies()` pass added
* `omnilang/src/codegen/omniview/runtime.rs` — `__omni_el` cache eviction
* `omnilang/src/codegen/omniview/store.rs` — scrollTo/scrollBy, css_only guard, JS comment fix
* `omnilang/src/codegen/omniview/mod.rs` — assert! upgrade, JS comment fix
* `omnilang/src/codegen/react/expr.rs` — dead field removal, formatting path unification
* `omnilang/src/codegen/react/component/state.rs` — `ref_element_type()` mapping

**Interface Changes:**
* `RefDeclIR` gains `element_tag: Option<String>` (backward compatible via `#[serde(default)]`)
* `useRef<HTMLDivElement>` → tag-aware type in generated React code
* `__omni_el()` now returns `null` instead of `undefined` for missing elements

## Consequences

### Positive

* **Correctness**: `__omni_el` no longer returns stale detached elements; memory leak fixed
* **Developer safety**: @ref inside @for now produces a clear compiler warning instead of silent wrong behavior
* **Cross-backend consistency**: scrollTo/scrollBy works on both OmniView and React
* **Type safety**: `useRef` types no longer produce false TypeScript errors for non-div elements
* **Maintainability**: Dead code removed, dual formatting paths unified, `assert!` hardened
* **Zero regressions**: All workspace tests pass (837+ tests)

### Negative

* `RefDeclIR` now has a nullable `element_tag` field that is `None` for most refs (populated only when lowering to IR can correlate ref declarations with element tags — deferred)
* `HTMLElement` fallback loses some type precision compared to `HTMLDivElement` for `<div>` elements (but gains correctness for all others)
* @ref inside @for is still unsupported — the warning prevents errors but doesn't solve the use case
* 6 deferred items remain from the v2 review (see Deferred Items)

### Neutral

* `element_tag` is `#[serde(default)]` — backward compatible for all serialization paths
* `__omni_el` now returns `null` instead of `undefined` for missing elements — callers using truthiness checks are unaffected; callers using `=== undefined` may need updates (none found in codebase)
* JS comment change (`///` → `//`) is cosmetic; generated output is functionally identical

## Implementation Strategy

### Blast Radius

**Impact Scope**: Medium — affects all code paths using `__omni_el` and all React codegen output with `@ref`

**Affected Components**:
- `omnilang/src/codegen/omniview/runtime.rs` — runtime behavior change (cache eviction + null return)
- `omnilang/src/codegen/react/component/state.rs` — generated TypeScript type change
- `crates/omnilang-core/src/semantic/validate_refs.rs` — new warning diagnostic
- `omnilang/src/codegen/omniview/store.rs` — ref wrapper API expansion

**User Impact**: Positive — fixes bugs, adds warnings, improves types. No breaking changes to the .omni language surface. Generated React code may have type changes (HTMLDivElement → HTMLElement for non-div refs).

**Risk Mitigation**:
- Full workspace test suite (837+ tests) passes with zero regressions
- React ref integration tests updated to match new HTMLElement fallback
- Semantic validation tests include new @for+@ref warning test

### Reversibility

**Reversibility Level**: High

**Rollback Feasibility**:
- All changes are additive or bugfixes — no irreversible architectural commitments
- `element_tag` is `#[serde(default)]` — can be removed without breaking serialization
- `__omni_el` behavior change is strictly more correct — rollback would reintroduce the bug
- Each decision is independently reversible

**Options Preserved**:
- Full per-element tag tracking (populating `element_tag` from ElementIR) is still possible
- @for+@ref semantic options remain open (callback refs, Map, container ref)
- Component ref forwarding (forwardRef) remains a Phase 2 option
- Signal batching remains gated on profiling data (ADR-037)

**Commitments Made**:
- @ref API surface now includes scrollTo/scrollBy — removal would be a breaking change
- `HTMLElement` is the fallback type — changing to a different fallback could break consumer TypeScript builds
- @for+@ref warning is a permanent diagnostic — removing it would re-expose users to silent bugs

## Alternatives Considered

### Keep `HTMLDivElement` as `useRef` type (rejected)

**Pros**: No code change needed; `<div>` refs have precise types
**Cons**: Non-div refs produce TypeScript errors; users must type-cast generated code; inconsistent with HTML element hierarchy

**Why rejected**: Type correctness for all elements outweighs minor precision loss for `<div>` elements. `HTMLElement` is the correct base type in the HTML DOM hierarchy.

### Make `@ref` inside `@for` a hard Error instead of Warning (rejected)

**Pros**: Stronger guard against undefined behavior; can't be ignored
**Cons**: Blocks compilation for code that might work accidentally (single-item lists, static lists); Warning allows developers to proceed consciously

**Why rejected**: Warning severity is appropriate — the compiler shouldn't block compilation for a pattern that works in simple cases (single-item @for). The warning communicates risk without preventing iteration.

### Keep `debug_assert!` in `push_ref_nid` (rejected)

**Pros**: Zero runtime cost in release builds
**Cons**: Semantic validator emits Warning (not Error) for duplicates; release builds would silently produce broken JS

**Why rejected**: Defense-in-depth requires the invariant in all build modes. The assert cost is negligible (one HashMap lookup per ref binding, typically 0-3 per component).

## Pragmatic Enforcer Analysis

**Reviewer**: Pragmatic Enforcer (v2 Architecture Review)
**Mode**: Balanced

**Overall Decision Complexity Assessment**: These are refinement decisions on an existing system, not new architectural commitments. Total complexity added is minimal (~10 lines of production code for scrollTo, ~20 lines for @for warning, ~15 lines for type mapping, 2 lines for cache fix). The ratio is well below threshold.

**Decision Challenge**:

**Proposed Decision**: "Refine the @ref system with cache correctness, @for diagnostic, API completion, and type safety"

**Necessity Assessment**: 7/10
- **Current need**: Cache bug is a real correctness+memory issue that affects all @ref users. @for gap is the #1 developer confusion scenario. scrollTo gap is a documented-but-broken API.
- **Future need**: Type safety will become critical as React production deployment approaches.
- **Cost of waiting**: Cache bug causes silent wrong behavior in any SPA with dynamic content. @for gap will generate bug reports. Low cost items (scrollTo: 4 lines, @for warning: 20 lines) with disproportionate impact.
- **Evidence of need**: Cache bug confirmed by Performance Specialist in v2 review (new finding). @for gap identified by 5/6 specialists independently. scrollTo gap verified by Compiler Architect.

**Complexity Assessment**: 3/10
- **Added complexity**: ~50 lines of production code across 7 files. No new abstractions, no new dependencies, no new IR concepts.
- **Maintenance burden**: Minimal — all changes are straightforward, well-tested, and localized.
- **Learning curve**: None — these are bugfixes and refinements, not new features.
- **Dependencies introduced**: None.

**Alternative Analysis**:
- All alternatives considered were either rejected with clear rationale or deferred to Phase 2
- The "do nothing" option was explicitly rejected for each decision
- Simpler alternatives (keep HTMLDivElement, make @for refs an Error) were considered and rejected with documented reasons

**Recommendation**: ✅ Approve decision

**Pragmatic Score**:
- **Necessity**: 7/10
- **Complexity**: 3/10
- **Ratio**: 0.43 *(Target: <1.5 for balanced mode)*

**Overall Assessment**: These are well-scoped refinements to an existing system. The complexity-to-necessity ratio (0.43) is one of the lowest of any @ref-related ADR. Each decision addresses a concrete, verified problem with a proportionate fix. No over-engineering detected.

## Validation

**Acceptance Criteria:**
- [x] All workspace tests pass (837+ tests, zero regressions)
- [x] `__omni_el` returns `null` for removed elements (not stale reference)
- [x] Compiler emits Warning for `@ref` inside `@for`
- [x] `box.scrollTo(x, y)` works in OmniView debug runtime
- [x] `useRef<HTMLInputElement>` generated for `<input @ref={inputRef}>`
- [x] `useRef<HTMLElement>` generated for unknown element tags
- [x] No `#[allow(dead_code)]` on `ReactStmtFormatter`
- [x] `assert!` (not `debug_assert!`) in `push_ref_nid`

**Testing Approach:**
* Unit tests: 7 semantic validation tests (6 original + 1 new @for warning test)
* Integration tests: 6 ref codegen tests (3 OmniView + 3 React) updated
* Regression: Full workspace test suite (837+ tests) — zero failures

## References

* [ADR-036: Element Reference System](ADR-036-element-reference-system.md) — original @ref system design
* [ADR-037: Signal Batching Deferral](ADR-037-defer-signal-batching-gate-on-profiling.md) — related performance decision
* [ADR-028: OmniView Debug Runtime Trade-offs](ADR-028-omniview-debug-runtime-tradeoffs.md) — D3: synchronous subscriber notification
* [Architecture Review v2](.architecture/reviews/component-omniview-high-freq-dom-v2.md) — 6-specialist review triggering these decisions
* [Architecture Review v1](.architecture/reviews/component-omniview-high-freq-dom.md) — prior review (all P0/P1 findings resolved)
* [Implementation Plan](docs/superpowers/plans/2026-07-30-high-freq-dom-fixes-v2.md) — 9-task plan executed for these decisions
