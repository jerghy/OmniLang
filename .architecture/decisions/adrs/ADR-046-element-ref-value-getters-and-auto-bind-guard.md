# ADR-046: Element Ref Wrapper Value/TextContent Getters and Auto-Bind Guard

## Status

Implemented

## Context

While running E2E browser tests on the learn-omniview showcase (8 tabs, 60+ source files), two codegen defects were discovered in the OmniView debug backend's JavaScript code generation:

### Defect 1: Ref Wrapper Missing `.value` and `.textContent` Getters

The `ElementRefDemo.omni` component demonstrates ref-based DOM operations including `mirrorInput.value` for reading an `<input>` element's value. The codegen's ref proxy wrapper (in `store.rs`) exposed a curated set of DOM properties and methods (`style`, `focus`, `blur`, `scrollTo`, `scrollBy`, `offsetWidth`, `offsetHeight`, `getBoundingClientRect`, `capturePointer`, `releasePointer`) but omitted `.value` and `.textContent` — two of the most commonly accessed properties on interactive DOM elements.

When `syncMirror()` called `mirrorInput.value`, the ref wrapper returned `undefined` because no `value` getter existed on the proxy object. The mirror text display remained blank despite the input containing typed content.

### Defect 2: Auto `bind:value` Collision with `@onInput` Event Handlers

The IR lowering pass in `view.rs` contains an auto-bind feature (lines 523-541) that automatically injects `bind:value` on `<input>` elements by assigning string-typed signals in declaration order. This convenience feature is intended to reduce boilerplate.

However, the auto-bind logic did not check whether the `<input>` element already had an `@onInput` event handler. When `@onInput={syncMirror}` was present (an explicit developer choice to handle input events manually), the auto-bind still fired and bound the input to the next unrelated string signal in declaration order (`scrollPos` instead of the intended `mirrorText`).

This caused two symptoms:
1. The `scrollPos` signal was overwritten with the input's text content on every keystroke
2. A reactive effect set the input's `.value` from `scrollPos`, creating an incorrect two-way binding cycle

## Decision Drivers

* Ref API completeness — developers need `.value` and `.textContent` on ref wrappers for basic DOM read/write operations
* Event handler autonomy — `@onInput` is an explicit developer declaration and must not be silently overridden by auto-binding logic
* Backward compatibility — existing `bind:value` patterns must continue to work unchanged
* Minimal blast radius — both fixes are surgical, touching only the ref wrapper template and the auto-bind guard condition

## Decision

### 1. Add `.value` and `.textContent` getters/setters to the Ref Wrapper

The ref proxy wrapper in `omnilang/src/codegen/omniview/store.rs` gains four new properties:

```javascript
get value() { const e = _el(); return e ? e.value : null; },
set value(v) { const e = _el(); if (e) e.value = v; },
get textContent() { const e = _el(); return e ? e.textContent : null; },
set textContent(v) { const e = _el(); if (e) e.textContent = v; },
```

This enables direct DOM read/write patterns like `let val = refName.value` and `refName.textContent = "new text"`.

### 2. Guard Auto-Bind Against `@onInput` Event Handlers

The auto-bind condition in `crates/omnilang-core/src/ir/lower/view.rs` adds a new check: skip auto-binding if the element already has an explicit `@onInput` or `@input` event handler.

```rust
let has_explicit_oninput = events.as_ref().is_some_and(|e| {
    e.keys().any(|k| {
        let lower = k.to_lowercase();
        lower == "oninput" || lower == "input"
    })
});
```

**Architectural Components Affected:**
* `omnilang/src/codegen/omniview/store.rs` — Ref wrapper template
* `crates/omnilang-core/src/ir/lower/view.rs` — Auto-bind guard condition

**Interface Changes:**
* Ref proxy objects now expose `value` (get+set) and `textContent` (get+set)
* No public API changes — these are generated JS output additions

## Consequences

### Positive

* Ref API is more complete — `.value` and `.textContent` are the two most commonly needed DOM properties and are now first-class
* `@onInput` event handlers are no longer silently overridden by auto-bind — developer intent is respected
* Existing `bind:value` patterns are completely unaffected (only new guard condition added)
* Fixes are each 4-8 lines, zero new abstractions, zero new dependencies

### Negative

* Ref wrapper grows slightly (from 10 to 14 properties/methods), though this is imperceptible in the generated JS
* Auto-bind no longer fires on inputs with `@onInput`, so if a developer used `@onInput` expecting auto-bind to also fire, they must add explicit `bind:value` — this is the correct contract

### Neutral

* Other getter candidates (`.innerHTML`, `.className`, `.dataset`) remain absent from the wrapper; they can be added ad-hoc as needed
* Fixes are backend-specific (OmniView debug backend only); React backend uses `useRef` natively

## Implementation Strategy

### Blast Radius

**Impact Scope**: Low — two isolated, single-function changes in the codegen and IR lowering pipeline.

**Affected Components**:
- `omnilang/src/codegen/omniview/store.rs` — Only the ref wrapper template; other codegen paths (store, props, events) unchanged
- `crates/omnilang-core/src/ir/lower/view.rs` — Only the auto-bind condition; rest of lowering pipeline unchanged

**User Impact**: Positive — mirror input demo now works, scroll position no longer contaminated by unrelated input

**Risk Mitigation**:
- Both fixes verified by E2E browser tests after rebuild
- All 8 tabs of learn-omniview tested for regressions

### Reversibility

**Reversibility Level**: High — both are additive changes with no structural impact.

**Rollback Feasibility**: Reverting the commits restores previous behavior in seconds. No data migration or schema changes.

### Sequencing & Timing

**Prerequisites**: None.

**System Readiness**: Already validated — both fixes built and tested in the same session.

**Readiness Assessment**: Ready to implement (already deployed and verified).

### Social Cost

**Learning Curve**: None — `.value` and `.textContent` are universal DOM concepts. The auto-bind guard simply makes `@onInput` behave as developers expect.

**Cognitive Load**: Zero net change — removing the auto-bind collision reduces confusion.

**Documentation Needs**: None — the fix aligns behavior with existing documentation.

### Confidence Assessment

**Model Correctness Confidence**: High — both fixes address direct contradictions between source code intent and generated behavior:
1. Source says `mirrorInput.value` → generated code made ref.value return `undefined` (clear bug)
2. Source says `@onInput={syncMirror}` without `bind:value` → generated code injected `bind:value` to unrelated signal (clear bug)

**Validation Approach**: E2E browser tests confirmed:
- Mirror input: typing "Fix confirmed" → mirror text displays "Fix confirmed" (was blank before)
- Scroll position: stays "top" after mirror input interaction (was contaminated before)
- All other interaction patterns verified working after both fixes

## Implementation

**Phase 1: Ref Wrapper Value/TextContent Getters**
- Edit `store.rs` to add `get value()`, `set value(v)`, `get textContent()`, `set textContent(v)` to the ref wrapper template
- Rebuild omnic release binary
- Regenerate learn-omniview build
- Verify: `mirrorInput.value` now returns the DOM input's `value`

**Phase 2: Auto-Bind Guard**
- Edit `view.rs` to add `has_explicit_oninput` check before auto-bind
- Rebuild omnic release binary
- Regenerate learn-omniview build
- Verify: scroll position no longer polluted by mirror input keystrokes

## Alternatives Considered

### Alternative 1: Use `Proxy` object for ref wrappers

Use JavaScript `Proxy` to forward all property accesses to the underlying DOM element, eliminating the need to maintain a curated getter list.

**Pros:**
* Zero-maintenance — any DOM property automatically available
* Future-proof — no need to add getters for new use cases

**Cons:**
* Performance overhead of Proxy traps on every property access (especially relevant for 60fps drag operations)
* Less explicit — harder to audit what's exposed
* Cannot type-check the exposed API
* Proxy behavior differs from direct property access in edge cases (e.g., `delete`, `in` operator)

**Decision**: Rejected — the curated wrapper approach is more explicit, debuggable, and performant. Adding getters as needed is YAGNI-compliant.

### Alternative 2: Skip auto-bind entirely; require explicit `bind:value` on all inputs

Remove the auto-bind convenience feature and require developers to always write `bind:value={signal}` explicitly.

**Pros:**
* Eliminates the collision class entirely
* More explicit — no "magic" binding

**Cons:**
* Breaking change for all existing components relying on auto-bind
* Reduces developer convenience — every `<input>` needs explicit `bind:value`
* Would require updating all test fixtures and showcase components

**Decision**: Rejected — the auto-bind convenience is valuable. The guard condition approach preserves convenience while fixing the collision.

### Alternative 3: Use `bind:value` priority — explicit binding always wins

Keep auto-bind but give explicit `bind:value` priority. This doesn't directly address the issue since the problem is that auto-bind fires when *no* `bind:value` exists but `@onInput` does.

**Decision**: Not applicable — the specific collision involves `@onInput`, not `bind:value`.

### Alternative 4: Auto-bind to the signal used in the `@onInput` handler

Instead of using declaration-order signals, analyze the `@onInput` handler body to find which signal it sets, and auto-bind to that.

**Pros:**
* More intelligent binding
* Reduces boilerplate further

**Cons:**
* Complex static analysis — requires walking the handler body for signal writes
* Fragile — renaming a signal in the handler would break the binding
* May bind to wrong signal if handler writes to multiple signals
* Over-engineered for the convenience use case

**Decision**: Rejected — violates YAGNI. If the developer writes `@onInput={fn}`, they have consciously chosen manual handling.

## Pragmatic Enforcer Analysis

**Reviewer**: Pragmatic Enforcer
**Mode**: Balanced

**Overall Decision Complexity Assessment**: Low — both fixes are 4-8 line changes with zero new abstractions, zero new dependencies, and no refactoring. They directly fix observed incorrect behavior.

**Decision Challenge**: No challenge. Both fixes reduce bugs and align generated code with source code intent.

**Necessity Assessment**: 9/10
- **Current need**: Bug was reproducible in E2E tests — mirror input didn't work, scroll position was silently corrupted
- **Future need**: `.value` and `.textContent` are fundamental DOM properties; their absence from ref wrappers would continue causing failures for any component using ref-based input access
- **Cost of waiting**: Every test/component using `ref.value` would silently break; auto-bind collisions are hard to diagnose
- **Evidence of need**: Two concrete, testable failures during routine E2E testing

**Complexity Assessment**: 1/10
- **Added complexity**: Four trivial getter methods (each 1 line) + one boolean check on event keys
- **Maintenance burden**: Near-zero — getters follow the existing wrapper pattern exactly
- **Learning curve**: None
- **Dependencies introduced**: None

**Alternative Analysis**: Alternatives (Proxy, remove auto-bind, static analysis) were considered but correctly rejected as over-engineering for the actual problem scope.

**Simpler Alternative Proposal**: No simpler alternative exists — the fixes are already the minimal possible change.

**Recommendation**: ✅ Approve decision

**Pragmatic Score**:
- **Necessity**: 9/10
- **Complexity**: 1/10
- **Ratio**: 0.11 *(Target: <1.5 for balanced mode)*

**Overall Assessment**: Appropriate engineering — both fixes are surgical, minimal, and directly address observed failures. Zero over-engineering concern.

## Validation

**Acceptance Criteria:**
- [x] `mirrorInput.value` returns the DOM element's value (verified via browser E2E: "Mirror: Fix confirmed")
- [x] `mirrorInput.textContent` is readable/writable (verified through generated code inspection)
- [x] Scroll position no longer changes when typing in mirror input (verified: stays "top")
- [x] All 8 tabs of learn-omniview render without new errors
- [x] Existing `bind:value` patterns unaffected (verified: ValidationInput, FormPage bindings still work)
- [x] Focus/blur/scrollTo/scrollBy/getBoundingClientRect/offsetWidth/offsetHeight unchanged (verified)

**Testing Approach:**
* E2E browser tests via Playwright MCP covering all 8 tabs
* Manual verification of the specific failing interactions (mirror input, scroll position)
* Generated JS code inspection to confirm wrapper structure

## References

* [ADR-038: Refined Element Reference System](ADR-038-refined-element-reference-system.md) — Previous element ref decisions (cache correction, @for diagnostics, scrollTo/scrollBy, HTMLElement fallback)
* [ADR-042: Ref Binding Syntax](ADR-042-ref-binding-syntax-and-inlined-wrappers.md) — Ref resolution and wrapper emission decisions
* `omnilang/src/codegen/omniview/store.rs` — Ref wrapper template (fix location)
* `crates/omnilang-core/src/ir/lower/view.rs` — Auto-bind guard (fix location)
